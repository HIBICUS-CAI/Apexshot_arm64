use super::super::{
    gst_audio, notify_daemon_event, RecordError, RecordResult, RecordingConfig,
    RecordingControlCommand, RecordingTerminalAction,
};
use super::profile::EncoderProfile;
use super::session::RecordingAudioExclusiveGuard;
use gst::prelude::*;
use gstreamer as gst;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

/// Best-effort audio for the X11 path: the shared GStreamer audio bin linked
/// into the video muxer. Audio is a new capability here — any setup failure
/// falls back to the historical video-only recording instead of failing.
fn build_x11_audio_bin(
    config: &RecordingConfig,
    profile: &EncoderProfile,
) -> Option<gst_audio::GstAudioBin> {
    use gst_audio::{
        audio_available, build_audio_bin, encoder_for_muxer, AudioTermination, GstAudioSetup,
    };

    let setup = GstAudioSetup::from_recording(config)?;
    if !audio_available(
        profile.muxer,
        AudioTermination::GhostPad,
        setup.noise_suppression,
    ) {
        eprintln!("[recording] X11 audio unavailable: GStreamer audio elements missing");
        return None;
    }
    let encoder = encoder_for_muxer(profile.muxer)?;
    match build_audio_bin(&setup, encoder, AudioTermination::GhostPad) {
        Ok(bin) => {
            println!("Recording audio via GStreamer (X11, first-class audio track)");
            Some(bin)
        }
        Err(err) => {
            eprintln!("[recording] X11 audio skipped: {err}");
            None
        }
    }
}

