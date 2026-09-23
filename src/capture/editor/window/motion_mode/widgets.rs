use gtk4::cairo::LineCap;
use gtk4::{
    gdk, prelude::*, Align, Box as GtkBox, ColorChooserWidget, DrawingArea, Entry, Label,
    MenuButton, Orientation, Overlay, Popover,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::i18n::t;
use crate::recording::editor::model::{
    MotionEffectTransformTiming, MotionTimingKind, MAX_MOTION_POS, MAX_MOTION_YAW, MIN_MOTION_POS,
    MIN_MOTION_YAW,
};
use crate::recording::editor::window::tool_sidebar::FillSlider;

pub(super) fn motion_color_control(
    initial: gdk::RGBA,
    tooltip: &str,
    initially_selected: bool,
    on_changed: impl Fn(gdk::RGBA) + 'static,
) -> GtkBox {
    let on_changed = Rc::new(on_changed);
    let trigger = MenuButton::new();
    trigger.set_size_request(32, 32);
    trigger.set_halign(Align::Start);
    trigger.set_valign(Align::Start);
    trigger.set_hexpand(false);
    trigger.set_vexpand(false);
    trigger.set_tooltip_text(Some(&t(tooltip)));
    trigger.add_css_class("editor-motion-color-button");
    if initially_selected {
        trigger.add_css_class("active-motion-color-control");
    }
    let select_control = gtk4::GestureClick::new();
    select_control.connect_pressed({
        let trigger = trigger.clone();
        move |_, _, _, _| trigger.add_css_class("active-motion-color-control")
    });
    trigger.add_controller(select_control);

    let swatch = DrawingArea::new();
    swatch.set_content_width(28);
    swatch.set_content_height(28);
    swatch.set_can_target(false);
    swatch.set_halign(Align::Center);
    swatch.set_valign(Align::Center);
    swatch.add_css_class("editor-motion-color-swatch");
    let selected = Rc::new(RefCell::new(initial));
    swatch.set_draw_func({
        let selected = selected.clone();
        move |_area, context, width, height| {
            let rgba = selected.borrow();
            context.set_source_rgba(
                rgba.red().into(),
                rgba.green().into(),
                rgba.blue().into(),
                rgba.alpha().into(),
            );
            context.rectangle(0.0, 0.0, width as f64, height as f64);
            context.fill().ok();
        }
    });
    let chooser = ColorChooserWidget::new();
    chooser.set_use_alpha(true);
    chooser.set_rgba(&initial);
    chooser.set_size_request(260, 260);
    chooser.set_hexpand(false);
    chooser.set_vexpand(false);
    chooser.add_css_class("editor-motion-color-chooser");
    let hex_entry = Entry::new();
    hex_entry.set_width_chars(9);
    hex_entry.set_text(&motion_color_hex(initial));
    hex_entry.set_tooltip_text(Some(&t("Hex color")));
    hex_entry.add_css_class("editor-motion-color-hex-entry");

    chooser.connect_rgba_notify({
        let hex_entry = hex_entry.clone();
        let on_changed = on_changed.clone();
        let selected = selected.clone();
        let swatch = swatch.clone();
        move |chooser| {
            let rgba = chooser.rgba();
            *selected.borrow_mut() = rgba;
            swatch.queue_draw();
            let hex = motion_color_hex(rgba);
            if hex_entry.text().as_str() != hex {
                hex_entry.set_text(&hex);
            }
            on_changed(rgba);
        }
    });
    hex_entry.connect_changed({
        let chooser = chooser.clone();
        move |entry| {
            if let Some(rgba) = motion_color_from_hex(entry.text().as_str()) {
                if chooser.rgba() != rgba {
                    chooser.set_rgba(&rgba);
                }
            }
        }
    });

    let body = GtkBox::new(Orientation::Vertical, 0);
    body.add_css_class("editor-motion-color-popover-body");
    body.set_width_request(280);
    body.append(&chooser);

    let popover = Popover::new();
    popover.set_has_arrow(false);
    popover.set_position(gtk4::PositionType::Bottom);
    popover.set_offset(0, 4);
    popover.add_css_class("editor-motion-color-popover");
    popover.set_child(Some(&body));
    trigger.set_popover(Some(&popover));
    let trigger_host = Overlay::new();
    trigger_host.set_size_request(32, 32);
    trigger_host.set_halign(Align::Start);
    trigger_host.set_valign(Align::Start);
    trigger_host.set_hexpand(false);
    trigger_host.set_vexpand(false);
    trigger_host.set_child(Some(&trigger));
    trigger_host.add_overlay(&swatch);

    let control = GtkBox::new(Orientation::Horizontal, 8);
    control.add_css_class("editor-motion-color-control");
    control.append(&trigger_host);
    control.append(&hex_entry);
    control
}

/// The two gradient stops share one hex field.  Selecting either swatch moves
/// that field to the corresponding color, which keeps the Motion inspector
/// narrow while making the edited stop unambiguous.
pub(super) fn motion_gradient_color_control(
    initial_start: gdk::RGBA,
    initial_end: gdk::RGBA,
    on_changed: impl Fn(usize, gdk::RGBA) + 'static,
) -> (GtkBox, Rc<dyn Fn(gdk::RGBA, gdk::RGBA)>) {
    let on_changed = Rc::new(on_changed);
    let colors = Rc::new(RefCell::new([initial_start, initial_end]));
    let active_stop = Rc::new(Cell::new(0usize));
    // Programmatic updates (gradient presets) must not re-enter `on_changed`,
    // which would clear the preset selection the update is installing.
    let syncing = Rc::new(Cell::new(false));

    let (start_host, start_button, start_swatch, start_chooser, start_swatch_color) =
        motion_gradient_stop_button(initial_start, "Gradient start color");
    let (end_host, end_button, end_swatch, end_chooser, end_swatch_color) =
        motion_gradient_stop_button(initial_end, "Gradient end color");
    start_button.add_css_class("active-motion-gradient-stop");

    let hex_entry = Entry::new();
    hex_entry.set_width_chars(9);
    hex_entry.set_text(&motion_color_hex(initial_start));
    hex_entry.set_tooltip_text(Some(&t("Selected gradient color")));
    hex_entry.add_css_class("editor-motion-color-hex-entry");

    let stop_buttons = [start_button.clone(), end_button.clone()];
    let refresh_active_stop = Rc::new({
        let active_stop = active_stop.clone();
        let colors = colors.clone();
        let hex_entry = hex_entry.clone();
        move || {
            let selected = active_stop.get();
            for (index, button) in stop_buttons.iter().enumerate() {
                if index == selected {
                    button.add_css_class("active-motion-gradient-stop");
                } else {
                    button.remove_css_class("active-motion-gradient-stop");
                }
            }
            let hex = motion_color_hex(colors.borrow()[selected]);
            if hex_entry.text().as_str() != hex {
                hex_entry.set_text(&hex);
            }
        }
    });

    for (index, button) in [start_button.clone(), end_button.clone()]
        .into_iter()
        .enumerate()
    {
        let select_stop = gtk4::GestureClick::new();
        select_stop.connect_pressed({
            let active_stop = active_stop.clone();
            let refresh_active_stop = refresh_active_stop.clone();
            move |_, _, _, _| {
                active_stop.set(index);
                refresh_active_stop();
            }
        });
        button.add_controller(select_stop);
    }

    let swatches = [start_swatch.clone(), end_swatch.clone()];
    let swatch_colors = [start_swatch_color, end_swatch_color];
    for (index, chooser) in [start_chooser.clone(), end_chooser.clone()]
        .into_iter()
        .enumerate()
    {
        chooser.connect_rgba_notify({
            let active_stop = active_stop.clone();
            let colors = colors.clone();
            let hex_entry = hex_entry.clone();
            let swatches = swatches.clone();
            let swatch_color = swatch_colors[index].clone();
            let on_changed = on_changed.clone();
            let syncing = syncing.clone();
            move |chooser| {
                if syncing.get() {
                    return;
                }
                let rgba = chooser.rgba();
                colors.borrow_mut()[index] = rgba;
                *swatch_color.borrow_mut() = rgba;
                swatches[index].queue_draw();
                if active_stop.get() == index {
                    let hex = motion_color_hex(rgba);
                    if hex_entry.text().as_str() != hex {
                        hex_entry.set_text(&hex);
                    }
                }
                on_changed(index, rgba);
            }
        });
    }

    hex_entry.connect_changed({
        let active_stop = active_stop.clone();
        let colors = colors.clone();
        let choosers = [start_chooser.clone(), end_chooser.clone()];
        move |entry| {
            let Some(rgba) = motion_color_from_hex(entry.text().as_str()) else {
                return;
            };
            let selected = active_stop.get();
            if colors.borrow()[selected] != rgba {
                choosers[selected].set_rgba(&rgba);
            }
        }
    });

    let apply_colors = Rc::new({
        let colors = colors.clone();
        let swatches = swatches.clone();
        let swatch_colors = swatch_colors.clone();
        let choosers = [start_chooser, end_chooser];
        let hex_entry = hex_entry.clone();
        let syncing = syncing.clone();
        move |start: gdk::RGBA, end: gdk::RGBA| {
            syncing.set(true);
            *colors.borrow_mut() = [start, end];
            for (index, rgba) in [start, end].into_iter().enumerate() {
                *swatch_colors[index].borrow_mut() = rgba;
                swatches[index].queue_draw();
                choosers[index].set_rgba(&rgba);
            }
            hex_entry.set_text(&motion_color_hex(start));
            syncing.set(false);
        }
    });

    let control = GtkBox::new(Orientation::Horizontal, 8);
    control.add_css_class("editor-motion-gradient-color-control");
    control.append(&start_host);
    control.append(&end_host);
    control.append(&hex_entry);
    (control, apply_colors)
}

fn motion_gradient_stop_button(
    initial: gdk::RGBA,
    tooltip: &str,
) -> (
    Overlay,
    MenuButton,
    DrawingArea,
    ColorChooserWidget,
    Rc<RefCell<gdk::RGBA>>,
) {
    let trigger = MenuButton::new();
    trigger.set_size_request(32, 32);
    trigger.set_halign(Align::Start);
    trigger.set_valign(Align::Start);
    trigger.set_hexpand(false);
    trigger.set_vexpand(false);
    trigger.set_tooltip_text(Some(&t(tooltip)));
    trigger.add_css_class("editor-motion-color-button");
    trigger.add_css_class("editor-motion-gradient-stop");

    let swatch = DrawingArea::new();
    // Fill the stop button's inner area so the selected outline frames the
    // color itself instead of looking like a second, padded border.
    swatch.set_content_width(28);
    swatch.set_content_height(28);
    swatch.set_can_target(false);
    swatch.set_halign(Align::Center);
    swatch.set_valign(Align::Center);
    swatch.add_css_class("editor-motion-color-swatch");
    let selected = Rc::new(RefCell::new(initial));
    swatch.set_draw_func({
        let selected = selected.clone();
        move |_area, context, width, height| {
            let rgba = selected.borrow();
            context.set_source_rgba(
                rgba.red().into(),
                rgba.green().into(),
                rgba.blue().into(),
                rgba.alpha().into(),
            );
            context.rectangle(0.0, 0.0, width as f64, height as f64);
            context.fill().ok();
        }
    });
    let chooser = ColorChooserWidget::new();
    chooser.set_use_alpha(true);
    chooser.set_rgba(&initial);
    chooser.set_size_request(260, 260);
    chooser.set_hexpand(false);
    chooser.set_vexpand(false);
    chooser.add_css_class("editor-motion-color-chooser");
    let body = GtkBox::new(Orientation::Vertical, 0);
    body.add_css_class("editor-motion-color-popover-body");
    body.set_width_request(280);
    body.append(&chooser);

    let popover = Popover::new();
    popover.set_has_arrow(false);
    popover.set_position(gtk4::PositionType::Bottom);
    popover.set_offset(0, 4);
    popover.add_css_class("editor-motion-color-popover");
    popover.set_child(Some(&body));
    trigger.set_popover(Some(&popover));

    let trigger_host = Overlay::new();
    trigger_host.set_size_request(32, 32);
    trigger_host.set_halign(Align::Start);
    trigger_host.set_valign(Align::Start);
    trigger_host.set_hexpand(false);
    trigger_host.set_vexpand(false);
    trigger_host.set_child(Some(&trigger));
    trigger_host.add_overlay(&swatch);

    (trigger_host, trigger, swatch, chooser, selected)
}

pub(super) fn motion_color_hex(rgba: gdk::RGBA) -> String {
    let component = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    let (r, g, b, a) = (
        component(rgba.red()),
        component(rgba.green()),
        component(rgba.blue()),
        component(rgba.alpha()),
    );
    if a == 255 {
        format!("#{r:02X}{g:02X}{b:02X}")
    } else {
        format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
    }
}

pub(super) fn motion_color_from_hex(value: &str) -> Option<gdk::RGBA> {
    let value = value.trim().trim_start_matches('#');
    if !matches!(value.len(), 6 | 8) {
        return None;
    }
    let byte = |range: std::ops::Range<usize>| u8::from_str_radix(&value[range], 16).ok();
    let (r, g, b) = (byte(0..2)?, byte(2..4)?, byte(4..6)?);
    let a = if value.len() == 8 { byte(6..8)? } else { 255 };
    Some(gdk::RGBA::new(
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0,
        f32::from(a) / 255.0,
    ))
}

pub(super) fn motion_rgba([red, green, blue, alpha]: [f64; 4]) -> gdk::RGBA {
    gdk::RGBA::new(red as f32, green as f32, blue as f32, alpha as f32)
}

pub(super) fn motion_appearance_slider(
    title: &str,
    min: f64,
    max: f64,
    value: f64,
    suffix: &'static str,
) -> FillSlider {
    let slider = FillSlider::new_with_value_text(&t(title), move |value, _, _| {
        if suffix == "%" {
            format!("{:.0}%", value * 100.0)
        } else {
            format!("{value:.0}{suffix}")
        }
    });
    slider.set_range(min, max);
    slider.set_increments((max - min) / 100.0, (max - min) / 10.0);
    slider.set_value(value);
    slider
}

/// Slider for values stored as "reference px against a 400px long edge"
/// (padding, corner radius). The stored unit is a fraction of the long edge,
/// so the honest readout is a percentage of it: a stored 40 renders as 10% per
/// side on any image size. Labelling these values "px" reads as literal pixels,
/// which is wrong for every image whose long edge is not exactly 400px.
pub(super) fn motion_reference_percent_slider(
    title: &str,
    min: f64,
    max: f64,
    value: f64,
) -> FillSlider {
    let slider =
        FillSlider::new_with_value_text(&t(title), |value, _, _| format!("{:.0}%", value / 4.0));
    slider.set_range(min, max);
    slider.set_increments((max - min) / 100.0, (max - min) / 10.0);
    slider.set_value(value);
    slider
}

pub(super) fn format_duration_label(duration: f64) -> String {
    format!("{duration:.1}s")
}

pub(super) fn span_slider_row(
    title: &str,
    initial: f64,
    min: f64,
    max: f64,
) -> (GtkBox, Label, FillSlider) {
    let header = GtkBox::new(Orientation::Horizontal, 8);
    let value = Label::new(Some(&format!("{:.0}%", initial * 100.0)));
    header.set_visible(false);
    let slider =
        FillSlider::new_with_value_text(title, |value, _, _| format!("{:.0}%", value * 100.0));
    slider.set_range(min, max);
    slider.set_increments(0.01, 0.1);
    slider.set_value(initial);
    (header, value, slider)
}

/// Motion position is stored as a normalized value for resolution-independent
/// rendering, but the inspector mirrors the direct-manipulation control in
/// whole 3D position units (for example, `-31` and `303`).
pub(super) fn position_slider_row(title: &str, initial: f64) -> (GtkBox, Label, FillSlider) {
    let header = GtkBox::new(Orientation::Horizontal, 8);
    let value = Label::new(Some(&format!("{:.0}", initial * 1000.0)));
    header.set_visible(false);
    let slider =
        FillSlider::new_with_value_text(title, |value, _, _| format!("{:.0}", value * 1000.0));
    slider.set_range(MIN_MOTION_POS, MAX_MOTION_POS);
    // One visible position unit per drag increment.
    slider.set_increments(0.001, 0.1);
    slider.set_value(initial);
    (header, value, slider)
}

pub(super) fn angle_slider_row(title: &str, initial: f64) -> (GtkBox, Label, FillSlider) {
    let header = GtkBox::new(Orientation::Horizontal, 8);
    let value = Label::new(Some(&format!("{initial:.0}°")));
    header.set_visible(false);
    let slider = FillSlider::new_with_value_text(title, |value, _, _| format!("{value:.0}°"));
    slider.set_range(MIN_MOTION_YAW, MAX_MOTION_YAW);
    slider.set_increments(1.0, 5.0);
    slider.set_value(initial);
    (header, value, slider)
}

/// Named cubic-Bézier presets for the Ease timing mode. Snappy is the
/// default curve; Smooth is the neutral S; Linear matches constant motion.
pub(super) const EASE_PRESETS: [(&str, f64, f64, f64, f64); 3] = [
    ("Smooth", 0.42, 0.0, 0.58, 1.0),
    ("Snappy", 0.25, 1.0, 0.50, 1.0),
    ("Linear", 0.0, 0.0, 1.0, 1.0),
];

/// Named spring presets as first-peak overshoot fractions: Smooth never
/// passes the target, Gentle (the default) rounds off softly, and Bouncy is
/// the playful one. The Bounce slider covers everything in between.
pub(super) const SPRING_PRESETS: [(&str, f64); 3] =
    [("Smooth", 0.0), ("Gentle", 0.05), ("Bouncy", 0.35)];

pub(super) fn ease_preset_timing(
    index: usize,
    base: MotionEffectTransformTiming,
) -> MotionEffectTransformTiming {
    let (_, x1, y1, x2, y2) = EASE_PRESETS[index];
    MotionEffectTransformTiming {
        easing_x1: x1,
        easing_y1: y1,
        easing_x2: x2,
        easing_y2: y2,
        ..base
    }
}

pub(super) fn spring_preset_timing(
    index: usize,
    base: MotionEffectTransformTiming,
) -> MotionEffectTransformTiming {
    MotionEffectTransformTiming {
        kind: MotionTimingKind::Spring,
        spring_bounce: SPRING_PRESETS[index].1,
        ..base
    }
}

#[allow(dead_code)]
pub(super) fn matching_ease_preset(timing: MotionEffectTransformTiming) -> Option<usize> {
    EASE_PRESETS.iter().position(|(_, x1, y1, x2, y2)| {
        (timing.easing_x1 - x1).abs() < 1e-6
            && (timing.easing_y1 - y1).abs() < 1e-6
            && (timing.easing_x2 - x2).abs() < 1e-6
            && (timing.easing_y2 - y2).abs() < 1e-6
    })
}

#[allow(dead_code)]
pub(super) fn matching_spring_preset(timing: MotionEffectTransformTiming) -> Option<usize> {
    SPRING_PRESETS
        .iter()
        .position(|(_, bounce)| (timing.spring_bounce - bounce).abs() < 1e-6)
}

/// Small curve preview for the timing buttons. The polyline is drawn from
/// the same `apply` the renderer uses, so the icon is literally the motion
/// it applies. A faint baseline marks the target (1.0): Ease touches it at
/// the end while a spring visibly crosses it, which keeps even a gentle
/// 5 % overshoot readable at icon size.
pub(super) fn timing_curve_icon(
    timing: MotionEffectTransformTiming,
    width: i32,
    height: i32,
) -> DrawingArea {
    let area = DrawingArea::new();
    area.set_content_width(width);
    area.set_content_height(height);
    area.set_halign(Align::Center);
    area.set_valign(Align::Center);
    area.set_can_target(false);
    area.set_draw_func(move |area, context, width, height| {
        let color = area.style_context().color();
        let pad = 3.0_f64;
        let usable_w = (f64::from(width) - pad * 2.0).max(1.0);
        let usable_h = (f64::from(height) - pad * 2.0).max(1.0);
        const SAMPLES: usize = 48;
        let values: Vec<f64> = (0..=SAMPLES)
            .map(|index| timing.apply(index as f64 / SAMPLES as f64))
            .collect();
        let top = values
            .iter()
            .fold(1.0_f64, |peak, value| peak.max(*value))
            .max(0.001);
        let point = |t: f64, value: f64| (pad + t * usable_w, pad + (1.0 - value / top) * usable_h);
        // ponytail: dashed baseline, springs cross it while ease only touches it
        let baseline_y = pad + (1.0 - 1.0 / top) * usable_h;
        context.set_source_rgba(
            color.red().into(),
            color.green().into(),
            color.blue().into(),
            f64::from(color.alpha()) * 0.35,
        );
        context.set_line_width(1.0);
        context.set_dash(&[2.0, 2.0], 0.0);
        context.move_to(pad, baseline_y);
        context.line_to(pad + usable_w, baseline_y);
        context.stroke().ok();
        context.set_dash(&[], 0.0);
        context.set_source_rgba(
            color.red().into(),
            color.green().into(),
            color.blue().into(),
            f64::from(color.alpha()) * 0.9,
        );
        context.set_line_width(1.6);
        context.set_line_cap(LineCap::Round);
        context.set_line_join(gtk4::cairo::LineJoin::Round);
        for (index, value) in values.iter().enumerate() {
            let (x, y) = point(index as f64 / SAMPLES as f64, *value);
            if index == 0 {
                context.move_to(x, y);
            } else {
                context.line_to(x, y);
            }
        }
        context.stroke().ok();
    });
    area
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timing_presets_match_their_curves() {
        let base = MotionEffectTransformTiming::default();
        for (index, _) in EASE_PRESETS.iter().enumerate() {
            assert_eq!(
                matching_ease_preset(ease_preset_timing(index, base)),
                Some(index)
            );
        }
        for (index, _) in SPRING_PRESETS.iter().enumerate() {
            assert_eq!(
                matching_spring_preset(spring_preset_timing(index, base)),
                Some(index)
            );
        }
        let custom = MotionEffectTransformTiming {
            easing_x1: 0.1,
            easing_y1: 0.2,
            ..base
        };
        assert_eq!(matching_ease_preset(custom), None);
    }

    #[test]
    fn motion_color_hex_input_round_trips_rgb_and_alpha() {
        let opaque = motion_color_from_hex("#FE8040").expect("opaque hex parses");
        assert_eq!(motion_color_hex(opaque), "#FE8040");
        let translucent = motion_color_from_hex("80A0C040").expect("rgba hex parses");
        assert_eq!(motion_color_hex(translucent), "#80A0C040");
        assert!(motion_color_from_hex("#bad").is_none());
    }
}
