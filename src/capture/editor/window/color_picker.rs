use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CssProvider, Entry, EventControllerMotion, GestureClick, Image, Label,
    MenuButton, Orientation, Overlay, Popover, Scale,
};
use image::RgbaImage;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::i18n::t;

use super::super::color::{
    load_persisted_custom_slot_colors, parse_hex_rgb, DEFAULT_COLOR_INDEX, DRAW_COLORS,
};
use super::super::state::EditorState;
use super::super::types::{BackgroundStyle, DrawColor, PickerColorState, Point, Tool};
use super::super::ui_support::color_swatch_button;
use crate::capture::editor::window::icon_names;

const PICKER_PANEL_WIDTH: i32 = 232;
const PICKER_GRADIENT_HEIGHT: i32 = 200;

pub struct ColorPickerParts {
    pub trigger_host: Overlay,
    pub popover: Popover,
    pub color_buttons: Vec<Button>,
    pub color_picker_dot: GtkBox,
    pub color_class_names: Vec<&'static str>,
    pub eyedropper_btn: Button,
    pub sync_for_active_tool: Rc<dyn Fn()>,
    pub sync_picker_from_color: Rc<dyn Fn(DrawColor)>,
    pub apply_picker_color: Rc<dyn Fn(DrawColor)>,
    pub set_picker_panel_visibility: Rc<dyn Fn(bool)>,
    pub custom_slot_colors: Rc<RefCell<Vec<Option<DrawColor>>>>,
    pub refresh_custom_color_slots: Rc<dyn Fn()>,
    pub register_external_sync: Rc<dyn Fn(Rc<dyn Fn()>)>,
}

