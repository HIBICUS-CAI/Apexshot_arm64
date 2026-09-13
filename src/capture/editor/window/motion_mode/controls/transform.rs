use gtk4::prelude::*;
use gtk4::ToggleButton;

use crate::recording::editor::model::MotionTimingKind;

use super::super::widgets::{ease_preset_timing, spring_preset_timing};
use super::super::{MotionModeParts, MotionSession};
use super::{Redraw, RequestLivePreview, RequestTransitionPreview};

pub(super) fn install(
    parts: &MotionModeParts,
    session: &MotionSession,
    redraw: Redraw,
    request_live_preview: RequestLivePreview,
    request_transition_preview: RequestTransitionPreview,
) {
    parts.transform.scale_slider.connect_value_changed({
        let session = session.runtime.clone();
        let redraw = redraw.clone();
        let request_transition_preview = request_transition_preview.clone();
        let syncing = parts.shared.inspector_syncing.clone();
        move |slider| {
            if syncing.get() {
                return;
            }
            let segment_start = {
                let runtime = session.borrow();
                runtime
                    .motion
                    .selected_segment()
                    .map(|segment| segment.start)
            };
            {
                let mut runtime = session.borrow_mut();
                runtime.begin_motion_edit();
                runtime.motion.set_selected_end_scale(slider.value());
            }
            match segment_start {
                Some(start) => request_transition_preview(start),
                None => redraw(),
            }
        }
    });

    parts.transform.intensity_slider.connect_value_changed({
        let session = session.runtime.clone();
        let intensity_value = parts.transform.intensity_value.clone();
        let request_transition_preview = request_transition_preview.clone();
        let request_live_preview = request_live_preview.clone();
        let syncing = parts.shared.inspector_syncing.clone();
        move |slider| {
            if syncing.get() {
                return;
            }
            let value = slider.value();
            let segment_start = {
                let runtime = session.borrow();
                runtime
                    .motion
                    .selected_segment()
                    .map(|segment| segment.start)
            };
            {
                let mut runtime = session.borrow_mut();
                runtime.begin_motion_edit();
                runtime.motion.set_selected_intensity(value);
            }
            intensity_value.set_label(&format!("{:.0}%", value * 100.0));
            match segment_start {
                Some(start) => request_transition_preview(start),
                None => request_live_preview(),
            }
        }
    });

    for (axis, slider, value_label) in [
        (
            0_u8,
            parts.transform.zoom_anchor_x_slider.clone(),
            parts.transform.zoom_anchor_x_value.clone(),
        ),
        (
            1_u8,
            parts.transform.zoom_anchor_y_slider.clone(),
            parts.transform.zoom_anchor_y_value.clone(),
        ),
    ] {
        let session = session.runtime.clone();
        let request_transition_preview = request_transition_preview.clone();
        let request_live_preview = request_live_preview.clone();
        let syncing = parts.shared.inspector_syncing.clone();
        slider.connect_value_changed(move |slider| {
            if syncing.get() {
                return;
            }
            let value = slider.value();
            let segment_start = {
                let runtime = session.borrow();
                runtime
                    .motion
                    .selected_segment()
                    .map(|segment| segment.start)
            };
            let mut runtime = session.borrow_mut();
            runtime.begin_motion_edit();
            let (mut x, mut y) = runtime
                .motion
                .selected_segment()
                .map_or((0.5, 0.5), |segment| {
                    (segment.zoom_anchor_x, segment.zoom_anchor_y)
                });
            if axis == 0 {
                x = value;
            } else {
                y = value;
            }
            runtime.motion.set_selected_zoom_anchor(x, y);
            drop(runtime);
            value_label.set_label(&format!("{:.0}%", value * 100.0));
            match segment_start {
                Some(start) => request_transition_preview(start),
                None => request_live_preview(),
            }
        });
    }

    parts.transform.yaw_slider.connect_value_changed({
        let session = session.runtime.clone();
        let yaw_value = parts.transform.yaw_value.clone();
        let request_transition_preview = request_transition_preview.clone();
        let request_live_preview = request_live_preview.clone();
        let syncing = parts.shared.inspector_syncing.clone();
        move |slider| {
            if syncing.get() {
                return;
            }
            let value = slider.value();
            let segment_start = {
                let runtime = session.borrow();
                runtime
                    .motion
                    .selected_segment()
                    .map(|segment| segment.start)
            };
            {
                let mut runtime = session.borrow_mut();
                runtime.begin_motion_edit();
                runtime.motion.set_selected_end_yaw(value);
            }
            yaw_value.set_label(&format!("{:.0}°", value));
            match segment_start {
                Some(start) => request_transition_preview(start),
                None => request_live_preview(),
            }
        }
    });
    parts.transform.pitch_slider.connect_value_changed({
        let session = session.runtime.clone();
        let pitch_value = parts.transform.pitch_value.clone();
        let request_transition_preview = request_transition_preview.clone();
        let request_live_preview = request_live_preview.clone();
        let syncing = parts.shared.inspector_syncing.clone();
        move |slider| {
            if syncing.get() {
                return;
            }
            let value = slider.value();
            let segment_start = {
                let runtime = session.borrow();
                runtime
                    .motion
                    .selected_segment()
                    .map(|segment| segment.start)
            };
            {
                let mut runtime = session.borrow_mut();
                runtime.begin_motion_edit();
                runtime.motion.set_selected_end_pitch(value);
            }
            pitch_value.set_label(&format!("{:.0}°", value));
            match segment_start {
                Some(start) => request_transition_preview(start),
                None => request_live_preview(),
            }
        }
    });
    parts.transform.roll_slider.connect_value_changed({
        let session = session.runtime.clone();
        let roll_value = parts.transform.roll_value.clone();
        let request_transition_preview = request_transition_preview.clone();
        let request_live_preview = request_live_preview.clone();
        let syncing = parts.shared.inspector_syncing.clone();
        move |slider| {
            if syncing.get() {
                return;
            }
            let value = slider.value();
            let segment_start = {
                let runtime = session.borrow();
                runtime
                    .motion
                    .selected_segment()
                    .map(|segment| segment.start)
            };
            {
                let mut runtime = session.borrow_mut();
                runtime.begin_motion_edit();
                runtime.motion.set_selected_end_roll(value);
            }
            roll_value.set_label(&format!("{:.0}°", value));
            match segment_start {
                Some(start) => request_transition_preview(start),
                None => request_live_preview(),
            }
        }
    });
    parts.transform.perspective_slider.connect_value_changed({
        let session = session.runtime.clone();
        let perspective_value = parts.transform.perspective_value.clone();
        let request_live_preview = request_live_preview.clone();
        let syncing = parts.shared.inspector_syncing.clone();
        move |slider| {
            if syncing.get() {
                return;
            }
            let value = slider.value();
            {
                let mut runtime = session.borrow_mut();
                runtime.begin_motion_edit();
                runtime.motion.set_selected_perspective(value);
            }
            perspective_value.set_label(&format!("{:.0}%", value * 100.0));
            request_live_preview();
        }
    });
    parts.transform.pos_x_slider.connect_value_changed({
        let session = session.runtime.clone();
        let position_pad = parts.transform.position_pad.clone();
        let pos_x_value = parts.transform.pos_x_value.clone();
        let request_transition_preview = request_transition_preview.clone();
        let request_live_preview = request_live_preview.clone();
        let syncing = parts.shared.inspector_syncing.clone();
        move |slider| {
            if syncing.get() {
                return;
            }
            let value = slider.value();
            let (segment_start, pos_y) = {
                let runtime = session.borrow();
                runtime
                    .motion
                    .selected_segment()
                    .map_or((None, 0.0), |segment| {
                        (Some(segment.start), segment.to.pos_y)
                    })
            };
            {
                let mut runtime = session.borrow_mut();
                runtime.begin_motion_edit();
                runtime.motion.set_selected_end_pos_x(value);
            }
            position_pad.set_position(value, pos_y);
            pos_x_value.set_label(&format!("{:.0}", value * 1000.0));
            match segment_start {
                Some(start) => request_transition_preview(start),
                None => request_live_preview(),
            }
        }
    });
    parts.transform.pos_y_slider.connect_value_changed({
        let session = session.runtime.clone();
        let position_pad = parts.transform.position_pad.clone();
        let pos_y_value = parts.transform.pos_y_value.clone();
        let request_transition_preview = request_transition_preview.clone();
        let request_live_preview = request_live_preview.clone();
        let syncing = parts.shared.inspector_syncing.clone();
        move |slider| {
            if syncing.get() {
                return;
            }
            let value = slider.value();
            let (segment_start, pos_x) = {
                let runtime = session.borrow();
                runtime
                    .motion
                    .selected_segment()
                    .map_or((None, 0.0), |segment| {
                        (Some(segment.start), segment.to.pos_x)
                    })
            };
            {
                let mut runtime = session.borrow_mut();
                runtime.begin_motion_edit();
                runtime.motion.set_selected_end_pos_y(value);
            }
            position_pad.set_position(pos_x, value);
            pos_y_value.set_label(&format!("{:.0}", value * 1000.0));
            match segment_start {
                Some(start) => request_transition_preview(start),
                None => request_live_preview(),
            }
        }
    });
    parts.transform.position_pad.connect_value_changed({
        let session = session.runtime.clone();
        let pos_x_slider = parts.transform.pos_x_slider.clone();
        let pos_x_value = parts.transform.pos_x_value.clone();
        let pos_y_slider = parts.transform.pos_y_slider.clone();
        let pos_y_value = parts.transform.pos_y_value.clone();
        let request_transition_preview = request_transition_preview.clone();
        let request_live_preview = request_live_preview.clone();
        let syncing = parts.shared.inspector_syncing.clone();
        move |x, y| {
            if syncing.get() {
                return;
            }
            let segment_start = {
                let runtime = session.borrow();
                runtime
                    .motion
                    .selected_segment()
                    .map(|segment| segment.start)
            };
            let mut runtime = session.borrow_mut();
            runtime.begin_motion_edit();
            runtime.motion.set_selected_end_pos_x(x);
            runtime.motion.set_selected_end_pos_y(y);
            drop(runtime);

            syncing.set(true);
            pos_x_slider.set_value(x);
            pos_x_value.set_label(&format!("{:.0}", x * 1000.0));
            pos_y_slider.set_value(y);
            pos_y_value.set_label(&format!("{:.0}", y * 1000.0));
            syncing.set(false);

            match segment_start {
                Some(start) => request_transition_preview(start),
                None => request_live_preview(),
            }
        }
    });
    parts.transform.ease_slider.connect_value_changed({
        let session = session.runtime.clone();
        let ease_value = parts.transform.ease_value.clone();
        let request_transition_preview = request_transition_preview.clone();
        let request_live_preview = request_live_preview.clone();
        let syncing = parts.shared.inspector_syncing.clone();
        move |slider| {
            if syncing.get() {
                return;
            }
            let transition_ms = slider.value().round() as u32;
            let segment_start = {
                let runtime = session.borrow();
                runtime
                    .motion
                    .selected_segment()
                    .map(|segment| segment.start)
            };
            {
                let mut runtime = session.borrow_mut();
                runtime.begin_motion_edit();
                runtime.motion.set_selected_transition_ms(transition_ms);
            }
            ease_value.set_label(&format!("{transition_ms}ms"));
            match segment_start {
                Some(start) => request_transition_preview(start),
                None => request_live_preview(),
            }
        }
    });

    for (axis, slider, value_label) in [
        (
            0_u8,
            parts.transform.easing_x1_slider.clone(),
            parts.transform.easing_x1_value.clone(),
        ),
        (
            1_u8,
            parts.transform.easing_y1_slider.clone(),
            parts.transform.easing_y1_value.clone(),
        ),
        (
            2_u8,
            parts.transform.easing_x2_slider.clone(),
            parts.transform.easing_x2_value.clone(),
        ),
        (
            3_u8,
            parts.transform.easing_y2_slider.clone(),
            parts.transform.easing_y2_value.clone(),
        ),
    ] {
        let session = session.runtime.clone();
        let request_transition_preview = request_transition_preview.clone();
        let request_live_preview = request_live_preview.clone();
        let syncing = parts.shared.inspector_syncing.clone();
        slider.connect_value_changed(move |slider| {
            if syncing.get() {
                return;
            }
            let value = slider.value();
            let segment_start = {
                let runtime = session.borrow();
                runtime
                    .motion
                    .selected_segment()
                    .map(|segment| segment.start)
            };
            let mut runtime = session.borrow_mut();
            runtime.begin_motion_edit();
            let mut timing = runtime.motion.selected_transform_timing();
            match axis {
                0 => timing.easing_x1 = value,
                1 => timing.easing_y1 = value,
                2 => timing.easing_x2 = value,
                _ => timing.easing_y2 = value,
            }
            runtime.motion.set_transform_timing(timing);
            drop(runtime);
            value_label.set_label(&format!("{:.0}%", value * 100.0));
            match segment_start {
                Some(start) => request_transition_preview(start),
                None => request_live_preview(),
            }
        });
    }

    for (kind, button) in &parts.transform.timing_kind_buttons {
        let kind = *kind;
        button.connect_clicked({
            let session = session.runtime.clone();
            let redraw = redraw.clone();
            let request_transition_preview = request_transition_preview.clone();
            let request_live_preview = request_live_preview.clone();
            let custom_timing_btn = parts.transform.custom_timing_btn.clone();
            move |_| {
                let segment_start = {
                    let runtime = session.borrow();
                    runtime
                        .motion
                        .selected_segment()
                        .map(|segment| segment.start)
                };
                {
                    let mut runtime = session.borrow_mut();
                    runtime.begin_motion_edit();
                    // Each family button applies the curve its icon previews
                    // (S for Ease, Gentle for Spring). Clicked, not toggled,
                    // so re-clicking an active family still converges the
                    // curve — e.g. a default clip onto the S.
                    let current = runtime.motion.selected_transform_timing();
                    let timing = match kind {
                        MotionTimingKind::Ease => {
                            let mut preset = ease_preset_timing(0, current);
                            preset.kind = MotionTimingKind::Ease;
                            preset
                        }
                        MotionTimingKind::Spring => spring_preset_timing(1, current),
                    };
                    runtime.motion.set_transform_timing(timing);
                }
                custom_timing_btn.set_active(false);
                redraw();
                match segment_start {
                    Some(start) => request_transition_preview(start),
                    None => request_live_preview(),
                }
            }
        });
    }

    parts.transform.custom_timing_btn.connect_toggled({
        let redraw = redraw.clone();
        let syncing = parts.shared.inspector_syncing.clone();
        move |_: &ToggleButton| {
            if syncing.get() {
                return;
            }
            redraw();
        }
    });

    parts.transform.spring_bounce_slider.connect_value_changed({
        let session = session.runtime.clone();
        let value_label = parts.transform.spring_bounce_value.clone();
        let request_transition_preview = request_transition_preview.clone();
        let request_live_preview = request_live_preview.clone();
        let syncing = parts.shared.inspector_syncing.clone();
        move |slider| {
            if syncing.get() {
                return;
            }
            let value = slider.value();
            let segment_start = {
                let runtime = session.borrow();
                runtime
                    .motion
                    .selected_segment()
                    .map(|segment| segment.start)
            };
            {
                let mut runtime = session.borrow_mut();
                runtime.begin_motion_edit();
                let mut timing = runtime.motion.selected_transform_timing();
                timing.spring_bounce = value;
                runtime.motion.set_transform_timing(timing);
            }
            value_label.set_label(&format!("{:.0}%", value * 100.0));
            match segment_start {
                Some(start) => request_transition_preview(start),
                None => request_live_preview(),
            }
        }
    });
}
