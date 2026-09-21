/// Motion blur configuration whose field order and clamp bounds were recovered
/// from the persisted `MotionBlurSettings` metadata and implementation.
///
/// The temporal composition policy below is ApexShot's current policy; it is
/// deliberately not described as a byte-for-byte reconstruction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionBlurSettings {
    pub enabled: bool,
    pub cursor_strength: f64,
    pub zoom_strength: f64,
    pub capture_movement_strength: f64,
    pub shutter_angle: f64,
    pub zoom_blur_amount_multiplier: f64,
    pub zoom_blur_max_amount: f64,
    pub transform_temporal_exposure_cap: f64,
    /// Recovered trail opacity. The renderer derives blur from the
    /// exposure window and averages temporal subframes instead of stacking
    /// ghost copies, so the field is retained for schema compatibility only.
    pub transform_trail_opacity: f64,
}

impl Default for MotionBlurSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            // These are ApexShot defaults. The legacy construction defaults
            // have not yet been recovered.
            cursor_strength: 0.4,
            zoom_strength: 0.0,
            capture_movement_strength: 0.35,
            shutter_angle: 180.0,
            zoom_blur_amount_multiplier: 1.0,
            zoom_blur_max_amount: 1.0,
            transform_temporal_exposure_cap: 1.0 / 24.0,
            transform_trail_opacity: 0.28,
        }
    }
}

/// Motion blur quality mode recovered from the persisted `MotionBlurBudgetMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionBlurBudgetMode {
    LivePreviewPlayback,
    FullQuality,
}

impl MotionBlurBudgetMode {
    /// Upper bound on temporal subframes accumulated for one output frame.
    /// Interactive playback stays near the old trail's mesh cost (a subframe
    /// mesh is lighter), while export spends what it needs for a smooth edge.
    fn max_samples(self) -> usize {
        match self {
            Self::LivePreviewPlayback => 16,
            Self::FullQuality => 64,
        }
    }

    /// Longest card travel, in output pixels, between two subframes. Denser
    /// sampling keeps the accumulated edge gradient smooth instead of stepped.
    fn sample_spacing_px(self) -> f64 {
        match self {
            Self::LivePreviewPlayback => 2.0,
            Self::FullQuality => 1.5,
        }
    }
}

/// Below this travel the exposure window holds one pose, so there is nothing
/// to blur.
const MIN_MOTION_BLUR_TRAVEL_PX: f64 = 0.75;

impl MotionBlurSettings {
    pub fn clamped(self) -> Self {
        let finite = |value: f64, fallback: f64| {
            if value.is_finite() {
                value
            } else {
                fallback
            }
        };
        Self {
            enabled: self.enabled,
            cursor_strength: finite(self.cursor_strength, 0.0).clamp(0.0, 5.0),
            zoom_strength: finite(self.zoom_strength, 0.0).clamp(0.0, 5.0),
            capture_movement_strength: finite(self.capture_movement_strength, 0.0).clamp(0.0, 5.0),
            shutter_angle: finite(self.shutter_angle, 0.0).clamp(0.0, 360.0),
            zoom_blur_amount_multiplier: finite(self.zoom_blur_amount_multiplier, 0.0)
                .clamp(0.0, 3.0),
            zoom_blur_max_amount: finite(self.zoom_blur_max_amount, 0.0).clamp(0.0, 120.0),
            transform_temporal_exposure_cap: finite(self.transform_temporal_exposure_cap, 0.0)
                .clamp(0.0, 8.0),
            transform_trail_opacity: finite(self.transform_trail_opacity, 0.0).clamp(0.0, 0.4),
        }
    }

    /// Strength of the still-image camera move. Cursor/capture strengths are
    /// intentionally excluded: there is no corresponding moving source in a
    /// Motion still, so applying them would incorrectly blur a static card.
    pub fn effective_zoom_amount(self) -> f64 {
        let settings = self.clamped();
        if !settings.enabled {
            return 0.0;
        }
        (settings.zoom_strength * settings.zoom_blur_amount_multiplier)
            .min(settings.zoom_blur_max_amount)
    }

