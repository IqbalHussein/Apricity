//! [`GradeParameters`] — the single source of truth for NGA output, Recipe
//! file format, and Manual Console controls.
//!
//! The struct is split along the pipeline's non-negotiable seam:
//!
//! - [`CorrectionParameters`] — **Stage 1**, technical correction (input
//!   decode, white balance, exposure normalization, black point).
//! - [`CreativeParameters`] — **Stage 2**, the creative grade the NGA and the
//!   Manual Console operate on.
//!
//! Every numeric field documents its **unit** and the **color space / signal
//! domain** it is applied in. Ranges, defaults, and labels for every scalar
//! live in [`crate::param_spec`]; never redefine them in a shell.
//!
//! ## Signal domains used in the docs below
//!
//! - **Scene-linear Rec.2020** — linear-light RGB, Rec.2020 primaries, D65.
//!   The pipeline interior.
//! - **Working encoding** — scene-linear Rec.2020 passed through a pure
//!   `x^(1/2.4)` power (sign-preserving). Perceptually spaced; used for tone
//!   and color-wheel operations so slider travel feels even. See
//!   [`crate::working_encode`].
//!
//! The schema is versioned via [`SCHEMA_VERSION`] and `serde` defaults, so a
//! Recipe written by an older build loads forward with new fields defaulted.

use serde::{Deserialize, Serialize};

/// Current [`GradeParameters`] schema version. Bump on any breaking change
/// and add a migration in `apricity-recipes`.
pub const SCHEMA_VERSION: u32 = 1;

/// The complete two-stage grade for one image or clip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GradeParameters {
    /// Schema version this value was written with. See [`SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Stage 1 — technical correction.
    pub stage1: CorrectionParameters,
    /// Stage 2 — creative grade.
    pub stage2: CreativeParameters,
}

impl Default for GradeParameters {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl GradeParameters {
    /// Identity grade — no adjustment in either stage. Rendering an sRGB
    /// image with this grade to an sRGB output reproduces it within 1 LSB.
    pub const IDENTITY: Self = Self {
        schema_version: SCHEMA_VERSION,
        stage1: CorrectionParameters::IDENTITY,
        stage2: CreativeParameters::IDENTITY,
    };

    /// A copy of `self` with Stage 2 reset to identity — the "Stage 1 only"
    /// comparison source.
    pub fn stage1_only(&self) -> Self {
        Self {
            stage2: CreativeParameters::IDENTITY,
            ..self.clone()
        }
    }

    /// A copy of `self` with both stages bypassed but the input decode
    /// preserved — the "Original" comparison source.
    pub fn original(&self) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            stage1: CorrectionParameters {
                input_profile: self.stage1.input_profile,
                ..CorrectionParameters::IDENTITY
            },
            stage2: CreativeParameters::IDENTITY,
        }
    }
}

/// Camera log encoding of the source, decoded in Stage 1 to scene-linear
/// Rec.2020. `None` means a display-referred sRGB source (JPEG/PNG/WebP).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LogProfile {
    /// Display-referred sRGB source. Decoded with the sRGB EOTF.
    #[default]
    None,
    /// ARRI LogC3 (EI 800), ARRI Wide Gamut 3.
    ArriLogC3,
    /// Sony S-Log3, S-Gamut3.Cine.
    SonySLog3,
    /// Panasonic V-Log, V-Gamut.
    PanasonicVLog,
    /// Apple Log (iPhone 15 Pro+), Rec.2020 primaries.
    AppleLog,
}

impl LogProfile {
    /// Every variant, in UI order.
    pub const ALL: [LogProfile; 5] = [
        LogProfile::None,
        LogProfile::ArriLogC3,
        LogProfile::SonySLog3,
        LogProfile::PanasonicVLog,
        LogProfile::AppleLog,
    ];

