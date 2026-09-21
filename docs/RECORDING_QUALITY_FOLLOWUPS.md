# Recording and export quality follow-ups

Working tracker for the recording/export audit done on 2026-09-21. Each item is
one logical change on its own branch, in the order listed. Update the status
line when an item lands so the next session can pick up from here.

Status: item 1 in progress; items 2 to 4 not started.

## Decisions already made

- All four items below are approved, and are worked one at a time.
- The editor gets a real export-quality control (Balanced / High / Ultra),
  defaulting to High, rather than only fixing the wording.
- "Maximum resolution" keeps its ceiling semantics (never upscales); the docs
  need to say so instead of the setting changing meaning.

## Item 1: the overlay resolution setting only offers three working options

**Confirmed** (code-level; a live run has not been done yet).

Two defects stack up so that 1440p, 900p, 480p and 2160p cannot be selected from
the overlay, and would be dropped even if they arrived:

1. `src/overlay/recording/hit_testing.rs:483-485` caps the resolution dropdown
   at three rows (`(SettingsTab::Video, 3) => 3`), while
   `src/overlay/drawing/settings_ui.rs:627-632` draws seven rows
   (`Original, 1080p, 720p, 1440p, 900p, 480p, 2160p`). The popup is drawn
   210px tall and hit-tested as 90px tall, so rows 4 to 7 close the dropdown
   without selecting anything. The existing test
   (`hit_testing.rs:300-330`) encodes the wrong count.
2. `src/recording/controls.rs:317-322` maps only indices 0, 1, 2 to a cap and
   sends everything else to `None` (Original). A value of 3 to 6 can arrive
   from Settings, which offers all seven
   (`src/settings/recording.rs:184-200`), because the overlay seeds its state
   from the config (`src/overlay/recording/state.rs:120`). The test at
   `controls.rs:1107-1137` only covers indices 0 to 2.

The same index table is written out three times:
`src/recording/mod.rs:251-266` (complete), `src/recording/controls.rs:317-322`
(truncated), and the overlay label list (`settings_ui.rs:627-632`).

**Fix scope**

- One source of truth in `src/recording/mod.rs`:
  `max_resolution_for_setting(u8) -> Option<(u32, u32)>` plus the setting count
  (`VIDEO_MAX_RES_SETTINGS` length) used by the overlay drawing and hit test.
  Keep the `t("...")` label literals at their call sites so i18n extraction
  keeps working; assert the label array length against the shared count in a
  test instead.
- `src/recording/controls.rs` uses the helper instead of its own match.
- `src/overlay/recording/hit_testing.rs` uses the shared count.
- Extend `prepare_overlay_recording_request_maps_video_setting_variants` to all
  seven indices plus an out-of-range value, and update the dropdown geometry
  test to seven rows.

**Acceptance**: picking 2160p in the overlay records a 1080p file on a 1080p
display (cap, no upscale) and 2160p on a 4K display; picking 480p yields 854x480.

**Separate risk found while tracing (not part of this fix)**: the C++ overlay
(`capture-overlay/`) still ships three options
(`CaptureOverlay_Events.cpp:410`, `CaptureOverlay_RecordingSettingsDrawing.cpp:210`)
and indexes a three-element list with a value taken straight from config
(`CaptureOverlay.h:163`, `:211`). A config saved from Settings with 1440p or
2160p can therefore index out of range on non-GNOME sessions.

## Item 2: export quality control in the video editor

**Confirmed gap.** An edited export always re-encodes with
`libx264 -preset veryfast -crf quality_to_crf(quality)`
(`src/recording/editor/ffmpeg.rs:435-441`, `helpers.rs:391`), and the default
quality of 70 maps to CRF 22 (`model_parts/state_impl.rs:14`), softer than a
High (20) or Ultra (16) recording (`src/recording/mod.rs:295`). No UI writes
`state.quality`, so the README claim "adjust quality"
(`README.md:44`, `README.md:110`) and the overlay caption
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

**Acceptance**: an untouched export stays bit-identical; selecting Ultra makes
the export args use CRF 16 and the estimate reflect the change.

## Item 3: document the cap semantics and fix the quality wording

**Confirmed drift.** No `.md` file mentions "Maximum resolution" at all, and the
README and overlay caption promise quality editing that does not exist.

- Add a short note where recording settings are described: the resolution
  setting is a ceiling (never upscales), and the FPS setting sets the container
  frame rate, not a guarantee of captured motion (the compositor decides what it
  delivers; see the mutter discussion in
  <https://gitlab.gnome.org/GNOME/mutter/-/work_items/4214>).
- Reword `README.md:44`, `README.md:110` and the overlay caption once item 2
  decides whether the control exists, so the text matches the code.

## Item 4: the X11 backend ignores the resolution cap

**Confirmed** (code-level). `build_x11_gstreamer_pipeline`
(`src/recording/backend/x11.rs:183`) has no `videoscale`, so `max_resolution`
never reaches the pipeline on Xorg sessions. There is no test on that pipeline
string today. Only worth doing if X11 stays supported.

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
- Not verified: no live recording session was run, so delivered frame rate,
  encoder behaviour at 4K60 and the overlay dropdown were confirmed by code and
  tests only.