    /// Exposure time of one output frame in seconds: the frame interval scaled
    /// by the shutter angle, capped by `transform_temporal_exposure_cap`,
    /// then scaled by the user's blur strength. This is the window a real
    /// camera would integrate over.
    pub fn exposure_seconds(self, frame_rate: f64) -> f64 {
        let settings = self.clamped();
        if !settings.enabled || settings.shutter_angle <= 0.0 {
            return 0.0;
        }
        let frame_duration = 1.0 / frame_rate.max(1.0);
        (frame_duration * settings.shutter_angle / 360.0)
            .min(settings.transform_temporal_exposure_cap)
            * settings.effective_zoom_amount()
    }

    /// Subframe times across the exposure window ending at the current frame,
    /// newest first. Every instant of the exposure contributes equally, so
    /// averaging these frames smears the moving card along its path the way a
    /// camera does. The renderer reports how far the card actually travels in
    /// pixels; the count is chosen so consecutive subframes stay within the
    /// budget's spacing, which is what keeps the result a continuous smear
    /// rather than distinct ghost copies.
    pub fn temporal_offsets(
        self,
        frame_rate: f64,
        travel_px: f64,
        budget: MotionBlurBudgetMode,
    ) -> Vec<f64> {
        let exposure = self.exposure_seconds(frame_rate);
        if exposure <= f64::EPSILON
            || !travel_px.is_finite()
            || travel_px < MIN_MOTION_BLUR_TRAVEL_PX
        {
            return Vec::new();
        }
        let intervals = (travel_px / budget.sample_spacing_px())
            .ceil()
            .clamp(1.0, budget.max_samples() as f64) as usize;
        let sample_count = (intervals + 1).min(budget.max_samples()).max(2);
        let last = (sample_count - 1) as f64;
        (0..sample_count)
            .map(|index| -exposure * index as f64 / last)
            .collect()
    }
}

pub(crate) fn cubic_bezier_ease(timing: MotionEffectTransformTiming, progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);
    if progress <= f64::EPSILON || (1.0 - progress) <= f64::EPSILON {
        return progress;
    }

    let sample = |u: f64, p1: f64, p2: f64| {
        let inverse = 1.0 - u;
        3.0 * inverse * inverse * u * p1 + 3.0 * inverse * u * u * p2 + u * u * u
    };

    // The X component represents time, so solve it before sampling Y. The
    // editor constrains both X coordinates to [0, 1], making bisection stable.
    let mut low = 0.0;
    let mut high = 1.0;
    for _ in 0..24 {
        let midpoint = (low + high) * 0.5;
        if sample(midpoint, timing.easing_x1, timing.easing_x2) < progress {
            low = midpoint;
        } else {
            high = midpoint;
        }
    }
    sample((low + high) * 0.5, timing.easing_y1, timing.easing_y2)
}

pub(super) fn lerp_transform(from: MotionTransform, to: MotionTransform, t: f64) -> MotionTransform {
    lerp_transform_raw(from, to, t.clamp(0.0, 1.0))
}

/// Like [`lerp_transform`], but preserves a spring's overshoot: the blend may
/// pass the target by up to half the move before it settles back.
pub(super) fn lerp_transform_spring(
    from: MotionTransform,
    to: MotionTransform,
    t: f64,
) -> MotionTransform {
    lerp_transform_raw(from, to, t.clamp(-0.5, 1.5))
}

fn lerp_transform_raw(
    from: MotionTransform,
    to: MotionTransform,
    t: f64,
) -> MotionTransform {
    MotionTransform {
        scale: from.scale + (to.scale - from.scale) * t,
        rotation_x: from.rotation_x + (to.rotation_x - from.rotation_x) * t,
        rotation_y: from.rotation_y + (to.rotation_y - from.rotation_y) * t,
        rotation_z: from.rotation_z + (to.rotation_z - from.rotation_z) * t,
        perspective: from.perspective + (to.perspective - from.perspective) * t,
        pos_x: from.pos_x + (to.pos_x - from.pos_x) * t,
        pos_y: from.pos_y + (to.pos_y - from.pos_y) * t,
    }
}
