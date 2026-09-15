use super::*;

use crate::capture::editor::numbering_style::{NumberSize, NumberingStyle};

fn number_run(state: &EditorState) -> Vec<u32> {
    state
        .actions
        .iter()
        .filter_map(|action| match action {
            AnnotationAction::Number { number, .. } => Some(*number),
            _ => None,
        })
        .collect()
}

#[test]
fn remove_selected_action_keeps_number_sequence_consistent() {
    let mut state = EditorState::new(RgbaImage::new(64, 64));
    state.add_number_marker(Point { x: 8.0, y: 8.0 });
    state.add_number_marker(Point { x: 52.0, y: 52.0 });

    assert!(state.select_action_at_point_with_scale(Point { x: 8.0, y: 8.0 }, 1.0));
    assert!(state.remove_selected_action());

    let numbers: Vec<u32> = state
        .actions
        .iter()
        .filter_map(|action| match action {
            AnnotationAction::Number { number, .. } => Some(*number),
            _ => None,
        })
        .collect();

    assert_eq!(numbers, vec![2]);
    assert_eq!(state.next_number, 1); // reuses the removed number slot
}

#[test]
fn add_number_marker_assigns_incrementing_numbers() {
    let mut state = EditorState::new(RgbaImage::new(200, 200));
    state.set_color_index(4);

    state.add_number_marker(Point { x: 20.0, y: 20.0 });
    state.add_number_marker(Point { x: 40.0, y: 30.0 });

    assert_eq!(state.next_number, 3);
    assert_eq!(state.actions.len(), 2);

    match &state.actions[0] {
        AnnotationAction::Number {
            position,
            number,
            color,
            ..
        } => {
            assert_eq!(*position, Point { x: 20.0, y: 20.0 });
            assert_eq!(*number, 1);
            assert_eq!(*color, DRAW_COLORS[4]);
        }
        other => panic!("unexpected action: {:?}", other),
    }

    match &state.actions[1] {
        AnnotationAction::Number {
            position, number, ..
        } => {
            assert_eq!(*position, Point { x: 40.0, y: 30.0 });
            assert_eq!(*number, 2);
        }
        other => panic!("unexpected action: {:?}", other),
    }
}

#[test]
fn undo_number_marker_reuses_number_slot() {
    let mut state = EditorState::new(RgbaImage::new(32, 32));

    state.add_number_marker(Point { x: 3.0, y: 3.0 });
    state.add_number_marker(Point { x: 9.0, y: 9.0 });
    assert!(state.undo());

    state.add_number_marker(Point { x: 12.0, y: 12.0 });

    let numbers: Vec<u32> = state
        .actions
        .iter()
        .filter_map(|action| match action {
            AnnotationAction::Number { number, .. } => Some(*number),
            _ => None,
        })
        .collect();

    assert_eq!(numbers, vec![1, 2]);
    assert_eq!(state.next_number, 3);
}

#[test]
fn add_number_marker_clamps_center_inside_image_bounds() {
    let mut state = EditorState::new(RgbaImage::new(40, 40));

    state.add_number_marker(Point { x: 39.0, y: 39.0 });

    match &state.actions[0] {
        AnnotationAction::Number { position, .. } => {
            assert_eq!(*position, Point { x: 25.0, y: 25.0 });
        }
        other => panic!("unexpected action: {:?}", other),
    }
}

#[test]
fn bar_style_switch_restyles_the_selected_marker_and_keeps_its_number() {
    let mut state = EditorState::new(RgbaImage::new(200, 200));
    state.add_number_marker(Point { x: 20.0, y: 20.0 });
    state.add_number_marker(Point { x: 60.0, y: 60.0 });

    // The marker placed last is the selected one, so the floating bar edits it.
    assert_eq!(state.active_number_start(), 2);
    assert!(state.set_numbering_style(NumberingStyle::Uppercase));

    match state.selected_action().expect("selected marker") {
        AnnotationAction::Number { number, style, .. } => {
            assert_eq!(*style, NumberingStyle::Uppercase);
            assert_eq!(*number, 2, "restyling must not renumber the marker");
        }
        other => panic!("unexpected action: {:?}", other),
    }
    // The first marker keeps the style it was placed with.
    match &state.actions[0] {
        AnnotationAction::Number { style, .. } => assert_eq!(*style, NumberingStyle::Numeric),
        other => panic!("unexpected action: {:?}", other),
    }
    // The next marker continues the uppercase run instead of restarting it.
    state.add_number_marker(Point { x: 100.0, y: 100.0 });
    assert_eq!(state.next_number, 4);
    match &state.actions[2] {
        AnnotationAction::Number { number, style, .. } => {
            assert_eq!(*number, 3);
            assert_eq!(*style, NumberingStyle::Uppercase);
        }
        other => panic!("unexpected action: {:?}", other),
    }
}

