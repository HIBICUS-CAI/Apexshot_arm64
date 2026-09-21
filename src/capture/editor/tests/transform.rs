use super::*;

#[test]
fn rect_from_points_normalizes_values() {
    let start = Point { x: 20.0, y: 10.0 };
    let end = Point { x: 2.0, y: 3.0 };
    let rect = Rect::from_points(start, end).unwrap();

    assert_eq!(rect.x, 2);
    assert_eq!(rect.y, 3);
    assert_eq!(rect.width, 18);
    assert_eq!(rect.height, 7);
}

#[test]
fn view_transform_scales_to_fit() {
    let t = ViewTransform::fit(4000.0, 2000.0, 1000.0, 500.0);

    assert!((t.scale - 0.25).abs() < f64::EPSILON);
    assert!((t.offset_x - 0.0).abs() < f64::EPSILON);
    assert!((t.offset_y - 0.0).abs() < f64::EPSILON);

    let mapped = t.view_to_image_clamped(Point { x: 500.0, y: 250.0 });
    assert!((mapped.x - 2000.0).abs() < f64::EPSILON);
    assert!((mapped.y - 1000.0).abs() < f64::EPSILON);
}

#[test]
fn background_transform_covers_full_canvas_and_allows_padding_coords() {
    // Screenshot 100x100 with a 10px canvas padding on every side:
    // canvas 120x120, screenshot at (10, 10), draw_scale 1.
    let mut t = ViewTransform::for_image(100.0, 100.0);
    t.has_background = true;
    t.scale = 1.0;
    t.offset_x = 10.0;
    t.offset_y = 10.0;
    t.canvas_offset_x = 0.0;
    t.canvas_offset_y = 0.0;
    t.canvas_scale = 1.0;
    t.canvas_width = 120.0;
    t.canvas_height = 120.0;
    t.image_rect_x = 10.0;
    t.image_rect_y = 10.0;
    t.canvas_draw_scale = 1.0;

    // Top-left wallpaper padding is inside the canvas (was rejected before).
    assert!(t.contains_view(Point { x: 5.0, y: 5.0 }));
    // Screenshot origin still maps correctly.
    assert!(t.contains_view(Point { x: 10.0, y: 10.0 }));

    // Wallpaper padding maps to negative screenshot coords (not clamped to 0).
    let mapped = t.view_to_image_clamped(Point { x: 5.0, y: 5.0 });
    assert!((mapped.x + 5.0).abs() < f64::EPSILON);
    assert!((mapped.y + 5.0).abs() < f64::EPSILON);

    let (min_x, min_y, max_x, max_y) = t.canvas_bounds_in_image_coords();
    assert!((min_x + 10.0).abs() < f64::EPSILON);
    assert!((min_y + 10.0).abs() < f64::EPSILON);
    assert!((max_x - 110.0).abs() < f64::EPSILON);
    assert!((max_y - 110.0).abs() < f64::EPSILON);
}

#[test]
fn plain_transform_still_clamps_to_screenshot() {
    let t = ViewTransform::for_image(100.0, 100.0);
    let mapped = t.view_to_image_clamped(Point { x: -20.0, y: -20.0 });
    assert!((mapped.x - 0.0).abs() < f64::EPSILON);
    assert!((mapped.y - 0.0).abs() < f64::EPSILON);
    assert!(!t.contains_view(Point { x: -5.0, y: -5.0 }));
}
