use super::*;

#[test]
fn final_image_applies_focus_overlay() {
    let mut image = RgbaImage::new(12, 12);
    for y in 0..12 {
        for x in 0..12 {
            image.put_pixel(x, y, image::Rgba([180, 170, 160, 255]));
        }
    }

    let mut state = EditorState::new(image);
    state.actions.push(AnnotationAction::Focus {
        rect: Rect {
            x: 3,
            y: 3,
            width: 6,
            height: 6,
        },
        intensity: 58.0,
    });
    state.rebuild_effect_layer();

    let final_image = state.to_final_image().unwrap();
    let inside = *final_image.get_pixel(4, 4);
    let outside = *final_image.get_pixel(1, 1);

    assert_eq!(inside, image::Rgba([180, 170, 160, 255]));
    assert!(outside[0] < 180);
    assert!(outside[1] < 170);
    assert!(outside[2] < 160);
}

#[test]
fn final_image_shadow_does_not_replace_background_with_black_mask() {
    let image = RgbaImage::from_pixel(400, 240, image::Rgba([220, 220, 220, 255]));
    let mut state = EditorState::new(image);
    state.background_style = BackgroundStyle::PlainColor(DrawColor::new(1.0, 1.0, 1.0, 1.0));
    state.background_padding = 40.0;
    state.background_shadow = 45.0;

    let final_image = state.to_final_image().expect("final image");
    let corner = *final_image.get_pixel(0, 0);

    assert_ne!(corner, image::Rgba([0, 0, 0, 255]));
    assert_eq!(corner, image::Rgba([255, 255, 255, 255]));
}

#[test]
fn final_image_background_keeps_screenshot_at_native_scale_by_default() {
    let mut image = RgbaImage::new(400, 300);
    image.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
    image.put_pixel(399, 299, image::Rgba([0, 0, 255, 255]));

    let mut state = EditorState::new(image.clone());
    state.background_style = BackgroundStyle::PlainColor(DrawColor::new(1.0, 1.0, 1.0, 1.0));
    state.background_shadow = 0.0;
    state.background_corner_radius = 0.0;

    let final_image = state.to_final_image().expect("final image");

    assert_eq!(final_image.dimensions(), (400, 300));
    assert_eq!(*final_image.get_pixel(0, 0), *image.get_pixel(0, 0));
    assert_eq!(*final_image.get_pixel(399, 299), *image.get_pixel(399, 299));
}

#[test]
fn final_image_background_shadow_visibly_darkens_pixels_below_card() {
    let image = RgbaImage::from_pixel(400, 240, image::Rgba([220, 220, 220, 255]));
    let mut state = EditorState::new(image);
    state.background_style = BackgroundStyle::PlainColor(DrawColor::new(1.0, 1.0, 1.0, 1.0));
    state.background_padding = 40.0;
    state.background_insert = 0.0;
    state.background_shadow = 45.0;
    state.background_corner_radius = 18.0;

    let final_image = state.to_final_image().expect("final image");
    let shadow_pixel = *final_image.get_pixel(final_image.width() / 2, 290);

    assert!(
        shadow_pixel[0] < 235,
        "expected visible shadow below the card, got pixel {:?}",
        shadow_pixel
    );
}

#[test]
fn final_image_draws_annotations_on_top_of_background_padding() {
    // Screenshot 100x100 mid-gray; white background padding; red pen stroke
    // placed on the top-left wallpaper (negative screenshot coords).
    // Before the fix the stroke was baked into the screenshot and clipped away,
    // so the wallpaper pixel stayed white (annotation "underneath" the wallpaper).
    let image = RgbaImage::from_pixel(100, 100, image::Rgba([100, 100, 100, 255]));
    let mut state = EditorState::new(image);
    state.background_style = BackgroundStyle::PlainColor(DrawColor::new(1.0, 1.0, 1.0, 1.0));
    state.background_padding = 40.0;
    state.background_shadow = 0.0;
    state.background_corner_radius = 0.0;
    state.background_insert = 0.0;
    state.actions.push(AnnotationAction::Pen {
        points: vec![Point { x: -8.0, y: -8.0 }, Point { x: -2.0, y: -8.0 }],
        color: DrawColor::new(1.0, 0.0, 0.0, 1.0),
        stroke_size: 4.0,
    });

    let final_image = state.to_final_image().expect("final image");
    // Canvas is 120x120; screenshot at (10, 10); stroke maps to canvas y=2, x=2..8.
    let pixel = *final_image.get_pixel(5, 2);
    assert!(
        pixel[0] > 200 && pixel[1] < 100 && pixel[2] < 100,
        "expected red annotation on wallpaper padding, got pixel {:?}",
        pixel
    );
}

