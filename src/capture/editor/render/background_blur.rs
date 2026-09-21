//! Appearance "Background blur".
//!
//! The blur belongs to the background layer: it softens the fill and nothing
//! else. The card, its backings, and its shadow composite afterwards, and the
//! grain is painted after the blur, so noise stays crisp on top of a blurred
//! background. The static canvas, the static export, and the Motion renderer
//! share one radius, so all three previews match.

use gtk4::cairo::{Context, Filter, Format, ImageSurface};
use image::RgbaImage;

use super::cairo_argb_to_rgba_image;
use super::effects::apply_blur_rect_to_buffer;
use super::rgba_image_to_surface;
use crate::capture::editor::types::Rect;

/// Blur radius in canvas pixels at a 100% slider. Motion blurs its fill with
/// the same peak (`blur * 32.0`), so the two editors agree.
pub const BACKGROUND_BLUR_MAX_RADIUS: f64 = 32.0;

/// Radius in the target image's own pixels. `radius_scale` is that image's
/// pixels per canvas pixel, so a screen-sized preview surface and a
/// full-resolution export blur by the same visible amount.
fn blur_radius(amount: f64, radius_scale: f64) -> usize {
    let amount = amount.clamp(0.0, 1.0);
    if amount <= 0.001 {
        return 0;
    }
    (amount * BACKGROUND_BLUR_MAX_RADIUS * radius_scale.max(0.01))
        .round()
        .max(1.0) as usize
}

/// Blur a background image in place. Three passes approximate a Gaussian, the
/// same way the Motion renderer blurs its fill.
pub fn apply_background_blur(image: &mut RgbaImage, amount: f64, radius_scale: f64) {
    let radius = blur_radius(amount, radius_scale);
    if radius == 0 || image.width() == 0 || image.height() == 0 {
        return;
    }
    let rect = Rect {
        x: 0,
        y: 0,
        width: image.width() as i32,
        height: image.height() as i32,
    };
    for _ in 0..3 {
        apply_blur_rect_to_buffer(image, rect, radius);
    }
}

/// Blurred copy of a background surface, for the canvas preview. The source is
/// rendered into a private surface first: the Motion runtime shares its decoded
/// wallpaper, so the pixels cannot be borrowed in place. Work is bounded to
/// `max_edge`, which is all a screen-sized preview can show.
pub fn blur_background_surface(
    source: &ImageSurface,
    amount: f64,
    canvas_width: f64,
    max_edge: f64,
) -> Option<ImageSurface> {
    let (width, height) = (source.width(), source.height());
    if width < 1 || height < 1 || blur_radius(amount, 1.0) == 0 {
        return None;
    }
    let scale = (max_edge / f64::from(width.max(height))).min(1.0);
    let work_w = ((f64::from(width) * scale).round() as i32).max(1);
    let work_h = ((f64::from(height) * scale).round() as i32).max(1);
    let mut rendered = ImageSurface::create(Format::ARgb32, work_w, work_h).ok()?;
    {
        let context = Context::new(&rendered).ok()?;
        context.scale(
            f64::from(work_w) / f64::from(width),
            f64::from(work_h) / f64::from(height),
        );
        context.set_source_surface(source, 0.0, 0.0).ok()?;
        context.source().set_filter(Filter::Bilinear);
        context.paint().ok()?;
    }
    rendered.flush();
    let mut rgba = {
        let stride = rendered.stride() as usize;
        let data = rendered.data().ok()?;
        cairo_argb_to_rgba_image(work_w as u32, work_h as u32, stride, data.as_ref())
    };
    apply_background_blur(&mut rgba, amount, f64::from(work_w) / canvas_width.max(1.0));
    rgba_image_to_surface(&rgba)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hard black/white split: the sharper the edge, the more distinct values a
    /// row across it keeps.
    fn split_image(width: u32, height: u32) -> RgbaImage {
        let mut image = RgbaImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let value = if x < width / 2 { 0 } else { 255 };
                image.put_pixel(x, y, image::Rgba([value, value, value, 255]));
            }
        }
        image
    }

    fn distinct_values_in_row(image: &RgbaImage, row: u32) -> usize {
        let mut values: Vec<u8> = (0..image.width())
            .map(|x| image.get_pixel(x, row)[0])
            .collect();
        values.sort_unstable();
        values.dedup();
        values.len()
    }

    #[test]
    fn zero_blur_leaves_the_background_untouched() {
        let original = split_image(80, 40);
        let mut image = original.clone();
        apply_background_blur(&mut image, 0.0, 1.0);
        assert_eq!(image.as_raw(), original.as_raw());
    }

    #[test]
    fn blur_softens_a_hard_edge_more_as_the_slider_rises() {
        let mut untouched = split_image(80, 40);
        apply_background_blur(&mut untouched, 0.0, 1.0);
        let sharp = distinct_values_in_row(&untouched, 20);

        let mut light = split_image(80, 40);
        apply_background_blur(&mut light, 0.3, 1.0);
        let mut heavy = split_image(80, 40);
        apply_background_blur(&mut heavy, 1.0, 1.0);

        assert_eq!(sharp, 2, "an unblurred split keeps two values");
        assert!(
            distinct_values_in_row(&light, 20) > sharp,
            "a light slider must already ramp the edge",
        );
        assert!(
            distinct_values_in_row(&heavy, 20) > distinct_values_in_row(&light, 20),
            "a heavier slider must ramp the edge further",
        );
    }

    #[test]
    fn blur_radius_follows_the_image_scale() {
        // The same visible blur: a half-size image needs half the radius, so the
        // softened edge covers the same fraction of the image either way.
        let ramp_fraction = |image: &RgbaImage| -> f64 {
            let row = image.height() / 2;
            let values: Vec<u8> = (0..image.width())
                .map(|x| image.get_pixel(x, row)[0])
                .collect();
            let ramp = values
                .iter()
                .filter(|value| **value > 20 && **value < 235)
                .count();
            ramp as f64 / f64::from(image.width())
        };

        let mut full = split_image(160, 80);
        apply_background_blur(&mut full, 1.0, 1.0);
        let mut half = image::imageops::resize(
            &split_image(160, 80),
            80,
            40,
            image::imageops::FilterType::Triangle,
        );
        apply_background_blur(&mut half, 1.0, 0.5);

        let full_ramp = ramp_fraction(&full);
        let half_ramp = ramp_fraction(&half);
        assert!(
            (full_ramp - half_ramp).abs() < 0.15,
            "scaled radius should blur by the same visible amount: {full_ramp} vs {half_ramp}",
        );
    }

    #[test]
    fn surface_blur_keeps_the_source_untouched() {
        let source = rgba_image_to_surface(&split_image(64, 32)).expect("source surface");
        let blurred = blur_background_surface(&source, 1.0, 64.0, 64.0).expect("blurred");
        assert_eq!(blurred.width(), 64);
        assert_eq!(blurred.height(), 32);
        assert!(
            blur_background_surface(&source, 0.0, 64.0, 64.0).is_none(),
            "zero blur asks for no new surface at all",
        );
    }
}
