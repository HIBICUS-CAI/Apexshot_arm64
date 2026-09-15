//! Floating shape bar: thickness for the line, box, and circle tools.
//!
//! One bar covers all three: thickness is their only pill-worthy control, so
//! three copies would triple the tick for an identical pill. Like the pen bar
//! it never anchors to an element — it stays docked in the chrome band above
//! the canvas (see [`super::floating_bar::dock_position`]) while a shape tool
//! is armed or the Select tool holds a shape, and it shows that shape's own
//! thickness so an existing outline can be re-thickened from the same place.

use std::sync::{Arc, Mutex};

use gtk4::{glib, prelude::*, Align, Box as GtkBox, DrawingArea, Label, Orientation};

use crate::capture::editor::{
    pen_weight::PenWeight, state::EditorState, types::AnnotationAction, types::Tool,
    ui_support::DockedBarInset,
};
use crate::i18n::t;

use super::build_arrow_thickness_preview;
use super::floating_bar::{
    build_option_row, build_pill, dock_position, dock_reserve, popdown_for, queue_draw,
    set_bar_shown, set_dock_reserve, sync_option_selection, wire_option_rows, DockRefs,
};

/// Shape thickness steps, matching the line/arrow inspector lists
/// (`window/mod.rs`, `window/events/options.rs`).
const SHAPE_SIZES: [(PenWeight, f64); 4] = [
    (PenWeight::Small, 2.0),
    (PenWeight::Medium, 4.0),
    (PenWeight::Large, 7.0),
    (PenWeight::ExtraLarge, 12.0),
];

fn weight_for_size(size: f64) -> PenWeight {
    SHAPE_SIZES
        .into_iter()
        .min_by(|a, b| (a.1 - size).abs().total_cmp(&(b.1 - size).abs()))
        .map(|(weight, _)| weight)
        .unwrap_or_default()
}

fn size_for_weight(weight: PenWeight) -> f64 {
    SHAPE_SIZES
        .into_iter()
        .find_map(|(candidate, size)| (candidate == weight).then_some(size))
        .unwrap_or(4.0)
}

const WEIGHT_ROW_CLASS: &str = "editor-shape-weight-option";
const WEIGHT_ACTIVE_CLASS: &str = "editor-shape-weight-option-active";

pub(super) struct ShapeBar {
    pub(super) root: GtkBox,
    weight_label: Label,
    weight_list: GtkBox,
}

pub(super) fn build_shape_bar(
    state: &Arc<Mutex<EditorState>>,
    drawing_area: &DrawingArea,
) -> ShapeBar {
    let root = GtkBox::new(Orientation::Horizontal, 8);
    root.add_css_class("editor-text-floating-bar");
    root.add_css_class("editor-shape-floating-bar");
    root.set_halign(Align::Start);
    root.set_valign(Align::Start);
    set_bar_shown(&root, false);

    let weight_pill = build_pill("Stroke Thickness");
    for weight in PenWeight::ALL {
        let row = build_option_row(&t(weight.label()), "editor-shape-thickness-check");
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
            let changed = {
                let mut st = state.lock().unwrap();
                let size = size_for_weight(weight);
                let default_changed = st.set_stroke_size(size);
                let selected_changed = st.set_selected_action_stroke_size(size);
                default_changed || selected_changed
            };
            popdown_for(button);
            if changed {
                queue_draw(&area);
            }
        });
    }

    root.append(&weight_pill.button);

    ShapeBar {
        root,
        weight_label: weight_pill.label,
        weight_list: weight_pill.list,
    }
}

/// Own the bar's visibility and keep the thickness label in sync with the tool state.
pub(super) fn install_shape_bar_tick(
    bar: &ShapeBar,
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
            let shape_tool = matches!(st.selected_tool, Tool::Line | Tool::Box | Tool::Circle);
            let selected_size = match st.selected_action() {
                Some(
                    AnnotationAction::Line { stroke_size, .. }
                    | AnnotationAction::Box { stroke_size, .. }
                    | AnnotationAction::Circle { stroke_size, .. },
                ) if st.selected_tool == Tool::Select => Some(*stroke_size),
                _ => None,
            };
            let size = selected_size.unwrap_or(st.stroke_size);
            (shape_tool || selected_size.is_some(), weight_for_size(size))
        };

        // Show or hide first: a tick that bails out before revealing would
        // leave the bar transparent for good.
        set_bar_shown(&root, show);
        if !show {
            set_dock_reserve(&inset, "shape", 0.0, widget);
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
        set_dock_reserve(&inset, "shape", dock_reserve(bar_h), widget);
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
        let source = include_str!("shape_bar.rs");
        source.split("#[cfg(test)]").next().unwrap_or(source)
    }

    #[test]
    fn shape_bar_offers_thickness_for_line_box_and_circle() {
        let source = production_source();
        assert!(
            source.contains("pub(super) fn build_shape_bar(")
                && source.contains("pub(super) fn install_shape_bar_tick(")
                && source.contains("Tool::Line | Tool::Box | Tool::Circle")
                && source.contains("st.selected_tool == Tool::Select")
                && source.contains("AnnotationAction::Line { stroke_size, .. }")
                && source.contains("AnnotationAction::Box { stroke_size, .. }")
                && source.contains("AnnotationAction::Circle { stroke_size, .. }")
                && source.contains("PenWeight::ALL"),
            "One bar covers the line, box, and circle thickness while armed and for a selected shape"
        );
    }

    #[test]
    fn shape_bar_routes_through_the_state_setters() {
        let source = production_source();
        assert!(
            source.contains("set_stroke_size(size)")
                && source.contains("set_selected_action_stroke_size(size)")
                && source.contains("dock_position(")
                && source.contains("&dock_refs,"),
            "Shape picks must go through the state setters and use the shared docked placement"
        );
    }
}
