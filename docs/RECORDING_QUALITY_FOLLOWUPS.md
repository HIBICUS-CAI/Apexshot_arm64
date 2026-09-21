# Recording and export quality follow-ups

Working tracker for the recording/export audit done on 2026-09-21. Each item is
one logical change on its own branch, in the order listed. Update the status
line when an item lands so the next session can pick up from here.

Status: item 1 in review (PR #55); items 2 to 4 and the item 5 cleanup not
started.

## Decisions already made

- All four items below are approved, and are worked one at a time.
- The editor gets a real export-quality control (Balanced / High / Ultra),
  defaulting to High, rather than only fixing the wording.
- "Maximum resolution" keeps its ceiling semantics (never upscales); the docs
  need to say so instead of the setting changing meaning.
- The recording resolution is chosen in Settings only. The overlay forwards the
  configured value and no longer offers a picker of its own (removed in PR #55),
  and the dead Rust panel that had one is item 5.

## Item 1: overlay recording requests dropped four of the seven resolution options

**Confirmed** (code-level; the manual check is still pending).

Settings offers seven options for "Maximum resolution"
(`src/settings/recording.rs`), but the request built for the capture overlay only
knew the first three: `src/recording/controls.rs:317-322` mapped indices 0 to 2
and sent everything else to `None` (Original). Values 3 to 6 do reach that code,
because the overlay seeds its state from the config
(`src/overlay/recording/state.rs:120`), so a 480p, 900p, 1440p or 2160p cap was
silently recorded at native size.

Each UI also carried its own copy of the list. The Rust recording panel drew
seven rows but hit-tested three (`hit_testing.rs:483-485` versus
`settings_ui.rs:627-632`), and the C++ overlay
(`capture-overlay/src/CaptureOverlay_RecordingSettingsDrawing.cpp:210`) offered
only `Original / 1080p / 720p` while indexing that three-element list with the
value taken straight from the config (`CaptureOverlay.h:163`), which is out of
range for anything above 720p.

**Correction (same day)**: the Rust recording panel is dead code. It is drawn
only when `st.recording.panel_open` is true and nothing outside tests sets it
(`src/overlay/recording/hit_testing.rs`, `src/overlay/geometry.rs` are the only
writers), and `OverlayIntent::Record` is never assigned in production
(`src/overlay/api.rs:116` produces `Area` and `Ocr` only). On GNOME Wayland the
selector is the C++ overlay regardless (`src/main.rs:437-444`: GNOME has no
layer-shell). The seven-row dropdown fix was therefore dropped from the PR, and
the whole panel is scheduled for removal as item 5. There is one settings UI for
recording resolution, and it is Settings; the quick access menu never had that
option.

**Fix scope (PR #55)**

- One source of truth in `src/recording/mod.rs`:
  `max_resolution_for_setting(u8) -> Option<(u32, u32)>`, plus
  `VIDEO_MAX_RES_OPTION_COUNT` for how many options Settings offers.
- `src/recording/controls.rs` maps every index through the helper instead of its
  own three-case match; the test covers all seven indices and an out-of-range
  value.
- `src/config.rs` keeps a hand-edited `rec_video_max_res` inside the table.
- The C++ overlay's resolution picker is removed rather than extended: the Video
  tab is now frame rate, mono and open-video-editor, and the value the overlay
  forwards comes from Settings
  (`CaptureOverlay_RecordingSettingsDrawing.cpp`, `CaptureOverlay_Events.cpp`,
  `CaptureOverlay.h`). Settings is the only place the resolution is chosen.

**Status**: PR #55 (branch `fix/overlay-resolution-options`), awaiting merge and
the manual check below.

**Acceptance**: with Settings on 480p, a recording started from the overlay
produces an 854x480 file; 2160p on a 1080p display records 1920x1080 (a ceiling,
never an upscale).

**Separate risks found while tracing (not part of this fix)**: the C++ overlay's
frame-rate picker (`CaptureOverlay_Events.cpp`) indexes a four-element list with
`m_videoFps`, which also comes from the config
(`CaptureOverlay.h:164`, `CaptureOverlay_RecordingSettingsDrawing.cpp:224`), and
`rec_video_fps` is not clamped in `config.rs`; a hand-edited value indexes out of
range. `config.rs:67` documents the Ultra tier as CRF 17 where `crf_for_quality`
uses 16, and `rec_video_fps` shares the hand-edited-config indexing risk.

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

## Item 5: retire the dead Rust recording panel

**Confirmed dead code.** `st.recording.panel_open` is written only by tests, so
`recording_ui::draw_recording_panel` (`src/overlay/drawing/mod.rs:675`, `:755`)
never paints in the running app, the recording settings menu with its resolution
popup (`src/overlay/drawing/settings_ui.rs`) is unreachable, and the request
path in `src/overlay/recording/result.rs` never produces a recording that way.
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
- Not verified: no live recording session was run, so delivered frame rate,
  encoder behaviour at 4K60 and the overlay dropdown were confirmed by code and
  tests only.
- Item 1 as it now stands (PR #55): `cargo fmt --all -- --check` clean,
  `cargo clippy --workspace --all-targets` with the two pre-existing warnings and
  no new ones, `cargo test --jobs 2 -- --test-threads=1` 1088 lib tests plus the
  integration targets with 0 failures, `python3 scripts/check-i18n-catalogs.py`
  ok, and `capture-overlay` builds clean with
  `cmake -S . -B build && cmake --build build -j`.
- Not verified for item 1: the overlay was not run against a live session. The
  resolution row is gone from the built overlay and the request path is covered
  by tests, but the recording check in the acceptance line is still open.
