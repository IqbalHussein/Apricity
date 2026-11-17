//! The v1 lexicon: a static table mapping prompt phrases to deltas on Stage 2
//! parameters.
//!
//! Every delta is expressed in the target parameter's own unit (see
//! `apricity_core::param_spec`): percent sliders, mireds, tint units, stops,
//! degrees, or normalized wheel offsets in the working encoding. Paths are
//! relative to `stage2.`. An entry applied at strength `1.0` is a moderate
//! move; modifiers scale it.
//!
//! Film stock, director and film names are descriptive keywords for a look,
//! not claims of equivalence.

use std::collections::HashMap;
use std::sync::OnceLock;

use apricity_core::grade_parameters::CurvePoint;

/// Which family an entry belongs to; used to pick entries for "Surprise Me".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    /// Emotion or atmosphere ("moody", "dreamy").
    Mood,
    /// Color temperature or palette ("warm", "teal and orange").
    Palette,
    /// Technical grading term ("matte", "high contrast").
    Tech,
    /// Comparative ("warmer", "punchier"); meaningful mainly in Refine.
    Comparative,
    /// Film stock look ("portra 400", "tri-x").
    Stock,
    /// Genre, era or reference look ("noir", "80s", "blade runner").
    Genre,
    /// Nature or scene ("forest", "snow").
    Scene,
}

/// A named tone-curve shape applied to `stage2.tone_curve.master`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurvePreset {
    /// Steeper mid-contrast: darker shadows, brighter highlights.
    SCurve,
    /// Gentler mid-contrast (inverse S).
    Soft,
    /// Lifted blacks (matte).
    Fade,
    /// Crushed near-blacks.
    Crush,
    /// Brightened midtones.
    Brighten,
    /// Darkened midtones.
    Darken,
}

impl CurvePreset {
    /// Output offset at strength `1.0` for input `x`, both in the working
    /// encoding normalized to `[0, 1]`. Positive strength multiplies this;
    /// negative strength applies the opposite shape.
    pub fn offset(self, x: f32) -> f32 {
        let tau = std::f32::consts::TAU;
        let pi = std::f32::consts::PI;
        match self {
            CurvePreset::SCurve => -0.07 * (tau * x).sin(),
            CurvePreset::Soft => 0.05 * (tau * x).sin(),
            CurvePreset::Fade => 0.07 * (1.0 - x) * (1.0 - x),
            CurvePreset::Crush => -0.12 * (1.0 - x).powi(3),
            CurvePreset::Brighten => 0.08 * (pi * x).sin(),
            CurvePreset::Darken => -0.08 * (pi * x).sin(),
        }
    }

    /// Input positions of the control points emitted for a preset.
    pub const XS: [f32; 7] = [0.0, 0.125, 0.25, 0.5, 0.75, 0.875, 1.0];

    /// Control points for this preset at `strength` on an identity curve.
    pub fn points(self, strength: f32) -> Vec<CurvePoint> {
        Self::XS
            .iter()
            .map(|&x| CurvePoint::new(x, (x + strength * self.offset(x)).clamp(0.0, 1.0)))
            .collect()
    }
}

/// One lexicon entry.
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    /// Lowercase phrases (aliases); the first is the canonical term. Words
    /// are single-space separated with punctuation already normalized away.
    pub phrases: &'static [&'static str],
    /// Family, used by "Surprise Me".
    pub category: Category,
    /// `(path relative to "stage2.", delta)` pairs. Deltas are added at
    /// strength 1.0 in the parameter's own unit.
    pub deltas: &'static [(&'static str, f32)],
    /// `(path relative to "stage2.", absolute value)` pairs, used for
    /// split-tone hues in **degrees** (0..360). Applied only at positive
    /// strength.
    pub sets: &'static [(&'static str, f32)],
    /// Optional master tone-curve preset.
    pub curve: Option<CurvePreset>,
}

type D = &'static [(&'static str, f32)];

const fn e(phrases: &'static [&'static str], category: Category, deltas: D) -> Entry {
    Entry {
        phrases,
        category,
        deltas,
        sets: &[],
        curve: None,
    }
}

impl Entry {
    const fn sets(self, sets: D) -> Self {
        Self { sets, ..self }
    }

    const fn curve(self, curve: CurvePreset) -> Self {
        Self {
            curve: Some(curve),
            ..self
        }
    }

    /// Fully qualified path (`stage2.<short>`) for a table-relative path.
    pub fn full_path(short: &str) -> String {
        format!("stage2.{short}")
    }
}

use Category::{Comparative as Cmp, Genre, Mood, Palette, Scene, Stock, Tech};
use CurvePreset::{Brighten, Crush, Darken, Fade, SCurve, Soft};

