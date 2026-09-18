use super::*;

use crate::capture::editor::pen_weight::{HighlighterMode, PenWeight};

#[test]
fn set_text_size_clamps_to_allowed_range() {
    let mut state = EditorState::new(RgbaImage::new(32, 32));
    assert!(state.set_text_size(2.0));
    assert_eq!(state.text_size, MIN_TEXT_SIZE);

    assert!(state.set_text_size(500.0));
    assert_eq!(state.text_size, MAX_TEXT_SIZE);
}

#[test]
fn set_stroke_size_clamps_to_allowed_range() {
    let mut state = EditorState::new(RgbaImage::new(32, 32));
    assert!(state.set_stroke_size(0.1));
    assert_eq!(state.stroke_size, MIN_STROKE_SIZE);

    assert!(state.set_stroke_size(500.0));
    assert_eq!(state.stroke_size, MAX_STROKE_SIZE);
}

#[test]
fn set_selected_action_stroke_size_updates_selected_annotation() {
    let mut state = EditorState::new(RgbaImage::new(64, 64));
    state.push_action(AnnotationAction::Line {
        start: Point { x: 6.0, y: 8.0 },
        end: Point { x: 20.0, y: 24.0 },
        color: DRAW_COLORS[0],
        stroke_size: 4.0,
        shadow: false,
    });

    state.selected_action_index = Some(0);
    assert_eq!(state.selected_action_stroke_size(), Some(4.0));
    assert!(state.set_selected_action_stroke_size(9.0));
    assert_eq!(state.selected_action_stroke_size(), Some(9.0));
}

#[test]
fn adjust_stroke_size_updates_selected_annotation_size() {
    let mut state = EditorState::new(RgbaImage::new(64, 64));
    state.push_action(AnnotationAction::Box {
        rect: Rect {
            x: 10,
            y: 10,
            width: 16,
            height: 16,
        },
        color: DRAW_COLORS[2],
        stroke_size: 6.0,
        shadow: false,
    });

    state.selected_action_index = Some(0);
    state.set_stroke_size(6.0);
    assert!(state.set_selected_action_stroke_size(7.0));
    assert_eq!(state.selected_action_stroke_size(), Some(7.0));
}

#[test]
fn set_selected_text_action_size_updates_selected_text_annotation() {
    let mut state = EditorState::new(RgbaImage::new(64, 64));
    state.push_action(AnnotationAction::Text {
        position: Point { x: 12.0, y: 16.0 },
        text: "text".to_string(),
        color: DRAW_COLORS[0],
        font: FontSettings {
            family: "Sans".to_string(),
            size: 20.0,
            style: FontStyle::Normal,
            decoration: TextDecoration::None,
            alignment: TextAlignment::Left,
        },
        max_width: None,
        shadow: false,
        background_color: None,
    });

    state.selected_action_index = Some(0);
    assert_eq!(state.selected_text_action_size(), Some(20.0));
    assert!(state.set_selected_text_action_size(34.0));
    assert_eq!(state.selected_text_action_size(), Some(34.0));
}

#[test]
fn adjust_text_size_updates_selected_text_annotation_size() {
    let mut state = EditorState::new(RgbaImage::new(64, 64));
    state.push_action(AnnotationAction::Text {
        position: Point { x: 12.0, y: 16.0 },
        text: "text".to_string(),
        color: DRAW_COLORS[0],
        font: FontSettings {
            family: "Sans".to_string(),
            size: 20.0,
            style: FontStyle::Normal,
            decoration: TextDecoration::None,
            alignment: TextAlignment::Left,
        },
        max_width: None,
        shadow: false,
        background_color: None,
    });

    state.selected_action_index = Some(0);
    state.set_text_size(20.0);
    assert!(state.set_selected_text_action_size(22.0));
    assert_eq!(state.selected_text_action_size(), Some(22.0));
}

