//! Floating highlighter bar: text-aware/freehand mode and thickness.
//!
//! The highlighter paints, so unlike the text/number/obfuscate bars this one never
//! anchors to an element: it stays docked while the tool is armed (or while the
//! Select tool holds a stroke), which keeps it still while you draw stroke after
//! stroke instead of parking it over the last freehand bounding box. Docking is
//! image-aware — see [`super::floating_bar::dock_position`] — so the bar sits in
//! the empty chrome band above the canvas and never swallows a press meant for the
//! top of the image. It also shows the selected stroke's own thickness, so an
//! existing highlight can be re-thickened from the same place.

use std::sync::{Arc, Mutex};

use gtk4::{
    glib, prelude::*, Align, ApplicationWindow, Box as GtkBox, DrawingArea, Label, Orientation,
};

use crate::capture::editor::{
    pen_weight::{HighlighterMode, PenWeight},
    state::EditorState,
    types::Tool,
    ui_support::DockedBarInset,
};
use crate::i18n::t;

use super::build_arrow_thickness_preview;
use super::cursor::{set_highlighter_cursor, DEFAULT_HIGHLIGHTER_CURSOR_SIZE};
use super::floating_bar::{
    build_option_row, build_pill, dock_position, dock_reserve, popdown_for, queue_draw,
    set_bar_shown, set_dock_reserve, sync_option_selection, wire_option_rows, DockRefs,
};

/// Alpha the highlighter cursor preview uses (matches the toolbar wiring).
const CURSOR_ALPHA: f64 = 0.4;

const MODE_ROW_CLASS: &str = "editor-highlighter-mode-option";
const MODE_ACTIVE_CLASS: &str = "editor-highlighter-mode-option-active";
const WEIGHT_ROW_CLASS: &str = "editor-highlighter-weight-option";
const WEIGHT_ACTIVE_CLASS: &str = "editor-highlighter-weight-option-active";

pub(super) struct HighlighterBar {
    pub(super) root: GtkBox,
    mode_label: Label,
    mode_list: GtkBox,
    weight_label: Label,
    weight_list: GtkBox,
}

pub(super) fn build_highlighter_bar(
    state: &Arc<Mutex<EditorState>>,
    drawing_area: &DrawingArea,
    window: &ApplicationWindow,
) -> HighlighterBar {
    let root = GtkBox::new(Orientation::Horizontal, 8);
    root.add_css_class("editor-text-floating-bar");
    root.add_css_class("editor-highlighter-floating-bar");
    root.set_halign(Align::Start);
    root.set_valign(Align::Start);
    set_bar_shown(&root, false);

    let mode_pill = build_pill("Highlight mode");
    for mode in HIGHLIGHTER_MODES {
        let row = build_option_row(&highlighter_mode_label(mode), "editor-highlighter-check");
        row.add_css_class(MODE_ROW_CLASS);
        mode_pill.list.append(&row);
    }
    {
        let state = state.clone();
        let area = drawing_area.downgrade();
        let window = window.clone();
        wire_option_rows(&mode_pill.list, MODE_ROW_CLASS, move |index, button| {
            let Some(mode) = HIGHLIGHTER_MODES.get(index).copied() else {
                return;
            };
            let changed = {
                let mut st = state.lock().unwrap();
                let changed = st.set_highlighter_mode_and_check(mode);
                let (size, color) = highlighter_cursor_params(&st);
                drop(st);
                set_highlighter_cursor(&window, size, color);
                changed
            };
            popdown_for(button);
            if changed {
                queue_draw(&area);
            }
        });
    }

    let weight_pill = build_pill("Highlight thickness");
    for weight in PenWeight::ALL {
        let row = build_option_row(&t(weight.label()), "editor-highlighter-thickness-check");
        row.add_css_class(WEIGHT_ROW_CLASS);
        prepend_thickness_preview(&row, weight);
        weight_pill.list.append(&row);
    }
    {
        let state = state.clone();
        let area = drawing_area.downgrade();
        let window = window.clone();
        wire_option_rows(&weight_pill.list, WEIGHT_ROW_CLASS, move |index, button| {
            let Some(weight) = PenWeight::ALL.get(index).copied() else {
                return;
            };
            let changed = {
                let mut st = state.lock().unwrap();
                let changed = st.set_highlighter_weight(weight);
                let (size, color) = highlighter_cursor_params(&st);
                drop(st);
                set_highlighter_cursor(&window, size, color);
                changed
            };
            popdown_for(button);
            if changed {
                queue_draw(&area);
            }
        });
    }

    root.append(&mode_pill.button);
    root.append(&weight_pill.button);

    HighlighterBar {
        root,
        mode_label: mode_pill.label,
        mode_list: mode_pill.list,
        weight_label: weight_pill.label,
        weight_list: weight_pill.list,
    }
}

