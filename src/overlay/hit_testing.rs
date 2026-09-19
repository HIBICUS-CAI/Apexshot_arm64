use super::icons::ToolbarIcon;
use super::layout::*;
use super::recording::state::OverlayIntent;
use super::state::{OverlayMode, SelectorState};
// Recording-specific hit-testing lives in recording/hit_testing.rs

/// Mirrors C++ `topBarVisible()`: the screen-fixed bar is hidden while
/// recording, picking a window, counting down, or showing the scroll popup.
/// Crosshair capture and the recording intent never show it either.
pub(crate) fn top_bar_visible(st: &SelectorState) -> bool {
    if st.overlay_mode == OverlayMode::CrosshairCapture {
        return false;
    }
    if st.recording.panel_open {
        return false;
    }
    if st.window_picker_open {
        return false;
    }
    if st.countdown_active {
        return false;
    }
    if st.scroll_popup_open {
        return false;
    }
    if st.intent == OverlayIntent::Record {
        return false;
    }
    true
}

/// Pill index under the pointer (0=Free, 1=16:9, 2=4:3, 3=1:1).
pub(crate) fn top_bar_aspect_at(
    st: &SelectorState,
    screen_width: f64,
    x: f64,
    y: f64,
) -> Option<usize> {
    if !top_bar_visible(st) {
        return None;
    }
    let layout = compute_top_bar_layout(screen_width)?;
    if !layout.bar.contains(x, y) {
        return None;
    }
    layout.pills.iter().position(|p| p.contains(x, y))
}

/// Button slot under the pointer (0=crop dropdown, 1=cancel).
pub(crate) fn top_bar_button_at(
    st: &SelectorState,
    screen_width: f64,
    x: f64,
    y: f64,
) -> Option<usize> {
    if !top_bar_visible(st) {
        return None;
    }
    let layout = compute_top_bar_layout(screen_width)?;
    if !layout.bar.contains(x, y) {
        return None;
    }
    layout.buttons.iter().position(|b| b.contains(x, y))
}

pub(crate) fn point_in_top_bar(st: &SelectorState, screen_width: f64, x: f64, y: f64) -> bool {
    if !top_bar_visible(st) {
        return false;
    }
    compute_top_bar_layout(screen_width).is_some_and(|layout| layout.bar.contains(x, y))
}

fn top_bar_crop_menu_rects(st: &SelectorState, screen_width: f64) -> Option<(RectF, Vec<RectF>)> {
    if !top_bar_visible(st) || !st.top_bar_crop_menu_open {
        return None;
    }
    let layout = compute_top_bar_layout(screen_width)?;
    let anchor = layout.buttons[TOP_BAR_CROP_BUTTON];
    Some(compute_top_bar_crop_menu(
        anchor,
        layout.bar.y + layout.bar.height,
        screen_width,
    ))
}

/// Crop dropdown row under the pointer (0=Reset … 10=Snap toggle).
pub(crate) fn top_bar_crop_item_at(
    st: &SelectorState,
    screen_width: f64,
    x: f64,
    y: f64,
) -> Option<usize> {
    let (_panel, items) = top_bar_crop_menu_rects(st, screen_width)?;
    items.iter().position(|r| r.contains(x, y))
}

pub(crate) fn top_bar_crop_menu_contains(
    st: &SelectorState,
    screen_width: f64,
    x: f64,
    y: f64,
) -> bool {
    top_bar_crop_menu_rects(st, screen_width).is_some_and(|(panel, _)| panel.contains(x, y))
}

pub(crate) fn toolbar_item_at(
    _selection_x: f64,
    _selection_y: f64,
    _selection_width: f64,
    _selection_height: f64,
    _screen_width: f64,
    _screen_height: f64,
    _x: f64,
    _y: f64,
) -> Option<ToolbarIcon> {
    // Rail retired (see toolbar_hit_at): no icon ever resolves here.
    None
}

