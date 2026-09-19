//! Toolbar and recording-panel click state transitions.

use super::ClickEffect;
use crate::capture_overlay::RecordingType;
use crate::overlay::api::OverlaySelection;
use crate::overlay::geometry::{
    apply_aspect_to_selection, current_selection_rect, is_inside_selection, top_bar_ratio_for_pill,
};
use crate::overlay::hit_testing::{
    point_in_top_bar, toolbar_hit_at, top_bar_aspect_at, top_bar_button_at,
    top_bar_crop_menu_contains, top_bar_visible,
};
use crate::overlay::icons::{
    ToolbarIcon, TOOLBAR_AREA_INDEX, TOOLBAR_FULLSCREEN_INDEX, TOOLBAR_ICONS,
    TOOLBAR_RECORDING_INDEX, TOOLBAR_SCROLL_INDEX,
};
use crate::overlay::layout::{
    ToolbarHit, DEFAULT_SELECTION_HEIGHT, DEFAULT_SELECTION_WIDTH, MIN_SELECTION_HEIGHT,
    MIN_SELECTION_WIDTH, TOP_BAR_CANCEL_BUTTON, TOP_BAR_CROP_BUTTON, TOP_BAR_PILL_LEGACY_INDICES,
};
use crate::overlay::recording::hit_testing::recording_tile_at;
use crate::overlay::recording::layout::RecordPanelTile;
use crate::overlay::recording::state::OverlayIntent;
use crate::overlay::state::SelectorState;
use crate::overlay::window::result::recording_request_from_state;