    /// Human-readable name.
    pub fn label(self) -> &'static str {
        match self {
            LogProfile::None => "sRGB (no log)",
            LogProfile::ArriLogC3 => "ARRI LogC3",
            LogProfile::SonySLog3 => "Sony S-Log3",
            LogProfile::PanasonicVLog => "Panasonic V-Log",
            LogProfile::AppleLog => "Apple Log",
        }
    }
}

/// An RGB triple. Unit and color space are documented on each field that
/// uses it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Rgb {
    /// Red channel.
    pub r: f32,
    /// Green channel.
    pub g: f32,
    /// Blue channel.
    pub b: f32,
}

impl Rgb {
    /// All-zero triple.
    pub const ZERO: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
    };
}

/// Stage 1 — technical correction. Establishes a neutral, calibrated
/// baseline before the creative grade. Non-destructive: these values are
/// metadata applied at render time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CorrectionParameters {
    /// When `false`, Stage 1 is bypassed except for input decode.
    pub enabled: bool,
    /// Source encoding decoded to scene-linear Rec.2020.
    pub input_profile: LogProfile,
    /// When `true` and `input_profile` is a log profile, a filmic
    /// display-rendering curve maps scene-linear to display-referred linear
    /// (the "log-to-display" step). Ignored for `LogProfile::None`.
    pub log_to_display: bool,
    /// Exposure normalization in **stops** (multiplier `2^exposure`),
    /// applied in scene-linear Rec.2020.
    pub exposure: f32,
    /// White-balance correction in **mireds** relative to D65 (6504 K).
    /// Positive = warmer (multiplies by the white of a lower-CCT Planckian
    /// illuminant). Applied in scene-linear Rec.2020.
    pub temperature: f32,
    /// Green–magenta correction in **tint units** (±100 ≈ ±16% green gain,
    /// luminance-preserving). Positive = magenta. Scene-linear Rec.2020.
    pub tint: f32,
    /// Per-channel black point in **scene-linear Rec.2020** units. Each
    /// channel is remapped `(x - bp) / (1 - bp)`, so `bp` becomes 0.
    pub black_point: Rgb,
    /// Whether `exposure` was set by auto-analysis (re-run on new images).
    pub auto_exposure: bool,
    /// Whether `temperature`/`tint` were set by auto-analysis.
    pub auto_white_balance: bool,
    /// Whether `black_point` was set by auto-analysis.
    pub auto_black_point: bool,
    /// Auto-exposure target for the median luminance, as an
    /// **sRGB-encoded** value in [0, 1]. PRD default: 0.45 (45%).
    pub exposure_target: f32,
}

impl Default for CorrectionParameters {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl CorrectionParameters {
    /// Identity Stage 1: decode only.
    pub const IDENTITY: Self = Self {
        enabled: true,
        input_profile: LogProfile::None,
        log_to_display: true,
        exposure: 0.0,
        temperature: 0.0,
        tint: 0.0,
        black_point: Rgb::ZERO,
        auto_exposure: false,
        auto_white_balance: false,
        auto_black_point: false,
        exposure_target: 0.45,
    };
}

/// A three-way color wheel (lift, gamma, or gain). Each channel is a
/// **normalized offset** in the working encoding; `master` is added to all
/// three channels. Zero everywhere is identity.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ColorWheel {
    /// Red offset, normalized (−1..1).
    pub r: f32,
    /// Green offset, normalized (−1..1).
    pub g: f32,
    /// Blue offset, normalized (−1..1).
    pub b: f32,
    /// Offset added to all channels, normalized (−1..1).
    pub master: f32,
}

impl ColorWheel {
    /// Identity wheel.
    pub const IDENTITY: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        master: 0.0,
    };

    /// Effective per-channel value (`channel + master`).
    pub fn effective(&self) -> [f32; 3] {
        [
            self.r + self.master,
            self.g + self.master,
            self.b + self.master,
        ]
    }
}

/// Hue/saturation/luminance adjustment for one hue band.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct HslBand {
    /// Hue shift in **degrees** (−30..30) for colors in this band.
    pub hue: f32,
    /// Saturation change in **percent** (−100..100); multiplier `1 + s/100`.
    pub saturation: f32,
    /// Luminance change in **percent** (−100..100), weighted by chroma so
    /// neutrals are unaffected. Applied in the working encoding.
    pub luminance: f32,
}

