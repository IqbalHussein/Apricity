//! `.cube` 3D LUT parsing, writing, and sampling.
//!
//! ## Color-space contract
//!
//! Apricity treats every `.cube` LUT as mapping **sRGB-encoded Rec.709 in
//! [0, 1]** to **sRGB-encoded Rec.709**, which is the de-facto convention for
//! creative LUTs exchanged with Resolve / Premiere / Final Cut. The render
//! pipeline converts its working signal to that domain before sampling and
//! back afterwards. LUTs with a non-[0, 1] `DOMAIN_MIN`/`DOMAIN_MAX` are
//! rescaled on load.
//!
//! Data order follows the Adobe/Resolve `.cube` spec: **red varies fastest**,
//! then green, then blue.
//!
//! [`Lut3d::sample_trilinear`] is mirrored exactly in the render crate's
//! `stage2.wgsl`, so CPU and GPU LUT application agree.

#![forbid(unsafe_code)]

use std::fmt::Write as _;

/// Default bake resolution (PRD §6.5: 33-point).
pub const DEFAULT_SIZE: usize = 33;
/// Largest LUT accepted on import.
pub const MAX_SIZE: usize = 129;

/// Errors from [`parse_cube`].
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum LutError {
    /// No `LUT_3D_SIZE` (or a 1D-only file).
    #[error("missing LUT_3D_SIZE (1D LUTs are not supported as creative LUTs)")]
    MissingSize,
    /// Size outside 2..=129.
    #[error("unsupported LUT size {0} (expected 2..=129)")]
    BadSize(usize),
    /// Wrong number of data rows.
    #[error("expected {expected} data rows, found {found}")]
    WrongRowCount {
        /// Rows required by the size.
        expected: usize,
        /// Rows present.
        found: usize,
    },
    /// Unparseable line.
    #[error("line {line}: {message}")]
    Syntax {
        /// 1-based line number.
        line: usize,
        /// Description.
        message: String,
    },
}

/// A cubic 3D LUT with `size³` RGB entries, red fastest.
#[derive(Debug, Clone, PartialEq)]
pub struct Lut3d {
    /// Optional title from the `TITLE` keyword.
    pub title: Option<String>,
    /// Points per axis.
    pub size: usize,
    /// `size³` output triples, **sRGB-encoded Rec.709**, red fastest.
    pub data: Vec<[f32; 3]>,
}

impl Lut3d {
    /// The identity LUT of the given size.
    pub fn identity(size: usize) -> Self {
        Self::from_fn(size, |rgb| rgb)
    }

    /// Build a LUT by evaluating `f` at every lattice point. Input and
    /// output are sRGB-encoded Rec.709 in [0, 1].
    pub fn from_fn(size: usize, mut f: impl FnMut([f32; 3]) -> [f32; 3]) -> Self {
        let n = size.max(2);
        let mut data = Vec::with_capacity(n * n * n);
        let scale = 1.0 / (n - 1) as f32;
        for b in 0..n {
            for g in 0..n {
                for r in 0..n {
                    data.push(f([r as f32 * scale, g as f32 * scale, b as f32 * scale]));
                }
            }
        }
        Self {
            title: None,
            size: n,
            data,
        }
    }

    /// Entry at lattice coordinate (r, g, b).
    #[inline]
    pub fn at(&self, r: usize, g: usize, b: usize) -> [f32; 3] {
        self.data[r + g * self.size + b * self.size * self.size]
    }

    /// Trilinear sample at an **sRGB-encoded Rec.709** input, clamped to
    /// [0, 1]. Mirrored in `stage2.wgsl`.
    pub fn sample_trilinear(&self, rgb: [f32; 3]) -> [f32; 3] {
        let n1 = (self.size - 1) as f32;
        let mut i0 = [0usize; 3];
        let mut i1 = [0usize; 3];
        let mut f = [0f32; 3];
        for c in 0..3 {
            let x = rgb[c].clamp(0.0, 1.0) * n1;
            let fl = x.floor();
            i0[c] = fl as usize;
            i1[c] = (i0[c] + 1).min(self.size - 1);
            f[c] = x - fl;
        }
        let lerp = |a: [f32; 3], b: [f32; 3], t: f32| {
            [
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t,
            ]
        };
        let c00 = lerp(
            self.at(i0[0], i0[1], i0[2]),
            self.at(i1[0], i0[1], i0[2]),
            f[0],
        );
        let c10 = lerp(
            self.at(i0[0], i1[1], i0[2]),
            self.at(i1[0], i1[1], i0[2]),
            f[0],
        );
        let c01 = lerp(
            self.at(i0[0], i0[1], i1[2]),
            self.at(i1[0], i0[1], i1[2]),
            f[0],
        );
        let c11 = lerp(
            self.at(i0[0], i1[1], i1[2]),
            self.at(i1[0], i1[1], i1[2]),
            f[0],
        );
        let c0 = lerp(c00, c10, f[1]);
        let c1 = lerp(c01, c11, f[1]);
        lerp(c0, c1, f[2])
    }