pub(super) fn handle_toolbar_click(
    st: &mut SelectorState,
    n_press: i32,
    x: f64,
    y: f64,
    screen_width: i32,
    screen_height: i32,
) -> ClickEffect {
    // The screen-fixed top bar owns its presses (they never start a drag):
    // aspect pills apply a ratio, crop toggles the dropdown, X cancels.
    // Bar chrome and menu padding swallow the click.
    if top_bar_visible(st) {
        let sw = screen_width as f64;
        let sh = screen_height as f64;
        if let Some(pill) = top_bar_aspect_at(st, sw, x, y) {
            let ratio = top_bar_ratio_for_pill(pill);
            st.top_bar_aspect_ratio = ratio;
            st.capture_aspect_ratio_index =
                TOP_BAR_PILL_LEGACY_INDICES.get(pill).copied().unwrap_or(0);
            st.top_bar_crop_menu_open = false;
            st.hovered_top_bar_crop_item = -1;
            apply_aspect_to_selection(st, ratio, sw, sh);
            st.hover_tool_index = None;
            return ClickEffect::Redraw;
        }
        if let Some(button) = top_bar_button_at(st, sw, x, y) {
            if button == TOP_BAR_CROP_BUTTON {
                st.top_bar_crop_menu_open = !st.top_bar_crop_menu_open;
                st.hovered_top_bar_crop_item = -1;
                st.hover_tool_index = None;
                return ClickEffect::Redraw;
            }
            if button == TOP_BAR_CANCEL_BUTTON {
                return ClickEffect::Cancel;
            }
        }
        if point_in_top_bar(st, sw, x, y) || top_bar_crop_menu_contains(st, sw, x, y) {
            return ClickEffect::None;
        }
    }
    let rect = current_selection_rect(st);
    let recording_panel_open = st.recording.panel_open;
    let record_hit = recording_panel_open
        .then(|| {
            recording_tile_at(
                rect.left,
                rect.top,
                rect.width(),
                rect.height(),
                screen_width as f64,
                screen_height as f64,
                x,
                y,
            )
        })
        .flatten();
    let hit = if recording_panel_open {
        None
    } else {
        toolbar_hit_at(
            rect.left,
            rect.top,
            rect.width(),
            rect.height(),
            screen_width as f64,
            screen_height as f64,
            x,
            y,
        )
    };
    // Capture-menu Area already committed the capture intent.  Its C++
    // counterpart keeps frame/crop controls but disables the legacy tool rail.
    let hit = if st.capture_menu_area_mode && matches!(hit, Some(ToolbarHit::Tool(_))) {
        None
    } else {
        hit
    };
    let clicked = match hit {
        Some(ToolbarHit::Tool(index)) => Some(TOOLBAR_ICONS[index]),
        _ => None,
    };

    match clicked {
        Some(ToolbarIcon::Fullscreen) => {
            st.active_tool_index = TOOLBAR_FULLSCREEN_INDEX;
            st.intent = OverlayIntent::Area;
            st.start_x = 0.0;
            st.start_y = 0.0;
            st.current_x = screen_width as f64;
            st.current_y = screen_height as f64;
            st.completed = true;
            st.is_dragging = false;
            st.fullscreen_mode = true;
            ClickEffect::Redraw
        }
        Some(ToolbarIcon::Area) => {
            st.active_tool_index = TOOLBAR_AREA_INDEX;
            st.intent = OverlayIntent::Area;
            let screen_w = screen_width as f64;
            let screen_h = screen_height as f64;
            let width = DEFAULT_SELECTION_WIDTH
                .min(screen_w)
                .max(MIN_SELECTION_WIDTH.min(screen_w));
            let height = DEFAULT_SELECTION_HEIGHT
                .min(screen_h)
                .max(MIN_SELECTION_HEIGHT.min(screen_h));
            st.start_x = ((screen_w - width) / 2.0).max(0.0);
            st.start_y = ((screen_h - height) / 2.0).max(0.0);
            st.current_x = st.start_x + width;
            st.current_y = st.start_y + height;
            st.completed = true;
            st.is_dragging = false;
            st.fullscreen_mode = false;
            st.recording.panel_open = false;
            ClickEffect::Redraw
        }
        Some(ToolbarIcon::Recording) => {
            st.active_tool_index = TOOLBAR_RECORDING_INDEX;
            st.recording.panel_open = true;
            st.intent = OverlayIntent::Record;
            clear_toolbar_hover(st);
            ClickEffect::Redraw
        }
        Some(ToolbarIcon::Timer) => {
            if !st.timer_delay_active {
                st.timer_delay_active = true;
                if st.capture_delay_seconds <= 0 {
                    st.capture_delay_seconds = 5;
                }
            } else {
                st.capture_delay_seconds = match st.capture_delay_seconds {
                    3 => 5,
                    5 => 10,
                    _ => 0,
                };
            }
            st.timer_delay_active = st.capture_delay_seconds > 0;
            st.hover_tool_index = None;
            ClickEffect::Redraw
        }
        Some(ToolbarIcon::Scroll) => {
            st.top_bar_crop_menu_open = false;
            st.hovered_top_bar_crop_item = -1;
            st.scroll_popup_open = true;
            st.active_tool_index = TOOLBAR_SCROLL_INDEX;
            st.intent = OverlayIntent::Area;
            st.hover_tool_index = None;
            ClickEffect::Redraw
        }
        Some(ToolbarIcon::Ocr) => {
            st.active_tool_index = crate::overlay::icons::TOOLBAR_OCR_INDEX;
            st.intent = OverlayIntent::Ocr;
            st.hover_tool_index = None;
            ClickEffect::Redraw
        }
        _ => handle_panel_or_selection_click(st, n_press, x, y, record_hit),
    }
}

