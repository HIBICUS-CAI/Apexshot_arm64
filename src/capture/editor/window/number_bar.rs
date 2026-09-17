//! Floating number-tool bar: numbering style, marker number, marker size.
//!
//! One contextual bar instead of one bar per marker. It stays docked in the
//! chrome band above the canvas while the Number tool is armed (or while the
//! Select tool holds a number marker), like the pen/arrow/shape/highlighter
//! bars — never parked over the marker itself, so it cannot cover the drawing.
//! Clicking an existing marker with the Number tool re-selects it, which is how
//! the bar comes back for an older marker. Mirrors the docked bars owned by
//! `window/mod.rs`.

use std::sync::{Arc, Mutex};

use gtk4::{
    glib, prelude::*, Align, Box as GtkBox, Button, DrawingArea, Entry, Label, Orientation,
};

use crate::capture::editor::{
    numbering_style::{NumberSize, NumberingStyle},
    state::EditorState,
    types::{AnnotationAction, Tool},
    ui_support::DockedBarInset,
};
use crate::i18n::t;

use super::floating_bar::{
    build_option_row, build_pill, dock_position, dock_reserve, popdown_for, queue_draw,
    set_bar_shown, set_dock_reserve, sync_option_selection, wire_option_rows, DockRefs,
};

const STYLE_ROW_CLASS: &str = "editor-number-style-option";
const STYLE_ACTIVE_CLASS: &str = "editor-number-style-option-active";
const SIZE_ROW_CLASS: &str = "editor-number-size-option";
const SIZE_ACTIVE_CLASS: &str = "editor-number-size-option-active";

pub(super) struct NumberBar {
    pub(super) root: GtkBox,
    style_label: Label,
    style_list: GtkBox,
    start_entry: Entry,
    size_label: Label,
    size_list: GtkBox,
}

pub(super) fn build_number_bar(
    state: &Arc<Mutex<EditorState>>,
    drawing_area: &DrawingArea,
) -> NumberBar {
    let root = GtkBox::new(Orientation::Horizontal, 8);
    root.add_css_class("editor-text-floating-bar");
    root.add_css_class("editor-number-floating-bar");
    root.set_halign(Align::Start);
    root.set_valign(Align::Start);
    set_bar_shown(&root, false);

    let style_pill = build_pill("Numbering style");
    for style in NumberingStyle::ALL {
        let row = build_option_row(&t(style.label()), "editor-number-style-check");
        row.add_css_class(STYLE_ROW_CLASS);
        style_pill.list.append(&row);
    }
    {
        let state = state.clone();
        let area = drawing_area.downgrade();
        wire_option_rows(&style_pill.list, STYLE_ROW_CLASS, move |index, button| {
            let style = NumberingStyle::from_index(index);
            let changed = state.lock().unwrap().set_numbering_style(style);
            popdown_for(button);
            if changed {
                queue_draw(&area);
            }
        });
    }

    let start_row = GtkBox::new(Orientation::Horizontal, 4);
    start_row.add_css_class("editor-number-bar-stepper");
    start_row.set_valign(Align::Center);
    let start_dec = Button::with_label("-");
    start_dec.set_focusable(false);
    start_dec.add_css_class("editor-number-start-stepper");
    let start_inc = Button::with_label("+");
    start_inc.set_focusable(false);
    start_inc.add_css_class("editor-number-start-stepper");
    let start_entry = Entry::new();
    start_entry.set_width_chars(3);
    start_entry.set_max_width_chars(4);
    // `Entry` implements both EntryExt and EditableExt, so name the one we want.
    gtk4::prelude::EntryExt::set_alignment(&start_entry, 0.5);
    start_entry.set_editable(false);
    start_entry.set_can_focus(false);
    start_entry.add_css_class("editor-number-start-entry");
    start_row.append(&start_dec);
    start_row.append(&start_entry);
    start_row.append(&start_inc);
    wire_stepper(&start_dec, -1, state, drawing_area);
    wire_stepper(&start_inc, 1, state, drawing_area);

    let size_pill = build_pill("Marker size");
    for size in NumberSize::ALL {
        let row = build_option_row(&t(size.label()), "editor-number-size-check");
        row.add_css_class(SIZE_ROW_CLASS);
        size_pill.list.append(&row);
    }
    {
        let state = state.clone();
        let area = drawing_area.downgrade();
        wire_option_rows(&size_pill.list, SIZE_ROW_CLASS, move |index, button| {
            let size = NumberSize::from_index(index);
            let changed = state.lock().unwrap().set_number_size(size);
            popdown_for(button);
            if changed {
                queue_draw(&area);
            }
        });
    }

    root.append(&style_pill.button);
    root.append(&start_row);
    root.append(&size_pill.button);

    NumberBar {
        root,
        style_label: style_pill.label,
        style_list: style_pill.list,
        start_entry,
        size_label: size_pill.label,
        size_list: size_pill.list,
    }
}