/// Every lexicon entry.
pub static ENTRIES: &[Entry] = &[
    // ---- Moods ----
    e(
        &["moody", "brooding"],
        Mood,
        &[
            ("contrast", 15.0),
            ("exposure", -0.3),
            ("saturation", -12.0),
            ("shadows", -10.0),
            ("vignette.amount", -15.0),
            ("temperature", -8.0),
        ],
    ),
    e(
        &["dreamy", "dreamlike"],
        Mood,
        &[
            ("contrast", -12.0),
            ("highlights", 12.0),
            ("vibrance", 10.0),
            ("temperature", 8.0),
            ("tint", 6.0),
        ],
    )
    .curve(Soft),
    e(
        &["hopeful", "optimistic"],
        Mood,
        &[
            ("exposure", 0.25),
            ("temperature", 12.0),
            ("vibrance", 15.0),
            ("highlights", 8.0),
            ("contrast", -4.0),
        ],
    ),
    e(
        &["melancholic", "melancholy", "sad", "sorrowful"],
        Mood,
        &[
            ("saturation", -22.0),
            ("temperature", -14.0),
            ("exposure", -0.2),
            ("contrast", -8.0),
            ("split_tone.shadow_saturation", 12.0),
        ],
    )
    .sets(&[("split_tone.shadow_hue", 215.0)]),
    e(
        &["nostalgic", "sentimental", "wistful"],
        Mood,
        &[
            ("temperature", 14.0),
            ("saturation", -10.0),
            ("blacks", 8.0),
            ("grain.amount", 14.0),
            ("vignette.amount", -8.0),
            ("tint", 4.0),
        ],
    )
    .curve(Fade),
    e(
        &["eerie", "creepy", "uncanny"],
        Mood,
        &[
            ("saturation", -22.0),
            ("tint", -14.0),
            ("temperature", -10.0),
            ("contrast", 8.0),
            ("exposure", -0.3),
            ("vignette.amount", -20.0),
            ("shadows", -8.0),
        ],
    ),
    e(
        &["romantic", "romance"],
        Mood,
        &[
            ("temperature", 12.0),
            ("tint", 8.0),
            ("highlights", -6.0),
            ("vibrance", 8.0),
            ("contrast", -8.0),
            ("gain.r", 0.02),
        ],
    )
    .curve(Soft),
    e(
        &["gritty", "raw"],
        Mood,
        &[
            ("contrast", 25.0),
            ("saturation", -20.0),
            ("grain.amount", 30.0),
            ("grain.roughness", 20.0),
            ("grain.size", 0.3),
            ("blacks", -10.0),
            ("shadows", -8.0),
        ],
    ),
    e(
        &["serene", "calm", "peaceful", "tranquil"],
        Mood,
        &[
            ("contrast", -10.0),
            ("saturation", -8.0),
            ("temperature", -6.0),
            ("highlights", -6.0),
            ("vibrance", 4.0),
            ("exposure", 0.1),
        ],
    ),
    e(
        &["energetic", "lively", "dynamic"],
        Mood,
        &[
            ("vibrance", 25.0),
            ("saturation", 12.0),
            ("contrast", 15.0),
            ("exposure", 0.15),
            ("temperature", 6.0),
        ],
    ),
    e(
        &["dramatic", "intense"],
        Mood,
        &[
            ("contrast", 30.0),
            ("shadows", -15.0),
            ("highlights", -8.0),
            ("saturation", 5.0),
            ("vignette.amount", -18.0),
        ],
    )
    .curve(SCurve),
    e(
        &["mysterious", "enigmatic"],
        Mood,
        &[
            ("exposure", -0.4),
            ("contrast", 12.0),
            ("temperature", -10.0),
            ("tint", -6.0),
            ("saturation", -10.0),
            ("vignette.amount", -22.0),
            ("split_tone.shadow_saturation", 14.0),
        ],
    )
    .sets(&[("split_tone.shadow_hue", 230.0)]),
    e(
        &["tense", "suspenseful", "anxious"],
        Mood,
        &[
            ("contrast", 20.0),
            ("saturation", -18.0),
            ("temperature", -6.0),
            ("tint", -4.0),
            ("shadows", -10.0),
            ("grain.amount", 10.0),
        ],
    ),
    e(
        &["cozy", "cosy", "snug"],
        Mood,
        &[
            ("temperature", 20.0),
            ("exposure", -0.1),
            ("saturation", 6.0),
            ("contrast", -8.0),
            ("highlights", -8.0),
            ("vignette.amount", -8.0),
            ("gain.r", 0.02),
        ],
    ),
    e(
        &["lonely", "isolated", "solitary"],
        Mood,
        &[
            ("saturation", -30.0),
            ("temperature", -14.0),
            ("exposure", -0.2),
            ("vignette.amount", -14.0),
            ("contrast", -4.0),
        ],
    ),
    e(
        &["joyful", "cheerful", "happy", "bubbly"],
        Mood,
        &[
            ("exposure", 0.3),
            ("vibrance", 22.0),
            ("saturation", 10.0),
            ("temperature", 10.0),
            ("contrast", 6.0),
        ],
    ),
    e(
        &["gloomy", "dreary", "somber"],
        Mood,
        &[
            ("exposure", -0.4),
            ("saturation", -22.0),
            ("temperature", -10.0),
            ("contrast", -6.0),
            ("highlights", -10.0),
        ],
    ),
    e(
        &["ethereal", "angelic"],
        Mood,
        &[
            ("exposure", 0.3),
            ("contrast", -18.0),
            ("highlights", 12.0),
            ("saturation", -8.0),
            ("vibrance", 8.0),
            ("temperature", -4.0),
            ("tint", 4.0),
        ],
    )
    .curve(Soft),
    e(
        &["intimate"],
        Mood,
        &[
            ("temperature", 10.0),
            ("exposure", -0.2),
            ("vignette.amount", -12.0),
            ("contrast", -4.0),
            ("saturation", -4.0),
            ("highlights", -6.0),
        ],
    ),
    e(
        &["ominous", "foreboding", "sinister"],
        Mood,
        &[
            ("exposure", -0.5),
            ("contrast", 18.0),
            ("saturation", -25.0),
            ("temperature", -6.0),
            ("vignette.amount", -28.0),
            ("tint", -6.0),
            ("shadows", -12.0),
        ],
    ),
    e(
        &["whimsical", "playful", "quirky"],
        Mood,
        &[
            ("vibrance", 22.0),
            ("saturation", 8.0),
            ("temperature", 4.0),
            ("contrast", -4.0),
            ("tint", 4.0),
            ("exposure", 0.15),
        ],
    ),
    e(
        &["nightmarish", "disturbing"],
        Mood,
        &[
            ("tint", -18.0),
            ("saturation", -10.0),
            ("contrast", 25.0),
            ("exposure", -0.4),
            ("vignette.amount", -25.0),
            ("grain.amount", 20.0),
        ],
    ),
    e(
        &["lush", "verdant", "overgrown"],
        Mood,
        &[
            ("saturation", 8.0),
            ("vibrance", 14.0),
            ("hsl.green.saturation", 25.0),
            ("hsl.green.hue", -8.0),
            ("hsl.yellow.saturation", 10.0),
            ("contrast", 6.0),
        ],
    ),
    e(
        &["glowy", "glowing", "radiant"],
        Mood,
        &[
            ("highlights", 15.0),
            ("whites", 8.0),
            ("contrast", -12.0),
            ("exposure", 0.2),
            ("vibrance", 10.0),
            ("temperature", 6.0),
        ],
    )
    .curve(Soft),
    e(
        &["haunting", "spooky"],
        Mood,
        &[
            ("saturation", -28.0),
            ("temperature", -12.0),
            ("tint", -8.0),
            ("vignette.amount", -20.0),
            ("exposure", -0.2),
            ("grain.amount", 14.0),
        ],
    ),
    e(
        &["bleak", "desolate", "barren"],
        Mood,
        &[
            ("saturation", -35.0),
            ("contrast", 10.0),
            ("exposure", -0.2),
            ("temperature", -10.0),
            ("vibrance", -8.0),
        ],
    ),
    e(
        &["uplifting", "inspiring"],
        Mood,
        &[
            ("exposure", 0.25),
            ("temperature", 8.0),
            ("vibrance", 15.0),
            ("contrast", 4.0),
            ("highlights", 6.0),
        ],
    ),
    e(
        &["angry", "furious", "aggressive"],
        Mood,
        &[
            ("contrast", 25.0),
            ("saturation", 10.0),
            ("temperature", 12.0),
            ("hsl.red.saturation", 25.0),
            ("vignette.amount", -20.0),
        ],
    ),
    e(
        &["elegant", "refined", "sophisticated"],
        Mood,
        &[
            ("contrast", 8.0),
            ("saturation", -10.0),
            ("highlights", -6.0),
            ("blacks", -4.0),
            ("vibrance", 4.0),
        ],
    ),
    // ---- Palette and temperature ----
    e(
        &["warm", "warmth"],
        Palette,
        &[("temperature", 25.0), ("saturation", 3.0)],
    ),
    e(
        &["cool", "cold", "chilly"],
        Palette,
        &[("temperature", -25.0)],
    ),
    e(
        &["icy", "frosty", "frozen"],
        Palette,
        &[
            ("temperature", -35.0),
            ("saturation", -10.0),
            ("exposure", 0.1),
            ("tint", -4.0),
        ],
    ),
    e(
        &["golden hour", "golden", "magic hour"],
        Palette,
        &[
            ("temperature", 30.0),
            ("exposure", 0.15),
            ("highlights", -8.0),
            ("saturation", 8.0),
            ("contrast", 4.0),
            ("hsl.orange.saturation", 10.0),
            ("gain.r", 0.03),
            ("gain.b", -0.03),
        ],
    ),
    e(
        &["sunset", "dusk", "twilight"],
        Palette,
        &[
            ("temperature", 32.0),
            ("tint", 8.0),
            ("saturation", 14.0),
            ("contrast", 8.0),
            ("exposure", -0.15),
            ("split_tone.highlight_saturation", 20.0),
            ("split_tone.shadow_saturation", 14.0),
        ],
    )
    .sets(&[
        ("split_tone.highlight_hue", 25.0),
        ("split_tone.shadow_hue", 270.0),
    ]),
    e(
        &["sunrise", "dawn", "daybreak"],
        Palette,
        &[
            ("temperature", 18.0),
            ("tint", 6.0),
            ("exposure", 0.1),
            ("contrast", -4.0),
            ("vibrance", 8.0),
            ("split_tone.highlight_saturation", 15.0),
            ("split_tone.shadow_saturation", 10.0),
        ],
    )
    .sets(&[
        ("split_tone.highlight_hue", 40.0),
        ("split_tone.shadow_hue", 240.0),
    ]),
    e(
        &["blue hour"],
        Palette,
        &[
            ("temperature", -28.0),
            ("exposure", -0.3),
            ("saturation", 4.0),
            ("contrast", 4.0),
            ("split_tone.shadow_saturation", 22.0),
            ("split_tone.highlight_saturation", 8.0),
        ],
    )
    .sets(&[
        ("split_tone.shadow_hue", 225.0),
        ("split_tone.highlight_hue", 40.0),
    ]),
    e(
        &[
            "teal and orange",
            "teal orange",
            "orange and teal",
            "orange teal",
        ],
        Palette,
        &[
            ("contrast", 10.0),
            ("saturation", 4.0),
            ("split_tone.shadow_saturation", 28.0),
            ("split_tone.highlight_saturation", 24.0),
            ("hsl.orange.saturation", 8.0),
            ("lift.b", 0.02),
            ("lift.g", 0.01),
            ("gain.r", 0.025),
            ("gain.b", -0.02),
        ],
    )
    .sets(&[
        ("split_tone.shadow_hue", 190.0),
        ("split_tone.highlight_hue", 28.0),
    ]),
    e(
        &["pastel", "pastels"],
        Palette,
        &[
            ("saturation", -15.0),
            ("vibrance", 10.0),
            ("exposure", 0.3),
            ("contrast", -18.0),
            ("highlights", -4.0),
        ],
    )
    .curve(Fade),
    e(
        &["earthy", "earth tones"],
        Palette,
        &[
            ("saturation", -10.0),
            ("temperature", 10.0),
            ("hsl.orange.saturation", 10.0),
            ("hsl.green.hue", -10.0),
            ("hsl.green.saturation", -15.0),
            ("contrast", 4.0),
        ],
    ),
    e(
        &["neon", "neons"],
        Palette,
        &[
            ("saturation", 25.0),
            ("vibrance", 20.0),
            ("contrast", 18.0),
            ("exposure", -0.2),
            ("blacks", -8.0),
            ("hsl.magenta.saturation", 15.0),
            ("split_tone.shadow_saturation", 25.0),
            ("split_tone.highlight_saturation", 20.0),
        ],
    )
    .sets(&[
        ("split_tone.shadow_hue", 280.0),
        ("split_tone.highlight_hue", 180.0),
    ]),
    e(
        &["monochrome", "monochromatic"],
        Palette,
        &[("saturation", -100.0), ("contrast", 8.0)],
    ),
    e(
        &[
            "black and white",
            "b and w",
            "bw",
            "black white",
            "greyscale",
            "grayscale",
        ],
        Palette,
        &[("saturation", -100.0), ("contrast", 12.0)],
    ),
    e(
        &["sepia"],
        Palette,
        &[
            ("saturation", -70.0),
            ("temperature", 15.0),
            ("split_tone.shadow_saturation", 30.0),
            ("split_tone.highlight_saturation", 30.0),
        ],
    )
    .sets(&[
        ("split_tone.shadow_hue", 30.0),
        ("split_tone.highlight_hue", 40.0),
    ])
    .curve(Fade),
    e(
        &["green", "greenish", "green tint"],
        Palette,
        &[("tint", -20.0)],
    ),
    e(
        &["magenta", "magenta tint"],
        Palette,
        &[("tint", 20.0), ("hsl.magenta.saturation", 10.0)],
    ),
    e(
        &["blue", "bluish", "blue tint"],
        Palette,
        &[("temperature", -20.0), ("lift.b", 0.015)],
    ),
    e(
        &["yellow", "yellowish"],
        Palette,
        &[("temperature", 14.0), ("hsl.yellow.saturation", 12.0)],
    ),
    e(
        &["red", "reddish"],
        Palette,
        &[("gain.r", 0.03), ("hsl.red.saturation", 15.0)],
    ),
    e(
        &["orange"],
        Palette,
        &[("temperature", 10.0), ("hsl.orange.saturation", 15.0)],
    ),
    e(
        &["teal", "cyan", "aqua"],
        Palette,
        &[
            ("tint", -4.0),
            ("hsl.aqua.saturation", 10.0),
            ("split_tone.shadow_saturation", 25.0),
        ],
    )
    .sets(&[("split_tone.shadow_hue", 190.0)]),
    e(
        &["pink", "pinkish", "rosy"],
        Palette,
        &[
            ("tint", 12.0),
            ("temperature", 6.0),
            ("hsl.magenta.saturation", 8.0),
        ],
    ),
    e(
        &["purple", "violet"],
        Palette,
        &[
            ("hsl.purple.saturation", 18.0),
            ("split_tone.shadow_saturation", 15.0),
        ],
    )
    .sets(&[("split_tone.shadow_hue", 270.0)]),
    e(
        &["amber"],
        Palette,
        &[
            ("temperature", 28.0),
            ("tint", 4.0),
            ("hsl.orange.saturation", 14.0),
            ("hsl.yellow.saturation", 10.0),
        ],
    ),
    e(
        &["tropical", "vacation"],
        Palette,
        &[
            ("vibrance", 20.0),
            ("saturation", 10.0),
            ("hsl.aqua.saturation", 20.0),
            ("hsl.green.saturation", 14.0),
            ("temperature", 8.0),
        ],
    ),
    e(
        &["candlelight", "candlelit", "firelight"],
        Palette,
        &[
            ("temperature", 40.0),
            ("exposure", -0.5),
            ("contrast", 6.0),
            ("vignette.amount", -25.0),
            ("saturation", -4.0),
            ("shadows", -10.0),
            ("split_tone.highlight_saturation", 20.0),
        ],
    )
    .sets(&[("split_tone.highlight_hue", 28.0)]),
    e(
        &["moonlight", "moonlit"],
        Palette,
        &[
            ("temperature", -30.0),
            ("exposure", -0.6),
            ("saturation", -15.0),
            ("contrast", 8.0),
            ("vignette.amount", -15.0),
            ("split_tone.shadow_saturation", 22.0),
        ],
    )
    .sets(&[("split_tone.shadow_hue", 230.0)]),
    // ---- Technical terms ----
    e(
        &["lifted blacks", "raised blacks", "lift blacks"],
        Tech,
        &[("blacks", 18.0)],
    ),
    e(
        &["milky", "milky blacks"],
        Tech,
        &[("blacks", 22.0), ("contrast", -14.0), ("saturation", -6.0)],
    ),
    e(
        &["crushed blacks", "crush blacks", "deep blacks"],
        Tech,
        &[("blacks", -18.0)],
    )
    .curve(Crush),
    e(
        &["high contrast", "contrasty", "contrasted"],
        Tech,
        &[("contrast", 30.0)],
    ),
    e(&["low contrast"], Tech, &[("contrast", -30.0)]),
    e(&["contrast"], Tech, &[("contrast", 20.0)]),
    e(
        &["flat", "flat look"],
        Tech,
        &[("contrast", -25.0), ("saturation", -8.0)],
    ),
    e(
        &["matte"],
        Tech,
        &[("blacks", 14.0), ("contrast", -10.0), ("highlights", -4.0)],
    )
    .curve(Fade),
    e(
        &["faded", "fade"],
        Tech,
        &[("saturation", -14.0), ("contrast", -10.0), ("blacks", 8.0)],
    )
    .curve(Fade),
    e(
        &["washed out", "washed"],
        Tech,
        &[
            ("saturation", -22.0),
            ("contrast", -18.0),
            ("blacks", 14.0),
            ("exposure", 0.2),
        ],
    )
    .curve(Fade),
    e(
        &["punchy", "punch", "pop"],
        Tech,
        &[("contrast", 20.0), ("saturation", 14.0), ("vibrance", 10.0)],
    )
    .curve(SCurve),
    e(
        &["vibrant", "vivid", "colorful", "colourful"],
        Tech,
        &[("vibrance", 28.0), ("saturation", 10.0)],
    ),
    e(&["saturated", "saturation"], Tech, &[("saturation", 20.0)]),
    e(&["vibrance"], Tech, &[("vibrance", 20.0)]),
    e(
        &["muted", "subdued", "understated"],
        Tech,
        &[
            ("saturation", -25.0),
            ("vibrance", -10.0),
            ("contrast", -4.0),
        ],
    ),
    e(
        &["desaturated", "desaturate", "desat"],
        Tech,
        &[("saturation", -35.0)],
    ),
    e(
        &["bright", "brighten", "brightness"],
        Tech,
        &[("exposure", 0.4), ("whites", 5.0)],
    ),
    e(
        &["dark", "darken", "darkness"],
        Tech,
        &[("exposure", -0.5), ("shadows", -10.0)],
    ),
    e(
        &["high key"],
        Tech,
        &[
            ("exposure", 0.5),
            ("contrast", -12.0),
            ("shadows", 20.0),
            ("whites", 10.0),
            ("saturation", -8.0),
            ("highlights", -4.0),
        ],
    ),
    e(
        &["low key"],
        Tech,
        &[
            ("exposure", -0.6),
            ("contrast", 20.0),
            ("shadows", -22.0),
            ("highlights", -6.0),
            ("vignette.amount", -12.0),
        ],
    ),
    e(
        &[
            "soft highlights",
            "recover highlights",
            "tame highlights",
            "protect highlights",
        ],
        Tech,
        &[("highlights", -22.0), ("whites", -8.0)],
    ),
    e(
        &["bright highlights", "hot highlights"],
        Tech,
        &[("highlights", 20.0), ("whites", 10.0)],
    ),
    e(
        &["lift shadows", "open shadows", "brighten shadows"],
        Tech,
        &[("shadows", 22.0)],
    ),
    e(
        &["deep shadows", "dark shadows", "heavy shadows"],
        Tech,
        &[("shadows", -22.0)],
    ),
    e(
        &["overexposed", "overexposure", "blown out"],
        Tech,
        &[
            ("exposure", 0.8),
            ("highlights", 14.0),
            ("whites", 14.0),
            ("saturation", -10.0),
            ("contrast", -8.0),
        ],
    ),
    e(
        &["underexposed", "underexposure"],
        Tech,
        &[("exposure", -0.8), ("shadows", -8.0), ("saturation", -6.0)],
    ),
    e(
        &["airy", "light and airy"],
        Tech,
        &[
            ("exposure", 0.3),
            ("contrast", -12.0),
            ("saturation", -6.0),
            ("highlights", 6.0),
        ],
    ),
    e(
        &["crisp", "crispy"],
        Tech,
        &[
            ("contrast", 14.0),
            ("whites", 8.0),
            ("blacks", -6.0),
            ("vibrance", 6.0),
        ],
    ),
    e(
        &["clean", "neutral"],
        Tech,
        &[
            ("contrast", 6.0),
            ("saturation", -4.0),
            ("vibrance", 6.0),
            ("exposure", 0.1),
        ],
    ),
    e(
        &["hazy", "haze"],
        Tech,
        &[
            ("contrast", -20.0),
            ("blacks", 16.0),
            ("exposure", 0.1),
            ("saturation", -10.0),
            ("highlights", 6.0),
            ("temperature", 4.0),
        ],
    ),
    e(
        &["vignette", "vignetting", "darker edges"],
        Tech,
        &[("vignette.amount", -25.0)],
    ),
    e(
        &["grain", "grainy", "film grain", "noise"],
        Tech,
        &[
            ("grain.amount", 25.0),
            ("grain.size", 0.2),
            ("grain.roughness", 5.0),
        ],
    ),
    e(
        &["rich", "rich colors", "rich colours"],
        Tech,
        &[
            ("saturation", 10.0),
            ("vibrance", 14.0),
            ("contrast", 8.0),
            ("blacks", -6.0),
        ],
    ),
    e(
        &["bleach bypass"],
        Tech,
        &[
            ("saturation", -40.0),
            ("contrast", 30.0),
            ("grain.amount", 10.0),
            ("blacks", -8.0),
        ],
    )
    .curve(SCurve),
    e(
        &["cross process", "cross processed", "cross processing"],
        Tech,
        &[
            ("tint", -12.0),
            ("contrast", 18.0),
            ("saturation", 14.0),
            ("split_tone.shadow_saturation", 18.0),
            ("split_tone.highlight_saturation", 20.0),
        ],
    )
    .sets(&[
        ("split_tone.shadow_hue", 160.0),
        ("split_tone.highlight_hue", 55.0),
    ]),
    e(
        &["brighter mids", "lift mids"],
        Tech,
        &[("gamma.master", 0.03)],
    )
    .curve(Brighten),
    e(
        &["darker mids", "lower mids"],
        Tech,
        &[("gamma.master", -0.03)],
    )
    .curve(Darken),
    // ---- Comparatives ----
    e(&["warmer"], Cmp, &[("temperature", 15.0)]),
    e(&["cooler", "colder"], Cmp, &[("temperature", -15.0)]),
    e(&["brighter"], Cmp, &[("exposure", 0.25)]),
    e(&["darker"], Cmp, &[("exposure", -0.3)]),
    e(&["greener"], Cmp, &[("tint", -12.0)]),
    e(&["bluer"], Cmp, &[("temperature", -14.0), ("lift.b", 0.01)]),
    e(
        &["redder"],
        Cmp,
        &[("gain.r", 0.02), ("hsl.red.saturation", 12.0)],
    ),
    e(
        &["yellower"],
        Cmp,
        &[("temperature", 10.0), ("hsl.yellow.saturation", 8.0)],
    ),
    e(&["pinker"], Cmp, &[("tint", 10.0), ("temperature", 4.0)]),
    e(
        &["softer"],
        Cmp,
        &[("contrast", -12.0), ("highlights", -6.0)],
    )
    .curve(Soft),
    e(
        &["punchier"],
        Cmp,
        &[("contrast", 15.0), ("saturation", 10.0), ("vibrance", 8.0)],
    ),
    e(
        &["moodier"],
        Cmp,
        &[
            ("shadows", -10.0),
            ("exposure", -0.2),
            ("saturation", -8.0),
            ("contrast", 10.0),
            ("vignette.amount", -10.0),
        ],
    ),
    e(
        &["richer"],
        Cmp,
        &[("saturation", 8.0), ("vibrance", 10.0), ("contrast", 6.0)],
    ),
    e(
        &["flatter"],
        Cmp,
        &[("contrast", -14.0), ("saturation", -4.0)],
    ),
    e(&["crisper"], Cmp, &[("contrast", 10.0), ("whites", 6.0)]),
    e(&["lighter"], Cmp, &[("exposure", 0.25), ("shadows", 8.0)]),
    e(
        &["deeper"],
        Cmp,
        &[("contrast", 10.0), ("shadows", -10.0), ("saturation", 6.0)],
    ),
    e(
        &["grittier"],
        Cmp,
        &[
            ("contrast", 15.0),
            ("grain.amount", 18.0),
            ("saturation", -10.0),
        ],
    ),
    e(
        &["dreamier"],
        Cmp,
        &[("contrast", -10.0), ("highlights", 8.0)],
    )
    .curve(Soft),
    e(
        &["bolder"],
        Cmp,
        &[("contrast", 16.0), ("saturation", 12.0)],
    ),
    e(&["paler"], Cmp, &[("saturation", -12.0), ("exposure", 0.1)]),
    e(
        &["sunnier"],
        Cmp,
        &[
            ("temperature", 12.0),
            ("exposure", 0.15),
            ("vibrance", 10.0),
        ],
    ),
    // ---- Film stocks ----
    e(
        &["kodak portra", "portra"],
        Stock,
        &[
            ("temperature", 8.0),
            ("saturation", -8.0),
            ("contrast", -6.0),
            ("tint", 3.0),
            ("shadows", 8.0),
            ("highlights", -6.0),
            ("hsl.orange.luminance", 6.0),
            ("grain.amount", 8.0),
        ],
    ),
    e(
        &[
            "portra 400",
            "kodak portra 400",
            "portra 800",
            "kodak portra 800",
        ],
        Stock,
        &[
            ("temperature", 10.0),
            ("saturation", -6.0),
            ("contrast", -4.0),
            ("tint", 2.0),
            ("shadows", 6.0),
            ("hsl.orange.luminance", 6.0),
            ("grain.amount", 12.0),
        ],
    ),
    e(
        &["portra 160", "kodak portra 160"],
        Stock,
        &[
            ("temperature", 6.0),
            ("saturation", -10.0),
            ("contrast", -8.0),
            ("shadows", 8.0),
            ("highlights", -6.0),
            ("grain.amount", 5.0),
        ],
    ),
    e(
        &["ektar", "kodak ektar", "ektar 100", "kodak ektar 100"],
        Stock,
        &[
            ("saturation", 20.0),
            ("contrast", 14.0),
            ("vibrance", 8.0),
            ("temperature", 4.0),
            ("hsl.red.saturation", 10.0),
            ("grain.amount", 4.0),
        ],
    ),
    e(
        &["kodachrome"],
        Stock,
        &[
            ("saturation", 12.0),
            ("contrast", 16.0),
            ("temperature", 10.0),
            ("hsl.red.saturation", 14.0),
            ("hsl.yellow.saturation", 10.0),
            ("hsl.blue.saturation", 6.0),
            ("blacks", -4.0),
            ("grain.amount", 8.0),
        ],
    ),
    e(
        &[
            "fuji velvia",
            "velvia",
            "velvia 50",
            "fuji velvia 50",
            "velvia 100",
        ],
        Stock,
        &[
            ("saturation", 28.0),
            ("contrast", 18.0),
            ("vibrance", 10.0),
            ("hsl.green.saturation", 10.0),
            ("hsl.blue.saturation", 10.0),
            ("blacks", -6.0),
            ("tint", -2.0),
        ],
    ),
    e(
        &["fuji superia", "superia", "superia 400", "fuji superia 400"],
        Stock,
        &[
            ("tint", -8.0),
            ("temperature", -4.0),
            ("contrast", 8.0),
            ("saturation", 6.0),
            ("grain.amount", 16.0),
            ("split_tone.shadow_saturation", 8.0),
        ],
    )
    .sets(&[("split_tone.shadow_hue", 160.0)]),
    e(
        &["fuji provia", "provia"],
        Stock,
        &[
            ("contrast", 8.0),
            ("saturation", 8.0),
            ("temperature", -3.0),
            ("grain.amount", 4.0),
        ],
    ),
    e(
        &["fuji pro 400h", "pro 400h"],
        Stock,
        &[
            ("tint", -6.0),
            ("temperature", -4.0),
            ("exposure", 0.2),
            ("saturation", -8.0),
            ("contrast", -10.0),
            ("highlights", -4.0),
            ("grain.amount", 8.0),
        ],
    ),
    e(
        &["cinestill 800t", "cinestill", "800t"],
        Stock,
        &[
            ("temperature", -16.0),
            ("tint", -4.0),
            ("grain.amount", 24.0),
            ("contrast", -6.0),
            ("blacks", 6.0),
            ("saturation", -4.0),
            ("split_tone.highlight_saturation", 20.0),
            ("split_tone.shadow_saturation", 18.0),
        ],
    )
    .sets(&[
        ("split_tone.highlight_hue", 5.0),
        ("split_tone.shadow_hue", 195.0),
    ]),
    e(
        &["kodak vision3", "vision3", "500t"],
        Stock,
        &[
            ("temperature", -10.0),
            ("contrast", -6.0),
            ("grain.amount", 14.0),
            ("saturation", -4.0),
        ],
    )
    .curve(Soft),
    e(
        &["ilford hp5", "hp5", "ilford", "hp5 plus", "ilford hp5 plus"],
        Stock,
        &[
            ("saturation", -100.0),
            ("contrast", 14.0),
            ("grain.amount", 30.0),
            ("grain.roughness", 10.0),
        ],
    ),
    e(
        &[
            "tri x",
            "trix",
            "kodak tri x",
            "tri x 400",
            "kodak tri x 400",
            "trix 400",
        ],
        Stock,
        &[
            ("saturation", -100.0),
            ("contrast", 24.0),
            ("grain.amount", 38.0),
            ("grain.roughness", 15.0),
            ("blacks", -6.0),
        ],
    ),
    e(
        &["kodak gold", "gold 200"],
        Stock,
        &[
            ("temperature", 16.0),
            ("saturation", 10.0),
            ("contrast", 4.0),
            ("grain.amount", 14.0),
            ("tint", 3.0),
        ],
    ),
    e(
        &["polaroid", "instant film"],
        Stock,
        &[
            ("temperature", 10.0),
            ("contrast", -14.0),
            ("blacks", 16.0),
            ("saturation", -10.0),
            ("vignette.amount", -14.0),
            ("grain.amount", 8.0),
        ],
    )
    .curve(Fade),
    e(
        &["disposable camera", "disposable", "point and shoot"],
        Stock,
        &[
            ("contrast", 12.0),
            ("saturation", 10.0),
            ("vignette.amount", -22.0),
            ("grain.amount", 28.0),
            ("temperature", 8.0),
            ("highlights", 10.0),
        ],
    ),
    e(
        &["lomo", "lomography"],
        Stock,
        &[
            ("saturation", 25.0),
            ("contrast", 25.0),
            ("vignette.amount", -40.0),
            ("grain.amount", 10.0),
        ],
    ),
    // ---- Genres, eras and references ----
    e(
        &["noir", "film noir"],
        Genre,
        &[
            ("saturation", -85.0),
            ("contrast", 35.0),
            ("exposure", -0.3),
            ("shadows", -20.0),
            ("highlights", 4.0),
            ("vignette.amount", -25.0),
            ("grain.amount", 18.0),
        ],
    )
    .curve(SCurve),
    e(
        &["cinematic", "movie look", "filmic"],
        Genre,
        &[
            ("contrast", 12.0),
            ("saturation", -6.0),
            ("vignette.amount", -8.0),
            ("exposure", -0.1),
            ("split_tone.shadow_saturation", 14.0),
            ("split_tone.highlight_saturation", 10.0),
        ],
    )
    .sets(&[
        ("split_tone.shadow_hue", 200.0),
        ("split_tone.highlight_hue", 35.0),
    ])
    .curve(SCurve),
    e(
        &["blockbuster", "hollywood"],
        Genre,
        &[
            ("contrast", 20.0),
            ("saturation", 8.0),
            ("blacks", -6.0),
            ("whites", 4.0),
            ("split_tone.shadow_saturation", 20.0),
            ("split_tone.highlight_saturation", 16.0),
        ],
    )
    .sets(&[
        ("split_tone.shadow_hue", 195.0),
        ("split_tone.highlight_hue", 30.0),
    ]),
    e(
        &["western", "spaghetti western"],
        Genre,
        &[
            ("temperature", 25.0),
            ("saturation", -10.0),
            ("contrast", 12.0),
            ("hsl.orange.saturation", 12.0),
            ("grain.amount", 14.0),
            ("vignette.amount", -12.0),
            ("highlights", -8.0),
            ("tint", 4.0),
        ],
    ),
    e(
        &["sci fi", "scifi", "science fiction"],
        Genre,
        &[
            ("temperature", -18.0),
            ("saturation", -8.0),
            ("contrast", 14.0),
            ("hsl.aqua.saturation", 12.0),
            ("split_tone.shadow_saturation", 20.0),
            ("split_tone.highlight_saturation", 10.0),
        ],
    )
    .sets(&[
        ("split_tone.shadow_hue", 200.0),
        ("split_tone.highlight_hue", 190.0),
    ]),
    e(
        &["horror", "scary"],
        Genre,
        &[
            ("saturation", -30.0),
            ("tint", -14.0),
            ("temperature", -8.0),
            ("contrast", 20.0),
            ("exposure", -0.5),
            ("vignette.amount", -30.0),
            ("grain.amount", 20.0),
            ("shadows", -14.0),
        ],
    ),
    e(
        &["vintage", "antique"],
        Genre,
        &[
            ("temperature", 12.0),
            ("saturation", -12.0),
            ("grain.amount", 18.0),
            ("vignette.amount", -12.0),
            ("contrast", -6.0),
            ("tint", 4.0),
        ],
    )
    .curve(Fade),
    e(
        &["70s", "1970s", "seventies"],
        Genre,
        &[
            ("temperature", 22.0),
            ("saturation", -8.0),
            ("hsl.orange.saturation", 12.0),
            ("contrast", -6.0),
            ("grain.amount", 22.0),
            ("tint", 5.0),
            ("vignette.amount", -10.0),
        ],
    )
    .curve(Fade),
    e(
        &["80s", "1980s", "eighties"],
        Genre,
        &[
            ("saturation", 18.0),
            ("contrast", 10.0),
            ("tint", 6.0),
            ("vibrance", 10.0),
            ("grain.amount", 6.0),
            ("split_tone.shadow_saturation", 20.0),
            ("split_tone.highlight_saturation", 12.0),
        ],
    )
    .sets(&[
        ("split_tone.shadow_hue", 290.0),
        ("split_tone.highlight_hue", 180.0),
    ]),
    e(
        &["90s", "1990s", "nineties"],
        Genre,
        &[
            ("saturation", -6.0),
            ("contrast", 6.0),
            ("temperature", 4.0),
            ("grain.amount", 16.0),
            ("tint", -3.0),
            ("blacks", 4.0),
        ],
    ),
    e(
        &["vhs", "camcorder", "tape"],
        Genre,
        &[
            ("saturation", -18.0),
            ("contrast", -10.0),
            ("blacks", 16.0),
            ("temperature", -4.0),
            ("grain.amount", 30.0),
            ("grain.roughness", 30.0),
            ("split_tone.shadow_saturation", 12.0),
            ("split_tone.highlight_saturation", 8.0),
        ],
    )
    .sets(&[
        ("split_tone.shadow_hue", 270.0),
        ("split_tone.highlight_hue", 180.0),
    ])
    .curve(Fade),
    e(
        &["wes anderson", "grand budapest"],
        Genre,
        &[
            ("saturation", -4.0),
            ("vibrance", 18.0),
            ("temperature", 14.0),
            ("tint", 12.0),
            ("contrast", -8.0),
            ("exposure", 0.2),
            ("hsl.yellow.saturation", 12.0),
        ],
    )
    .curve(Soft),
    e(
        &["blade runner", "blade runner 2049", "bladerunner"],
        Genre,
        &[
            ("temperature", 20.0),
            ("tint", 4.0),
            ("contrast", 16.0),
            ("saturation", -6.0),
            ("exposure", -0.4),
            ("vignette.amount", -18.0),
            ("hsl.orange.saturation", 16.0),
            ("grain.amount", 10.0),
            ("split_tone.shadow_saturation", 22.0),
            ("split_tone.highlight_saturation", 22.0),
        ],
    )
    .sets(&[
        ("split_tone.shadow_hue", 190.0),
        ("split_tone.highlight_hue", 25.0),
    ]),
    e(
        &["matrix", "the matrix"],
        Genre,
        &[
            ("tint", -26.0),
            ("temperature", -6.0),
            ("saturation", -12.0),
            ("contrast", 18.0),
            ("exposure", -0.2),
            ("split_tone.shadow_saturation", 20.0),
            ("split_tone.highlight_saturation", 10.0),
        ],
    )
    .sets(&[
        ("split_tone.shadow_hue", 140.0),
        ("split_tone.highlight_hue", 120.0),
    ]),
    e(
        &["se7en", "seven"],
        Genre,
        &[
            ("saturation", -35.0),
            ("temperature", 12.0),
            ("contrast", 28.0),
            ("exposure", -0.5),
            ("shadows", -18.0),
            ("grain.amount", 24.0),
            ("highlights", -10.0),
        ],
    )
    .curve(Crush),
    e(
        &["mad max", "fury road"],
        Genre,
        &[
            ("temperature", 30.0),
            ("saturation", 14.0),
            ("contrast", 20.0),
            ("exposure", 0.1),
            ("hsl.orange.saturation", 20.0),
            ("split_tone.shadow_saturation", 24.0),
            ("split_tone.highlight_saturation", 22.0),
        ],
    )
    .sets(&[
        ("split_tone.shadow_hue", 195.0),
        ("split_tone.highlight_hue", 28.0),
    ]),
    e(
        &["amelie"],
        Genre,
        &[
            ("temperature", 24.0),
            ("tint", -10.0),
            ("saturation", 10.0),
            ("hsl.green.saturation", 16.0),
            ("hsl.red.saturation", 18.0),
            ("contrast", 6.0),
            ("vignette.amount", -12.0),
            ("vibrance", 8.0),
            ("split_tone.highlight_saturation", 18.0),
        ],
    )
    .sets(&[("split_tone.highlight_hue", 50.0)]),
    e(
        &["deakins", "roger deakins"],
        Genre,
        &[
            ("contrast", 8.0),
            ("saturation", -6.0),
            ("highlights", -10.0),
            ("temperature", 4.0),
            ("shadows", -6.0),
            ("vignette.amount", -6.0),
        ],
    )
    .curve(SCurve),
    e(
        &["wong kar wai", "in the mood for love", "chungking express"],
        Genre,
        &[
            ("saturation", 14.0),
            ("tint", 6.0),
            ("contrast", 12.0),
            ("grain.amount", 20.0),
            ("blacks", -4.0),
            ("hsl.red.saturation", 16.0),
            ("exposure", -0.2),
            ("split_tone.shadow_saturation", 24.0),
            ("split_tone.highlight_saturation", 18.0),
        ],
    )
    .sets(&[
        ("split_tone.shadow_hue", 160.0),
        ("split_tone.highlight_hue", 20.0),
    ]),
    e(
        &["thriller"],
        Genre,
        &[
            ("contrast", 14.0),
            ("saturation", -14.0),
            ("temperature", -8.0),
            ("tint", -6.0),
            ("shadows", -10.0),
            ("vignette.amount", -12.0),
        ],
    ),
    e(
        &["documentary", "docu"],
        Genre,
        &[
            ("contrast", 2.0),
            ("saturation", -8.0),
            ("vibrance", 4.0),
            ("highlights", -4.0),
        ],
    ),
    e(
        &["fantasy", "fairytale", "fairy tale"],
        Genre,
        &[
            ("vibrance", 16.0),
            ("saturation", 6.0),
            ("contrast", -6.0),
            ("highlights", 8.0),
            ("temperature", 6.0),
            ("tint", 4.0),
        ],
    )
    .curve(Soft),
    e(
        &["retro"],
        Genre,
        &[
            ("temperature", 10.0),
            ("saturation", 4.0),
            ("contrast", -4.0),
            ("grain.amount", 12.0),
            ("tint", 4.0),
        ],
    )
    .curve(Fade),
    e(
        &["cyberpunk", "cyber punk"],
        Genre,
        &[
            ("saturation", 18.0),
            ("contrast", 22.0),
            ("exposure", -0.4),
            ("vibrance", 12.0),
            ("tint", 8.0),
            ("vignette.amount", -12.0),
            ("blacks", -8.0),
            ("hsl.magenta.saturation", 14.0),
            ("hsl.aqua.saturation", 14.0),
            ("split_tone.shadow_saturation", 30.0),
            ("split_tone.highlight_saturation", 24.0),
        ],
    )
    .sets(&[
        ("split_tone.shadow_hue", 280.0),
        ("split_tone.highlight_hue", 185.0),
    ]),
    e(
        &["vaporwave", "synthwave", "outrun"],
        Genre,
        &[
            ("saturation", 10.0),
            ("vibrance", 20.0),
            ("tint", 18.0),
            ("contrast", -8.0),
            ("temperature", -6.0),
            ("split_tone.shadow_saturation", 28.0),
            ("split_tone.highlight_saturation", 22.0),
        ],
    )
    .sets(&[
        ("split_tone.shadow_hue", 270.0),
        ("split_tone.highlight_hue", 320.0),
    ])
    .curve(Fade),
    e(
        &["indie", "indie film"],
        Genre,
        &[
            ("saturation", -8.0),
            ("temperature", 10.0),
            ("contrast", -6.0),
            ("grain.amount", 14.0),
            ("highlights", -6.0),
            ("vignette.amount", -6.0),
        ],
    )
    .curve(Fade),
    e(
        &["dystopian", "dystopia"],
        Genre,
        &[
            ("saturation", -35.0),
            ("temperature", -8.0),
            ("tint", -8.0),
            ("contrast", 18.0),
            ("exposure", -0.4),
            ("vignette.amount", -22.0),
            ("grain.amount", 16.0),
        ],
    ),
    e(
        &["post apocalyptic", "apocalyptic", "wasteland"],
        Genre,
        &[
            ("saturation", -28.0),
            ("temperature", 16.0),
            ("tint", -6.0),
            ("contrast", 18.0),
            ("exposure", -0.2),
            ("grain.amount", 24.0),
            ("vignette.amount", -22.0),
        ],
    ),
    e(
        &["analog", "35mm", "film look"],
        Genre,
        &[
            ("contrast", 6.0),
            ("saturation", -6.0),
            ("grain.amount", 16.0),
            ("blacks", 6.0),
            ("highlights", -6.0),
            ("vibrance", 4.0),
        ],
    )
    .curve(SCurve),
    e(
        &["super 8", "super8", "8mm"],
        Genre,
        &[
            ("grain.amount", 40.0),
            ("grain.roughness", 20.0),
            ("saturation", -8.0),
            ("temperature", 14.0),
            ("blacks", 14.0),
            ("contrast", -12.0),
            ("vignette.amount", -20.0),
        ],
    )
    .curve(Fade),
    e(
        &["16mm"],
        Genre,
        &[
            ("grain.amount", 28.0),
            ("temperature", 6.0),
            ("saturation", -6.0),
            ("contrast", 4.0),
            ("blacks", 6.0),
        ],
    ),
    e(
        &["dune"],
        Genre,
        &[
            ("saturation", -28.0),
            ("temperature", 24.0),
            ("contrast", 8.0),
            ("exposure", 0.1),
            ("highlights", -6.0),
            ("grain.amount", 6.0),
        ],
    ),
    e(
        &["la la land", "lalaland"],
        Genre,
        &[
            ("vibrance", 20.0),
            ("saturation", 12.0),
            ("tint", 6.0),
            ("split_tone.shadow_saturation", 18.0),
            ("split_tone.highlight_saturation", 14.0),
        ],
    )
    .sets(&[
        ("split_tone.shadow_hue", 270.0),
        ("split_tone.highlight_hue", 35.0),
    ]),
    e(
        &["ghibli", "studio ghibli", "anime"],
        Genre,
        &[
            ("vibrance", 20.0),
            ("saturation", 8.0),
            ("exposure", 0.2),
            ("contrast", -8.0),
            ("temperature", 4.0),
            ("hsl.green.saturation", 16.0),
            ("hsl.aqua.saturation", 14.0),
            ("highlights", 4.0),
        ],
    )
    .curve(Soft),
    e(
        &["travel", "travel photography"],
        Genre,
        &[
            ("vibrance", 16.0),
            ("contrast", 8.0),
            ("saturation", 6.0),
            ("exposure", 0.1),
        ],
    ),
    e(
        &["wedding"],
        Genre,
        &[
            ("exposure", 0.3),
            ("contrast", -10.0),
            ("temperature", 6.0),
            ("saturation", -8.0),
            ("highlights", -6.0),
            ("tint", 4.0),
        ],
    ),
    e(
        &["fashion", "editorial"],
        Genre,
        &[
            ("contrast", 10.0),
            ("saturation", -10.0),
            ("highlights", -6.0),
        ],
    )
    .curve(SCurve),
    e(
        &["street", "street photography"],
        Genre,
        &[
            ("contrast", 14.0),
            ("saturation", -10.0),
            ("grain.amount", 14.0),
            ("shadows", -8.0),
        ],
    ),
    e(
        &["food", "food photography"],
        Genre,
        &[
            ("temperature", 10.0),
            ("saturation", 12.0),
            ("vibrance", 10.0),
            ("contrast", 8.0),
            ("exposure", 0.15),
        ],
    ),
    // ---- Nature and scene ----
    e(
        &["forest", "woods", "woodland"],
        Scene,
        &[
            ("hsl.green.saturation", 16.0),
            ("hsl.green.hue", -8.0),
            ("temperature", -4.0),
            ("contrast", 6.0),
            ("shadows", -6.0),
            ("tint", -4.0),
            ("exposure", -0.15),
            ("vibrance", 6.0),
        ],
    ),
    e(
        &["beach", "seaside", "coast"],
        Scene,
        &[
            ("exposure", 0.25),
            ("vibrance", 14.0),
            ("hsl.aqua.saturation", 20.0),
            ("hsl.blue.saturation", 12.0),
            ("temperature", 6.0),
            ("contrast", -4.0),
            ("highlights", -6.0),
        ],
    ),
    e(
        &["snow", "snowy"],
        Scene,
        &[
            ("exposure", 0.4),
            ("temperature", -14.0),
            ("saturation", -10.0),
            ("contrast", -6.0),
            ("highlights", -10.0),
            ("whites", 4.0),
            ("split_tone.shadow_saturation", 12.0),
        ],
    )
    .sets(&[("split_tone.shadow_hue", 220.0)]),
    e(
        &["desert", "arid"],
        Scene,
        &[
            ("temperature", 26.0),
            ("saturation", -6.0),
            ("hsl.orange.saturation", 16.0),
            ("hsl.yellow.saturation", 10.0),
            ("contrast", 6.0),
            ("exposure", 0.2),
            ("highlights", -8.0),
        ],
    ),
    e(
        &["night", "nighttime", "night time"],
        Scene,
        &[
            ("exposure", -0.6),
            ("temperature", -16.0),
            ("contrast", 10.0),
            ("shadows", -8.0),
            ("saturation", -4.0),
            ("vignette.amount", -10.0),
            ("highlights", -4.0),
            ("vibrance", 5.0),
            ("split_tone.shadow_saturation", 16.0),
        ],
    )
    .sets(&[("split_tone.shadow_hue", 225.0)]),
    e(
        &["autumn", "fall", "autumnal"],
        Scene,
        &[
            ("temperature", 18.0),
            ("hsl.orange.saturation", 22.0),
            ("hsl.yellow.saturation", 14.0),
            ("hsl.red.saturation", 12.0),
            ("hsl.green.saturation", -12.0),
            ("hsl.green.hue", -12.0),
            ("vibrance", 8.0),
            ("contrast", 4.0),
        ],
    ),
    e(
        &["spring", "springtime", "blossom"],
        Scene,
        &[
            ("vibrance", 18.0),
            ("hsl.green.saturation", 14.0),
            ("hsl.green.hue", 4.0),
            ("exposure", 0.2),
            ("temperature", 2.0),
            ("contrast", -4.0),
            ("hsl.magenta.saturation", 8.0),
            ("saturation", 4.0),
        ],
    ),
    e(
        &["summer", "summertime", "sunny day"],
        Scene,
        &[
            ("temperature", 14.0),
            ("vibrance", 16.0),
            ("exposure", 0.2),
            ("contrast", 6.0),
            ("hsl.blue.saturation", 10.0),
            ("hsl.yellow.saturation", 8.0),
        ],
    ),
    e(
        &["winter", "wintry"],
        Scene,
        &[
            ("temperature", -18.0),
            ("saturation", -14.0),
            ("contrast", 4.0),
            ("exposure", 0.1),
            ("split_tone.shadow_saturation", 10.0),
        ],
    )
    .sets(&[("split_tone.shadow_hue", 220.0)]),
    e(
        &["ocean", "sea", "underwater"],
        Scene,
        &[
            ("hsl.aqua.saturation", 18.0),
            ("hsl.blue.saturation", 16.0),
            ("hsl.blue.luminance", -8.0),
            ("temperature", -8.0),
            ("contrast", 6.0),
            ("vibrance", 8.0),
        ],
    ),
    e(
        &["mountains", "mountain", "alpine"],
        Scene,
        &[
            ("contrast", 14.0),
            ("saturation", -6.0),
            ("temperature", -6.0),
            ("highlights", -8.0),
            ("hsl.blue.saturation", 8.0),
            ("whites", 4.0),
            ("vibrance", 6.0),
        ],
    ),
    e(
        &["city", "urban", "downtown"],
        Scene,
        &[
            ("contrast", 12.0),
            ("saturation", -8.0),
            ("temperature", -4.0),
            ("shadows", -6.0),
            ("grain.amount", 6.0),
        ],
    ),
    e(
        &["rain", "rainy", "raining"],
        Scene,
        &[
            ("saturation", -16.0),
            ("temperature", -10.0),
            ("contrast", -4.0),
            ("exposure", -0.3),
            ("tint", -3.0),
            ("highlights", -8.0),
        ],
    ),
    e(
        &["fog", "foggy", "misty", "mist"],
        Scene,
        &[
            ("contrast", -24.0),
            ("blacks", 20.0),
            ("exposure", 0.2),
            ("saturation", -16.0),
            ("highlights", -4.0),
            ("temperature", -4.0),
        ],
    )
    .curve(Fade),
    e(
        &["overcast", "cloudy", "grey sky", "gray sky"],
        Scene,
        &[
            ("saturation", -10.0),
            ("contrast", -8.0),
            ("temperature", -6.0),
            ("highlights", -6.0),
            ("exposure", -0.1),
        ],
    ),
    e(
        &["sunny", "sunshine", "sunlit"],
        Scene,
        &[
            ("exposure", 0.2),
            ("temperature", 10.0),
            ("vibrance", 14.0),
            ("contrast", 8.0),
            ("saturation", 6.0),
        ],
    ),
    e(
        &["blue sky", "clear sky"],
        Scene,
        &[
            ("hsl.blue.saturation", 20.0),
            ("hsl.aqua.saturation", 10.0),
            ("hsl.blue.luminance", -8.0),
            ("contrast", 4.0),
        ],
    ),
    e(
        &["neon city", "neon night", "night city"],
        Scene,
        &[
            ("exposure", -0.5),
            ("saturation", 14.0),
            ("contrast", 20.0),
            ("vibrance", 14.0),
            ("blacks", -8.0),
            ("hsl.magenta.saturation", 14.0),
            ("hsl.aqua.saturation", 14.0),
            ("split_tone.shadow_saturation", 22.0),
            ("split_tone.highlight_saturation", 18.0),
        ],
    )
    .sets(&[
        ("split_tone.shadow_hue", 270.0),
        ("split_tone.highlight_hue", 185.0),
    ]),
];

