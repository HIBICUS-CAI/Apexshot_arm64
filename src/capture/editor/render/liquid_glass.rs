//! Liquid Glass frame rendering.
//!
//! A CPU port of the glass pipeline from <https://github.com/ybouane/liquidglass>
//! applied to the frame band around a captured card. The band is treated as a
//! curved slab of glass sitting on the composited backdrop:
//!
//! * A rounded-rect signed distance field gives the band its shape and the
//!   surface normal of the bevel.
//! * Refraction bends the backdrop along that bevel, so whatever sits behind
//!   the frame is pulled through the glass instead of just tinted.
//! * Chromatic aberration fringes red and blue along the same normal.
//! * Fresnel reflection, four Blinn-Phong rim lights and an inner stroke
//!   highlight give the glass its highlights, which is what carries the look
//!   when the backdrop is flat, dark, or transparent.

use image::RgbaImage;

/// The card the glass wraps, in backdrop pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassRing {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub radius: f64,
    /// Glass thickness: the visible band plus its specular rim.
    pub gap: f64,
}

/// Look of the glass, mirroring the reference shader's uniforms.
#[derive(Debug, Clone, Copy)]
pub struct GlassLook {
    /// How far samples bend, at most this fraction of the band.
    pub refraction: f64,
    /// Colour fringing, as a fraction of the band.
    pub chroma: f64,
    /// Backdrop blur, as a fraction of the band (0 keeps the backdrop sharp).
    pub blur: f64,
    /// Blinn-Phong rim highlight intensity.
    pub specular: f64,
    /// Fresnel reflection intensity.
    pub fresnel: f64,
    /// Broad reflection across the band, brightest along the lit top lip.
    pub reflection: f64,
    /// Inner stroke and rim glow intensity.
    pub edge_highlight: f64,
    /// Cool blue glass tint.
    pub tint: f64,
    pub brightness: f64,
    pub saturation: f64,
    /// Body shade for the tinted glass siblings: >0 deepens the refracted
    /// body toward black (Glass Dark), <0 lifts it toward white (Glass
    /// Light). The speculars and rim are untouched, so shaded glass keeps
    /// its bright edge light like dark-mode system glass.
    pub body_shade: f64,
}

impl Default for GlassLook {
    fn default() -> Self {
        // Apple Liquid Glass (NSGlassEffectView) recipe: the band stays clear
        // so the backdrop reads through it. Frost comes from blur plus a
        // saturation lift — not from a white veil — while the brightness
        // lives in the top-weighted specular and the crisp edge rim. The old
        // recipe stacked flat white additions that rendered a milky gray tube,
        // especially over dark backdrops.
        Self {
            refraction: 0.5,
            chroma: 0.04,
            blur: 0.18,
            specular: 0.5,
            fresnel: 1.0,
            reflection: 0.25,
            edge_highlight: 0.9,
            tint: 0.03,
            brightness: 0.0,
            saturation: 0.35,
            body_shade: 0.0,
        }
    }
}

impl GlassLook {
    /// Frosted Glass Light (Shots.so language): the band smears the backdrop
    /// into milk instead of lensing it. Refraction and fringing go near
    /// zero, blur goes heavy, form lighting stays whisper-soft, and the
    /// body lifts whitish while the rim keeps its definition.
    pub fn frost_light() -> Self {
        Self {
            refraction: 0.06,
            chroma: 0.0,
            blur: 1.2,
            specular: 0.12,
            fresnel: 0.6,
            reflection: 0.10,
            edge_highlight: 0.5,
            tint: 0.0,
            brightness: 0.0,
            saturation: 0.0,
            body_shade: -0.55,
        }
    }

    /// Frosted Glass Dark: the same smear, smoked instead of milky.
    pub fn frost_dark() -> Self {
        Self {
            body_shade: 0.55,
            edge_highlight: 0.6,
            ..Self::frost_light()
        }
    }
}

/// Refracted and lit glass band, cropped to the pixels it actually covers.
pub struct GlassLayer {
    pub image: RgbaImage,
    pub x: i64,
    pub y: i64,
}