/// Mode rows in popover order; index order is the tick index the bar syncs to.
const HIGHLIGHTER_MODES: [HighlighterMode; 2] =
    [HighlighterMode::TextAware, HighlighterMode::Freehand];

/// Own the bar's visibility and keep its labels in sync with the tool state.
pub(super) fn install_highlighter_bar_tick(
    bar: &HighlighterBar,
    drawing_area: &DrawingArea,
    state: &Arc<Mutex<EditorState>>,
    dock_refs: &DockRefs,
    inset: &DockedBarInset,
) {
    let root = bar.root.clone();
    let mode_label = bar.mode_label.clone();
    let mode_list = bar.mode_list.clone();
    let weight_label = bar.weight_label.clone();
    let weight_list = bar.weight_list.clone();
    let state = state.clone();
    let inset = inset.clone();
    let dock_refs = DockRefs {
        scroller: dock_refs.scroller.clone(),
        drawing_area: dock_refs.drawing_area.clone(),
    };

    drawing_area.add_tick_callback(move |widget, _| {
        let (show, mode, weight) = {
            let st = state.lock().unwrap();
            let highlighter_tool = st.selected_tool == Tool::Highlighter;
            let has_stroke =
                st.selected_tool == Tool::Select && st.selected_highlighter_stroke_size().is_some();
            (
                highlighter_tool || has_stroke,
                st.highlighter_mode,
                st.active_highlighter_weight(),
            )
        };

        // Show or hide first: a tick that bails out before revealing would
        // leave the bar transparent for good.
        set_bar_shown(&root, show);
        if !show {
            set_dock_reserve(&inset, "highlighter", 0.0, widget);
            return glib::ControlFlow::Continue;
        }

        mode_label.set_label(&highlighter_mode_label(mode));
        sync_option_selection(
            &mode_list,
            HIGHLIGHTER_MODES
                .iter()
                .position(|candidate| *candidate == mode)
                .unwrap_or(0),
            MODE_ACTIVE_CLASS,
        );
        // Text-aware strokes follow the detected text height, so the preset reads "Auto".
        weight_label.set_label(&match weight {
            Some(weight) => t(weight.label()),
            None => t("Auto"),
        });
        sync_option_selection(
            &weight_list,
            weight.map(PenWeight::index).unwrap_or(usize::MAX),
            WEIGHT_ACTIVE_CLASS,
        );

        // Claim the band above the canvas; the layout moves the image down under
        // it, so a press on the bar is never a press the image needed. The claim
        // happens before the bar is measured so the image and the bar move in the
        // same step, and the bar is only placed once GTK knows its size — placing
        // it at 0 would park it half a bar off centre for a frame.
        let (bar_w, bar_h) = (root.width() as f64, root.height() as f64);
        set_dock_reserve(&inset, "highlighter", dock_reserve(bar_h), widget);
        let (left, top) = dock_position(&dock_refs, bar_w);
        if (root.margin_start() as f64 - left).abs() >= 1.0
            || (root.margin_top() as f64 - top).abs() >= 1.0
        {
            root.set_margin_start(left as i32);
            root.set_margin_top(top as i32);
        }
        glib::ControlFlow::Continue
    });
}

