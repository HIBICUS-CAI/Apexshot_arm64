// Rail icons (Area/Fullscreen/Scroll/Timer/Ocr/Recording) retired with the
// legacy left toolbar, and the recording-panel glyphs (Controls/Mic/Speaker/
// Video) left with the retired recording panel. Only Crop remains, serving
// the top bar's crop dropdown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ToolbarIcon {
    Crop,
}

pub(crate) fn draw_toolbar_icon(
    context: &gtk4::cairo::Context,
    icon: ToolbarIcon,
    cx: f64,
    cy: f64,
    color: (f64, f64, f64, f64),
) {
    let _ = context.save();
    context.new_path();
    context.set_source_rgba(color.0, color.1, color.2, color.3);
    context.set_line_width(1.6);
    context.set_line_cap(gtk4::cairo::LineCap::Round);
    context.set_line_join(gtk4::cairo::LineJoin::Round);

    match icon {
        ToolbarIcon::Crop => {
            context.set_line_cap(gtk4::cairo::LineCap::Butt);
            context.set_line_join(gtk4::cairo::LineJoin::Miter);
            let s = 10.5;
            let t = 2.8;
            let o = 1.2;
            context.move_to(cx - s / 2.0 - t, cy - s / 2.0 + o);
            context.line_to(cx + s / 2.0 - o, cy - s / 2.0 + o);
            context.move_to(cx - s / 2.0 + o, cy - s / 2.0 - t);
            context.line_to(cx - s / 2.0 + o, cy + s / 2.0 - o);
            context.move_to(cx + s / 2.0 + t, cy + s / 2.0 - o);
            context.line_to(cx - s / 2.0 + o, cy + s / 2.0 - o);
            context.move_to(cx + s / 2.0 - o, cy + s / 2.0 + t);
            context.line_to(cx + s / 2.0 - o, cy - s / 2.0 + o);
            let _ = context.stroke();
        }
    }

    let _ = context.restore();
}