#[test]
fn annotation_canvas_bounds_include_wallpaper_padding() {
    let image = RgbaImage::from_pixel(100, 100, image::Rgba([100, 100, 100, 255]));
    let mut state = EditorState::new(image);
    state.background_style = BackgroundStyle::PlainColor(DrawColor::new(1.0, 1.0, 1.0, 1.0));
    state.background_padding = 40.0;
    state.background_shadow = 0.0;
    state.background_corner_radius = 0.0;

    let (min_x, min_y, max_x, max_y) = state.annotation_canvas_bounds();
    assert!(min_x < 0.0 && min_y < 0.0, "padding should map to negative coords, got ({}, {})", min_x, min_y);
    assert!(max_x > 100.0 && max_y > 100.0, "padding should extend beyond screenshot, got ({}, {})", max_x, max_y);
}

#[test]
fn number_marker_can_be_placed_on_background_padding() {
    let image = RgbaImage::from_pixel(400, 300, image::Rgba([100, 100, 100, 255]));
    let mut state = EditorState::new(image);
    state.background_style = BackgroundStyle::PlainColor(DrawColor::new(1.0, 1.0, 1.0, 1.0));
    state.background_padding = 40.0;
    state.background_shadow = 0.0;
    state.background_corner_radius = 0.0;

    let (min_x, _, _, _) = state.annotation_canvas_bounds();
    assert!(min_x < 0.0, "expected negative canvas origin, got {}", min_x);
    // Request a point inside the left wallpaper padding; the marker must not be
    // snapped back onto the screenshot (old behavior clamped to 0..image).
    state.add_number_marker(Point {
        x: min_x + 20.0,
        y: 150.0,
    });
    match state.actions.last().expect("number action") {
        AnnotationAction::Number { position, .. } => {
            assert!(
                position.x < 0.0,
                "number should stay on wallpaper padding, got x={}",
                position.x
            );
        }
        other => panic!("expected number action, got {:?}", other),
    }
}

#[test]
fn final_image_draws_border_on_top_of_wallpaper_background() {
    let image = RgbaImage::from_pixel(100, 100, image::Rgba([100, 100, 100, 255]));
    let mut state = EditorState::new(image);
    state.background_style = BackgroundStyle::PlainColor(DrawColor::new(1.0, 1.0, 1.0, 1.0));
    state.background_padding = 40.0;
    state.background_shadow = 0.0;
    state.background_corner_radius = 0.0;
    state.background_insert = 0.0;
    state.border_thickness = 6.0;
    state.border_color = DrawColor::new(1.0, 0.0, 0.0, 1.0);

    let final_image = state.to_final_image().expect("final image");
    // Canvas is 120x120 with the screenshot at (10, 10); the outside border
    // sits around the card, so sample just left of the image edge.
    let edge = *final_image.get_pixel(9, 60);
    assert!(
        edge[0] as i32 - edge[1] as i32 > 30 && edge[0] as i32 - edge[2] as i32 > 30,
        "expected red border on card edge, got pixel {:?}",
        edge
    );
}

#[test]
fn frame_style_presets_resolve_to_distinct_outside_borders() {
    assert_eq!(FrameStyle::ALL.len(), 12);
    assert_eq!(FrameStyle::Default.spec().border_thickness, 0.0);
    assert!(FrameStyle::Border.spec().border_thickness > FrameStyle::Outline.spec().border_thickness);
    assert!(FrameStyle::Card.spec().backing1.is_some());
    assert!(FrameStyle::Card.spec().backing2.is_none());
    assert!(FrameStyle::Stack.spec().backing1.is_some());
    assert!(FrameStyle::Stack.spec().backing2.is_none());
    assert!(FrameStyle::Stack2.spec().backing1.is_some());
    assert!(FrameStyle::Stack2.spec().backing2.is_some());
    assert!(FrameStyle::Retro.spec().outer1.is_some());
    assert_eq!(FrameStyle::Default.label(), "Default");
    assert_eq!(FrameStyle::Stack2.label(), "Stack 2");
}