/// Rounded-rect signed distance plus the outward unit normal at that point.
/// `px`/`py` are relative to the rect's centre.
fn rounded_rect_sdf(px: f64, py: f64, half_w: f64, half_h: f64, radius: f64) -> (f64, f64, f64) {
    let qx = px.abs() - half_w + radius;
    let qy = py.abs() - half_h + radius;
    let mx = qx.max(0.0);
    let my = qy.max(0.0);
    let length = (mx * mx + my * my).sqrt();
    let distance = length + qx.min(0.0).max(qy.min(0.0)) - radius;
    if length <= 1e-6 {
        return (distance, 0.0, 0.0);
    }
    let sign_x = if px < 0.0 { -1.0 } else { 1.0 };
    let sign_y = if py < 0.0 { -1.0 } else { 1.0 };
    (distance, sign_x * mx / length, sign_y * my / length)
}

/// Glass cross-section across the band, ported from the reference
/// `bevelHeight(d) = sqrt(d * (2*zR - d))` half-circle profile. The band is a
/// tiny glass tube: height is zero at both lips and peaks in the middle, so
/// `d` is the distance to the nearest lip and `zR` is half the band. Returns
/// dh/distance along the outward normal (+ on the inner half, - on the outer
/// half), clamped like the reference's 2px finite difference so the lips stay
/// bright without sampling halfway across the canvas.
fn lens_slope(u: f64, gap: f64) -> f64 {
    let gap = gap.max(1.0);
    let z_r = gap * 0.5;
    let dist = (u.clamp(0.0, 1.0) * gap).min(gap - u.clamp(0.0, 1.0) * gap);
    let d = dist.clamp(0.0, z_r);
    let denom = (d * (2.0 * z_r - d)).max(1e-6).sqrt();
    let mut slope = (z_r - d) / denom;
    if u < 0.5 {
        slope = slope.abs();
    } else {
        slope = -slope.abs();
    }
    slope.clamp(-4.0, 4.0)
}

fn smoothstep(edge0: f64, edge1: f64, value: f64) -> f64 {
    if (edge1 - edge0).abs() <= f64::EPSILON {
        return if value < edge0 { 0.0 } else { 1.0 };
    }
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Bilinear sample of the backdrop, clamped at the edges. Returns straight
/// (non-premultiplied) 0..1 RGBA.
fn sample(backdrop: &RgbaImage, x: f64, y: f64) -> [f64; 4] {
    let (width, height) = backdrop.dimensions();
    if width == 0 || height == 0 {
        return [0.0, 0.0, 0.0, 0.0];
    }
    let max_x = width as f64 - 1.0;
    let max_y = height as f64 - 1.0;
    let x = x.clamp(0.0, max_x);
    let y = y.clamp(0.0, max_y);
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);
    let fx = x - f64::from(x0);
    let fy = y - f64::from(y0);
    let p00 = backdrop.get_pixel(x0, y0);
    let p10 = backdrop.get_pixel(x1, y0);
    let p01 = backdrop.get_pixel(x0, y1);
    let p11 = backdrop.get_pixel(x1, y1);
    let mut out = [0.0; 4];
    for channel in 0..4 {
        let top = f64::from(p00[channel]) * (1.0 - fx) + f64::from(p10[channel]) * fx;
        let bottom = f64::from(p01[channel]) * (1.0 - fx) + f64::from(p11[channel]) * fx;
        out[channel] = (top * (1.0 - fy) + bottom * fy) / 255.0;
    }
    out
}

