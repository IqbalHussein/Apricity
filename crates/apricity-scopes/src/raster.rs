use serde::Serialize;

/// An RGBA8 raster produced by a scope.
///
/// Pixel values are display-referred sRGB-encoded 8-bit, **non-premultiplied**
/// alpha. Row-major, top row first; `rgba.len() == width * height * 4`.
/// Alpha is 0 where no pixel landed.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScopeImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// RGBA8 bytes, 4 per pixel.
    pub rgba: Vec<u8>,
}

impl ScopeImage {
    pub(crate) fn empty(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            rgba: vec![0; width as usize * height as usize * 4],
        }
    }
}

/// Reference hit count for intensity normalisation: the 99th percentile of
/// the nonzero bins, so a few saturated bins cannot crush the rest of the
/// scope. `None` when the scope is empty.
pub(crate) fn reference_count(counts: &[u32]) -> Option<f32> {
    let mut nz: Vec<u32> = counts.iter().copied().filter(|&c| c > 0).collect();
    if nz.is_empty() {
        return None;
    }
    let idx = (nz.len() - 1) * 99 / 100;
    let (_, v, _) = nz.select_nth_unstable(idx);
    Some(*v as f32)
}

/// Alpha lookup for hit counts: log curve normalised by a reference count.
/// Small counts (the overwhelmingly common case) are tabulated so the
/// per-bin cost is a table read, not two `ln` calls.
pub(crate) struct AlphaMap {
    table: Vec<u8>,
    reference: f32,
    denom: f32,
}

impl AlphaMap {
    const TABLE_LEN: usize = 1024;

    pub(crate) fn new(reference: f32) -> Self {
        let denom = (1.0 + reference).ln();
        let mut m = Self {
            table: Vec::new(),
            reference,
            denom,
        };
        m.table = (0..Self::TABLE_LEN as u32).map(|c| m.compute(c)).collect();
        m
    }

    fn compute(&self, count: u32) -> u8 {
        let i = ((1.0 + count as f32).ln() / self.denom).min(1.0);
        (255.0 * (0.2 + 0.8 * i)).round() as u8
    }

    /// Alpha for a nonzero `count`; floors at 0.2 so single hits stay visible.
    #[inline]
    pub(crate) fn alpha(&self, count: u32) -> u8 {
        match self.table.get(count as usize) {
            Some(&a) => a,
            None if count as f32 >= self.reference => 255,
            None => self.compute(count),
        }
    }
}
