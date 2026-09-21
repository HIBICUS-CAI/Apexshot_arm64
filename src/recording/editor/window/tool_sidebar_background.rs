// Background panel for the recording (video) editor.
//
// Accessed through the left tool rail next to Cursor; opens on the right
// just like the Cursor panel. Unlike the image editor, video only supports
// Wallpaper (same bundled wallpapers as the image editor) and Color.
//
// Included into `tool_sidebar.rs`, so parent imports (gtk, state, FillSlider,
// color dots, `t`, ..) are already in scope; only truly new items are
// imported here.

use std::path::PathBuf;

const BACKGROUND_COLOR_PRESETS: [(u8, u8, u8); 8] = [
    (17, 17, 17),
    (44, 36, 56),
    (176, 92, 56),
    (80, 160, 255),
    (72, 210, 140),
    (255, 200, 80),
    (240, 80, 110),
    (245, 245, 247),
];

fn video_wallpaper_files() -> Vec<&'static str> {
    crate::capture::editor::window::background_panel::MOTION_WALLPAPER_FILES
        .iter()
        .copied()
        .filter(|name| name.starts_with("wallpaper-"))
        .collect()
}

fn wallpaper_full_path(file_name: &str) -> PathBuf {
    crate::capture::editor::window::background_panel::background_gradient_asset_path(file_name)
}

fn wallpaper_thumb_path(file_name: &str) -> PathBuf {
    crate::capture::editor::window::background_panel::motion_wallpaper_preview_asset_path(file_name)
}

fn wallpaper_file_name(path: &PathBuf) -> Option<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_owned())
}

struct BackgroundPanel {
    widget: GtkBox,
    refresh: Rc<dyn Fn()>,
}