    /// Tetrahedral sample (higher quality on diagonals; used for offline
    /// resampling, not by the realtime pipeline).
    pub fn sample_tetrahedral(&self, rgb: [f32; 3]) -> [f32; 3] {
        let n1 = (self.size - 1) as f32;
        let mut i0 = [0usize; 3];
        let mut f = [0f32; 3];
        for c in 0..3 {
            let x = rgb[c].clamp(0.0, 1.0) * n1;
            let fl = x.floor().min(n1 - 1.0).max(0.0);
            i0[c] = fl as usize;
            f[c] = x - fl;
        }
        let p = |dr: usize, dg: usize, db: usize| self.at(i0[0] + dr, i0[1] + dg, i0[2] + db);
        let (fr, fg, fb) = (f[0], f[1], f[2]);
        let c000 = p(0, 0, 0);
        let c111 = p(1, 1, 1);
        let (a, b, w) = if fr > fg {
            if fg > fb {
                (p(1, 0, 0), p(1, 1, 0), [1.0 - fr, fr - fg, fg - fb, fb])
            } else if fr > fb {
                (p(1, 0, 0), p(1, 0, 1), [1.0 - fr, fr - fb, fb - fg, fg])
            } else {
                (p(0, 0, 1), p(1, 0, 1), [1.0 - fb, fb - fr, fr - fg, fg])
            }
        } else if fb > fg {
            (p(0, 0, 1), p(0, 1, 1), [1.0 - fb, fb - fg, fg - fr, fr])
        } else if fb > fr {
            (p(0, 1, 0), p(0, 1, 1), [1.0 - fg, fg - fb, fb - fr, fr])
        } else {
            (p(0, 1, 0), p(1, 1, 0), [1.0 - fg, fg - fr, fr - fb, fb])
        };
        let mut out = [0f32; 3];
        for c in 0..3 {
            out[c] = w[0] * c000[c] + w[1] * a[c] + w[2] * b[c] + w[3] * c111[c];
        }
        out
    }

    /// Flattened RGBA f32 data (alpha = 1), red fastest — the layout of the
    /// GPU 3D texture.
    pub fn to_rgba_f32(&self) -> Vec<f32> {
        self.data
            .iter()
            .flat_map(|c| [c[0], c[1], c[2], 1.0])
            .collect()
    }
}

/// Parse a `.cube` file.
pub fn parse_cube(text: &str) -> Result<Lut3d, LutError> {
    let mut size: Option<usize> = None;
    let mut title = None;
    let mut dmin = [0f32; 3];
    let mut dmax = [1f32; 3];
    let mut data: Vec<[f32; 3]> = Vec::new();

    for (i, raw) in text.lines().enumerate() {
        let line_no = i + 1;
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let head = parts.next().unwrap_or("");
        let syntax = |m: &str| LutError::Syntax {
            line: line_no,
            message: m.to_string(),
        };
        match head {
            "TITLE" => {
                let rest = line["TITLE".len()..].trim().trim_matches('"');
                title = Some(rest.to_string());
            }
            "LUT_3D_SIZE" => {
                let n: usize = parts
                    .next()
                    .and_then(|s| s.parse().ok())
                    .ok_or_else(|| syntax("bad LUT_3D_SIZE"))?;
                if !(2..=MAX_SIZE).contains(&n) {
                    return Err(LutError::BadSize(n));
                }
                size = Some(n);
            }
            "LUT_1D_SIZE" => return Err(LutError::MissingSize),
            "DOMAIN_MIN" | "DOMAIN_MAX" => {
                let v: Vec<f32> = parts.filter_map(|s| s.parse().ok()).collect();
                if v.len() != 3 {
                    return Err(syntax("domain needs three values"));
                }
                let target = if head == "DOMAIN_MIN" {
                    &mut dmin
                } else {
                    &mut dmax
                };
                target.copy_from_slice(&v);
            }
            "LUT_3D_INPUT_RANGE" => {
                let v: Vec<f32> = parts.filter_map(|s| s.parse().ok()).collect();
                if v.len() != 2 {
                    return Err(syntax("input range needs two values"));
                }
                dmin = [v[0]; 3];
                dmax = [v[1]; 3];
            }
            h if h.starts_with(|c: char| c.is_ascii_digit() || c == '-' || c == '.') => {
                let nums: Result<Vec<f32>, _> = line.split_whitespace().map(str::parse).collect();
                match nums {
                    Ok(v) if v.len() == 3 => data.push([v[0], v[1], v[2]]),
                    _ => return Err(syntax("expected three numbers")),
                }
            }
            // Unknown keywords (e.g. Resolve's LUT_IN_VIDEO_RANGE) are ignored.
            _ => {}
        }
    }

    let n = size.ok_or(LutError::MissingSize)?;
    if data.len() != n * n * n {
        return Err(LutError::WrongRowCount {
            expected: n * n * n,
            found: data.len(),
        });
    }
    let lut = Lut3d {
        title,
        size: n,
        data,
    };
    if dmin == [0.0; 3] && dmax == [1.0; 3] {
        return Ok(lut);
    }
    // Resample a non-unit domain onto [0, 1] so the pipeline's sampling
    // stays uniform.
    let src = lut.clone();
    let mut out = Lut3d::from_fn(n, |rgb| {
        let mut x = [0f32; 3];
        for c in 0..3 {
            let span = (dmax[c] - dmin[c]).max(1e-6);
            x[c] = ((rgb[c] - dmin[c]) / span).clamp(0.0, 1.0);
        }
        src.sample_tetrahedral(x)
    });
    out.title = lut.title;
    Ok(out)
}

