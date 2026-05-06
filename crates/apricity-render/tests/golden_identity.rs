//! M1 first golden fixture: identity-grade sRGB → LinearRec2020 → sRGB.
//!
//! Run with `APRICITY_REGEN_GOLDEN=1 cargo test -p apricity-render
//! --features golden-images golden_identity` to (re)write the reference PNG
//! after a deliberate pipeline change. Without that env var, the test asserts
//! the current pipeline output matches the checked-in reference within
//! [`MAX_DELTA_8BIT`] per channel.

#![cfg(feature = "golden-images")]

use std::path::PathBuf;

use apricity_render::golden::{max_rgb_delta, render_identity_patch, PATCH_SIZE};
use image::{ImageBuffer, Rgba};

/// Tolerance in 8-bit code values per channel. The identity pipeline drives
/// f32 round-trip + matrix×inverse error only; a delta of 1 covers
/// rounding-to-nearest-code variation across hosts. Tighten as parity tests
/// for individual stages land in M2.
const MAX_DELTA_8BIT: u8 = 1;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join("identity_srgb_ramp.png")
}

#[test]
fn identity_patch_matches_fixture() {
    let actual = render_identity_patch();
    let path = fixture_path();

    if std::env::var_os("APRICITY_REGEN_GOLDEN").is_some() {
        let img = ImageBuffer::<Rgba<u8>, _>::from_raw(PATCH_SIZE, PATCH_SIZE, actual.clone())
            .expect("buffer matches PATCH_SIZE * PATCH_SIZE * 4");
        img.save(&path).expect("write golden fixture");
        eprintln!("wrote golden fixture: {}", path.display());
        return;
    }

    let reference = image::open(&path)
        .unwrap_or_else(|e| panic!("read golden fixture {}: {e}", path.display()))
        .to_rgba8();
    assert_eq!(reference.width(), PATCH_SIZE);
    assert_eq!(reference.height(), PATCH_SIZE);

    let delta = max_rgb_delta(&actual, reference.as_raw());
    assert!(
        delta <= MAX_DELTA_8BIT,
        "identity-pipeline output drifted: max delta {delta} > {MAX_DELTA_8BIT}"
    );
}