fn build_background_panel(
    state: Arc<Mutex<VideoEditState>>,
    on_change: Rc<dyn Fn()>,
) -> BackgroundPanel {
    let panel = GtkBox::new(Orientation::Vertical, 0);
    panel.add_css_class("recording-editor-zoom-panel");
    panel.set_hexpand(true);
    panel.set_vexpand(true);

    let header = GtkBox::new(Orientation::Horizontal, 8);
    header.add_css_class("recording-editor-zoom-header");
    header.set_hexpand(true);
    let title = Label::new(Some(&t("Background")));
    title.add_css_class("recording-editor-zoom-title");
    title.set_xalign(0.0);
    title.set_hexpand(true);
    header.append(&title);
    panel.append(&header);

    // Mode row mirrors the image editor Appearance choices, trimmed to the
    // two fills video supports plus an explicit off state.
    let mode_row = GtkBox::new(Orientation::Horizontal, 0);
    mode_row.add_css_class("recording-editor-zoom-mode");
    mode_row.set_hexpand(true);
    mode_row.set_homogeneous(true);
    let none_btn = ToggleButton::with_label(&t("None"));
    none_btn.add_css_class("recording-editor-zoom-mode-btn");
    none_btn.set_has_frame(false);
    none_btn.set_hexpand(true);
    let wallpaper_btn = ToggleButton::with_label(&t("Wallpaper"));
    wallpaper_btn.add_css_class("recording-editor-zoom-mode-btn");
    wallpaper_btn.set_has_frame(false);
    wallpaper_btn.set_hexpand(true);
    wallpaper_btn.set_group(Some(&none_btn));
    let color_btn = ToggleButton::with_label(&t("Color"));
    color_btn.add_css_class("recording-editor-zoom-mode-btn");
    color_btn.set_has_frame(false);
    color_btn.set_hexpand(true);
    color_btn.set_group(Some(&none_btn));
    mode_row.append(&none_btn);
    mode_row.append(&wallpaper_btn);
    mode_row.append(&color_btn);

    let hint = Label::new(Some(&t(
        "Wallpaper or color fills the canvas behind the video in preview and export",
    )));
    hint.add_css_class("recording-editor-zoom-hint");
    hint.set_wrap(true);
    hint.set_xalign(0.0);
    hint.set_max_width_chars(34);

    let body = GtkBox::new(Orientation::Vertical, 8);
    body.add_css_class("recording-editor-zoom-body");
    body.add_css_class("recording-editor-cursor-tab-body");
    body.set_hexpand(true);
    body.append(&mode_row);
    body.append(&hint);

    // --- Wallpaper page ---
    let wallpaper_page = GtkBox::new(Orientation::Vertical, 8);
    wallpaper_page.set_hexpand(true);
    let wallpaper_label = Label::new(Some(&t("WALLPAPER")));
    wallpaper_label.add_css_class("recording-editor-zoom-kicker");
    wallpaper_label.set_xalign(0.0);
    wallpaper_label.set_hexpand(false);
    wallpaper_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    wallpaper_label.set_max_width_chars(24);
    wallpaper_page.append(&wallpaper_label);

    // Thumbnail tiles mirror the image editor: fixed 56px squares with the
    // same button chrome and rounded cover-fit paint, so both editors look
    // identical. Fixed sizes also keep the grid from ever stretching the
    // 288px sidebar when a tile image is large.
    let grid = Grid::new();
    grid.add_css_class("editor-motion-wallpaper-grid");
    grid.set_column_spacing(8);
    grid.set_row_spacing(8);
    grid.set_column_homogeneous(true);
    grid.set_hexpand(false);
    grid.set_halign(Align::Fill);

    let syncing = Rc::new(Cell::new(false));
    let files = video_wallpaper_files();
    let cards: Vec<(String, ToggleButton)> = files
        .iter()
        .enumerate()
        .map(|(index, &file_name)| {
            let card = ToggleButton::new();
            card.add_css_class("editor-background-gradient-button");
            card.add_css_class("editor-background-preview-size-regular");
            card.add_css_class("editor-motion-wallpaper-thumbnail");
            card.set_has_frame(false);
            card.set_size_request(56, 56);
            card.set_hexpand(false);
            card.set_halign(Align::Center);
            card.set_valign(Align::Start);
            card.set_tooltip_text(Some(file_name));
            let thumb = DrawingArea::new();
            thumb.set_content_width(56);
            thumb.set_content_height(56);
            card.set_child(Some(&thumb));
            // Decode the small bundled thumb off the critical path so
            // opening the panel stays instant with 60 tiles.
            {
                let thumb = thumb.clone();
                let thumb_path = wallpaper_thumb_path(file_name);
                glib::idle_add_local_once(move || {
                    let Some(surface) = decode_wallpaper_thumb(&thumb_path) else {
                        return;
                    };
                    thumb.set_draw_func(move |_, cr, width, height| {
                        paint_wallpaper_thumb(cr, &surface, width, height);
                    });
                    thumb.queue_draw();
                });
            }
            {
                let state = state.clone();
                let on_change = on_change.clone();
                let syncing = syncing.clone();
                let file_name = file_name.to_string();
                card.connect_clicked(move |button| {
                    if syncing.get() || !button.is_active() {
                        return;
                    }
                    let full = wallpaper_full_path(&file_name);
                    let mut guard = state.lock().unwrap();
                    // Re-clicking the active wallpaper must not recomposite.
                    if guard.background == VideoBackground::Wallpaper(full.clone()) {
                        return;
                    }
                    guard.background = VideoBackground::Wallpaper(full);
                    drop(guard);
                    on_change();
                });
            }
            grid.attach(&card, (index % 3) as i32, (index / 3) as i32, 1, 1);
            (file_name.to_string(), card)
        })
        .collect();
    if let Some(first) = cards.first().map(|(_, card)| card.clone()) {
        for (_, card) in cards.iter().skip(1) {
            card.set_group(Some(&first));
        }
    }
    wallpaper_page.append(&grid);

    // --- Color page ---
    let color_page = GtkBox::new(Orientation::Vertical, 8);
    color_page.set_hexpand(true);
    let color_label = Label::new(Some(&t("COLOR")));
    color_label.add_css_class("recording-editor-zoom-kicker");
    color_label.set_xalign(0.0);
    color_label.set_hexpand(false);
    color_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    color_label.set_max_width_chars(24);
    color_page.append(&color_label);

    let color_row = GtkBox::new(Orientation::Horizontal, 8);
    color_row.add_css_class("recording-editor-click-color-row");
    color_row.set_hexpand(true);
    let swatch_label = Label::new(Some(&t("Color")));
    swatch_label.add_css_class("recording-editor-zoom-classic-label");
    swatch_label.set_xalign(0.0);
    swatch_label.set_hexpand(true);
    swatch_label.set_valign(Align::Center);
    let swatch = Button::new();
    swatch.add_css_class("recording-editor-click-color-swatch");
    swatch.set_has_frame(false);
    swatch.set_tooltip_text(Some(&t("Choose background color")));
    let swatch_paint = DrawingArea::new();
    swatch_paint.set_content_width(28);
    swatch_paint.set_content_height(22);
    swatch_paint.set_can_target(false);
    swatch_paint.set_draw_func({
        let state = state.clone();
        move |_, cr, width, height| {
            let (r, g, b) = match &state.lock().unwrap().background {
                VideoBackground::Plain { r, g, b } => (*r, *g, *b),
                _ => (17, 17, 17),
            };
            draw_color_chip(cr, width as f64, height as f64, (r, g, b), 6.0);
        }
    });
    swatch.set_child(Some(&swatch_paint));
    swatch.connect_clicked({
        let state = state.clone();
        let on_change = on_change.clone();
        move |button| open_background_color_dialog(button, state.clone(), on_change.clone())
    });
    let hex = Label::new(Some("#111111"));
    hex.add_css_class("recording-editor-click-color-hex");
    hex.set_xalign(0.0);
    hex.set_valign(Align::Center);
    color_row.append(&swatch_label);
    color_row.append(&swatch);
    color_row.append(&hex);
    color_page.append(&color_row);

    let dots = GtkBox::new(Orientation::Horizontal, 6);
    dots.add_css_class("recording-editor-click-color-dots");
    dots.set_halign(Align::End);
    for color in BACKGROUND_COLOR_PRESETS {
        let dot = color_dot_button(color);
        dot.connect_clicked({
            let state = state.clone();
            let on_change = on_change.clone();
            move |_| {
                state.lock().unwrap().background = VideoBackground::Plain {
                    r: color.0,
                    g: color.1,
                    b: color.2,
                };
                on_change();
            }
        });
        dots.append(&dot);
    }
    color_page.append(&dots);

    // Padding controls how much wallpaper/color surrounds the video. It
    // lives above the fill pages so it is visible on both Wallpaper and
    // Color, and hidden entirely when there is no background.
    let padding_row = cursor_slider_row(&t("Padding"));
    padding_row.scale.set_range(0.0, 80.0);
    padding_row.scale.set_increments(1.0, 4.0);
    padding_row.scale.connect_value_changed({
        let state = state.clone();
        let on_change = on_change.clone();
        let syncing = syncing.clone();
        move |scale| {
            if syncing.get() {
                return;
            }
            state.lock().unwrap().background_padding = scale.value();
            on_change();
        }
    });

    let pages = GtkBox::new(Orientation::Vertical, 0);
    pages.set_hexpand(true);
    pages.append(&wallpaper_page);
    pages.append(&color_page);
    body.append(&padding_row.widget);
    body.append(&pages);

    let scroll = ScrolledWindow::new();
    scroll.add_css_class("recording-editor-zoom-scroll");
    scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    scroll.set_vexpand(true);
    scroll.set_hexpand(true);
    scroll.set_child(Some(&body));
    panel.append(&scroll);

    // Mode switching only flips the stored fill; tile/dot clicks pick values.
    none_btn.connect_clicked({
        let state = state.clone();
        let on_change = on_change.clone();
        let syncing = syncing.clone();
        move |button| {
            if syncing.get() || !button.is_active() {
                return;
            }
            state.lock().unwrap().background = VideoBackground::None;
            on_change();
        }
    });
    wallpaper_btn.connect_clicked({
        let state = state.clone();
        let on_change = on_change.clone();
        let syncing = syncing.clone();
        move |button| {
            if syncing.get() || !button.is_active() {
                return;
            }
            let mut guard = state.lock().unwrap();
            if !matches!(guard.background, VideoBackground::Wallpaper(_)) {
                guard.background = VideoBackground::Wallpaper(wallpaper_full_path(
                    video_wallpaper_files()
                        .first()
                        .copied()
                        .unwrap_or("wallpaper-001.jpg"),
                ));
                drop(guard);
                on_change();
            }
        }
    });
    color_btn.connect_clicked({
        let state = state.clone();
        let on_change = on_change.clone();
        let syncing = syncing.clone();
        move |button| {
            if syncing.get() || !button.is_active() {
                return;
            }
            let mut guard = state.lock().unwrap();
            if !matches!(guard.background, VideoBackground::Plain { .. }) {
                guard.background = VideoBackground::Plain {
                    r: 17,
                    g: 17,
                    b: 17,
                };
                drop(guard);
                on_change();
            }
        }
    });

    let refresh = {
        let none_btn = none_btn.clone();
        let wallpaper_btn = wallpaper_btn.clone();
        let color_btn = color_btn.clone();
        let wallpaper_page = wallpaper_page.clone();
        let color_page = color_page.clone();
        let padding_widget = padding_row.widget.clone();
        let padding_scale = padding_row.scale.clone();
        let swatch_paint = swatch_paint.clone();
        let hex = hex.clone();
        let cards = cards.clone();
        let syncing = syncing.clone();
        Rc::new(move || {
            let (background, padding) = {
                let guard = state.lock().unwrap();
                (guard.background.clone(), guard.background_padding)
            };
            syncing.set(true);
            let is_wallpaper = matches!(background, VideoBackground::Wallpaper(_));
            let is_color = matches!(background, VideoBackground::Plain { .. });
            let has_fill = is_wallpaper || is_color;
            none_btn.set_active(!has_fill);
            wallpaper_btn.set_active(is_wallpaper);
            color_btn.set_active(is_color);
            wallpaper_page.set_visible(is_wallpaper);
            color_page.set_visible(is_color);
            padding_widget.set_visible(has_fill);
            padding_scale.set_value(padding);
            if let VideoBackground::Wallpaper(path) = &background {
                let active = wallpaper_file_name(path);
                for (file_name, card) in &cards {
                    let on = active.as_deref() == Some(file_name.as_str());
                    card.set_active(on);
                    if on {
                        card.add_css_class("active-background-option");
                    } else {
                        card.remove_css_class("active-background-option");
                    }
                }
            } else {
                for (_, card) in &cards {
                    card.set_active(false);
                    card.remove_css_class("active-background-option");
                }
            }
            if let VideoBackground::Plain { r, g, b } = background {
                hex.set_text(&format!("#{r:02X}{g:02X}{b:02X}"));
            }
            swatch_paint.queue_draw();
            syncing.set(false);
        }) as Rc<dyn Fn()>
    };

    BackgroundPanel {
        widget: panel,
        refresh,
    }
}