/// Brush size and color the cursor preview adopts for the current state.
fn highlighter_cursor_params(state: &EditorState) -> (f64, (f64, f64, f64, f64)) {
    let size = match state.highlighter_mode {
        HighlighterMode::TextAware => DEFAULT_HIGHLIGHTER_CURSOR_SIZE,
        HighlighterMode::Freehand => state.pen_weight.highlighter_stroke_width(),
    };
    (
        size,
        (
            state.selected_color.r,
            state.selected_color.g,
            state.selected_color.b,
            CURSOR_ALPHA,
        ),
    )
}

/// Static `t()` literals so the i18n catalog checker registers both messages.
fn highlighter_mode_label(mode: HighlighterMode) -> String {
    match mode {
        HighlighterMode::TextAware => t("Text-aware"),
        HighlighterMode::Freehand => t("Freehand"),
    }
}

/// Insert a stroke sample before the row's label, matching the sidebar rows.
fn prepend_thickness_preview(row: &gtk4::Button, weight: PenWeight) {
    let Some(content) = row.child() else {
        return;
    };
    let Ok(content) = content.downcast::<GtkBox>() else {
        return;
    };
    let preview = build_arrow_thickness_preview(weight, false);
    preview.set_margin_end(4);
    content.prepend(&preview);
}

#[cfg(test)]
mod tests {
    fn production_source() -> &'static str {
        let source = include_str!("highlighter_bar.rs");
        source.split("#[cfg(test)]").next().unwrap_or(source)
    }

    #[test]
    fn highlighter_bar_shows_mode_and_thickness_for_tool_and_selected_stroke() {
        let source = production_source();
        assert!(
            source.contains("pub(super) fn build_highlighter_bar(")
                && source.contains("pub(super) fn install_highlighter_bar_tick(")
                && source.contains("st.selected_tool == Tool::Highlighter")
                && source.contains("st.selected_tool == Tool::Select")
                && source.contains("st.selected_highlighter_stroke_size().is_some()")
                && source.contains("HighlighterMode::TextAware")
                && source.contains("HighlighterMode::Freehand")
                && source.contains("PenWeight::ALL"),
            "The highlighter bar shows mode plus thickness while the tool is armed, and for a selected stroke"
        );
    }

    #[test]
    fn highlighter_bar_controls_route_through_the_state_setters() {
        let source = production_source();
        assert!(
            source.contains("set_highlighter_mode_and_check(mode)")
                && source.contains("set_highlighter_weight(weight)")
                && source.contains("set_highlighter_cursor(&window, size, color)"),
            "Bar picks must go through the state setters and refresh the brush cursor"
        );
    }

    #[test]
    fn highlighter_bar_reserves_its_own_space_instead_of_anchoring_to_the_stroke() {
        let source = production_source();
        assert!(
            source.contains("let (left, top) = dock_position(&dock_refs, bar_w);")
                && source.contains("dock_reserve(bar_h)")
                && source.contains("0.0, widget);"),
            "The bar claims the reserved band while docked and hands it back when hidden"
        );
        // No per-element anchoring: a paint tool's bar does not follow a stroke.
        assert!(
            !source.contains("view.offset_x") && !source.contains("transform"),
            "a paint tool's bar should not anchor to a freehand bounding box"
        );
    }

    #[test]
    fn highlighter_bar_labels_are_registered_for_translation() {
        let source = production_source();
        assert!(
            source.contains("t(\"Text-aware\")")
                && source.contains("t(\"Freehand\")")
                && source.contains("t(\"Auto\")"),
            "Mode and auto-thickness labels need static t() literals for the i18n catalog checker"
        );
    }
}
