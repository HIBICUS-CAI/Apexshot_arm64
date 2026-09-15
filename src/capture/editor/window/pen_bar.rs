//! Floating pen bar: stroke thickness.
//!
//! Like the highlighter bar this one never anchors to an element — the pen paints, and
//! a freehand bounding box is no place to hang controls. It stays docked in the chrome
//! band above the canvas (see [`super::floating_bar::dock_position`]) while the tool is
//! armed or the Select tool holds a pen stroke, and it shows that stroke's own
//! thickness so an existing line can be re-thickened from the same place. The pen's
//! thickness list lives in the right inspector today; this bar replaces reaching for it.

use std::sync::{Arc, Mutex};

use gtk4::{glib, prelude::*, Align, Box as GtkBox, DrawingArea, Label, Orientation};

use crate::capture::editor::{
    pen_weight::PenWeight, state::EditorState, types::Tool, ui_support::DockedBarInset,
};
use crate::i18n::t;

use super::build_arrow_thickness_preview;
use super::floating_bar::{
    build_option_row, build_pill, dock_position, dock_reserve, popdown_for, queue_draw,
    set_bar_shown, set_dock_reserve, sync_option_selection, wire_option_rows, DockRefs,
};

const WEIGHT_ROW_CLASS: &str = "editor-pen-weight-option";
const WEIGHT_ACTIVE_CLASS: &str = "editor-pen-weight-option-active";

pub(super) struct PenBar {
    pub(super) root: GtkBox,
    weight_label: Label,
    weight_list: GtkBox,
}

pub(super) fn build_pen_bar(state: &Arc<Mutex<EditorState>>, drawing_area: &DrawingArea) -> PenBar {
    let root = GtkBox::new(Orientation::Horizontal, 8);
    root.add_css_class("editor-text-floating-bar");
    root.add_css_class("editor-pen-floating-bar");
    root.set_halign(Align::Start);
    root.set_valign(Align::Start);
    set_bar_shown(&root, false);

    let weight_pill = build_pill("Pen thickness");
    for weight in PenWeight::ALL {
        let row = build_option_row(&t(weight.label()), "editor-pen-thickness-check");
        row.add_css_class(WEIGHT_ROW_CLASS);
        prepend_thickness_preview(&row, weight);
        weight_pill.list.append(&row);
    }
    {
        let state = state.clone();
        let area = drawing_area.downgrade();
        wire_option_rows(&weight_pill.list, WEIGHT_ROW_CLASS, move |index, button| {
            let Some(weight) = PenWeight::ALL.get(index).copied() else {
                return;
            };
            let changed = state.lock().unwrap().set_pen_weight_and_apply(weight);
            popdown_for(button);
            if changed {
                queue_draw(&area);
            }
        });
    }

    root.append(&weight_pill.button);

    PenBar {
        root,
        weight_label: weight_pill.label,
        weight_list: weight_pill.list,
    }
}

/// Own the bar's visibility and keep the thickness label in sync with the tool state.
pub(super) fn install_pen_bar_tick(
    bar: &PenBar,
    drawing_area: &DrawingArea,
    state: &Arc<Mutex<EditorState>>,
    dock_refs: &DockRefs,
    inset: &DockedBarInset,
) {
    let root = bar.root.clone();
    let weight_label = bar.weight_label.clone();
    let weight_list = bar.weight_list.clone();
    let state = state.clone();
    let inset = inset.clone();
    let dock_refs = DockRefs {
        scroller: dock_refs.scroller.clone(),
        drawing_area: dock_refs.drawing_area.clone(),
    };

    drawing_area.add_tick_callback(move |widget, _| {
        let (show, weight) = {
            let st = state.lock().unwrap();
            let pen_tool = st.selected_tool == Tool::Pen;
            let has_stroke =
                st.selected_tool == Tool::Select && st.selected_pen_stroke_size().is_some();
            (pen_tool || has_stroke, st.active_pen_weight())
        };

        if !show {
            set_bar_shown(&root, false);
            set_dock_reserve(&inset, "pen", 0.0, widget);
            return glib::ControlFlow::Continue;
        }

        weight_label.set_label(&t(weight.label()));
        sync_option_selection(&weight_list, weight.index(), WEIGHT_ACTIVE_CLASS);

        // Claim the band above the canvas; the layout moves the image down under
        // it, so a press on the bar is never a press the image needed. The claim
        // happens before the bar is measured so the image and the bar move in the
        // same step, and the bar is only placed once GTK knows its size — placing
        // it at 0 would park it half a bar off centre for a frame.
        let (bar_w, bar_h) = (root.width() as f64, root.height() as f64);
        set_dock_reserve(&inset, "pen", dock_reserve(bar_h), widget);
        let (left, top) = dock_position(&dock_refs, bar_w);
        if (root.margin_start() as f64 - left).abs() >= 1.0
            || (root.margin_top() as f64 - top).abs() >= 1.0
        {
            root.set_margin_start(left as i32);
            root.set_margin_top(top as i32);
        }
        set_bar_shown(&root, true);
        glib::ControlFlow::Continue
    });
}

/// Insert a stroke sample before the row's label, matching the inspector rows.
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
        let source = include_str!("pen_bar.rs");
        source.split("#[cfg(test)]").next().unwrap_or(source)
    }

    #[test]
    fn pen_bar_offers_thickness_for_the_tool_and_a_selected_stroke() {
        let source = production_source();
        assert!(
            source.contains("pub(super) fn build_pen_bar(")
                && source.contains("pub(super) fn install_pen_bar_tick(")
                && source.contains("st.selected_tool == Tool::Pen")
                && source.contains("st.selected_tool == Tool::Select")
                && source.contains("st.selected_pen_stroke_size().is_some()")
                && source.contains("PenWeight::ALL"),
            "The pen bar replaces the inspector thickness list while the tool is armed and for a selected stroke"
        );
    }

    #[test]
    fn pen_bar_routes_through_the_state_setter() {
        let source = production_source();
        assert!(
            source.contains("set_pen_weight_and_apply(weight)")
                && source.contains("dock_position(")
                && source.contains("&dock_refs,"),
            "Pen picks must go through the state setter and use the shared docked placement"
        );
    }
}