// Small bundled thumbs decode fast; a missing thumb leaves the tile empty
// rather than decoding a multi-megapixel wallpaper on the UI thread.
fn decode_wallpaper_thumb(path: &PathBuf) -> Option<gtk4::cairo::ImageSurface> {
    let image = image::open(path).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return None;
    }
    let stride = gtk4::cairo::Format::ARgb32.stride_for_width(width).ok()?;
    // Premultiplied ARgb32 bytes, matching the image editor's conversion.
    let data: Vec<u8> = image
        .pixels()
        .flat_map(|pixel| {
            let [r, g, b, a] = pixel.0;
            let a = a as u32;
            let pr = ((r as u32 * a + 127) / 255) as u8;
            let pg = ((g as u32 * a + 127) / 255) as u8;
            let pb = ((b as u32 * a + 127) / 255) as u8;
            [pb, pg, pr, a as u8]
        })
        .collect();
    gtk4::cairo::ImageSurface::create_for_data(
        data,
        gtk4::cairo::Format::ARgb32,
        width as i32,
        height as i32,
        stride,
    )
    .ok()
}

// Cover-fit paint with the same rounded corners as the image editor tiles.
fn paint_wallpaper_thumb(
    cr: &gtk4::cairo::Context,
    surface: &gtk4::cairo::ImageSurface,
    width: i32,
    height: i32,
) {
    let source_w = surface.width().max(1) as f64;
    let source_h = surface.height().max(1) as f64;
    let scale = (width as f64 / source_w).max(height as f64 / source_h);
    let _ = cr.save();
    wallpaper_thumb_rounded_rect(cr, 0.0, 0.0, width as f64, height as f64, 11.0);
    cr.clip();
    cr.translate(
        (width as f64 - source_w * scale) * 0.5,
        (height as f64 - source_h * scale) * 0.5,
    );
    cr.scale(scale, scale);
    let _ = cr.set_source_surface(surface, 0.0, 0.0);
    let _ = cr.paint();
    let _ = cr.restore();
}