fn handle_panel_or_selection_click(
    st: &mut SelectorState,
    n_press: i32,
    x: f64,
    y: f64,
    record_hit: Option<RecordPanelTile>,
) -> ClickEffect {
    if let Some(tile) = record_hit {
        match tile {
            RecordPanelTile::Crop => {
                st.recording.crop_menu_open = !st.recording.crop_menu_open;
                st.recording.hovered_crop_menu_item = -1;
                st.recording.settings_menu_open = false;
                st.recording.settings_dropdown_open = None;
                st.recording.hovered_settings_dropdown_item = -1;
                st.recording.mic_volume_popup_open = false;
                st.recording.speaker_volume_popup_open = false;
                st.hover_tool_index = None;
            }
            RecordPanelTile::Controls => {
                st.recording.settings_menu_open = !st.recording.settings_menu_open;
                st.recording.hovered_settings_item = -1;
                st.recording.settings_dropdown_open = None;
                st.recording.hovered_settings_dropdown_item = -1;
                st.recording.crop_menu_open = false;
                st.recording.mic_volume_popup_open = false;
                st.recording.speaker_volume_popup_open = false;
                st.recording.hover_record_tile = None;
                st.hover_tool_index = None;
            }
            RecordPanelTile::Mic => {
                st.recording.mic_toggle = !st.recording.mic_toggle;
                st.recording.mic_volume_popup_open = false;
            }
            RecordPanelTile::Speaker => {
                st.recording.speaker_toggle = !st.recording.speaker_toggle;
                st.recording.speaker_volume_popup_open = false;
            }
            RecordPanelTile::Size => {}
            RecordPanelTile::RecordVideo => {
                let record_type = RecordingType::Video;
                if st.recording.selected_record_type == Some(record_type) {
                    let request = recording_request_from_state(st, record_type);
                    return ClickEffect::SendRecording(OverlaySelection::Recording(request));
                }
                st.recording.selected_record_type = Some(record_type);
                st.recording.crop_menu_open = false;
                st.recording.settings_menu_open = false;
                st.recording.settings_dropdown_open = None;
                st.recording.hovered_settings_dropdown_item = -1;
                st.recording.mic_volume_popup_open = false;
                st.recording.speaker_volume_popup_open = false;
                st.recording.hover_record_tile = None;
                st.hover_tool_index = None;
            }
        }
        return ClickEffect::Redraw;
    }

    if n_press == 2 && st.completed && is_inside_selection(x, y, current_selection_rect(st)) {
        ClickEffect::SendSelection
    } else {
        ClickEffect::None
    }
}

fn clear_toolbar_hover(st: &mut SelectorState) {
    st.hover_tool_index = None;
    st.hover_size_panel = false;
    st.hover_crop_panel = false;
}

#[cfg(test)]
mod tests {
    #[test]
    fn toolbar_owner_covers_tools_recording_tiles_and_double_click() {
        let source = include_str!("toolbar.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production toolbar owner");
        for tool in ["Fullscreen", "Area", "Recording", "Timer", "Scroll", "Ocr"] {
            assert!(
                production.contains(&format!("ToolbarIcon::{tool}")),
                "missing toolbar tool {tool}"
            );
        }
        assert!(
            production.contains("RecordPanelTile::Crop")
                && production.contains("RecordPanelTile::RecordVideo"),
            "toolbar owner must cover recording panel tiles"
        );
        assert!(
            production.contains("n_press == 2")
                && production.contains("ClickEffect::SendSelection")
                && production.contains("ClickEffect::SendRecording"),
            "toolbar owner must return delivery effects"
        );
        assert!(
            !production.contains("result_tx")
                && !production.contains("queue_draw")
                && !production.contains("window.close"),
            "toolbar state owner must not perform channel or GTK effects"
        );
    }

    #[test]
    fn toolbar_owner_routes_top_bar_pills_buttons_and_chrome() {
        let source = include_str!("toolbar.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production toolbar owner");
        assert!(
            production.contains("top_bar_aspect_at")
                && production.contains("top_bar_ratio_for_pill")
                && production.contains("apply_aspect_to_selection"),
            "toolbar owner must apply pill ratios to the selection"
        );
        assert!(
            production.contains("top_bar_button_at")
                && production.contains("TOP_BAR_CROP_BUTTON")
                && production.contains("TOP_BAR_CANCEL_BUTTON")
                && production.contains("ClickEffect::Cancel"),
            "toolbar owner must toggle the crop menu and cancel via X"
        );
        assert!(
            production.contains("point_in_top_bar")
                && production.contains("top_bar_crop_menu_contains"),
            "toolbar owner must swallow bar chrome and menu padding clicks"
        );
        assert!(
            !production.contains("ToolbarHit::CropPanel")
                && !production.contains("capture_crop_menu_open"),
            "toolbar owner must not use the legacy crop panel path"
        );
    }
}