#[test]
fn highlighter_bar_weight_applies_to_the_selected_stroke() {
    let mut state = EditorState::new(RgbaImage::new(128, 128));
    state.selected_tool = Tool::Highlighter;
    state.push_action(AnnotationAction::Highlighter {
        points: vec![Point { x: 10.0, y: 10.0 }, Point { x: 80.0, y: 10.0 }],
        color: DRAW_COLORS[0],
        stroke_size: PenWeight::Medium.highlighter_stroke_width(),
    });
    state.selected_action_index = Some(0);

    assert!(state.set_highlighter_weight(PenWeight::ExtraLarge));
    assert_eq!(
        state.selected_highlighter_stroke_size(),
        Some(PenWeight::ExtraLarge.highlighter_stroke_width())
    );
    // The pick also becomes the brush for the next stroke and leaves text-aware sizing.
    assert_eq!(state.pen_weight, PenWeight::ExtraLarge);
    assert_eq!(state.highlighter_mode, HighlighterMode::Freehand);
    assert_eq!(
        state.active_highlighter_weight(),
        Some(PenWeight::ExtraLarge)
    );
}

#[test]
fn pen_and_highlighter_draw_at_the_toolbar_stroke_size() {
    let mut state = EditorState::new(RgbaImage::new(128, 128));
    state.set_stroke_size(18.0);

    state.selected_tool = Tool::Pen;
    state.clear_selection();
    state.begin_drag(Point { x: 4.0, y: 4.0 });
    state.update_drag(Point { x: 40.0, y: 4.0 });
    match state.draft_action() {
        Some(AnnotationAction::Pen { stroke_size, .. }) => assert_eq!(
            stroke_size, 18.0,
            "the toolbar slider sets the pen thickness"
        ),
        other => panic!("expected a pen draft, got {other:?}"),
    }

    state.selected_tool = Tool::Highlighter;
    state.set_highlighter_mode(HighlighterMode::Freehand);
    state.begin_drag(Point { x: 4.0, y: 4.0 });
    state.update_drag(Point { x: 40.0, y: 4.0 });
    match state.draft_action() {
        Some(AnnotationAction::Highlighter { stroke_size, .. }) => assert_eq!(
            stroke_size, 18.0,
            "the toolbar slider sets the freehand highlighter thickness"
        ),
        other => panic!("expected a highlighter draft, got {other:?}"),
    }
}

#[test]
fn highlighter_bar_mode_switch_reports_changes_once() {
    let mut state = EditorState::new(RgbaImage::new(64, 64));
    state.selected_tool = Tool::Highlighter;

    assert_eq!(state.highlighter_mode, HighlighterMode::TextAware);
    assert!(state.set_highlighter_mode_and_check(HighlighterMode::Freehand));
    assert!(!state.set_highlighter_mode_and_check(HighlighterMode::Freehand));
    assert!(state.set_highlighter_mode_and_check(HighlighterMode::TextAware));
}

#[test]
fn highlighter_bar_shows_the_selected_strokes_own_weight() {
    let mut state = EditorState::new(RgbaImage::new(128, 128));
    state.highlighter_mode = HighlighterMode::Freehand;
    state.push_action(AnnotationAction::Highlighter {
        points: vec![Point { x: 10.0, y: 10.0 }, Point { x: 60.0, y: 10.0 }],
        color: DRAW_COLORS[0],
        // A text-aware stroke is as thick as the text line it covered.
        stroke_size: 22.0,
    });
    state.selected_action_index = Some(0);

    assert_eq!(
        state.active_highlighter_weight(),
        Some(PenWeight::nearest_for_highlighter_stroke(22.0))
    );

    // The stroke's thickness still shows in text-aware mode: only the *next*
    // stroke is auto-sized then.
    state.highlighter_mode = HighlighterMode::TextAware;
    assert_eq!(
        state.active_highlighter_weight(),
        Some(PenWeight::nearest_for_highlighter_stroke(22.0))
    );
    state.clear_selection();
    assert_eq!(state.active_highlighter_weight(), None);
}
