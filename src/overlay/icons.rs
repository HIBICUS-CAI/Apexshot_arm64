use super::drawing::rounded_rect_path;
use std::f64::consts::PI;

// Rail icons (Area/Fullscreen/Scroll/Timer/Ocr/Recording) retired with the
// legacy left toolbar. Remaining glyphs serve the top bar and panels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ToolbarIcon {
    Controls,
    Crop,
    Mic,
    Speaker,

    Video,
}

// Rail menu table retired with the legacy left toolbar: mode selection
// belongs to quick capture up front. The `ToolbarIcon` glyphs above stay —
// Crop/Mic/Speaker/Video/Controls still serve the top bar and panels.

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
        ToolbarIcon::Controls => {
            for i in 0..3 {
                let x = cx - 4.5 + i as f64 * 4.5;
                context.move_to(x, cy - 6.0);
                context.line_to(x, cy + 6.0);
                let slider_y = if i == 0 {
                    cy - 2.0
                } else if i == 1 {
                    cy + 2.0
                } else {
                    cy - 1.0
                };
                context.arc(x, slider_y, 1.8, 0.0, PI * 2.0);
            }
            let _ = context.stroke();
        }
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
        ToolbarIcon::Mic => {
            rounded_rect_path(context, cx - 3.1, cy - 7.0, 6.2, 9.6, 3.1);
            let _ = context.stroke();
            context.move_to(cx - 5.0, cy - 0.3);
            context.line_to(cx - 5.0, cy + 1.6);
            context.move_to(cx + 5.0, cy - 0.3);
            context.line_to(cx + 5.0, cy + 1.6);
            context.arc(cx, cy + 0.7, 5.0, 0.0, PI);
            context.move_to(cx, cy + 6.1);
            context.line_to(cx, cy + 8.3);
            context.move_to(cx - 3.4, cy + 8.3);
            context.line_to(cx + 3.4, cy + 8.3);
            let _ = context.stroke();
        }
        ToolbarIcon::Speaker => {
            context.move_to(cx - 6.8, cy - 2.3);
            context.line_to(cx - 4.4, cy - 2.3);
            context.line_to(cx - 1.2, cy - 5.1);
            context.line_to(cx - 1.2, cy + 5.1);
            context.line_to(cx - 4.4, cy + 2.3);
            context.line_to(cx - 6.8, cy + 2.3);
            context.close_path();
            let _ = context.stroke();
            context.arc(cx - 0.8, cy, 5.0, -0.7, 0.7);
            context.arc(cx + 1.2, cy, 7.0, -0.7, 0.7);
            let _ = context.stroke();
        }
        ToolbarIcon::Video => {
            rounded_rect_path(context, cx - 8.0, cy - 5.0, 10.5, 10.0, 2.5);
            let _ = context.stroke();
            context.move_to(cx + 2.4, cy - 2.8);
            context.line_to(cx + 7.4, cy - 5.2);
            context.line_to(cx + 7.4, cy + 5.2);
            context.line_to(cx + 2.4, cy + 2.8);
            context.close_path();
            let _ = context.stroke();
        }
    }

    let _ = context.restore();
}