#[test]
fn bar_size_switch_resizes_the_selected_marker_and_the_next_one() {
    let mut state = EditorState::new(RgbaImage::new(200, 200));
    state.add_number_marker(Point { x: 40.0, y: 40.0 });

    assert!(state.set_number_size(NumberSize::ExtraLarge));
    match state.selected_action().expect("selected marker") {
        AnnotationAction::Number { size, .. } => assert_eq!(*size, NumberSize::ExtraLarge),
        other => panic!("unexpected action: {:?}", other),
    }

    state.add_number_marker(Point { x: 150.0, y: 150.0 });
    match state.selected_action().expect("selected marker") {
        AnnotationAction::Number { size, .. } => assert_eq!(*size, NumberSize::ExtraLarge),
        other => panic!("unexpected action: {:?}", other),
    }
}

#[test]
fn number_marker_hit_box_follows_the_marker_size() {
    let mut state = EditorState::new(RgbaImage::new(200, 200));
    state.add_number_marker(Point { x: 40.0, y: 40.0 });
    state.set_number_size(NumberSize::ExtraLarge);
    state.clear_selection();

    // 30px from the center: inside an ExtraLarge marker (radius 25) but outside
    // the old fixed 15px hit box, and not part of any other action.
    assert!(state.select_number_action_at_point_with_scale(Point { x: 70.0, y: 40.0 }, 1.0));
    assert_eq!(state.selected_action_index, Some(0));

    state.clear_selection();
    assert!(!state.select_number_action_at_point_with_scale(Point { x: 100.0, y: 40.0 }, 1.0));
    assert_eq!(state.selected_action_index, None);
}

#[test]
fn number_tool_hit_test_ignores_other_actions_under_the_cursor() {
    let mut state = EditorState::new(RgbaImage::new(200, 200));
    state.push_action(AnnotationAction::Box {
        rect: Rect {
            x: 10,
            y: 10,
            width: 80,
            height: 80,
        },
        color: DRAW_COLORS[0],
        stroke_size: STROKE_WIDTH,
        shadow: false,
    });

    assert!(!state.select_number_action_at_point_with_scale(Point { x: 50.0, y: 50.0 }, 1.0));
    assert_eq!(state.selected_action_index, None);
    assert!(state.select_action_at_point_with_scale(Point { x: 50.0, y: 50.0 }, 1.0));
}

#[test]
fn bar_start_control_renumbers_the_selected_marker_and_the_run_after_it() {
    let mut state = EditorState::new(RgbaImage::new(200, 200));
    state.add_number_marker(Point { x: 20.0, y: 20.0 });
    state.add_number_marker(Point { x: 60.0, y: 60.0 });
    state.add_number_marker(Point { x: 100.0, y: 100.0 });

    assert!(state.select_number_action_at_point_with_scale(Point { x: 60.0, y: 60.0 }, 1.0));
    assert_eq!(state.active_number_start(), 2);
    assert_eq!(state.active_number_start_display(), "2".to_string());

    assert!(state.set_active_number_start(10));
    assert_eq!(number_run(&state), vec![1, 10, 11]);
    assert_eq!(state.next_number, 12, "the next marker follows the run");
}

#[test]
fn bar_start_control_without_a_selection_seeds_the_next_marker() {
    let mut state = EditorState::new(RgbaImage::new(200, 200));
    state.clear_selection();

    assert!(state.set_active_number_start(7));
    assert_eq!(state.numbering_start, 7);
    assert_eq!(state.next_number, 7);
    assert_eq!(state.active_number_start_display(), "7".to_string());

    state.add_number_marker(Point { x: 40.0, y: 40.0 });
    assert_eq!(number_run(&state), vec![7]);
    assert_eq!(state.next_number, 8);

    // With nothing selected the bar shows the number the next marker will get, so
    // the armed tool never advertises a value it will not use.
    state.clear_selection();
    assert_eq!(state.active_number_start(), 8);
    assert_eq!(state.active_number_start_display(), "8".to_string());
}

#[test]
fn bar_start_control_formats_the_selected_marker_in_its_own_style() {
    let mut state = EditorState::new(RgbaImage::new(200, 200));
    state.set_numbering_style(NumberingStyle::Uppercase);
    state.add_number_marker(Point { x: 40.0, y: 40.0 });
    state.add_number_marker(Point { x: 120.0, y: 120.0 });

    assert_eq!(state.active_number_start_display(), "B".to_string());
    assert!(state.set_active_number_start(3));
    assert_eq!(state.active_number_start_display(), "C".to_string());
}