fn wallpaper_thumb_rounded_rect(
    cr: &gtk4::cairo::Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    r: f64,
) {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
    cr.arc(
        x + r,
        y + h - r,
        r,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );
    cr.arc(
        x + r,
        y + r,
        r,
        std::f64::consts::PI,
        3.0 * std::f64::consts::FRAC_PI_2,
    );
    cr.close_path();
}

fn open_background_color_dialog(
    widget: &impl IsA<Widget>,
    state: Arc<Mutex<VideoEditState>>,
    on_change: Rc<dyn Fn()>,
) {
    let (r, g, b) = match &state.lock().unwrap().background {
        VideoBackground::Plain { r, g, b } => (*r, *g, *b),
        _ => (17, 17, 17),
    };
    let initial = gdk::RGBA::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0);
    let parent = widget
        .root()
        .and_then(|root| root.downcast::<Window>().ok());
    let dialog = ColorChooserDialog::new(Some(&t("Background color")), parent.as_ref());
    dialog.set_modal(true);
    dialog.set_use_alpha(false);
    dialog.set_rgba(&initial);
    // Custom colors in the chooser use the shared image-editor slots; video
    // intentionally keeps only its own presets plus this dialog.
    let _ = Image::from_icon_name("dialog-cancel-symbolic");
    dialog.connect_response(move |dialog, response| {
        if response == gtk4::ResponseType::Ok {
            let color = dialog.rgba();
            state.lock().unwrap().background = VideoBackground::Plain {
                r: (color.red() * 255.0).round().clamp(0.0, 255.0) as u8,
                g: (color.green() * 255.0).round().clamp(0.0, 255.0) as u8,
                b: (color.blue() * 255.0).round().clamp(0.0, 255.0) as u8,
            };
            on_change();
        }
        dialog.close();
    });
    dialog.present();
}