impl HslBand {
    /// Identity band.
    pub const IDENTITY: Self = Self {
        hue: 0.0,
        saturation: 0.0,
        luminance: 0.0,
    };
}

/// HSL curves expressed as eight hue-band control points. Between band
/// centers the adjustment is interpolated with a periodic smoothstep, which
/// makes this a hue-vs-hue / hue-vs-sat / hue-vs-lum curve with eight fixed
/// knots. Band centers (degrees, HSV hue of the working-encoded signal) are
/// in [`HslBands::CENTERS_DEG`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct HslBands {
    /// Reds, centered 0°.
    pub red: HslBand,
    /// Oranges, centered 30°.
    pub orange: HslBand,
    /// Yellows, centered 60°.
    pub yellow: HslBand,
    /// Greens, centered 120°.
    pub green: HslBand,
    /// Aquas, centered 180°.
    pub aqua: HslBand,
    /// Blues, centered 225°.
    pub blue: HslBand,
    /// Purples, centered 270°.
    pub purple: HslBand,
    /// Magentas, centered 315°.
    pub magenta: HslBand,
}

impl HslBands {
    /// Band center hues in degrees, in field order.
    pub const CENTERS_DEG: [f32; 8] = [0.0, 30.0, 60.0, 120.0, 180.0, 225.0, 270.0, 315.0];

    /// Band names, in field order.
    pub const NAMES: [&'static str; 8] = [
        "red", "orange", "yellow", "green", "aqua", "blue", "purple", "magenta",
    ];

    /// Identity: every band zero.
    pub const IDENTITY: Self = Self {
        red: HslBand::IDENTITY,
        orange: HslBand::IDENTITY,
        yellow: HslBand::IDENTITY,
        green: HslBand::IDENTITY,
        aqua: HslBand::IDENTITY,
        blue: HslBand::IDENTITY,
        purple: HslBand::IDENTITY,
        magenta: HslBand::IDENTITY,
    };

    /// Bands in field order.
    pub fn as_array(&self) -> [HslBand; 8] {
        [
            self.red,
            self.orange,
            self.yellow,
            self.green,
            self.aqua,
            self.blue,
            self.purple,
            self.magenta,
        ]
    }
}

/// Split toning: tint shadows and highlights independently.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SplitTone {
    /// Shadow tint hue in **degrees** (0..360).
    pub shadow_hue: f32,
    /// Shadow tint strength in **percent** (0..100).
    pub shadow_saturation: f32,
    /// Highlight tint hue in **degrees** (0..360).
    pub highlight_hue: f32,
    /// Highlight tint strength in **percent** (0..100).
    pub highlight_saturation: f32,
    /// Balance in **percent** (−100..100). Positive moves the crossover
    /// toward the shadows, so highlights tint covers more of the image.
    pub balance: f32,
}

impl SplitTone {
    /// Identity: no tint.
    pub const IDENTITY: Self = Self {
        shadow_hue: 0.0,
        shadow_saturation: 0.0,
        highlight_hue: 0.0,
        highlight_saturation: 0.0,
        balance: 0.0,
    };
}

/// One control point of a tone curve, both coordinates in the **working
/// encoding** normalized to [0, 1].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct CurvePoint {
    /// Input value, [0, 1].
    pub x: f32,
    /// Output value, [0, 1].
    pub y: f32,
}

impl CurvePoint {
    /// Construct a point.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Master + per-channel tone curves. Each is a list of control points,
/// interpolated with a monotone cubic spline (see [`crate::curve`]). The
/// master curve is applied first, then the channel curve. Applied in the
/// working encoding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ToneCurve {
    /// Master curve applied to R, G and B.
    pub master: Vec<CurvePoint>,
    /// Red channel curve.
    pub red: Vec<CurvePoint>,
    /// Green channel curve.
    pub green: Vec<CurvePoint>,
    /// Blue channel curve.
    pub blue: Vec<CurvePoint>,
}