/// Every phrase in the lexicon (all aliases of all entries), in table order,
/// for UI autocomplete and documentation.
pub fn lexicon_terms() -> Vec<&'static str> {
    ENTRIES
        .iter()
        .flat_map(|e| e.phrases.iter().copied())
        .collect()
}

/// Prompt modifier kinds. See [`MODIFIERS`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Modifier {
    /// Multiply the next term's strength by the factor.
    Scale(f32),
    /// Negate the next term's direction.
    Negate,
    /// Reset the next term's parameters to their defaults.
    Remove,
}

const DOWN: Modifier = Modifier::Scale(0.5);
const UP: Modifier = Modifier::Scale(1.6);
const SAME: Modifier = Modifier::Scale(1.0);

/// Modifier phrases and their effect on the next matched term.
pub(crate) static MODIFIERS: &[(&str, Modifier)] = &[
    ("slightly", DOWN),
    ("slight", DOWN),
    ("a bit", DOWN),
    ("just a bit", DOWN),
    ("little bit", DOWN),
    ("a little", DOWN),
    ("a little bit", DOWN),
    ("a tad", DOWN),
    ("bit", DOWN),
    ("little", DOWN),
    ("subtle", DOWN),
    ("subtly", DOWN),
    ("touch of", DOWN),
    ("hint of", DOWN),
    ("very", UP),
    ("really", UP),
    ("extremely", UP),
    ("super", UP),
    ("heavy", UP),
    ("heavily", UP),
    ("strong", UP),
    ("strongly", UP),
    ("lots of", UP),
    ("a lot of", UP),
    ("more", UP),
    ("increase", SAME),
    ("add", SAME),
    ("boost", SAME),
    ("raise", SAME),
    ("less", Modifier::Negate),
    ("not", Modifier::Negate),
    ("dont", Modifier::Negate),
    ("too", Modifier::Negate),
    ("reduce", Modifier::Negate),
    ("decrease", Modifier::Negate),
    ("lower", Modifier::Negate),
    ("fewer", Modifier::Negate),
    ("no", Modifier::Remove),
    ("without", Modifier::Remove),
    ("remove", Modifier::Remove),
];