/// Write a `.cube` file. Values are written with 6 decimal places.
pub fn write_cube(lut: &Lut3d) -> String {
    let mut s = String::with_capacity(lut.data.len() * 28 + 128);
    if let Some(t) = &lut.title {
        let _ = writeln!(s, "TITLE \"{}\"", t.replace('"', "'"));
    }
    s.push_str("# Created by Apricity. Input/output: sRGB-encoded Rec.709.\n");
    let _ = writeln!(s, "LUT_3D_SIZE {}", lut.size);
    s.push_str("DOMAIN_MIN 0.0 0.0 0.0\nDOMAIN_MAX 1.0 1.0 1.0\n");
    for c in &lut.data {
        let _ = writeln!(s, "{:.6} {:.6} {:.6}", c[0], c[1], c[2]);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_samples_identity() {
        let lut = Lut3d::identity(17);
        for rgb in [
            [0.0, 0.0, 0.0],
            [0.3, 0.6, 0.9],
            [1.0, 1.0, 1.0],
            [0.51, 0.02, 0.77],
        ] {
            let t = lut.sample_trilinear(rgb);
            let h = lut.sample_tetrahedral(rgb);
            for c in 0..3 {
                assert!((t[c] - rgb[c]).abs() < 1e-5);
                assert!((h[c] - rgb[c]).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn round_trip_through_text() {
        let lut = Lut3d::from_fn(5, |[r, g, b]| [g, b * 0.5, r]);
        let text = write_cube(&Lut3d {
            title: Some("Test".into()),
            ..lut.clone()
        });
        let back = parse_cube(&text).unwrap();
        assert_eq!(back.size, 5);
        assert_eq!(back.title.as_deref(), Some("Test"));
        for (a, b) in lut.data.iter().zip(&back.data) {
            for c in 0..3 {
                assert!((a[c] - b[c]).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn red_varies_fastest() {
        let lut =
            parse_cube("LUT_3D_SIZE 2\n0 0 0\n1 0 0\n0 1 0\n1 1 0\n0 0 1\n1 0 1\n0 1 1\n1 1 1\n")
                .unwrap();
        assert_eq!(lut, Lut3d::identity(2));
    }

    #[test]
    fn errors_are_reported() {
        assert_eq!(parse_cube("0 0 0\n"), Err(LutError::MissingSize));
        assert_eq!(parse_cube("LUT_3D_SIZE 500\n"), Err(LutError::BadSize(500)));
        assert!(matches!(
            parse_cube("LUT_3D_SIZE 2\n0 0 0\n"),
            Err(LutError::WrongRowCount {
                expected: 8,
                found: 1
            })
        ));
        assert!(matches!(
            parse_cube("LUT_3D_SIZE 2\n0 0 x\n"),
            Err(LutError::Syntax { line: 2, .. })
        ));
        assert_eq!(parse_cube("LUT_1D_SIZE 4\n"), Err(LutError::MissingSize));
    }

    #[test]
    fn domain_is_normalized() {
        let mut text = String::from("LUT_3D_SIZE 2\nDOMAIN_MIN 0 0 0\nDOMAIN_MAX 2 2 2\n");
        for b in 0..2 {
            for g in 0..2 {
                for r in 0..2 {
                    let _ = writeln!(
                        text,
                        "{} {} {}",
                        r as f32 * 2.0,
                        g as f32 * 2.0,
                        b as f32 * 2.0
                    );
                }
            }
        }
        // This LUT is the identity over the domain [0, 2]; after
        // normalization, an input of 0.5 must still map to 0.5.
        let lut = parse_cube(&text).unwrap();
        let v = lut.sample_trilinear([0.5, 0.25, 1.0]);
        assert!(
            (v[0] - 0.5).abs() < 1e-5 && (v[1] - 0.25).abs() < 1e-5,
            "{v:?}"
        );
    }

    #[test]
    fn comments_and_unknown_keywords_ignored() {
        let lut = parse_cube(
            "# hello\nTITLE \"x\"\nLUT_IN_VIDEO_RANGE\nLUT_3D_SIZE 2\n0 0 0 # c\n1 0 0\n0 1 0\n1 1 0\n0 0 1\n1 0 1\n0 1 1\n1 1 1\n",
        )
        .unwrap();
        assert_eq!(lut.size, 2);
    }
}
