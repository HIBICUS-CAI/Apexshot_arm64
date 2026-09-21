//! Overlay motion/leave: hover priority, cursors, popup ownership, slider drags.

use super::super::super::geometry::{
    clamp_point_to_bounds, current_selection_rect, cursor_name_for_handle, detect_resize_handle,
    is_inside_selection,
};
use super::super::super::hit_testing::{
    point_in_top_bar, toolbar_hit_at, top_bar_aspect_at, top_bar_button_at, top_bar_crop_item_at,
    top_bar_visible,
};
use super::super::super::layout::{
    compute_scroll_popup_layout, compute_window_picker_layout, ToolbarHit,
};
use super::super::super::state::{OverlayMode, SelectorState};
use gtk4::gdk;
use gtk4::prelude::*;
use gtk4::{ApplicationWindow, DrawingArea, EventControllerMotion};
use std::sync::{Arc, Mutex};

/// Install motion + leave controllers on the drawing area.
///
/// Preserves hover priority (menus → popups → toolbar → selection),
/// cursor selection, crosshair updates, and leave/reset of hover state.
pub(in crate::overlay::window) fn wire_selection_motion(
    window: &ApplicationWindow,
    state: Arc<Mutex<SelectorState>>,
    drawing_area: &DrawingArea,
    screen_width: i32,
    screen_height: i32,
) {
    let motion_controller = EventControllerMotion::new();
    let state_motion = state.clone();
    let drawing_area_weak_motion = drawing_area.downgrade();
    let window_weak_motion = window.downgrade();
    motion_controller.connect_motion(move |_, x, y| {
        let (cursor_name, hover_changed, _done) = {
            let mut st = state_motion.lock().unwrap();
            if st.overlay_mode == OverlayMode::CrosshairCapture {
                let (x, y) = clamp_point_to_bounds(x, y, screen_width as f64, screen_height as f64);
                st.current_x = x;
                st.current_y = y;
                drop(st);
                if let Some(da) = drawing_area_weak_motion.upgrade() {
                    da.queue_draw();
                }
                ("crosshair".to_string(), false, true)
            } else {
                let rect = current_selection_rect(&st);

                // Top-bar crop menu hover first (skipped while dragging so
                // drags pass underneath). Only claims the pointer on an item;
                // elsewhere execution falls through to bar/toolbar hover.
                if st.top_bar_crop_menu_open && !st.is_dragging {
                    let item = top_bar_crop_item_at(&st, screen_width as f64, x, y);
                    let next = item.map(|i| i as i32).unwrap_or(-1);
                    let changed = next != st.hovered_top_bar_crop_item;
                    if changed {
                        st.hovered_top_bar_crop_item = next;
                    }
                    st.hover_tool_index = None;
                    st.hover_size_panel = false;
                    st.hover_crop_panel = false;
                    if next >= 0 {
                        drop(st);
                        if let Some(win) = window_weak_motion.upgrade() {
                            if let Some(surf) = win.surface() {
                                let cursor = gdk::Cursor::from_name("pointer", None);
                                surf.set_cursor(cursor.as_ref());
                            }
                        }
                        if changed {
                            if let Some(da) = drawing_area_weak_motion.upgrade() {
                                da.queue_draw();
                            }
                        }
                        return;
                    }
                }
                if st.window_picker_open {
                    let n = st.windows.len();
                    let (center_x, center_y) = if st.completed || st.is_dragging {
                        let r = current_selection_rect(&st);
                        (r.left + r.width() / 2.0, r.top + r.height() / 2.0)
                    } else {
                        (screen_width as f64 / 2.0, screen_height as f64 / 2.0)
                    };
                    let picker = compute_window_picker_layout(
                        center_x,
                        center_y,
                        screen_width as f64,
                        screen_height as f64,
                        n,
                    );

                    let mut next_entry = -1;
                    if picker.panel.contains(x, y) && y >= picker.list_y {
                        let idx = ((y - picker.list_y) / picker.item_h) as i32;
                        if idx >= 0 && (idx as usize) < st.windows.len() {
                            next_entry = idx;
                        }
                    }

                    let changed = st.hovered_window_picker_entry != next_entry;
                    st.hovered_window_picker_entry = next_entry;
                    st.hovered_scroll_popup_close = false;
                    st.hovered_scroll_download = false;
                    st.hover_tool_index = None;
                    st.hover_size_panel = false;
                    st.hover_crop_panel = false;
                    ("pointer".to_string(), changed, true)
                } else if st.scroll_popup_open {
                    // Scroll popup hover handling
                    let (cx, cy) = if st.completed || st.is_dragging {
                        let r = current_selection_rect(&st);
                        (r.left + r.width() / 2.0, r.top + r.height() / 2.0)
                    } else {
                        (screen_width as f64 / 2.0, screen_height as f64 / 2.0)
                    };
                    let scroll = compute_scroll_popup_layout(cx, cy);

                    let next_hover_close = scroll.close.contains(x, y);
                    let changed_close = st.hovered_scroll_popup_close != next_hover_close;
                    if changed_close {
                        st.hovered_scroll_popup_close = next_hover_close;
                    }

                    // Download button hover
                    let next_hover_download = scroll.download.contains(x, y);
                    let changed_download = st.hovered_scroll_download != next_hover_download;
                    if changed_download {
                        st.hovered_scroll_download = next_hover_download;
                    }

                    st.hover_tool_index = None;
                    st.hover_size_panel = false;
                    st.hover_crop_panel = false;
                    ("pointer".to_string(), true, true)
                } else if top_bar_visible(&st)
                    && !st.is_dragging
                    && (top_bar_aspect_at(&st, screen_width as f64, x, y).is_some()
                        || top_bar_button_at(&st, screen_width as f64, x, y).is_some())
                {
                    // Top-center bar pills/buttons claim the pointer + highlight.
                    let pill = top_bar_aspect_at(&st, screen_width as f64, x, y);
                    let button = if pill.is_none() {
                        top_bar_button_at(&st, screen_width as f64, x, y)
                    } else {
                        None
                    };
                    let next_aspect = pill.map(|p| p as i32).unwrap_or(-1);
                    let next_button = button.map(|b| b as i32).unwrap_or(-1);
                    let changed = st.hovered_top_bar_aspect != next_aspect
                        || st.hovered_top_bar_button != next_button;
                    st.hovered_top_bar_aspect = next_aspect;
                    st.hovered_top_bar_button = next_button;
                    st.hover_tool_index = None;
                    st.hover_size_panel = false;
                    st.hover_crop_panel = false;
                    st.hovered_capture_crop_menu_item = -1;
                    ("pointer".to_string(), changed, true)
                } else {
                    let hit = toolbar_hit_at(
                        rect.left,
                        rect.top,
                        rect.width(),
                        rect.height(),
                        screen_width as f64,
                        screen_height as f64,
                        x,
                        y,
                    );
                    let hit =
                        if st.capture_menu_area_mode && matches!(hit, Some(ToolbarHit::Tool(_))) {
                            None
                        } else {
                            hit
                        };

                    let mut next_hovered_window = None;
                    if !st.completed && !st.is_dragging && hit.is_none() {
                        for (i, win) in st.windows.iter().enumerate() {
                            if x >= win.x as f64
                                && x <= (win.x + win.width) as f64
                                && y >= win.y as f64
                                && y <= (win.y + win.height) as f64
                            {
                                next_hovered_window = Some(i);
                                break;
                            }
                        }
                    }

                    let (
                        next_hover_tool_index,
                        next_hover_size_panel,
                        next_hover_crop_panel,
                        cursor_name,
                    ) = match hit {
                        // Legacy rail retired: Tool never hits (see hit_testing),
                        // so only the compat arms below remain.
                        Some(ToolbarHit::SizePanel) => (None, true, false, "default"),
                        Some(ToolbarHit::CropPanel) => (None, false, true, "pointer"),
                        None => {
                            let c = if st.completed || st.is_dragging {
                                detect_resize_handle(x, y, rect)
                                    .map(cursor_name_for_handle)
                                    .unwrap_or_else(|| {
                                        if is_inside_selection(x, y, rect) {
                                            "fleur"
                                        } else {
                                            "crosshair"
                                        }
                                    })
                            } else if next_hovered_window.is_some() {
                                "pointer"
                            } else if point_in_top_bar(&st, screen_width as f64, x, y) {
                                "default"
                            } else {
                                "crosshair"
                            };
                            (None, false, false, c)
                        }
                        _ => (None, false, false, "crosshair"),
                    };

                    let hover_changed = st.hover_tool_index != next_hover_tool_index
                        || st.hover_size_panel != next_hover_size_panel
                        || st.hover_crop_panel != next_hover_crop_panel
                        || st.hovered_window != next_hovered_window
                        || st.hovered_top_bar_aspect != -1
                        || st.hovered_top_bar_button != -1;

                    st.hover_tool_index = next_hover_tool_index;
                    st.hover_size_panel = next_hover_size_panel;
                    st.hover_crop_panel = next_hover_crop_panel;
                    st.hovered_top_bar_aspect = -1;
                    st.hovered_top_bar_button = -1;
                    st.hovered_window = next_hovered_window;
                    st.hovered_capture_crop_menu_item = -1;

                    (cursor_name.to_string(), hover_changed, false)
                }
            }
        };

        if let Some(win) = window_weak_motion.upgrade() {
            if let Some(surf) = win.surface() {
                let cursor = gdk::Cursor::from_name(&cursor_name, None);
                surf.set_cursor(cursor.as_ref());
            }
        }
        if hover_changed {
            if let Some(drawing_area) = drawing_area_weak_motion.upgrade() {
                drawing_area.queue_draw();
            }
        }
    });

    let state_motion_leave = state;
    let drawing_area_weak_leave = drawing_area.downgrade();
    let window_weak_leave = window.downgrade();
    motion_controller.connect_leave(move |_| {
        let mut st = state_motion_leave.lock().unwrap();
        let was_hovering = st.hover_tool_index.is_some()
            || st.hover_size_panel
            || st.hover_crop_panel
            || st.hovered_capture_crop_menu_item != -1
            || st.hovered_top_bar_aspect != -1
            || st.hovered_top_bar_button != -1
            || st.hovered_top_bar_crop_item != -1;
        st.hover_tool_index = None;
        st.hover_size_panel = false;
        st.hover_crop_panel = false;
        st.hovered_top_bar_aspect = -1;
        st.hovered_top_bar_button = -1;
        st.hovered_top_bar_crop_item = -1;
        st.hovered_capture_crop_menu_item = -1;
        drop(st);

        // Reset cursor
        if let Some(win) = window_weak_leave.upgrade() {
            if let Some(surf) = win.surface() {
                let cursor = gdk::Cursor::from_name("crosshair", None);
                surf.set_cursor(cursor.as_ref());
            }
        }
        if was_hovering {
            if let Some(drawing_area) = drawing_area_weak_leave.upgrade() {
                drawing_area.queue_draw();
            }
        }
    });

    drawing_area.add_controller(motion_controller);
}

#[cfg(test)]
mod tests {
    #[test]
    fn motion_owner_covers_hover_priority_cursors_and_leave() {
        let source = include_str!("motion.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production motion source");
        assert!(
            production.contains("fn wire_selection_motion"),
            "motion must own wire_selection_motion"
        );
        assert!(
            production.contains("connect_motion") && production.contains("connect_leave"),
            "motion must own motion + leave together"
        );
        assert!(
            production.contains("top_bar_crop_menu_open")
                && production.contains("window_picker_open")
                && production.contains("scroll_popup_open"),
            "motion must keep popup hover priority"
        );
        assert!(
            !production.contains("volume_slider_dragging")
                && !production.contains("settings_menu_open")
                && !production.contains("hover_record_tile"),
            "the retired recording-panel hovers must stay removed"
        );
        assert!(
            production.contains("cursor_name_for_handle")
                && production.contains("crosshair")
                && production.contains("fleur"),
            "motion must keep cursor selection"
        );
        assert!(
            production.contains("hover_tool_index = None")
                && production.contains("from_name(\"crosshair\""),
            "leave must reset hover state and crosshair cursor"
        );
    }
}