pub fn build_color_picker(
    state: Arc<Mutex<EditorState>>,
    canvas_queue_draw_signal: Rc<dyn Fn()>,
    drawing_area: Rc<RefCell<Option<glib::object::WeakRef<gtk4::DrawingArea>>>>,
    _show_color_names: bool,
) -> ColorPickerParts {
    // Color specs (kept for trigger-dot + palette-index mapping; no palette UI).
    let color_specs = [
        ("Black", "editor-color-black"),
        ("Blue", "editor-color-blue"),
        ("Dark Green", "editor-color-dark-green"),
        ("Red", "editor-color-red"),
        ("Orange", "editor-color-orange"),
        ("Yellow", "editor-color-yellow"),
        ("Green", "editor-color-green"),
        ("Cyan", "editor-color-cyan"),
        ("Blue Bright", "editor-color-blue-bright"),
        ("Purple", "editor-color-purple"),
        ("Pink", "editor-color-pink"),
        ("White", "editor-color-white"),
    ];
    let visible_color_specs = &color_specs[..10];
    let color_class_names: Vec<&'static str> = color_specs
        .iter()
        .map(|(_, class_name)| *class_name)
        .collect();
    let color_buttons: Vec<Button> = visible_color_specs
        .iter()
        .map(|(tooltip, class_name)| color_swatch_button(class_name, tooltip))
        .collect();

    // Color picker trigger (unchanged).
    let color_picker_trigger = MenuButton::new();
    color_picker_trigger.set_has_frame(false);
    color_picker_trigger.set_focusable(false);
    color_picker_trigger.set_can_target(false);
    color_picker_trigger.set_tooltip_text(Some(&t("Colors")));
    color_picker_trigger.set_icon_name("");
    color_picker_trigger.set_hexpand(true);
    color_picker_trigger.set_vexpand(true);
    color_picker_trigger.set_halign(gtk4::Align::Fill);
    color_picker_trigger.set_valign(gtk4::Align::Fill);
    color_picker_trigger.add_css_class("editor-color-trigger-menu-button");
    color_picker_trigger.add_css_class("flat");

    let color_picker_dot = GtkBox::new(Orientation::Horizontal, 0);
    color_picker_dot.set_size_request(20, 20);
    color_picker_dot.set_halign(gtk4::Align::Center);
    color_picker_dot.set_valign(gtk4::Align::Center);
    color_picker_dot.add_css_class("editor-color-trigger-dot");
    color_picker_dot.add_css_class(color_specs[DEFAULT_COLOR_INDEX].1);
    color_picker_dot.set_widget_name("editor-color-trigger-dot");

    // Exact-color override for the trigger dot: the free gradient picker can
    // produce any color, so the dot shows the real pick instead of the
    // nearest palette swatch. ID selector outranks the palette classes.
    let trigger_dot_css = CssProvider::new();
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &trigger_dot_css,
            gtk4::STYLE_PROVIDER_PRIORITY_USER,
        );
    }
    let set_trigger_dot_exact_color: Rc<dyn Fn(DrawColor)> = Rc::new({
        let trigger_dot_css = trigger_dot_css.clone();
        move |color| {
            let (r, g, b, _) = super::super::color::draw_color_to_rgba_u8(color);
            let alpha = color.a.clamp(0.0, 1.0);
            trigger_dot_css.load_from_data(&format!(
                "#editor-color-trigger-dot {{ background: rgba({r}, {g}, {b}, {alpha:.3}); background-image: none; }}"
            ));
        }
    });
    let clear_trigger_dot_exact_color: Rc<dyn Fn()> = Rc::new({
        let trigger_dot_css = trigger_dot_css.clone();
        move || {
            trigger_dot_css.load_from_data("");
        }
    });

    let trigger_divider = GtkBox::new(Orientation::Vertical, 0);
    trigger_divider.add_css_class("editor-color-trigger-divider");

    let color_picker_arrow_box = GtkBox::new(Orientation::Horizontal, 0);
    color_picker_arrow_box.add_css_class("editor-color-trigger-arrow-box");
    color_picker_arrow_box.set_halign(gtk4::Align::Center);
    color_picker_arrow_box.set_valign(gtk4::Align::Center);
    let color_picker_arrow = Image::from_icon_name(icon_names::CHEVRON_DOWN_REGULAR);
    color_picker_arrow.set_pixel_size(10);
    color_picker_arrow.add_css_class("editor-color-trigger-arrow");
    color_picker_arrow_box.append(&color_picker_arrow);

    let color_picker_trigger_shell = GtkBox::new(Orientation::Horizontal, 0);
    color_picker_trigger_shell.add_css_class("editor-color-trigger-shell");
    color_picker_trigger_shell.set_valign(gtk4::Align::Center);
    color_picker_trigger_shell.append(&color_picker_dot);
    color_picker_trigger_shell.append(&trigger_divider);
    color_picker_trigger_shell.append(&color_picker_arrow_box);

    let color_picker_trigger_host = Overlay::new();
    color_picker_trigger_host.set_child(Some(&color_picker_trigger_shell));
    color_picker_trigger_host.add_overlay(&color_picker_trigger);

    let color_picker_trigger_popup = color_picker_trigger.clone();
    let color_picker_shell_click = GestureClick::new();
    color_picker_shell_click.connect_pressed(move |_, _, _, _| {
        color_picker_trigger_popup.popup();
    });
    color_picker_trigger_shell.add_controller(color_picker_shell_click);

    // Popover
    let color_popover = Popover::new();
    color_popover.set_has_arrow(false);
    color_popover.set_autohide(true);
    color_popover.set_position(gtk4::PositionType::Bottom);
    color_popover.set_offset(0, 4);
    color_popover.add_css_class("editor-color-popover");

    // Shared custom colors stay alive for the sidebar panel; this popover
    // no longer renders its own slots.
    let custom_slot_colors = Rc::new(RefCell::new(load_persisted_custom_slot_colors(
        color_buttons.len(),
    )));
    let refresh_custom_color_slots: Rc<dyn Fn()> = Rc::new(|| {});

    // Picker panel (always visible — the mockup has no collapsed state).
    let picker_panel = GtkBox::new(Orientation::Vertical, 10);
    picker_panel.add_css_class("editor-color-picker-panel");
    picker_panel.add_css_class("editor-color-picker-panel-minimal");
    picker_panel.set_halign(gtk4::Align::Start);
    picker_panel.set_hexpand(false);
    picker_panel.set_width_request(PICKER_PANEL_WIDTH);
    picker_panel.set_visible(true);

    let picker_state = Rc::new(RefCell::new(PickerColorState::from_color(
        DRAW_COLORS[DEFAULT_COLOR_INDEX],
    )));
    let picker_update_in_progress = Rc::new(Cell::new(false));
    let external_sync = Rc::new(RefCell::new(None::<Rc<dyn Fn()>>));

    // Header: title + eyedropper (the only retained tool).
    let header_row = GtkBox::new(Orientation::Horizontal, 8);
    header_row.add_css_class("editor-color-picker-header");
    header_row.set_halign(gtk4::Align::Fill);
    header_row.set_hexpand(true);
    let title = Label::new(Some(&t("Color picker")));
    title.add_css_class("editor-color-picker-title");
    title.set_halign(gtk4::Align::Start);
    title.set_hexpand(true);
    title.set_xalign(0.0);

    let eyedropper_btn = Button::new();
    eyedropper_btn.set_has_frame(false);
    eyedropper_btn.set_focusable(false);
    eyedropper_btn.set_tooltip_text(Some(&t("Pick from screen")));
    eyedropper_btn.set_halign(gtk4::Align::End);
    eyedropper_btn.add_css_class("editor-eyedropper-button");
    eyedropper_btn.add_css_class("editor-eyedropper-button-minimal");
    let eyedropper_icon = Image::from_icon_name(icon_names::custom::PIPETTE_SYMBOLIC);
    eyedropper_icon.set_pixel_size(15);
    eyedropper_btn.set_child(Some(&eyedropper_icon));

    header_row.append(&title);
    header_row.append(&eyedropper_btn);

    // Gradient area
    let gradient_area = gtk4::DrawingArea::new();
    gradient_area.set_content_width(PICKER_PANEL_WIDTH);
    gradient_area.set_content_height(PICKER_GRADIENT_HEIGHT);
    gradient_area.set_size_request(PICKER_PANEL_WIDTH, PICKER_GRADIENT_HEIGHT);
    gradient_area.set_halign(gtk4::Align::Start);
    gradient_area.set_hexpand(false);
    gradient_area.add_css_class("editor-gradient-area");
    let picker_state_draw = picker_state.clone();
    gradient_area.set_draw_func(move |_area, cr: &gtk4::cairo::Context, width, height| {
        let picker = *picker_state_draw.borrow();
        let w = width as f64;
        let h = height as f64;
        let (hue_r, hue_g, hue_b) = super::super::types::hsv_to_rgb(picker.hue, 1.0, 1.0);
        cr.set_source_rgb(hue_r, hue_g, hue_b);
        cr.rectangle(0.0, 0.0, w, h);
        let _ = cr.fill();
        let white_grad = gtk4::cairo::LinearGradient::new(0.0, 0.0, w, 0.0);
        white_grad.add_color_stop_rgba(0.0, 1.0, 1.0, 1.0, 1.0);
        white_grad.add_color_stop_rgba(1.0, 1.0, 1.0, 1.0, 0.0);
        let _ = cr.set_source(&white_grad);
        cr.rectangle(0.0, 0.0, w, h);
        let _ = cr.fill();
        let black_grad = gtk4::cairo::LinearGradient::new(0.0, 0.0, 0.0, h);
        black_grad.add_color_stop_rgba(0.0, 0.0, 0.0, 0.0, 0.0);
        black_grad.add_color_stop_rgba(1.0, 0.0, 0.0, 0.0, 1.0);
        let _ = cr.set_source(&black_grad);
        cr.rectangle(0.0, 0.0, w, h);
        let _ = cr.fill();
        let cx = (picker.saturation * w).clamp(10.0, w - 10.0);
        let cy = ((1.0 - picker.value) * h).clamp(10.0, h - 10.0);
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.arc(cx, cy, 10.0, 0.0, std::f64::consts::TAU);
        let _ = cr.fill_preserve();
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.28);
        cr.set_line_width(1.5);
        let _ = cr.stroke();
    });

    // Hue slider + live current-color preview (restored from the old picker:
    // the gradient shows every color, so this chip is what shows YOUR color).
    let hue_slider = Scale::with_range(Orientation::Horizontal, 0.0, 360.0, 1.0);
    hue_slider.set_draw_value(false);
    hue_slider.set_hexpand(true);
    hue_slider.set_halign(gtk4::Align::Fill);
    hue_slider.set_vexpand(false);
    hue_slider.set_valign(gtk4::Align::Center);
    hue_slider.add_css_class("editor-hue-slider");
    hue_slider.add_css_class("editor-hue-slider-minimal");

    let preview_chip = GtkBox::new(Orientation::Horizontal, 0);
    preview_chip.set_size_request(30, 30);
    preview_chip.set_halign(gtk4::Align::Center);
    preview_chip.set_valign(gtk4::Align::Center);
    preview_chip.add_css_class("editor-color-preview");
    preview_chip.set_widget_name("editor-picker-preview");
    let preview_css = CssProvider::new();
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &preview_css,
            gtk4::STYLE_PROVIDER_PRIORITY_USER,
        );
    }

    let hue_row = GtkBox::new(Orientation::Horizontal, 8);
    hue_row.set_halign(gtk4::Align::Fill);
    hue_row.set_hexpand(true);
    hue_row.append(&hue_slider);
    hue_row.append(&preview_chip);

    // Hex entry with leading `#`, matching the mockup.
    let hex_entry = Entry::new();
    hex_entry.set_max_length(7);
    hex_entry.set_width_chars(7);
    hex_entry.set_max_width_chars(7);
    hex_entry.set_placeholder_text(Some("#ffffff"));
    hex_entry.set_halign(gtk4::Align::Fill);
    hex_entry.set_hexpand(true);
    gtk4::prelude::EditableExt::set_alignment(&hex_entry, 0.5);
    hex_entry.add_css_class("editor-hex-entry");
    hex_entry.add_css_class("editor-hex-entry-minimal");

    picker_panel.append(&header_row);
    picker_panel.append(&gradient_area);
    picker_panel.append(&hue_row);
    picker_panel.append(&hex_entry);

    let popover_root = GtkBox::new(Orientation::Horizontal, 0);
    popover_root.add_css_class("editor-color-popover-body");
    popover_root.add_css_class("editor-color-popover-body-minimal");
    popover_root.set_halign(gtk4::Align::Start);
    popover_root.set_hexpand(false);
    popover_root.append(&picker_panel);

    // Panel is always expanded now; keep the hook as a no-op for callers.
    let set_picker_panel_visibility: Rc<dyn Fn(bool)> = Rc::new({
        let picker_panel = picker_panel.clone();
        move |_| {
            picker_panel.set_visible(true);
        }
    });

    color_popover.set_child(Some(&popover_root));
    color_picker_trigger.set_popover(Some(&color_popover));

    let update_picker_ui: Rc<dyn Fn(PickerColorState)> = Rc::new({
        let hue_slider = hue_slider.clone();
        let hex_entry = hex_entry.clone();
        let gradient_area = gradient_area.clone();
        let preview_css = preview_css.clone();
        let picker_update_in_progress = picker_update_in_progress.clone();
        move |picker| {
            picker_update_in_progress.set(true);
            hue_slider.set_value(picker.hue);
            let color = picker.to_color();
            hex_entry.set_text(&format!(
                "#{}",
                super::super::color::draw_color_to_hex(color).to_lowercase()
            ));
            let (r, g, b, _) = super::super::color::draw_color_to_rgba_u8(color);
            let alpha = color.a.clamp(0.0, 1.0);
            preview_css.load_from_data(&format!(
                "#editor-picker-preview {{ background: rgba({r}, {g}, {b}, {alpha:.3}); background-image: none; }}"
            ));
            gradient_area.queue_draw();
            picker_update_in_progress.set(false);
        }
    });

    let apply_picker_color_to_editor: Rc<dyn Fn(DrawColor)> = Rc::new({
        let state_picker_apply = state.clone();
        let color_buttons_picker = color_buttons.clone();
        let color_picker_dot_picker = color_picker_dot.clone();
        let color_class_names_picker = color_class_names.clone();
        let drawing_area_picker = drawing_area.clone();
        let external_sync = external_sync.clone();
        let set_trigger_dot_exact_color_apply = set_trigger_dot_exact_color.clone();
        move |color| {
            let has_active_text = {
                let mut st = state_picker_apply.lock().unwrap();
                let has_active_text = st.active_text_input.is_some();
                if st.selected_tool == Tool::Crop {
                    st.set_crop_background_color(color);
                } else if st.selected_tool == Tool::Background {
                    st.background_style = BackgroundStyle::PlainColor(color);
                } else if has_active_text {
                    st.selected_color = color;
                    let _ = st.set_selected_action_color(color);
                } else {
                    st.selected_color = color;
                }
                has_active_text
            };

            let nearest_index = super::super::color::palette_index_for_color(color);
            clear_active_color_picker_palette_state(&color_buttons_picker);
            set_color_picker_trigger_dot_state(
                &color_picker_dot_picker,
                &color_class_names_picker,
                nearest_index,
            );
            set_trigger_dot_exact_color_apply(color);

            if has_active_text {
                if let Some(area) = drawing_area_picker
                    .borrow()
                    .as_ref()
                    .and_then(|weak| weak.upgrade())
                {
                    area.grab_focus();
                }
            }
            canvas_queue_draw_signal();
            if let Some(sync) = external_sync.borrow().as_ref() {
                sync();
            }
        }
    });

    let sync_picker_from_color: Rc<dyn Fn(DrawColor)> = Rc::new({
        let picker_state = picker_state.clone();
        let update_picker_ui = update_picker_ui.clone();
        move |color| {
            let picker = PickerColorState::from_color(color);
            *picker_state.borrow_mut() = picker;
            update_picker_ui(picker);
        }
    });

    let commit_picker_state: Rc<dyn Fn()> = Rc::new({
        let picker_state = picker_state.clone();
        let update_picker_ui = update_picker_ui.clone();
        let apply_picker_color_to_editor = apply_picker_color_to_editor.clone();
        move || {
            let picker = *picker_state.borrow();
            update_picker_ui(picker);
            apply_picker_color_to_editor(picker.to_color());
        }
    });

    let sync_picker_for_active_tool: Rc<dyn Fn()> = Rc::new({
        let state = state.clone();
        let color_buttons = color_buttons.clone();
        let color_picker_dot = color_picker_dot.clone();
        let color_class_names = color_class_names.clone();
        let sync_picker_from_color = sync_picker_from_color.clone();
        let set_trigger_dot_exact_color_sync = set_trigger_dot_exact_color.clone();
        let clear_trigger_dot_exact_color_sync = clear_trigger_dot_exact_color.clone();
        move || {
            let (active_color, show_palette_state) = {
                let st = state.lock().unwrap();
                if st.selected_tool == Tool::Crop {
                    (st.crop_background_color, st.crop_background_color_explicit)
                } else if st.selected_tool == Tool::Background {
                    if let BackgroundStyle::PlainColor(color) = st.background_style {
                        (color, true)
                    } else {
                        (st.selected_color, false)
                    }
                } else {
                    (st.selected_color, true)
                }
            };
            sync_picker_from_color(active_color);
            clear_active_color_picker_palette_state(&color_buttons);
            if show_palette_state {
                set_color_picker_trigger_dot_state(
                    &color_picker_dot,
                    &color_class_names,
                    super::super::color::palette_index_for_color(active_color),
                );
                set_trigger_dot_exact_color_sync(active_color);
            } else {
                clear_color_picker_trigger_dot_state(&color_picker_dot, &color_class_names);
                clear_trigger_dot_exact_color_sync();
            }
        }
    });

    sync_picker_for_active_tool();

    // Hue slider
    let picker_state_hue = picker_state.clone();
    let picker_update_in_progress_hue = picker_update_in_progress.clone();
    let commit_picker_state_hue = commit_picker_state.clone();
    hue_slider.connect_value_changed(move |slider| {
        if picker_update_in_progress_hue.get() {
            return;
        }
        picker_state_hue.borrow_mut().hue = super::super::types::normalize_hue(slider.value());
        commit_picker_state_hue();
    });

    // Gradient area interactions
    let update_sv_from_position: Rc<dyn Fn(f64, f64)> = Rc::new({
        let gradient_area = gradient_area.clone();
        let picker_state = picker_state.clone();
        let commit_picker_state = commit_picker_state.clone();
        move |x, y| {
            let width = gradient_area.allocated_width().max(1) as f64;
            let height = gradient_area.allocated_height().max(1) as f64;
            let saturation = (x / width).clamp(0.0, 1.0);
            let value = (1.0 - (y / height)).clamp(0.0, 1.0);
            {
                let mut picker = picker_state.borrow_mut();
                picker.saturation = saturation;
                picker.value = value;
            }
            commit_picker_state();
        }
    });

    let gradient_dragging = Rc::new(Cell::new(false));

    let gradient_click = GestureClick::new();
    let gradient_dragging_press = gradient_dragging.clone();
    let update_sv_click = update_sv_from_position.clone();
    gradient_click.connect_pressed(move |_, _, x, y| {
        gradient_dragging_press.set(true);
        update_sv_click(x, y);
    });

    let gradient_dragging_release = gradient_dragging.clone();
    gradient_click.connect_released(move |_, _, _, _| {
        gradient_dragging_release.set(false);
    });
    gradient_area.add_controller(gradient_click);

    let gradient_motion = EventControllerMotion::new();
    let gradient_dragging_motion = gradient_dragging.clone();
    let update_sv_motion = update_sv_from_position.clone();
    gradient_motion.connect_motion(move |_, x, y| {
        if gradient_dragging_motion.get() {
            update_sv_motion(x, y);
        }
    });
    gradient_area.add_controller(gradient_motion);

    // Hex entry (accepts `#rrggbb` or `rrggbb`; alpha is preserved).
    let picker_state_hex = picker_state.clone();
    let picker_update_in_progress_hex = picker_update_in_progress.clone();
    let commit_picker_state_hex = commit_picker_state.clone();
    hex_entry.connect_changed(move |entry| {
        if picker_update_in_progress_hex.get() {
            return;
        }
        let text = entry.text();
        let Some((r, g, b)) = parse_hex_rgb(text.as_str()) else {
            return;
        };
        let (hue, saturation, value) =
            super::super::types::rgb_to_hsv(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
        {
            let mut picker = picker_state_hex.borrow_mut();
            picker.hue = hue;
            picker.saturation = saturation;
            picker.value = value;
        }
        commit_picker_state_hex();
    });

    let register_external_sync: Rc<dyn Fn(Rc<dyn Fn()>)> = Rc::new({
        let external_sync = external_sync.clone();
        move |sync| {
            *external_sync.borrow_mut() = Some(sync);
        }
    });

    ColorPickerParts {
        trigger_host: color_picker_trigger_host,
        popover: color_popover,
        color_buttons,
        color_picker_dot,
        color_class_names,
        eyedropper_btn,
        sync_for_active_tool: sync_picker_for_active_tool,
        sync_picker_from_color,
        apply_picker_color: apply_picker_color_to_editor,
        set_picker_panel_visibility,
        custom_slot_colors,
        refresh_custom_color_slots,
        register_external_sync,
    }
}

