//! Floating focus bar: background intensity slider.
//!
//! Like the obfuscate slider this stays anchored below the active focus rect
//! (draft or selected) instead of docking, because it edits that specific
//! rect. There is no method pill — intensity is the only control — so this is
//! just the slider pill, reusing the obfuscate slider styling as-is.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use gtk4::{glib, prelude::*, Align, Box as GtkBox, DrawingArea, Orientation, Scale};

use crate::capture::editor::{
    color::{MAX_FOCUS_INTENSITY, MIN_FOCUS_INTENSITY},
    state::EditorState,
    types::{AnnotationAction, Tool, ViewTransform},
};
use crate::i18n::t;

use super::floating_bar::move_bar;

pub(super) struct FocusBar {
    pub(super) slider_bar: GtkBox,
    slider: Scale,
}

pub(super) fn build_focus_bar(
    state: &Arc<Mutex<EditorState>>,
    drawing_area: &DrawingArea,
    toolbar_slider: &Scale,
    rebuild: &Rc<dyn Fn()>,
) -> FocusBar {
    let slider = Scale::with_range(
        Orientation::Horizontal,
        MIN_FOCUS_INTENSITY,
        MAX_FOCUS_INTENSITY,
        0.5,
    );
    slider.add_css_class("editor-toolbar-size-slider");
    slider.add_css_class("editor-obfuscate-intensity-slider");
    slider.set_draw_value(false);
    slider.set_size_request(200, -1);
    slider.set_halign(Align::Fill);
    slider.set_valign(Align::Center);
    slider.set_hexpand(true);
    slider.set_value(state.lock().unwrap().current_focus_intensity());
    {
        let state = state.clone();
        let area = drawing_area.clone();
        let toolbar_slider = toolbar_slider.clone();
        let rebuild = rebuild.clone();
        slider.connect_value_changed(move |slider| {
            let value = slider.value();
            if state.lock().unwrap().set_active_size_without_rebuild(value) {
                if (toolbar_slider.value() - value).abs() > f64::EPSILON {
                    toolbar_slider.set_value(value);
                }
                rebuild();
                area.queue_draw();
            }
        });
    }

    let slider_bar = GtkBox::new(Orientation::Horizontal, 4);
    slider_bar.add_css_class("editor-text-floating-bar");
    slider_bar.add_css_class("editor-obfuscate-slider-bar");
    slider_bar.set_halign(Align::Start);
    slider_bar.set_valign(Align::Start);
    slider_bar.append(&slider);
    slider_bar.set_visible(false);

    FocusBar {
        slider_bar,
        slider: slider.clone(),
    }
}

