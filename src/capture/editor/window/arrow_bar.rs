//! Floating arrow bar: arrow style.
//!
//! Like the highlighter bar this one never anchors to an element — the arrow is
//! painted with a drag, and a diagonal bounding box is no place to hang
//! controls. It stays docked in the chrome band above the canvas (see
//! [`super::floating_bar::dock_position`]) while the tool is armed or the
//! Select tool holds an arrow, and it shows that arrow's own style so an
//! existing arrow can be restyled from the same place. Thickness is not here:
//! it rides the toolbar's stroke-size slider.

use std::sync::{Arc, Mutex};

use gtk4::{glib, prelude::*, Align, Box as GtkBox, DrawingArea, Label, Orientation};

use crate::capture::editor::{
    state::EditorState, types::AnnotationAction, types::ArrowStyle, types::Tool,
    ui_support::DockedBarInset,
};
use crate::i18n::t;

use super::floating_bar::{
    build_option_row, build_pill, dock_position, dock_reserve, popdown_for, queue_draw,
    set_bar_shown, set_dock_reserve, sync_option_selection, wire_option_rows, DockRefs,
};

const STYLE_ROW_CLASS: &str = "editor-arrow-style-option";
const STYLE_ACTIVE_CLASS: &str = "editor-arrow-style-option-active";

pub(super) struct ArrowBar {
    pub(super) root: GtkBox,
    style_label: Label,
    style_list: GtkBox,
}

pub(super) fn build_arrow_bar(
    state: &Arc<Mutex<EditorState>>,
    drawing_area: &DrawingArea,
) -> ArrowBar {
    let root = GtkBox::new(Orientation::Horizontal, 8);
    root.add_css_class("editor-text-floating-bar");
    root.add_css_class("editor-arrow-floating-bar");
    root.set_halign(Align::Start);
    root.set_valign(Align::Start);
    set_bar_shown(&root, false);

    let style_pill = build_pill("Arrow style");
    for style in ArrowStyle::ALL {
        let row = build_option_row(&t(style.display_name()), "editor-arrow-style-check");
        row.add_css_class(STYLE_ROW_CLASS);
        style_pill.list.append(&row);
    }
    {
        let state = state.clone();
        let area = drawing_area.downgrade();
        wire_option_rows(&style_pill.list, STYLE_ROW_CLASS, move |index, button| {
            let Some(style) = ArrowStyle::ALL.get(index).copied() else {
                return;
            };
            let changed = {
                let mut st = state.lock().unwrap();
                let before = st.arrow_style;
                st.set_arrow_style(style);
                let selected_changed = st.set_selected_arrow_style(style);
                before != style || selected_changed
            };
            popdown_for(button);
            if changed {
                queue_draw(&area);
            }
        });
    }

    root.append(&style_pill.button);

    ArrowBar {
        root,
        style_label: style_pill.label,
        style_list: style_pill.list,
    }
}

/// Own the bar's visibility and keep its label in sync with the tool state.
pub(super) fn install_arrow_bar_tick(
    bar: &ArrowBar,
    drawing_area: &DrawingArea,
    state: &Arc<Mutex<EditorState>>,
    dock_refs: &DockRefs,
    inset: &DockedBarInset,
) {
    let root = bar.root.clone();
    let style_label = bar.style_label.clone();
    let style_list = bar.style_list.clone();
    let state = state.clone();
    let inset = inset.clone();
    let dock_refs = DockRefs {
        scroller: dock_refs.scroller.clone(),
        drawing_area: dock_refs.drawing_area.clone(),
    };

    drawing_area.add_tick_callback(move |widget, _| {
        let (show, style) = {
            let st = state.lock().unwrap();
            let arrow_tool = st.selected_tool == Tool::Arrow;
            let selected = match st.selected_action() {
                Some(AnnotationAction::Arrow { style, .. }) if st.selected_tool == Tool::Select => {
                    Some(*style)
                }
                _ => None,
            };
            let show = arrow_tool || selected.is_some();
            (show, selected.unwrap_or(st.arrow_style))
        };

        // Show or hide first: a tick that bails out before revealing would
        // leave the bar transparent for good.
        set_bar_shown(&root, show);
        if !show {
            set_dock_reserve(&inset, "arrow", 0.0, widget);
            return glib::ControlFlow::Continue;
        }

        style_label.set_label(&t(style.display_name()));
        sync_option_selection(
            &style_list,
            ArrowStyle::ALL
                .iter()
                .position(|candidate| *candidate == style)
                .unwrap_or(0),
            STYLE_ACTIVE_CLASS,
        );

        // Claim the band above the canvas; the layout moves the image down under
        // it, so a press on the bar is never a press the image needed. The claim
        // happens before the bar is measured so the image and the bar move in the
        // same step, and the bar is only placed once GTK knows its size — placing
        // it at 0 would park it half a bar off centre for a frame.
        let (bar_w, bar_h) = (root.width() as f64, root.height() as f64);
        set_dock_reserve(&inset, "arrow", dock_reserve(bar_h), widget);
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

#[cfg(test)]
mod tests {
    fn production_source() -> &'static str {
        let source = include_str!("arrow_bar.rs");
        source.split("#[cfg(test)]").next().unwrap_or(source)
    }

    #[test]
    fn arrow_bar_offers_style_for_the_tool_and_a_selected_arrow() {
        let source = production_source();
        assert!(
            source.contains("pub(super) fn build_arrow_bar(")
                && source.contains("pub(super) fn install_arrow_bar_tick(")
                && source.contains("st.selected_tool == Tool::Arrow")
                && source.contains("st.selected_tool == Tool::Select")
                && source.contains("AnnotationAction::Arrow {")
                && source.contains("ArrowStyle::ALL"),
            "The arrow bar replaces the inspector style list while the tool is armed and for a selected arrow"
        );
    }

    #[test]
    fn arrow_bar_carries_style_only_and_reserves_its_own_space() {
        let source = production_source();
        assert!(
            !source.contains("PenWeight") && !source.contains("set_stroke_size"),
            "Thickness rides the toolbar slider, so the arrow bar must not own a thickness pill"
        );
        assert!(
            source.contains("dock_position(")
                && source.contains("dock_reserve(bar_h)")
                && source.contains("0.0, widget);"),
            "The bar claims the reserved band while docked and hands it back when hidden"
        );
    }

    #[test]
    fn arrow_bar_routes_through_the_state_setters() {
        let source = production_source();
        assert!(
            source.contains("set_arrow_style(style)")
                && source.contains("set_selected_arrow_style(style)")
                && source.contains("dock_position(")
                && source.contains("&dock_refs,"),
            "Arrow picks must go through the state setters and use the shared docked placement"
        );
    }
}