#[test]
fn card_preset_draws_a_single_backing_sheet_behind_the_card() {
    let image = RgbaImage::from_pixel(100, 100, image::Rgba([100, 100, 100, 255]));
    let mut state = EditorState::new(image);
    state.background_style = BackgroundStyle::PlainColor(DrawColor::new(1.0, 1.0, 1.0, 1.0));
    state.background_padding = 80.0;
    state.background_shadow = 0.0;
    state.background_corner_radius = 0.0;
    state.background_insert = 0.0;
    let spec = FrameStyle::Card.spec();
    state.frame_style = FrameStyle::Card;
    state.border_thickness = spec.border_thickness;
    state.border_color = spec.border_color;

    let final_image = state.to_final_image().expect("final image");
    // Padding 80 at scale 0.25 => 20px base; the framed stack is centered,
    // so the card sits at (23, 28) with its backing sheet at (17, 12).
    let sheet = *final_image.get_pixel(20, 60);
    assert!(
        sheet[0] > 170 && sheet[0] < 230
            && (sheet[0] as i32 - sheet[1] as i32).abs() < 12
            && (sheet[0] as i32 - sheet[2] as i32).abs() < 12,
        "expected gray backing sheet behind card, got pixel {:?}",
        sheet
    );
    // Main image pixels stay intact.
    assert_eq!(*final_image.get_pixel(70, 70), image::Rgba([100, 100, 100, 255]));
}

#[test]
fn stack_preset_fans_diagonally_top_right_and_bottom_left() {
    let image = RgbaImage::from_pixel(100, 100, image::Rgba([100, 100, 100, 255]));
    let mut state = EditorState::new(image);
    state.background_style = BackgroundStyle::PlainColor(DrawColor::new(1.0, 1.0, 1.0, 1.0));
    state.background_padding = 80.0;
    state.background_shadow = 0.0;
    state.background_corner_radius = 0.0;
    state.background_insert = 0.0;
    let spec = FrameStyle::Stack.spec();
    state.frame_style = FrameStyle::Stack;
    state.border_thickness = spec.border_thickness;
    state.border_color = spec.border_color;

    let final_image = state.to_final_image().expect("final image");
    // Card stays centered at (20, 20); the center-pivot sheet peeks above
    // the top edge on the right half and below the bottom edge on the left.
    let top = *final_image.get_pixel(100, 19);
    assert!(
        top[0] > 150 && top[0] < 235
            && (top[0] as i32 - top[1] as i32).abs() < 14
            && (top[0] as i32 - top[2] as i32).abs() < 14,
        "expected diagonal sheet peeking above top-right, got pixel {:?}",
        top
    );
    let bottom = *final_image.get_pixel(40, 120);
    assert!(
        bottom[0] > 150 && bottom[0] < 235
            && (bottom[0] as i32 - bottom[1] as i32).abs() < 14
            && (bottom[0] as i32 - bottom[2] as i32).abs() < 14,
        "expected diagonal sheet peeking below bottom-left, got pixel {:?}",
        bottom
    );
    // Top-left and bottom-right stay clean background.
    assert_eq!(*final_image.get_pixel(10, 10), image::Rgba([255, 255, 255, 255]));
    assert_eq!(*final_image.get_pixel(70, 70), image::Rgba([100, 100, 100, 255]));
}

#[test]
fn stack2_preset_draws_two_backing_sheets_like_stacked_prints() {
    // Mirrors the reference: red backdrop, gray sheets peeking top-left.
    let image = RgbaImage::from_pixel(200, 120, image::Rgba([100, 100, 100, 255]));
    let mut state = EditorState::new(image);
    state.background_style = BackgroundStyle::PlainColor(DrawColor::new(1.0, 0.0, 0.0, 1.0));
    state.background_padding = 80.0;
    state.background_shadow = 0.0;
    state.background_corner_radius = 0.0;
    state.background_insert = 0.0;
    let spec = FrameStyle::Stack2.spec();
    state.frame_style = FrameStyle::Stack2;
    state.border_thickness = spec.border_thickness;
    state.border_color = spec.border_color;

    let final_image = state.to_final_image().expect("final image");
    // Padding 80 at scale 0.5 => 40px base; the framed stack is centered,
    // so the card sits at (48, 54) with fanned sheets behind it. Sample the
    // far sheet left of the near sheet's left edge.
    let far = *final_image.get_pixel(37, 60);
    assert!(
        far[0] > 115 && far[0] < 175
            && (far[0] as i32 - far[1] as i32).abs() < 12
            && (far[0] as i32 - far[2] as i32).abs() < 12,
        "expected far gray backing sheet, got pixel {:?}",
        far
    );
    // Near sheet peeks between far sheet and card.
    let near = *final_image.get_pixel(44, 50);
    assert!(
        near[0] > 175 && near[0] < 225
            && (near[0] as i32 - near[1] as i32).abs() < 12,
        "expected near gray backing sheet, got pixel {:?}",
        near
    );
    // Card itself stays intact and borderless.
    assert_eq!(*final_image.get_pixel(140, 100), image::Rgba([100, 100, 100, 255]));
}