/// The explicit two-point identity curve `[(0,0), (1,1)]`.
pub fn identity_curve() -> Vec<CurvePoint> {
    vec![CurvePoint::new(0.0, 0.0), CurvePoint::new(1.0, 1.0)]
}

impl Default for ToneCurve {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl ToneCurve {
    /// Identity curves. An **empty** point list is the canonical identity
    /// (equivalent to `[(0,0), (1,1)]`); this keeps the identity grade a
    /// `const`.
    pub const IDENTITY: Self = Self {
        master: Vec::new(),
        red: Vec::new(),
        green: Vec::new(),
        blue: Vec::new(),
    };

    /// Maximum control points per curve.
    pub const MAX_POINTS: usize = 16;

    /// Whether every channel is the identity line.
    pub fn is_identity(&self) -> bool {
        [&self.master, &self.red, &self.green, &self.blue]
            .iter()
            .all(|c| c.iter().all(|p| (p.x - p.y).abs() < 1e-6))
    }

    /// Curves in `[master, red, green, blue]` order.
    pub fn channels(&self) -> [&Vec<CurvePoint>; 4] {
        [&self.master, &self.red, &self.green, &self.blue]
    }
}

/// Elliptical vignette.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Vignette {
    /// Strength in **percent** (−100..100). Negative darkens the edges,
    /// positive lightens them. Multiplicative, scene-linear.
    pub amount: f32,
    /// Distance from center where falloff begins, **percent** of the half
    /// diagonal (0..100).
    pub midpoint: f32,
    /// Width of the falloff, **percent** (0..100).
    pub feather: f32,
    /// Shape in **percent** (−100..100): −100 follows the frame's aspect
    /// (oval), +100 is a circle.
    pub roundness: f32,
    /// Horizontal center offset, **percent** of half width (−100..100).
    pub center_x: f32,
    /// Vertical center offset, **percent** of half height (−100..100).
    pub center_y: f32,
}

impl Vignette {
    /// Identity: no vignette.
    pub const IDENTITY: Self = Self {
        amount: 0.0,
        midpoint: 50.0,
        feather: 50.0,
        roundness: 0.0,
        center_x: 0.0,
        center_y: 0.0,
    };
}

/// Deterministic film grain. Same seed + same pixel coordinate → same noise
/// on every backend.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Grain {
    /// Strength in **percent** (0..100). Applied additively in the working
    /// encoding, weighted toward midtones.
    pub amount: f32,
    /// Grain cell size in **source pixels** (0.5..4).
    pub size: f32,
    /// Mix of a finer second octave, **percent** (0..100).
    pub roughness: f32,
    /// Noise seed. Explicit so renders are reproducible.
    pub seed: u32,
}

impl Grain {
    /// Identity: no grain.
    pub const IDENTITY: Self = Self {
        amount: 0.0,
        size: 1.0,
        roughness: 50.0,
        seed: 0,
    };
}

