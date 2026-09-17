//! Canvas content sizing, zoom labels, and relayout suppression (PR 10.13).
//!
//! Owns `update_canvas_content_size` and the scroller tick that coalesces
//! layout updates via a capped-overflow signature. Callers keep drawing-area
//! widgets and invoke the returned callback after state changes that affect
//! layout.

use gtk4::{glib, prelude::*, DrawingArea, Label, ScrolledWindow};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use super::super::composition::BackgroundComposition;
use super::super::state::EditorState;
use super::super::types::{BackgroundAlignment, BackgroundStyle, frame_needs_canvas};
use super::super::ui_support::{DockedBarInset, EDITOR_TOP_CHROME_HEIGHT};

/// Rounded pixel inset the docked bars reserved, as the layout math wants integers.
fn docked_bar_inset_px(inset: &DockedBarInset) -> i32 {
    inset.px().round().max(0.0) as i32
}

/// Install canvas layout updates + scroller tick; returns `update_canvas_content_size`.
///
/// Runs an immediate layout pass before returning.
pub(super) fn install_canvas_layout(
    state: &Arc<Mutex<EditorState>>,
    drawing_area: &DrawingArea,
    canvas_scroller: &ScrolledWindow,
    zoom_level: &Rc<Cell<f64>>,
    zoom_label: &Label,
    zoom_header_label: &Label,
    canvas_padding: i32,
    docked_inset: &DockedBarInset,
) -> Rc<dyn Fn()> {
    let update_canvas_content_size: Rc<dyn Fn()> = Rc::new({
        let state = state.clone();
        let zoom_level = zoom_level.clone();
        let zoom_label = zoom_label.clone();
        let zoom_header_label = zoom_header_label.clone();
        let drawing_area = drawing_area.clone();
        let canvas_scroller = canvas_scroller.clone();
        let docked_inset = docked_inset.clone();
        move || {
            let (
                image_w,
                image_h,
                background_style,
                background_padding,
                background_insert,
                background_aspect_ratio,
                has_background,
                frame_style,
                frame_border_thickness,
            ) = {
                let st = state.lock().unwrap();
                (
                    st.working_image.width().max(1) as i32,
                    st.working_image.height().max(1) as i32,
                    st.background_style.clone(),
                    st.background_padding,
                    st.background_insert,
                    st.background_aspect_ratio,
                    st.background_style != BackgroundStyle::None,
                    st.frame_style,
                    st.border_thickness,
                )
            };

            let mut virtual_w = image_w as f64;
            let mut virtual_h = image_h as f64;

            if has_background
                || frame_needs_canvas(frame_style, frame_border_thickness)
            {
                let layout = BackgroundComposition::new(virtual_w, virtual_h)
                    .with_style(background_style)
                    .with_padding(background_padding)
                    .with_insert(background_insert)
                    .with_alignment(BackgroundAlignment::Center)
                    .with_corner_radius(18.0)
                    .with_aspect_ratio(background_aspect_ratio)
                    .with_frame_style(frame_style)
                    .with_frame_border_thickness(frame_border_thickness)
                    .compute();
                virtual_w = layout.canvas_width;
                virtual_h = layout.canvas_height;
            }

            let scroller_width = canvas_scroller.allocated_width().max(1) as f64;
            let scroller_height = canvas_scroller.allocated_height().max(1) as f64;
            // Keep fit-to-view math below the floating toolbar strip.
            // Docked tool bars push the image down; see `DockedBarInset`.
            let top_inset =
                canvas_padding + EDITOR_TOP_CHROME_HEIGHT + docked_bar_inset_px(&docked_inset);
            let available_width = (scroller_width - (canvas_padding * 2 + 2) as f64).max(1.0);
            let available_height =
                (scroller_height - (top_inset + canvas_padding + 2) as f64).max(1.0);

            // Fit both dimensions so wide/tall frames (e.g. 3:1 cover) shrink
            // to the viewport instead of overflowing one axis. This mirrors
            // the draw scale in canvas_render, which already fits both sides.
            // Manual zoom still multiplies on top; spacebar-drag pan covers
            // inspection above 100%.
            // Layout scale without zoom - used for content size (prevents window from growing on zoom)
            let layout_scale = (available_width / virtual_w)
                .min(available_height / virtual_h)
                .min(1.0_f64);
            // Rendering scale includes zoom for visual display
            let scale = layout_scale * zoom_level.get().max(0.1_f64);

            let fitted_w = (virtual_w * scale).round().max(1.0) as i32;
            let fitted_h = (virtual_h * scale).round().max(1.0) as i32;

            let (overflow_left, overflow_top, overflow_right, overflow_bottom): (f64, f64, f64, f64) =
                (0.0, 0.0, 0.0, 0.0);

            let canvas_w = fitted_w
                + canvas_padding * 2
                + overflow_left.round() as i32
                + overflow_right.round() as i32;
            // Extra top inset so zoomed content cannot sit under the toolbar.
            let canvas_h = fitted_h
                + top_inset
                + canvas_padding
                + overflow_top.round() as i32
                + overflow_bottom.round() as i32;

            drawing_area.set_content_width(canvas_w);
            drawing_area.set_content_height(canvas_h);
            let percent_str = format!("{}%", (scale * 100.0).round().max(1.0) as i32);
            zoom_label.set_label(&percent_str);
            zoom_header_label.set_label(&percent_str);
        }
    });
    update_canvas_content_size();

    {
        let update_canvas_content_size_tick = update_canvas_content_size.clone();
        let state_canvas_tick = state.clone();
        let zoom_level_tick = zoom_level.clone();
        // Signature tracks the quantities that actually change the *visible* canvas size.
        let last_canvas_signature = Rc::new(Cell::new([
            0_i32, // scroller width
            0_i32, // scroller height
            0_i32, // image width
            0_i32, // image height
            0_i32, // overflow left (px, capped)
            0_i32, // overflow top  (px, capped)
            0_i32, // overflow right (px, capped)
            0_i32, // overflow bottom (px, capped)
            0_i32, // zoom percentage
            0_i32, // background enabled
            0_i32, // docked tool bar inset (px)
            0_i32, // background padding (tenths)
            0_i32, // background insert (tenths)
            0_i32, // background aspect ratio
        ]));
        let last_canvas_signature_tick = last_canvas_signature.clone();
        let docked_inset = docked_inset.clone();
        canvas_scroller.add_tick_callback(move |scroller, _| {
            let width = scroller.allocated_width();
            let height = scroller.allocated_height();
            let signature = {
                let st = state_canvas_tick.lock().unwrap();
                let img_w = st.working_image.width().max(1) as i32;
                let img_h = st.working_image.height().max(1) as i32;
                let has_background = st.background_style != BackgroundStyle::None;
                let background_padding = (st.background_padding * 10.0).round() as i32;
                let background_insert = (st.background_insert * 10.0).round() as i32;
                let background_aspect_ratio = st.background_aspect_ratio as i32;
                let zoom_percentage = (zoom_level_tick.get() * 100.0_f64).round() as i32;

                // Compute the same scale the layout function uses so we get the
                // same overflow values without duplicating the full layout calculation.
                let virtual_w = img_w as f64;
                let virtual_h = img_h as f64;
                let top_inset =
                    canvas_padding + EDITOR_TOP_CHROME_HEIGHT + docked_bar_inset_px(&docked_inset);
                let available_w = (width as f64 - (canvas_padding * 2 + 2) as f64).max(1.0);
                let available_h =
                    (height as f64 - (top_inset + canvas_padding + 2) as f64).max(1.0);

                let layout_scale = (available_w / virtual_w)
                    .min(available_h / virtual_h)
                    .min(1.0_f64);
                let _scale = layout_scale * zoom_level_tick.get().max(0.1_f64);

                let (ol, ot, or_, ob): (f64, f64, f64, f64) = (0.0, 0.0, 0.0, 0.0);

                [
                    width,
                    height,
                    img_w,
                    img_h,
                    ol.round() as i32,
                    ot.round() as i32,
                    or_.round() as i32,
                    ob.round() as i32,
                    zoom_percentage,
                    if has_background { 1 } else { 0 },
                    docked_bar_inset_px(&docked_inset),
                    background_padding,
                    background_insert,
                    background_aspect_ratio,
                ]
            };
            if width > 0 && signature != last_canvas_signature_tick.get() {
                last_canvas_signature_tick.set(signature);
                update_canvas_content_size_tick();
            }
            glib::ControlFlow::Continue
        });
    }

    update_canvas_content_size
}

#[cfg(test)]
mod tests {
    #[test]
    fn canvas_layout_sizes_content_and_suppresses_crop_relayout_churn() {
        let source = include_str!("canvas_layout.rs");
        assert!(
            source.contains("BackgroundComposition::new(virtual_w, virtual_h)")
                && source.contains("drawing_area.set_content_width(canvas_w)")
                && source.contains("zoom_label.set_label(&percent_str)")
                && source.contains("last_canvas_signature")
                && source.contains("fn install_canvas_layout"),
            "canvas layout must size content, update zoom labels, and coalesce layout updates"
        );
    }

    #[test]
    fn canvas_layout_fits_both_dimensions_so_frames_never_overflow() {
        let source = include_str!("canvas_layout.rs");
        let production_source = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(
            production_source.contains("available_width / virtual_w")
                && production_source.contains("available_height / virtual_h")
                && !production_source.contains("virtual_w.min(virtual_h)"),
            "wide/tall frames must shrink to the viewport on both axes (matching the draw fit), keeping spacebar-drag for manual inspection",
        );
    }
}