/// Render the glass band on top of `backdrop`.
///
/// The returned layer is transparent outside the band, so callers can overlay
/// it directly at [`GlassLayer::x`]/[`GlassLayer::y`].
pub fn glass_layer(backdrop: &RgbaImage, ring: &GlassRing, look: &GlassLook) -> Option<GlassLayer> {
    let (image_w, image_h) = backdrop.dimensions();
    if image_w == 0 || image_h == 0 {
        return None;
    }
    let gap = ring.gap;
    let half_w = ring.width * 0.5;
    let half_h = ring.height * 0.5;
    if gap <= 1.0 || half_w <= 0.5 || half_h <= 0.5 {
        return None;
    }
    let radius = ring.radius.clamp(0.0, half_w.min(half_h));
    let center_x = ring.x + half_w;
    let center_y = ring.y + half_h;

    // The band, plus the reach of the refraction so bent samples stay inside
    // the crop. Everything outside this box is untouched canvas.
    let reach = gap * (1.0 + look.refraction) + 4.0;
    let x0 = (center_x - half_w - reach).floor().max(0.0);
    let y0 = (center_y - half_h - reach).floor().max(0.0);
    let x1 = (center_x + half_w + reach).ceil().min(f64::from(image_w));
    let y1 = (center_y + half_h + reach).ceil().min(f64::from(image_h));
    if x1 - x0 < 1.0 || y1 - y0 < 1.0 {
        return None;
    }
    let layer_w = (x1 - x0) as u32;
    let layer_h = (y1 - y0) as u32;
    let mut layer = RgbaImage::from_pixel(layer_w, layer_h, image::Rgba([0, 0, 0, 0]));

    // Frosted glass reads better over busy backdrops, and the reference mixes
    // a blurred copy in everywhere except right at the lips, where the
    // refracted edge has to stay crisp. Blur only the crop we sample from.
    let blurred = if look.blur > 0.0 {
        let mut crop = RgbaImage::from_pixel(layer_w, layer_h, image::Rgba([0, 0, 0, 0]));
        image::imageops::replace(&mut crop, backdrop, -x0 as i64, -y0 as i64);
        let radius = (look.blur * gap).max(0.5);
        crate::capture::editor::render::apply_blur_rect(
            &mut crop,
            crate::capture::editor::types::Rect {
                x: 0,
                y: 0,
                width: layer_w as i32,
                height: layer_h as i32,
            },
            radius,
            false,
        );
        Some(crop)
    } else {
        None
    };

    // Reference scale: refrPx = hGrad * (1-1/1.5) * refract * 30px. For a thin
    // frame band that would throw samples across the card, so normalize the
    // slope (lips = +/-1) and bend by a fraction of the band instead.
    let max_bend = gap * look.refraction * 0.45;
    // Light rig from the reference shader (y negated: GL is y-up, images are
    // y-down): key upper-right, fill lower-left, broad sheen, tight sky light.
    let lights: [(f64, f64, f64, f64, f64); 4] = [
        (0.4, -0.7, 1.0, 90.0, 1.0),
        (-0.3, 0.5, 1.0, 50.0, 0.3),
        (0.1, -0.3, 1.0, 6.0, 0.1),
        (0.0, -0.9, 0.4, 120.0, 0.6),
    ];

    for ly in 0..layer_h {
        let pixel_y = y0 + f64::from(ly) + 0.5;
        let py = pixel_y - center_y;
        for lx in 0..layer_w {
            let pixel_x = x0 + f64::from(lx) + 0.5;
            let px = pixel_x - center_x;
            let (distance, normal_x, normal_y) = rounded_rect_sdf(px, py, half_w, half_h, radius);
            if distance > gap + 1.5 || distance < -1.5 {
                continue;
            }
            let distance_from_card = distance.clamp(0.0, gap);

            let u = distance_from_card / gap;
            let slope = lens_slope(u, gap);
            // Outside the band there is no glass: keep the backdrop sharp so
            // no gray fringe leaks past the outer lip.
            let bend_amount = if distance > gap {
                0.0
            } else {
                (slope / 4.0) * max_bend
            };
            let sample_x = pixel_x + normal_x * bend_amount;
            let sample_y = pixel_y + normal_y * bend_amount;

            // Colour fringing follows the reference: stronger at the rim where
            // the surface tilts, calmer through the middle.
            let edge_weight =
                (1.0 - (2.0 * (distance_from_card / gap - 0.5)).abs()).clamp(0.0, 1.0);
            let edge = 1.0 - edge_weight;
            let fringing = look.chroma * 18.0 * (edge * 0.7 + 0.3) * 0.12 * gap * 0.25;
            let red = sample(
                backdrop,
                sample_x + normal_x * fringing,
                sample_y + normal_y * fringing,
            );
            let green = sample(backdrop, sample_x, sample_y);
            let blue = sample(
                backdrop,
                sample_x - normal_x * fringing,
                sample_y - normal_y * fringing,
            );
            // Edge-weighted blur mix: frosted through the body, sharp enough
            // at the lips that the refracted edge stays readable.
            let blur_mix = 1.0 - 0.15 * edge_weight;
            let (red, green, blue) = match blurred.as_ref() {
                Some(frosted) => {
                    let frost = |offset_x: f64, offset_y: f64| {
                        sample(frosted, sample_x + offset_x - x0, sample_y + offset_y - y0)
                    };
                    let frosted_red = frost(normal_x * fringing, normal_y * fringing);
                    let frosted_green = frost(0.0, 0.0);
                    let frosted_blue = frost(-normal_x * fringing, -normal_y * fringing);
                    let mix =
                        |sharp: f64, blurred: f64| sharp * (1.0 - blur_mix) + blurred * blur_mix;
                    (
                        [
                            mix(red[0], frosted_red[0]),
                            mix(red[1], frosted_red[1]),
                            mix(red[2], frosted_red[2]),
                            mix(red[3], frosted_red[3]),
                        ],
                        [
                            mix(green[0], frosted_green[0]),
                            mix(green[1], frosted_green[1]),
                            mix(green[2], frosted_green[2]),
                            mix(green[3], frosted_green[3]),
                        ],
                        [
                            mix(blue[0], frosted_blue[0]),
                            mix(blue[1], frosted_blue[1]),
                            mix(blue[2], frosted_blue[2]),
                            mix(blue[3], frosted_blue[3]),
                        ],
                    )
                }
                None => (red, green, blue),
            };
            // Work premultiplied so a transparent or partly transparent
            // backdrop keeps its alpha and the highlights can light it up.
            let backdrop_alpha = green[3];
            let mut color = if backdrop_alpha > 0.001 {
                [
                    red[0] * red[3] / backdrop_alpha,
                    green[1] * green[3] / backdrop_alpha,
                    blue[2] * blue[3] / backdrop_alpha,
                ]
            } else {
                [0.0, 0.0, 0.0]
            };

            let depth = if gap > 0.0 {
                smoothstep(0.0, gap, distance_from_card)
            } else {
                0.0
            };
            for channel in color.iter_mut() {
                *channel *= 1.0 + look.brightness;
            }
            let luminance = 0.299 * color[0] + 0.587 * color[1] + 0.114 * color[2];
            for channel in color.iter_mut() {
                *channel = luminance + (*channel - luminance) * (1.0 + look.saturation);
            }
            // Cool glass tint from the reference, plus its depth lift.
            color = [
                color[0] * (1.0 - 0.08 * look.tint),
                color[1] * (1.0 - 0.05 * look.tint),
                color[2] * (1.0 + 0.05 * look.tint),
            ];
            for channel in color.iter_mut() {
                *channel *= 1.0 + 0.06 * depth * look.tint.max(0.2);
            }
            // Tinted siblings (Glass Light/Dark): lift or deepen the body
            // here, before the lighting terms are added, so the speculars
            // and rim keep full strength on the shaded body.
            if look.body_shade > 0.0 {
                let keep = (1.0 - look.body_shade.clamp(0.0, 0.9)).max(0.0);
                for channel in color.iter_mut() {
                    *channel *= keep;
                }
            } else if look.body_shade < 0.0 {
                let lift = (-look.body_shade).clamp(0.0, 0.9);
                for channel in color.iter_mut() {
                    *channel = *channel * (1.0 - lift * 0.5) + lift * 0.5;
                }
            }

            // Surface normal of the beveled glass: the gradient of the height
            // field points along the outward normal, so it only needs scaling.
            let gradient_x = normal_x * slope;
            let gradient_y = normal_y * slope;
            let normal = normalize3(-gradient_x, -gradient_y, 1.0);

            let mut specular = 0.0;
            for (light_x, light_y, light_z, shininess, weight) in lights {
                let (hx, hy, hz) = normalize3(light_x, light_y, light_z + 1.0);
                let dot = (normal.0 * hx + normal.1 * hy + normal.2 * hz).max(0.0);
                specular += dot.powf(shininess) * weight;
            }
            specular *= look.specular;

            let fresnel = (1.0 - normal.2.abs()).powf(4.0) * look.fresnel;
            let top_bias = (0.5 - 0.5 * py / half_h.max(1.0)).clamp(0.0, 1.0);
            // Tilt-driven reflection for the lips plus a broad top sheen
            // through the middle: the lips catch the light by curvature while
            // the top of the tube carries a Shots-style reflection even where
            // the surface is flat, so black backdrops still read as glass.
            let tilt = (1.0 - normal.2.abs()).powf(1.5);
            let reflection = tilt * (0.25 + 0.75 * top_bias) * look.reflection;
            // Narrow top sheen: Apple glass pools light at the top of the
            // tube instead of washing the whole band white.
            let sheen = edge_weight * top_bias.powf(2.0) * 0.20;
            // Strokes on both lips, top biased, like a real glass tube.
            let outer_sdf = distance - gap;
            let outer_stroke = smoothstep(-2.5, -1.5, outer_sdf)
                * (1.0 - smoothstep(-1.0, 0.0, outer_sdf))
                * (0.3 + 0.7 * top_bias);
            let inner_stroke = smoothstep(-1.5, -0.5, distance)
                * (1.0 - smoothstep(0.5, 1.5, distance))
                * (0.3 + 0.7 * top_bias);
            let rim = edge * look.edge_highlight * 0.20;
            let inner_glow = smoothstep(
                5.0,
                0.0,
                (gap - distance_from_card).min(distance_from_card + 1.0),
            ) * look.edge_highlight
                * 0.15;
            let environment = (0.5 - normal.1 * 0.5) * fresnel * 0.04;

            // Clear middle, bright lips: body terms (specular/reflection/
            // sheen/environment) are quieted where the tube is flat so the
            // backdrop shows through, while lip terms (rim/strokes/glow)
            // stay full strength for the crisp Apple edge light.
            let body = specular + reflection + sheen + environment;
            let lips =
                rim + inner_glow + (outer_stroke + inner_stroke * 0.8) * look.edge_highlight * 0.5;
            let envelope = (0.35 + 0.65 * edge.max(tilt)).clamp(0.0, 1.0);
            let addition = body * envelope + lips;
            for channel in color.iter_mut() {
                *channel *= backdrop_alpha;
                *channel += addition;
            }
            let alpha = (backdrop_alpha + addition).clamp(0.0, 1.0);
            let fresnel_alpha = fresnel * 0.2;
            for channel in color.iter_mut() {
                *channel = *channel * (1.0 - fresnel_alpha) + alpha * fresnel_alpha;
            }

            // Anti-aliased mask: opaque through the band, feathered on the
            // outer lip and where the glass meets the card.
            let inner_alpha = smoothstep(-1.2, -0.2, distance);
            let outer_alpha = 1.0 - smoothstep(gap - 1.2, gap + 0.6, distance);
            let coverage = (inner_alpha * outer_alpha).clamp(0.0, 1.0);
            let alpha = (alpha * coverage).clamp(0.0, 1.0);
            if coverage <= 0.002 || alpha <= 0.002 {
                continue;
            }
            let to_byte = |value: f64| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
            // Back to straight alpha for the compositors we hand this to.
            let straight = |value: f64| (value / alpha / coverage).clamp(0.0, 1.0);
            layer.put_pixel(
                lx,
                ly,
                image::Rgba([
                    to_byte(straight(color[0])),
                    to_byte(straight(color[1])),
                    to_byte(straight(color[2])),
                    to_byte(alpha),
                ]),
            );
        }
    }

    Some(GlassLayer {
        image: layer,
        x: x0 as i64,
        y: y0 as i64,
    })
}

fn normalize3(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    let length = (x * x + y * y + z * z).sqrt();
    if length <= 1e-9 {
        return (0.0, 0.0, 1.0);
    }
    (x / length, y / length, z / length)
}
