//! [`GradeParameters`] — the single source of truth for NGA output, Recipe
//! file format, and Manual Console controls.
//!
//! M0 stub: only the v0 fields (exposure, temperature, tint). M1 expands to
//! the full primary set; M5 freezes the schema before NGA training.
//!
//! Every field documents its **unit** and **color space**. Do not add a field
//! without that documentation.

use serde::{Deserialize, Serialize};

/// All Stage-2 creative-grade parameters that the NGA can emit, the Manual
/// Console can edit, and a Recipe can persist.
///
/// Schema is versioned via `serde` defaults so older Recipes load forward.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GradeParameters {
    /// Exposure adjustment in **stops** (log2 multiplier). Applied in
    /// scene-linear Rec.2020. Range: roughly [-5.0, +5.0].
    pub exposure: f32,

    /// White-balance temperature shift in **mireds** (Δ = 1e6 / Kelvin).
    /// Positive = warmer. Applied in scene-linear Rec.2020.
    pub temperature: f32,

    /// Green/magenta tint in **arbitrary units** matched to the temperature
    /// scale. Positive = magenta. Applied in scene-linear Rec.2020.
    pub tint: f32,
}

impl Default for GradeParameters {
    fn default() -> Self {
        Self { exposure: 0.0, temperature: 0.0, tint: 0.0 }
    }
}

impl GradeParameters {
    /// Identity grade — no adjustment.
    pub const IDENTITY: Self = Self { exposure: 0.0, temperature: 0.0, tint: 0.0 };
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
}