/// X11 fallback recording using GStreamer ximagesrc.
/// Preserved from the previous implementation for backward compatibility,
/// now with optional audio from the shared GStreamer audio bin.
#[allow(unused_assignments)]
pub(super) async fn record_x11_with_gstreamer(
    config: &RecordingConfig,
    profile: &EncoderProfile,
    final_path: &Path,
    command_rx: Option<mpsc::UnboundedReceiver<RecordingControlCommand>>,
) -> RecordResult<(PathBuf, RecordingTerminalAction)> {
    let _audio_exclusive =
        RecordingAudioExclusiveGuard::acquire(config.mic_enabled || config.speaker_enabled);
    gst::init().map_err(|e| RecordError::InitError(e.to_string()))?;

    let audio_bin = build_x11_audio_bin(config, profile);
    let pipeline_str =
        build_x11_gstreamer_pipeline(config, profile, final_path, audio_bin.is_some())?;
    println!("Starting recording (GStreamer X11) to: {:?}", final_path);
    println!("Pipeline: {}", pipeline_str);

    let pipeline = gst::parse::launch(&pipeline_str)
        .map_err(|e| RecordError::GStreamerError(format!("Failed to parse pipeline: {}", e)))?
        .downcast::<gst::Pipeline>()
        .map_err(|_| RecordError::GStreamerError("Cast to Pipeline failed".into()))?;

    if let Some(audio) = audio_bin {
        let muxer = pipeline
            .by_name("mux")
            .ok_or_else(|| RecordError::GStreamerError("named muxer not found".into()))?;
        pipeline
            .add(&audio.bin)
            .map_err(|e| RecordError::GStreamerError(format!("Failed to add audio bin: {e}")))?;
        let audio_pad = muxer
            .request_pad_simple("audio_%u")
            .ok_or_else(|| RecordError::GStreamerError("no audio pad on muxer".into()))?;
        let ghost = audio
            .bin
            .static_pad("src")
            .ok_or_else(|| RecordError::GStreamerError("audio bin has no ghost pad".into()))?;
        ghost.link(&audio_pad).map_err(|e| {
            RecordError::GStreamerError(format!("Failed to link audio into muxer: {e:?}"))
        })?;
    }

    pipeline
        .set_state(gst::State::Playing)
        .map_err(|e| RecordError::GStreamerError(format!("Failed to start pipeline: {}", e)))?;

    let bus = pipeline
        .bus()
        .ok_or_else(|| RecordError::GStreamerError("Pipeline has no bus".into()))?;

    let mut command_rx = command_rx;
    let mut stop_action = RecordingTerminalAction::Save;
    let mut stopping = false;
    let mut paused = false;

    loop {
        tokio::select! {
            command = async {
                match &mut command_rx {
                    Some(rx) => rx.recv().await,
                    None => futures_util::future::pending::<Option<RecordingControlCommand>>().await,
                }
            } => {
                let Some(command) = command else {
                    command_rx = None;
                    continue;
                };
                match command {
                    RecordingControlCommand::Restart => {
                        stop_action = RecordingTerminalAction::Restart;
                        pipeline.send_event(gst::event::Eos::new());
                        stopping = true;
                        break;
                    }
                    RecordingControlCommand::StopSave => {
                        stop_action = RecordingTerminalAction::Save;
                        pipeline.send_event(gst::event::Eos::new());
                        stopping = true;
                        break;
                    }
                    RecordingControlCommand::StopDiscard => {
                        stop_action = RecordingTerminalAction::Discard;
                        pipeline.send_event(gst::event::Eos::new());
                        stopping = true;
                        break;
                    }
                    RecordingControlCommand::Pause if !paused => {
                        pipeline
                            .set_state(gst::State::Paused)
                            .map_err(|e| RecordError::GStreamerError(format!("Failed to pause pipeline: {e}")))?;
                        paused = true;
                        notify_daemon_event("recording_session_paused");
                    }
                    RecordingControlCommand::Resume if paused => {
                        pipeline
                            .set_state(gst::State::Playing)
                            .map_err(|e| RecordError::GStreamerError(format!("Failed to resume pipeline: {e}")))?;
                        paused = false;
                        notify_daemon_event("recording_session_resumed");
                    }
                    _ => {}
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                for msg in bus.iter_timed(gst::ClockTime::ZERO) {
                    use gst::MessageView;
                    match msg.view() {
                        MessageView::Eos(..) => { stopping = true; break; }
                        MessageView::Error(err) => {
                            let _ = pipeline.set_state(gst::State::Null);
                            return Err(RecordError::GStreamerError(err.error().to_string()));
                        }
                        _ => (),
                    }
                }
                if stopping { break; }
            }
        }
    }

    if paused {
        let _ = pipeline.set_state(gst::State::Playing);
    }
    pipeline
        .set_state(gst::State::Null)
        .map_err(|e| RecordError::GStreamerError(format!("Cleanup failed: {}", e)))?;

    if stop_action == RecordingTerminalAction::Discard {
        let _ = std::fs::remove_file(final_path);
    }

    Ok((final_path.to_path_buf(), stop_action))
}

/// Build a GStreamer pipeline string for X11 capture (preserved from old code).
/// The muxer is named when audio is attached so the audio bin can request a pad.
///
/// `config.max_resolution` becomes a `videoscale` caps range after `videorate`,
/// mirroring the Wayland backend's `wayland_video_filter` ceiling: the range
/// never upscales (a source already inside the box negotiates unchanged) and
/// the `pixel-aspect-ratio=1/1` pin keeps pixels square, so the fit is
/// aspect-preserving (e.g. 1920x1200 → 768x480 under the 854x480 box) instead
/// of filling the box and relying on a non-square PAR. The scale segment must
/// not sit adjacent to the framerate caps: gst-parse rejects two consecutive
/// caps filters.
fn build_x11_gstreamer_pipeline(
    config: &RecordingConfig,
    profile: &EncoderProfile,
    output_path: &Path,
    with_audio: bool,
) -> RecordResult<String> {
    let output_str = output_path.to_string_lossy();
    let video_source = get_x11_source(config)?;
    let video_raw_caps = format!("video/x-raw,framerate={}/1", config.fps);
    let scale_segment = match config.max_resolution {
        Some((max_w, max_h)) => format!(
            " ! videoscale ! video/x-raw,width=[1,{max_w}],height=[1,{max_h}],pixel-aspect-ratio=1/1"
        ),
        None => String::new(),
    };
    let muxer = if with_audio {
        format!("{} name=mux", profile.muxer)
    } else {
        profile.muxer.to_string()
    };

    Ok(format!(
        "{} ! videoconvert ! {} ! videorate{} ! {} ! {} ! {} ! filesink location=\"{}\"",
        video_source, video_raw_caps, scale_segment, "queue", profile.encoder, muxer, output_str
    ))
}

pub(in crate::recording) fn get_x11_source(config: &RecordingConfig) -> RecordResult<String> {
    let show_pointer = if config.cursor { "true" } else { "false" };
    let mut source = format!("ximagesrc show-pointer={} use-damage=false", show_pointer);

    if let (Some(x), Some(y), Some(w), Some(h)) = (config.x, config.y, config.width, config.height)
    {
        source.push_str(&format!(
            " startx={} starty={} endx={} endy={}",
            x,
            y,
            x + w as i32 - 1,
            y + h as i32 - 1
        ));
    }

    Ok(source)
}

#[cfg(test)]
mod tests {
    use super::super::profile::PROFILES;
    use super::*;

    fn x11_config(max_resolution: Option<(u32, u32)>) -> RecordingConfig {
        RecordingConfig {
            output_path: PathBuf::from("/tmp/apexshot-test.mp4"),
            width: Some(1920),
            height: Some(1200),
            x: Some(0),
            y: Some(0),
            cursor: true,
            pointer_track: false,
            hidpi: false,
            max_resolution,
            fps: 30,
            crf: 20,
            mono_audio: false,
            mic_enabled: false,
            speaker_enabled: false,
            mic_source: None,
            speaker_source: None,
            noise_suppression: false,
        }
    }

    fn h264_profile() -> &'static EncoderProfile {
        PROFILES
            .iter()
            .find(|profile| profile.encoder == "x264enc")
            .expect("expected x264 profile to exist")
    }

    #[test]
    fn x11_pipeline_scales_down_to_the_configured_cap() {
        let config = x11_config(Some((854, 480)));

        let pipeline =
            build_x11_gstreamer_pipeline(&config, h264_profile(), &config.output_path, false)
                .expect("expected pipeline string");

        assert_eq!(
            pipeline,
            "ximagesrc show-pointer=true use-damage=false startx=0 starty=0 endx=1919 endy=1199 \
             ! videoconvert ! video/x-raw,framerate=30/1 ! videorate \
             ! videoscale ! video/x-raw,width=[1,854],height=[1,480],pixel-aspect-ratio=1/1 \
             ! queue ! x264enc ! mp4mux ! filesink location=\"/tmp/apexshot-test.mp4\""
        );
    }

    #[test]
    fn x11_pipeline_omits_scaling_without_a_cap() {
        let config = x11_config(None);

        let pipeline =
            build_x11_gstreamer_pipeline(&config, h264_profile(), &config.output_path, false)
                .expect("expected pipeline string");

        assert!(!pipeline.contains("videoscale"));
        assert!(!pipeline.contains("width=[1,"));
    }

    #[test]
    fn x11_pipeline_keeps_a_separator_between_framerate_caps_and_videorate() {
        // Regression: the string once concatenated them into
        // `framerate=30/1videorate`, which gst-parse folds into the caps value
        // and the pipeline then fails to link videoconvert to queue.
        for cap in [None, Some((1280, 720))] {
            let config = x11_config(cap);

            let pipeline =
                build_x11_gstreamer_pipeline(&config, h264_profile(), &config.output_path, false)
                    .expect("expected pipeline string");

            assert!(pipeline.contains("video/x-raw,framerate=30/1 ! videorate"));
            assert!(!pipeline.contains("30/1videorate"));
        }
    }

    #[test]
    fn x11_pipeline_scale_follows_videorate_not_the_framerate_caps() {
        // gst-parse rejects two adjacent caps filters, so the scale caps must
        // sit after the videorate element rather than next to the framerate caps.
        let config = x11_config(Some((1280, 720)));

        let pipeline =
            build_x11_gstreamer_pipeline(&config, h264_profile(), &config.output_path, false)
                .expect("expected pipeline string");

        let videorate = pipeline.find("videorate").expect("videorate in pipeline");
        let scale = pipeline
            .find("! videoscale !")
            .expect("videoscale segment in pipeline");
        assert!(videorate < scale);
        assert!(!pipeline.contains("1/1 ! video/x-raw,framerate"));
    }
}