/// Words that signal an iterative ("do it again, differently") prompt.
pub(crate) static ITERATIVE_MARKERS: &[&str] = &[
    "more", "less", "bit", "slightly", "little", "too", "increase", "decrease", "reduce", "add",
    "remove", "fewer", "lower", "raise", "boost", "tad",
];

/// Comparative entries (each is also a lexicon term); any one in a prompt
/// signals an iterative prompt.
pub(crate) static COMPARATIVES: &[&str] = &[
    "warmer", "cooler", "colder", "brighter", "darker", "greener", "bluer", "redder", "yellower",
    "pinker", "softer", "punchier", "moodier", "richer", "flatter", "crisper", "lighter", "deeper",
    "grittier", "dreamier", "bolder", "paler", "sunnier",
];

/// Filler words ignored when reporting unmatched words.
pub(crate) static STOPWORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "with", "of", "to", "it", "its", "this", "that", "is", "are",
    "be", "some", "in", "on", "for", "make", "making", "give", "me", "my", "i", "want", "like",
    "look", "looks", "looking", "feel", "feeling", "please", "can", "you", "grade", "color",
    "colour", "colors", "colours", "tone", "tones", "style", "vibe", "vibes", "overall", "image",
    "photo", "picture", "shot", "footage", "video", "clip", "film", "stock", "but", "just", "so",
    "then", "also", "bit", "little",
];