pub(crate) fn toolbar_hit_at(
    _selection_x: f64,
    _selection_y: f64,
    _selection_width: f64,
    _selection_height: f64,
    _screen_width: f64,
    _screen_height: f64,
    _x: f64,
    _y: f64,
) -> Option<ToolbarHit> {
    // Legacy capture chrome retired: the left tool rail is gone and the
    // top-center instruction bar owns aspect selection, so nothing here ever
    // hits on the capture path. (`ToolbarHit::SizePanel`/`CropPanel` stay for
    // API compat with defensive match arms.)
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolbar_hit_never_returns_tool_hits() {
        // Legacy left rail retired: a screen-wide sweep must yield no legacy
        // hit anywhere — quick capture owns mode selection up front and the
        // top bar owns aspects.
        let mut y = 0.0;
        while y <= 1080.0 {
            let mut x = 0.0;
            while x <= 1920.0 {
                assert_eq!(
                    toolbar_hit_at(200.0, 200.0, 400.0, 300.0, 1920.0, 1080.0, x, y),
                    None,
                    "retired rail hit at ({x},{y})"
                );
                assert_eq!(
                    toolbar_item_at(200.0, 200.0, 400.0, 300.0, 1920.0, 1080.0, x, y),
                    None,
                    "retired rail icon at ({x},{y})"
                );
                x += 64.0;
            }
            y += 64.0;
        }
    }

    #[test]
    fn top_bar_hits_resolve_pills_buttons_and_chrome() {
        use crate::overlay::state::SelectorState;
        let st = SelectorState::default();
        assert!(top_bar_visible(&st));
        let layout = compute_top_bar_layout(1920.0).expect("bar must fit");
        for (i, pill) in layout.pills.iter().enumerate() {
            let x = pill.x + pill.width / 2.0;
            let y = pill.y + pill.height / 2.0;
            assert_eq!(top_bar_aspect_at(&st, 1920.0, x, y), Some(i));
            assert_eq!(top_bar_button_at(&st, 1920.0, x, y), None);
            assert!(point_in_top_bar(&st, 1920.0, x, y));
        }
        for (i, button) in layout.buttons.iter().enumerate() {
            let x = button.x + button.width / 2.0;
            let y = button.y + button.height / 2.0;
            assert_eq!(top_bar_button_at(&st, 1920.0, x, y), Some(i));
            assert_eq!(top_bar_aspect_at(&st, 1920.0, x, y), None);
        }
        // Label lane is bar chrome, not a control.
        let lx = layout.label.x + 4.0;
        let ly = layout.label.y + layout.label.height / 2.0;
        assert!(point_in_top_bar(&st, 1920.0, lx, ly));
        assert_eq!(top_bar_aspect_at(&st, 1920.0, lx, ly), None);
        assert_eq!(top_bar_button_at(&st, 1920.0, lx, ly), None);
        // Outside the bar nothing hits.
        assert!(!point_in_top_bar(&st, 1920.0, 10.0, 500.0));
        assert_eq!(top_bar_aspect_at(&st, 1920.0, 10.0, 500.0), None);
    }

    #[test]
    fn top_bar_hits_hide_with_recording_picker_countdown_and_scroll() {
        use crate::overlay::state::SelectorState;
        let layout = compute_top_bar_layout(1920.0).expect("bar must fit");
        let pill = layout.pills[1];
        let (x, y) = (pill.x + 1.0, pill.y + 1.0);
        let mut st = SelectorState::default();
        assert!(top_bar_visible(&st));
        st.recording.panel_open = true;
        assert!(!top_bar_visible(&st));
        assert_eq!(top_bar_aspect_at(&st, 1920.0, x, y), None);
        st.recording.panel_open = false;
        st.window_picker_open = true;
        assert!(!top_bar_visible(&st));
        st.window_picker_open = false;
        st.countdown_active = true;
        assert!(!top_bar_visible(&st));
        assert!(!point_in_top_bar(&st, 1920.0, x, y));
        st.countdown_active = false;
        st.scroll_popup_open = true;
        assert!(!top_bar_visible(&st));
        st.scroll_popup_open = false;
        assert!(top_bar_visible(&st));
    }

    #[test]
    fn top_bar_crop_menu_hits_follow_open_state() {
        use crate::overlay::state::SelectorState;
        let mut st = SelectorState::default();
        assert_eq!(top_bar_crop_item_at(&st, 1920.0, 960.0, 120.0), None);
        assert!(!top_bar_crop_menu_contains(&st, 1920.0, 960.0, 120.0));
        st.top_bar_crop_menu_open = true;
        let layout = compute_top_bar_layout(1920.0).expect("bar must fit");
        let (_panel, items) = compute_top_bar_crop_menu(
            layout.buttons[TOP_BAR_CROP_BUTTON],
            layout.bar.y + layout.bar.height,
            1920.0,
        );
        for (i, item) in items.iter().enumerate() {
            let x = item.x + item.width / 2.0;
            let y = item.y + item.height / 2.0;
            assert_eq!(top_bar_crop_item_at(&st, 1920.0, x, y), Some(i));
            assert!(top_bar_crop_menu_contains(&st, 1920.0, x, y));
        }
        st.top_bar_crop_menu_open = false;
        let first = items[0];
        assert_eq!(
            top_bar_crop_item_at(&st, 1920.0, first.x + 1.0, first.y + 1.0),
            None
        );
    }

    #[test]
    fn toolbar_item_at_never_resolves_without_rail_hits() {
        // No rail hit can occur, so no icon ever resolves on the capture path.
        assert_eq!(
            toolbar_item_at(200.0, 200.0, 400.0, 300.0, 1920.0, 1080.0, 210.0, 210.0),
            None
        );
    }
}
