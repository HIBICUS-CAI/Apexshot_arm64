#[test]
fn window_tool_removed_from_toolbars() {
    let cpp_toolbar = include_str!("../capture-overlay/src/CaptureOverlay_ToolbarDrawing.cpp");
    let cpp_events = include_str!("../capture-overlay/src/CaptureOverlay_Events.cpp");
    let rust_icons = include_str!("../src/overlay/icons.rs");
    let rust_toolbar = include_str!("../src/overlay/window/input/click/toolbar.rs");

    assert!(
        cpp_toolbar.contains("\"Area\", \"Fullscreen\", \"Scroll\"")
            || cpp_toolbar.contains("\"Area\", \"Fullscreen\", \"Scroll\", \"Timer\""),
        "C++ toolbar labels must not include Window"
    );
    assert!(
        !cpp_toolbar.contains("\"Window\""),
        "C++ toolbar must not list Window as a tool label"
    );
    assert!(
        !cpp_events.contains("Window tool ignored") && !cpp_events.contains("enterWindowMode()"),
        "C++ toolbar click handler must not keep a Window tool branch"
    );

    // The rail icons (Area/Fullscreen/Scroll/Timer/Ocr/Recording) were retired
    // with the legacy left toolbar, so the Window tool must not survive in any
    // form — neither as an enum variant nor as a list entry.
    for retired in [
        "Window",
        "Area",
        "Fullscreen",
        "Scroll",
        "Timer",
        "Ocr",
        "Recording",
    ] {
        assert!(
            !rust_icons.contains(&format!("ToolbarIcon::{retired}")),
            "retired rail tool {retired} must not remain in the Rust icon set"
        );
    }
    assert!(
        !rust_toolbar.contains("ToolbarIcon::Window"),
        "Rust overlay click handler must not handle a Window toolbar tool"
    );
}

#[test]
fn capture_timer_badge_uses_timer_tool_tile() {
    let cpp_toolbar = include_str!("../capture-overlay/src/CaptureOverlay_ToolbarDrawing.cpp");

    assert!(
        cpp_toolbar.contains("if (i == kTimerToolIndex && timerToolActive)"),
        "C++ timer badge must be drawn on the Timer tile"
    );
    assert!(
        !cpp_toolbar.contains("if (i == 4 && timerToolActive)"),
        "C++ timer badge must not use the OCR tile index"
    );
}