/// Stage 2 — the creative grade. Order of application:
/// primaries → lift/gamma/gain → HSL → split tone → tone curve → LUT →
/// vignette → grain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CreativeParameters {
    /// When `false`, Stage 2 is bypassed.
    pub enabled: bool,
    /// Creative exposure in **stops**, scene-linear Rec.2020.
    pub exposure: f32,
    /// Contrast in **percent** (−100..100) around a mid-grey pivot in the
    /// working encoding; slope multiplier `2^(contrast/100)`.
    pub contrast: f32,
    /// Highlight recovery/boost in **percent** (−100..100), working encoding.
    pub highlights: f32,
    /// Shadow lift/crush in **percent** (−100..100), working encoding.
    pub shadows: f32,
    /// White level in **percent** (−100..100), working encoding.
    pub whites: f32,
    /// Black level in **percent** (−100..100), working encoding.
    pub blacks: f32,
    /// Creative temperature in **mireds** (positive = warmer), scene-linear
    /// Rec.2020. Same model as Stage 1 white balance.
    pub temperature: f32,
    /// Creative tint in **tint units** (positive = magenta).
    pub tint: f32,
    /// Vibrance in **percent** (−100..100): saturation weighted toward
    /// low-chroma colors. Scene-linear Rec.2020.
    pub vibrance: f32,
    /// Saturation in **percent** (−100..100); multiplier `1 + s/100` around
    /// Rec.2020 luminance. Scene-linear.
    pub saturation: f32,
    /// Lift wheel (shadows).
    pub lift: ColorWheel,
    /// Gamma wheel (midtones).
    pub gamma: ColorWheel,
    /// Gain wheel (highlights).
    pub gain: ColorWheel,
    /// Hue-band HSL curves.
    pub hsl: HslBands,
    /// Split toning.
    pub split_tone: SplitTone,
    /// Master + RGB tone curves.
    pub tone_curve: ToneCurve,
    /// Host-side identifier of an imported 3D LUT, or `None`.
    pub lut_id: Option<String>,
    /// LUT mix in **percent** (0..100). The LUT is applied in sRGB-encoded
    /// Rec.709 (the `.cube` convention).
    pub lut_strength: f32,
    /// Vignette.
    pub vignette: Vignette,
    /// Film grain.
    pub grain: Grain,
}

impl Default for CreativeParameters {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl CreativeParameters {
    /// Identity Stage 2.
    pub const IDENTITY: Self = Self {
        enabled: true,
        exposure: 0.0,
        contrast: 0.0,
        highlights: 0.0,
        shadows: 0.0,
        whites: 0.0,
        blacks: 0.0,
        temperature: 0.0,
        tint: 0.0,
        vibrance: 0.0,
        saturation: 0.0,
        lift: ColorWheel::IDENTITY,
        gamma: ColorWheel::IDENTITY,
        gain: ColorWheel::IDENTITY,
        hsl: HslBands::IDENTITY,
        split_tone: SplitTone::IDENTITY,
        tone_curve: ToneCurve::IDENTITY,
        lut_id: None,
        lut_strength: 100.0,
        vignette: Vignette::IDENTITY,
        grain: Grain::IDENTITY,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_round_trips_through_json() {
        let json = serde_json::to_string(&GradeParameters::IDENTITY).unwrap();
        let parsed: GradeParameters = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, GradeParameters::IDENTITY);
    }

    #[test]
    fn missing_fields_fall_back_to_default() {
        let parsed: GradeParameters = serde_json::from_str("{}").unwrap();
        assert_eq!(parsed, GradeParameters::default());
    }

    #[test]
    fn partial_nested_fields_fall_back_to_default() {
        let parsed: GradeParameters =
            serde_json::from_str(r#"{"stage2":{"contrast":20.0,"lift":{"r":0.1}}}"#).unwrap();
        assert_eq!(parsed.stage2.contrast, 20.0);
        assert_eq!(parsed.stage2.lift.r, 0.1);
        assert_eq!(parsed.stage2.lift.g, 0.0);
        assert_eq!(parsed.stage1, CorrectionParameters::IDENTITY);
    }

    #[test]
    fn log_profile_serializes_snake_case() {
        let json = serde_json::to_string(&LogProfile::SonySLog3).unwrap();
        assert_eq!(json, "\"sony_s_log3\"");
    }

    #[test]
    fn comparison_sources_reset_the_right_stage() {
        let mut p = GradeParameters::IDENTITY;
        p.stage1.exposure = 1.0;
        p.stage1.input_profile = LogProfile::AppleLog;
        p.stage2.contrast = 30.0;
        let s1 = p.stage1_only();
        assert_eq!(s1.stage1.exposure, 1.0);
        assert_eq!(s1.stage2, CreativeParameters::IDENTITY);
        let orig = p.original();
        assert_eq!(orig.stage1.exposure, 0.0);
        assert_eq!(orig.stage1.input_profile, LogProfile::AppleLog);
    }
}