/// What a normalized phrase resolves to.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Hit {
    /// Index into [`ENTRIES`].
    Entry(usize),
    /// A modifier.
    Mod(Modifier),
}

/// Phrase index with the longest phrase length in words.
pub(crate) struct Index {
    /// Normalized phrase to hit.
    pub map: HashMap<&'static str, Hit>,
    /// Longest phrase, in words.
    pub max_words: usize,
}

/// The lazily built phrase index.
pub(crate) fn index() -> &'static Index {
    static INDEX: OnceLock<Index> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut map = HashMap::new();
        let mut max_words = 1;
        for (i, e) in ENTRIES.iter().enumerate() {
            for p in e.phrases {
                max_words = max_words.max(p.split(' ').count());
                map.insert(*p, Hit::Entry(i));
            }
        }
        for (p, m) in MODIFIERS {
            max_words = max_words.max(p.split(' ').count());
            map.entry(*p).or_insert(Hit::Mod(*m));
        }
        Index { map, max_words }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use apricity_core::param_spec::scalar_spec;
    use std::collections::HashSet;

    #[test]
    fn at_least_120_entries() {
        assert!(ENTRIES.len() >= 120, "{}", ENTRIES.len());
    }

    #[test]
    fn phrases_are_unique_and_normalized() {
        let mut seen = HashSet::new();
        for p in lexicon_terms() {
            assert!(seen.insert(p), "duplicate phrase {p}");
            assert_eq!(p, p.to_lowercase());
            assert!(
                p.chars().all(|c| c.is_ascii_alphanumeric() || c == ' '),
                "{p}"
            );
            assert!(!p.starts_with(' ') && !p.ends_with(' ') && !p.contains("  "));
        }
    }

    #[test]
    fn no_phrase_collides_with_a_modifier() {
        for (m, _) in MODIFIERS {
            assert!(!lexicon_terms().contains(m), "{m}");
        }
    }

    #[test]
    fn entry_paths_resolve_to_nga_scalars() {
        for e in ENTRIES {
            for (short, d) in e.deltas.iter().chain(e.sets.iter()) {
                let spec = scalar_spec(&Entry::full_path(short))
                    .unwrap_or_else(|| panic!("unknown path {short}"));
                assert!(spec.nga, "{short} is not NGA-emittable");
                assert!(d.is_finite());
            }
        }
    }

    #[test]
    fn comparatives_are_lexicon_terms() {
        let t = lexicon_terms();
        for c in COMPARATIVES {
            assert!(t.contains(c), "{c}");
        }
    }

    #[test]
    fn curve_presets_stay_in_unit_range() {
        for p in [SCurve, Soft, Fade, Crush, Brighten, Darken] {
            for s in [-1.6, 1.0, 1.6] {
                for pt in p.points(s) {
                    assert!((0.0..=1.0).contains(&pt.y));
                }
            }
        }
    }
}
