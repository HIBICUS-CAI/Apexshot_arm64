# Recording and export quality follow-ups

Working tracker for the recording/export audit done on 2026-09-21, and the
handoff note for whoever continues it. Read this file, then take the first item
whose status is not done.

Status: item 1 merged to `main` in PR #55. Item 2 is implemented on
`feat/editor-export-quality` (PR #56) and waits on the maintainer's hand
check. Items 3 to 5 not started. Next action: check item 2, then start item 3
on a fresh branch.

## How to continue (read this first)

- One item at a time, in order. When an item is implemented, stop and tell the
  maintainer exactly what to check by hand. Do not start the next item until they
  confirm the previous one; they install and test the build themselves.
- Follow `AGENTS.md`: investigate before editing (no code changes while
  confirming), then take the lightest path that protects `main`. Behaviour
  changes: branch from `origin/main` plus a PR. Docs, comments and message
  strings: commit straight to `main`. Never merge the PR; that is the
  maintainer's call.
- Gates before pushing: `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets` (must not add warnings; two
  pre-existing ones live in `src/capture/editor/window/canvas_render.rs` and
  `src/recording/editor/window/tool_sidebar_background.rs`),
  `cargo test --jobs 2 -- --test-threads=1`, `python3
  scripts/check-i18n-catalogs.py` when UI strings change, and
  `cd capture-overlay && cmake -S . -B build && cmake --build build -j` when the
  C++ overlay changes.
- Verifying a recording by hand: the maintainer records, then inspect the file
  with
  `ffprobe -v error -show_entries stream=width,height,r_frame_rate,avg_frame_rate,nb_frames,codec_name -show_entries format=duration,size -of default=noprint_wrappers=1 <file>`,
  and `strings -n 6 <file> | grep -o "h264_nvenc\|libx264"` to see the encoder.
  The quality tier (CQP/CRF) is not stored in the container, so it stays a
  code-level fact.
- Recording settings live in `~/.config/apexshot/config.yml` (YAML). The app
  rewrites that file at record start, so its mtime marks the recording and it is
  the quickest way to see what the app actually had saved.

## Machine and session facts (the maintainer's machine)

- GNOME on Wayland, one 1920x1200 display, NVIDIA GPU. Recordings use NVENC
  (`encoder=Lavc62.11.100 h264_nvenc` in the file tags).
- GNOME has no layer-shell, so the live capture UI is the **C++ quick access**
  (`capture-overlay/`, routed in `src/main.rs:437-444`). The Rust overlay and the
  Rust capture menu are the wlroots path (`src/overlay/`,
  `src/capture_overlay/wlroots.rs`). The Rust recording panel is dead code and is
  item 5.
- Both quick accesses start a recording by sending a `RecordingRequest`; on GNOME
  the daemon handles it in `run_overlay_recording_request_with_gtk`.
- Watch the echo pattern in `prepare_overlay_recording_request`: it copies
  request fields into the config and the caller saves that config at record
  start. A request field with no picker behind it silently reverts the saved
  setting, which is what item 1 fixed for the resolution. The remaining fields
  (fps, mono, DND, HiDPI, countdown, dim screen, controls, notifications,
  remember-selection) are still echoed; that is correct while the C++ overlay
  offers those pickers, but a stale overlay process reverts them the same way.
- Build and install: the maintainer builds a local dev deb and installs it
  themselves (`apt install --reinstall /tmp/apexshot-dev-deb.*.deb`, then they
  restart the app; the installed binary matched `target/release/apexshot`). Push
  the branch and ask them to build, install and test; do not assume a build
  command of your own.

## Decisions already made

- All items below are approved and are worked one at a time.
- The editor gets a real export-quality control (Balanced / High / Ultra),
  defaulting to High, rather than only fixing the wording.
- "Maximum resolution" keeps its ceiling semantics (never upscales); the docs
  need to say so instead of the setting changing meaning.
- The recording resolution is chosen in Settings only. The overlays forward the
  configured value and no longer offer a picker (removed in PR #55); the dead
  Rust panel that had one is item 5.
- On a 16:10 screen a full-display 480p recording is 768x480, not 854x480: the
  cap box is 854x480 and the aspect fit is limited by height.

## Item 1: DONE (PR #55, verified)

Two defects kept a saved "Maximum resolution" from reaching a recording. Both
are fixed on `fix/overlay-resolution-options` and confirmed on a live recording.

1. The overlay request path only knew the first three options:
   `src/recording/controls.rs` mapped indices 0 to 2 to a cap and sent 3 to 6 to
   `None` (Original), so 480p, 900p, 1440p and 2160p were recorded at native
   size whenever the request came from the overlay.
2. The request's resolution was written back into the saved config.
   `prepare_overlay_recording_request` copied `request.video_max_res` into
   `app_config.rec_video_max_res` (`controls.rs:292`) and
   `run_overlay_recording_request_with_gtk` (`controls.rs:755`) saves that config
   at record start. Neither quick access has a resolution picker, so the request
   only echoes the value the overlay was launched with, and a resolution saved in
   Settings after the overlay started was reverted by the next recording.

**What landed**

- `src/recording/mod.rs` owns the table: `max_resolution_for_setting(u8)` plus
  `VIDEO_MAX_RES_OPTION_COUNT`, used by the request builder and by the config
  clamp in `src/config.rs`.
- `src/recording/controls.rs` maps every index through the helper and no longer
  copies the request's resolution into the config; the cap comes from the config
  the caller loaded. Tests drive the cap from the saved value and pin that a
  request value of 0 cannot change it.
- The C++ overlay's three-option picker is removed
  (`CaptureOverlay_RecordingSettingsDrawing.cpp`, `CaptureOverlay_Events.cpp`,
  `CaptureOverlay.h`): its Video tab is frame rate, mono and open-video-editor,
  and the panel shrinks to fit. Settings is the only place the resolution is
  chosen.

**Verification (maintainer's machine, 1920x1200)**

- Before, with 480p saved: `ApexShot Recording 2026-09-21 at 15-53-47.mp4` and
  `... at 16-08-11.mp4`, both 1920x1200, 60.000 fps CFR, H.264 High, NVENC,
  CQP 16 (Ultra), and `config.yml` written back to `rec_video_max_res: 0` at each
  recording's start second.
- After, with 480p saved: `ApexShot Recording 2026-09-21 at 16-37-36.mp4` is
  768x480, 60.000 fps CFR (637 frames / 10.617 s), H.264 High, NVENC, 432 kbps,
  573 KB, and `config.yml` kept `rec_video_max_res: 5`.
- Gates: fmt clean, clippy with no new warnings, full suite 1088 lib tests plus
  the integration targets with 0 failures, i18n catalogs ok, C++ overlay builds
  clean. CI on the fix commit passed (the push did not trigger a run, so it was
  dispatched with `gh workflow run "CI & Release" --ref
  fix/overlay-resolution-options`).

**Separate risks found while tracing (not fixed here)**

- The C++ overlay's frame-rate picker (`CaptureOverlay_Events.cpp`) indexes a
  four-element list with `m_videoFps`, which also comes from the config
  (`CaptureOverlay.h:164`, `CaptureOverlay_RecordingSettingsDrawing.cpp:224`), and
  `rec_video_fps` is not clamped in `config.rs`; a hand-edited value indexes out
  of range.
- `config.rs:67` documents the Ultra tier as CRF 17 where `crf_for_quality` uses
  16.

## Item 2: export quality control in the video editor

**Implemented on `feat/editor-export-quality`, awaiting maintainer check.**
An edited export always re-encodes with
`libx264 -preset veryfast -crf quality_to_crf(quality)`
(`src/recording/editor/ffmpeg.rs:435-441`, `helpers.rs:391`), and the default
quality of 70 maps to CRF 22 (`model_parts/state_impl.rs:14`), softer than a
High (20) or Ultra (16) recording (`src/recording/mod.rs:295`). No UI writes
`state.quality`, so the README claim "adjust quality" (`README.md:44`,
`README.md:110`) and the overlay caption
(`capture-overlay/src/CaptureOverlay_RecordingSettingsDrawing.cpp:246`) are
currently drift.

**Fix scope**

- Balanced / High / Ultra control in the editor footer, mapping through the same
  tiers as recording (23 / 20 / 16), default High.
- Trap: `needs_reencode()` uses `quality != 70` as its "untouched" sentinel
  (`src/recording/editor/model_parts/output_core_impl.rs:99`). Replace it with a
  named default constant before the control can write any other value, or every
  untouched export silently re-encodes.
- Keep the untouched-export path a stream copy; that behaviour is tested
  (`trim_only_command_uses_stream_copy`,
  `export_edited_uses_trim_when_no_reencode_needed`).

**What landed**

- `ExportQuality` enum (`Balanced` / `High` / `Ultra`, default `High`) in
  `model_parts/editor_types.rs`, with `tier()` (the `0 / 1 / 2` index shared
  with `rec_video_quality`) and `crf()` through `crate::recording::crf_for_quality`.
- `VideoEditState.quality` is the enum now; `needs_reencode()` compares against
  `ExportQuality::default()` instead of `70`; both ffmpeg convert paths encode
  `-crf state.quality.crf()`; the old 0-100 `quality_to_crf` helper is gone.
- Project files keep `quality: u8` and store the tier index; pre-tier files
  (always `70`) load as `High` (`quality_from_file` in `project.rs`).
- The estimate maps tiers to size factors (`Balanced 1.0 / High 1.18 /
  Ultra 1.5`) in `estimate_size_bytes`; `High` keeps the old default factor so
  the default estimate is unchanged.
- The stage-tools footer row (next to the Frame and Crop chips) has an
  "Export quality" chip with a Balanced / High / Ultra popover
  (`build_stage_tools` in `window/preview.rs`, reusing the stage chip and
  aspect-popover classes, no new CSS). One new UI string, `Export quality`
  (tooltip), added to all eight catalogs by hand.
- Item 3 owns the README and overlay-caption wording; this branch does not
  touch them.

**Verification**

- New tests: tiers map to CRF 23 / 20 / 16 (`export_quality_tiers_match_recording_crf`),
  Ultra forces re-encode and writes `-crf 16` (`convert_command_uses_ultra_crf_when_quality_is_ultra`),
  estimates order Balanced < High < Ultra and trim-only estimates ignore the tier
  (`estimate_size_follows_quality_tier`). Default convert is now `-crf 20`.
- Gates: fmt clean, clippy with no new warnings, full suite 1090 lib tests plus
  the integration targets with 0 failures, i18n catalogs ok. C++ overlay
  untouched.
- Hand check for the maintainer: open a recording in the video editor, pick
  Ultra in the new footer chip, export twice (once untouched for the
  bit-identical stream copy, once with a trim) and compare sizes against a
  Balanced export of the same timeline; the CRF itself stays a code-level fact.

**Separate finding (not fixed here)**

- The export-size estimate label (`recording-editor-estimate`, updated by
  `footer::update_estimate` on every edit) is parented nowhere: `f3f9195`
  removed `content.append(&estimate_label)` with the old inspector panels and
  no `append(&estimate_label)` exists today, so its text is computed but never
  shown. The item 2 acceptance covers the estimate at the code level only;
  making it visible is a UI placement decision for its own item.

**Acceptance**: an untouched export stays bit-identical; selecting Ultra makes
the export args use CRF 16 and the estimate reflect the change.

## Item 3: document the cap semantics and fix the quality wording

**Not started. Confirmed drift.** No `.md` file mentions "Maximum resolution" at
all, and the README and overlay caption promise quality editing that does not
exist.

- Add a short note where recording settings are described: the resolution
  setting is a ceiling (never upscales; a full-display 480p recording is 768x480
  on a 16:10 screen), and the FPS setting sets the container frame rate, not a
  guarantee of captured motion (the compositor decides what it delivers; see the
  mutter discussion in
  <https://gitlab.gnome.org/GNOME/mutter/-/work_items/4214>).
- Reword `README.md:44`, `README.md:110` and the overlay caption once item 2
  decides whether the control exists, so the text matches the code.

## Item 4: the X11 backend ignores the resolution cap

**Not started. Confirmed** (code-level). `build_x11_gstreamer_pipeline`
(`src/recording/backend/x11.rs:183`) has no `videoscale`, so `max_resolution`
never reaches the pipeline on Xorg sessions. There is no test on that pipeline
string today. Only worth doing if X11 stays supported. The maintainer's machine
is Wayland, so this needs a code-level check plus a test, not a live recording.

## Item 5: retire the dead Rust recording panel

**Not started. Confirmed dead code.** `st.recording.panel_open` is written only
by tests, so `recording_ui::draw_recording_panel`
(`src/overlay/drawing/mod.rs:675`, `:755`) never paints in the running app, the
recording settings menu with its resolution popup
(`src/overlay/drawing/settings_ui.rs`) is unreachable, and the request path in
`src/overlay/recording/result.rs` never produces a recording that way.
`OverlayIntent::Record` is assigned nowhere outside tests.

**Scope**: remove the panel state, drawing, hit testing and settings menu
(`src/overlay/recording/`, `src/overlay/drawing/recording_ui.rs`,
`src/overlay/drawing/settings_ui.rs`) together with the input branches that only
exist to serve them (`src/overlay/window/input/`, `src/overlay/window/audio.rs`),
keeping whatever the capture menu still uses (`src/overlay/capture_menu.rs`,
`src/capture_overlay/wlroots.rs`). Tests that only exercised the panel go with
it; `cargo clippy --workspace --all-targets` should report no new dead code.

## Unclaimed lead: empty recording file

`ApexShot Recording 2026-05-26 at 18-16-17.mp4` in the working tree (gitignored,
261 bytes) is a finalized MP4 with `ftyp` + empty `mdat` + a 213-byte `moov` and
no `trak` box: ffmpeg exited cleanly having written zero streams. Worth a step 1
investigation (how a session can end like that without a user-facing error), but
it is not part of the items above.

## Verification log for the audit

- `cargo test --lib recording -- --test-threads=1`: 294 passed.
- `cargo test --lib recording::editor -- --test-threads=1`: 198 passed.
- Synthetic ffmpeg check of the stream-copy trim path (30fps, GOP 2s,
  `-ss 3 -to 5`): exactly 60 frames / 2.000s, pre-roll covered by an mp4 edit
  list (`elst media_time=15360`).
- Item 1 live checks, before and after, are in the item 1 section above.
- Not verified: delivered frame rate against a demanding compositor, encoder
  behaviour at 4K60, and anything on X11. The maintainer's machine is GNOME
  Wayland at 1920x1200 with NVENC.