/// Own the bar's visibility and placement below the active focus rect (draft
/// or selected). Owns visibility: Focus/Select tool plus a rect shows it.
pub(super) fn install_focus_bar_tick(
    bar: &FocusBar,
    drawing_area: &DrawingArea,
    state: &Arc<Mutex<EditorState>>,
    transform: &Arc<Mutex<ViewTransform>>,
    toolbar_slider: &Scale,
) {
    let slider_bar = bar.slider_bar.clone();
    let slider = bar.slider.clone();
    let state = state.clone();
    let transform = transform.clone();
    let toolbar_slider = toolbar_slider.clone();
    // Sticky max height (never underestimate, so the bar never covers the
    // rect); width tracks the last measurement so a max never off-centers it.
    let known = Rc::new(Cell::new((220.0f64, 48.0f64)));

    drawing_area.add_tick_callback(move |widget, _| {
        let (show_bar, rect_opt, intensity, view) = {
            let st = state.lock().unwrap();
            // Live draft first; otherwise the selected rect (reselect) when the
            // Focus or Select tool is active. Prefer the rect's own intensity
            // so re-editing shows the truth.
            let mut show = false;
            let mut rect_opt = None;
            let mut intensity = st.current_focus_intensity();
            if st.selected_tool == Tool::Focus {
                if let Some(AnnotationAction::Focus { rect, .. }) = st.draft_action() {
                    show = true;
                    rect_opt = Some(rect);
                }
            }
            if rect_opt.is_none() {
                if let Some(AnnotationAction::Focus { rect, intensity: i }) = st.selected_action() {
                    if matches!(st.selected_tool, Tool::Focus | Tool::Select) {
                        show = true;
                        rect_opt = Some(*rect);
                        intensity = *i;
                    }
                }
            }
            drop(st);
            let view = *transform.lock().unwrap();
            (show, rect_opt, intensity, view)
        };
        let Some(rect) = rect_opt.filter(|_| show_bar) else {
            if slider_bar.is_visible() {
                slider_bar.set_visible(false);
            }
            return glib::ControlFlow::Continue;
        };
        slider.set_tooltip_text(Some(&t("Focus background intensity")));
        if (slider.value() - intensity).abs() > f64::EPSILON {
            slider.set_value(intensity);
        }
        // Keep the toolbar slider in sync while the floating pill owns the tool.
        if (toolbar_slider.value() - intensity).abs() > f64::EPSILON {
            toolbar_slider.set_range(MIN_FOCUS_INTENSITY, MAX_FOCUS_INTENSITY);
            toolbar_slider.set_value(intensity);
        }
        toolbar_slider.set_tooltip_text(Some(&t("Focus background intensity")));

        let area_w = widget.width() as f64;
        let area_h = widget.height() as f64;
        let y = rect.y as f64 * view.scale + view.offset_y;
        let rect_h = rect.height as f64 * view.scale;
        let rect_cx =
            rect.x as f64 * view.scale + view.offset_x + rect.width as f64 * view.scale / 2.0;
        let gap = 12.0;
        let (mut slider_w, mut slider_h) = known.get();
        let (sbw, sbh) = (slider_bar.width() as f64, slider_bar.height() as f64);
        if sbw > 1.0 {
            slider_w = sbw;
        }
        if sbh > 1.0 {
            slider_h = slider_h.max(sbh);
        }
        known.set((slider_w, slider_h));
        // Below the rect; flip above only when there is no room.
        let mut slider_top = y + rect_h + gap;
        if slider_top + slider_h > area_h {
            slider_top = y - slider_h - gap;
        }
        if slider_top < 0.0 {
            slider_top = (area_h - slider_h).max(0.0);
        }
        let slider_left = (rect_cx - slider_w / 2.0)
            .max(0.0)
            .min((area_w - slider_w).max(0.0));
        move_bar(&slider_bar, slider_left, slider_top);
        if !slider_bar.is_visible() {
            slider_bar.set_visible(true);
        }
        glib::ControlFlow::Continue
    });
}

#[cfg(test)]
mod tests {
    fn production_source() -> &'static str {
        let source = include_str!("focus_bar.rs");
        source.split("#[cfg(test)]").next().unwrap_or(source)
    }

    #[test]
    fn focus_bar_is_a_slider_only_rect_editor() {
        let source = production_source();
        assert!(
            source.contains("pub(super) fn build_focus_bar(")
                && source.contains("pub(super) fn install_focus_bar_tick(")
                && source.contains("st.selected_tool == Tool::Focus")
                && source.contains("AnnotationAction::Focus {")
                && source.contains("st.draft_action()")
                && source.contains("current_focus_intensity()")
                && source.contains("t(\"Focus background intensity\")"),
            "The focus bar edits intensity for the draft or selected rect, nothing else"
        );
    }

    #[test]
    fn focus_bar_stays_anchored_to_its_rect() {
        let source = production_source();
        assert!(
            source.contains("rect.x as f64 * view.scale + view.offset_x")
                && source.contains("set_visible(false)")
                && !source.contains("set_bar_shown(")
                && !source.contains("dock_position(")
                && !source.contains("build_pill("),
            "A slider-only rect editor follows its rect with set_visible, never the docked band"
        );
    }
}
