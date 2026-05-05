//! Color space identifiers and primaries.
//!
//! The pipeline interior is always [`ColorSpace::LinearRec2020`]. Input and
//! output transforms convert at the seams.

use serde::{Deserialize, Serialize};

/// A named color space. Variants identify both the primaries and the transfer
/// function; "Linear" prefix means a linear transfer function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColorSpace {
    /// sRGB primaries, sRGB (≈2.2) transfer.
    Srgb,
    /// Rec.709 primaries, BT.1886 transfer.
    Rec709,
    /// Rec.2020 primaries, BT.2020 transfer.
    Rec2020,
    /// Rec.2020 primaries, linear transfer. The pipeline interior space.
    LinearRec2020,
    /// sRGB primaries, linear transfer.
    LinearSrgb,
}