pub fn set_active_color_picker_state(
    color_buttons: &[Button],
    trigger_dot: &GtkBox,
    color_classes: &[&str],
    active_index: usize,
) {
    super::super::ui_support::set_active_color_button(color_buttons, active_index);
    set_color_picker_trigger_dot_state(trigger_dot, color_classes, active_index);
}

pub fn clear_active_color_picker_palette_state(color_buttons: &[Button]) {
    for button in color_buttons {
        button.remove_css_class("active-color");
    }
}

pub fn clear_color_picker_trigger_dot_state(trigger_dot: &GtkBox, color_classes: &[&str]) {
    for class_name in color_classes {
        trigger_dot.remove_css_class(class_name);
    }
}

pub fn set_color_picker_trigger_dot_state(
    trigger_dot: &GtkBox,
    color_classes: &[&str],
    active_index: usize,
) {
    clear_color_picker_trigger_dot_state(trigger_dot, color_classes);

    if let Some(class_name) = color_classes.get(active_index) {
        trigger_dot.add_css_class(class_name);
    }
}

pub fn activate_eyedropper(
    color_popover: &Popover,
    state: Arc<Mutex<EditorState>>,
    eyedropper_mode: Rc<Cell<bool>>,
    eyedropper_point: Rc<RefCell<Option<Point>>>,
    eyedropper_rendered: Rc<RefCell<Option<RgbaImage>>>,
    canvas_eyedropper_ring: &gtk4::DrawingArea,
    drawing_area: &gtk4::DrawingArea,
    set_cursor_crosshair: Rc<dyn Fn()>,
) {
    color_popover.popdown();
    eyedropper_mode.set(true);
    *eyedropper_point.borrow_mut() = None;
    *eyedropper_rendered.borrow_mut() = state.lock().unwrap().to_rendered_image().ok();
    canvas_eyedropper_ring.set_visible(false);
    canvas_eyedropper_ring.queue_draw();
    set_cursor_crosshair();

    drawing_area.queue_draw();
}

pub fn connect_eyedropper_activation(
    eyedropper_btn: &Button,
    color_popover: &Popover,
    state: Arc<Mutex<EditorState>>,
    eyedropper_mode: Rc<Cell<bool>>,
    eyedropper_point: Rc<RefCell<Option<Point>>>,
    eyedropper_rendered: Rc<RefCell<Option<RgbaImage>>>,
    canvas_eyedropper_ring: &gtk4::DrawingArea,
    drawing_area: &gtk4::DrawingArea,
    set_cursor_crosshair: Rc<dyn Fn()>,
) {
    let color_popover = color_popover.clone();
    let canvas_eyedropper_ring = canvas_eyedropper_ring.clone();
    let drawing_area = drawing_area.clone();
    eyedropper_btn.connect_clicked(move |_| {
        activate_eyedropper(
            &color_popover,
            state.clone(),
            eyedropper_mode.clone(),
            eyedropper_point.clone(),
            eyedropper_rendered.clone(),
            &canvas_eyedropper_ring,
            &drawing_area,
            set_cursor_crosshair.clone(),
        );
    });
}