/// Own the bar's visibility and keep it docked in the chrome band above the
/// canvas, whether the Number tool is armed or a number marker is selected.
///
/// Like the pen/arrow/shape/highlighter bars it never anchors to the marker:
/// anchoring parks the bar over the drawing it just made (covering the top of
/// the image), while docking claims the reserved band so the image slides down
/// and the bar sits clear of it (see [`super::floating_bar::dock_position`]).
pub(super) fn install_number_bar_tick(
    bar: &NumberBar,
    drawing_area: &DrawingArea,
    state: &Arc<Mutex<EditorState>>,
    _transform: &Arc<Mutex<crate::capture::editor::types::ViewTransform>>,
    dock_refs: &DockRefs,
    inset: &DockedBarInset,
) {
    let root = bar.root.clone();
    let style_label = bar.style_label.clone();
    let style_list = bar.style_list.clone();
    let size_label = bar.size_label.clone();
    let size_list = bar.size_list.clone();
    let start_entry = bar.start_entry.clone();
    let state = state.clone();
    let inset = inset.clone();
    let dock_refs = DockRefs {
        scroller: dock_refs.scroller.clone(),
        drawing_area: dock_refs.drawing_area.clone(),
    };
    drawing_area.add_tick_callback(move |widget, _| {
        let (show, has_marker, style, size, start_display) = {
            let st = state.lock().unwrap();
            let number_tool = st.selected_tool == Tool::Number;
            let has_marker = matches!(
                st.selected_action(),
                Some(AnnotationAction::Number { .. })
            ) && (number_tool || st.selected_tool == Tool::Select);
            (
                number_tool || has_marker,
                has_marker,
                st.active_numbering_style(),
                st.active_number_size(),
                st.active_number_start_display(),
            )
        };

        // Show or hide first: a tick that bails out before revealing would
        // leave the bar transparent for good.
        set_bar_shown(&root, show);
        if !show {
            set_dock_reserve(&inset, "number", 0.0, widget);
            return glib::ControlFlow::Continue;
        }
        style_label.set_label(&short_style_label(style));
        sync_option_selection(&style_list, style.index(), STYLE_ACTIVE_CLASS);
        size_label.set_label(&t(size.label()));
        sync_option_selection(&size_list, size.index(), SIZE_ACTIVE_CLASS);
        if start_entry.text().as_str() != start_display.as_str() {
            start_entry.set_text(&start_display);
        }
        start_entry.set_tooltip_text(Some(&number_start_tooltip(has_marker)));

        // Claim the band above the canvas; the layout moves the image down under
        // it, so a press on the bar is never a press the image needed. The claim
        // happens before the bar is measured so the image and the bar move in the
        // same step, and the bar is only placed once GTK knows its size — placing
        // it at 0 would park it half a bar off centre for a frame.
        // Live size, like every other docked bar: a remembered guess would reserve
        // more (or less) canvas than the bar actually needs.
        let (bar_w, bar_h) = (root.width() as f64, root.height() as f64);
        set_dock_reserve(&inset, "number", dock_reserve(bar_h), widget);
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

/// Stepper tooltip: what the number in the field refers to in each mode.
fn number_start_tooltip(has_marker: bool) -> String {
    if has_marker {
        t("Number of the selected marker")
    } else {
        t("Number the next marker starts at")
    }
}

/// Compact pill label: "1, 2, 3, 4" rather than the popover's "1, 2, 3, 4...".
fn short_style_label(style: NumberingStyle) -> String {
    t(style.label()).trim_end_matches('.').trim().to_string()
}

fn wire_stepper(
    button: &Button,
    delta: i64,
    state: &Arc<Mutex<EditorState>>,
    drawing_area: &DrawingArea,
) {
    let state = state.clone();
    let area = drawing_area.downgrade();
    button.connect_clicked(move |_| {
        let changed = {
            let mut st = state.lock().unwrap();
            let next = i64::from(st.active_number_start()) + delta;
            st.set_active_number_start(next.clamp(1, MAX_NUMBER) as u32)
        };
        if changed {
            queue_draw(&area);
        }
    });
}

/// Matches the render path's numbering ceiling.
const MAX_NUMBER: i64 = 9999;

#[cfg(test)]
mod tests {
    fn production_source() -> &'static str {
        let source = include_str!("number_bar.rs");
        source.split("#[cfg(test)]").next().unwrap_or(source)
    }

    #[test]
    fn number_bar_is_single_and_stays_docked_above_the_canvas() {
        let source = production_source();
        assert!(
            source.contains("pub(super) fn build_number_bar(")
                && source.contains("pub(super) fn install_number_bar_tick(")
                && source.contains("st.selected_tool == Tool::Number")
                && source.contains("Some(AnnotationAction::Number { .. })")
                && source.contains("Tool::Select")
                && source.contains("dock_position(&dock_refs, bar_w)")
                && source.contains("dock_reserve(bar_h)"),
            "One bar should stay docked above the canvas (armed or with a marker selected) instead of anchoring over the drawing"
        );
        assert!(
            !source.contains("BAR_GAP")
                && !source.contains("view.offset_x")
                && !source.contains("marker_size.radius()")
                && !source.contains("position.x * view.scale"),
            "A docked bar must not anchor to the marker circle, or it covers the drawing"
        );
    }

    #[test]
    fn number_bar_controls_route_through_the_state_setters() {
        let source = production_source();
        assert!(
            source.contains("set_numbering_style(style)")
                && source.contains("set_number_size(size)")
                && source.contains("set_active_number_start("),
            "Bar controls must go through the state setters so the selected marker updates with the bar"
        );
    }

    #[test]
    fn number_bar_reuses_the_floating_bar_shell() {
        let source = production_source();
        assert!(
            source.contains("root.add_css_class(\"editor-text-floating-bar\");")
                && source.contains("root.add_css_class(\"editor-number-floating-bar\");")
                && source.contains("use super::floating_bar::{")
                && source.contains("build_pill(\"Numbering style\")")
                && source.contains("build_pill(\"Marker size\")"),
            "The number bar should reuse the shared floating-bar shell and pill builder"
        );
    }

    #[test]
    fn number_bar_tooltips_are_registered_for_translation() {
        let source = production_source();
        assert!(
            source.contains("t(\"Number of the selected marker\")")
                && source.contains("t(\"Number the next marker starts at\")"),
            "Conditional messages need static t() literals so the i18n catalog checker sees them"
        );
    }

    #[test]
    fn number_bar_shell_is_a_rounded_rectangle() {
        let css = include_str!("../css/06-text-actions.css");
        let start = css
            .find(".editor-number-floating-bar,")
            .expect("number bar shell rule");
        let end = css[start..]
            .find('}')
            .map(|offset| start + offset)
            .unwrap_or(css.len());
        let rule = &css[start..end];
        assert!(
            rule.contains("border-radius: 12px;") && !rule.contains("999px"),
            "The number bar should be a rounded rectangle like the other floating bars, not a full pill"
        );
    }
}
