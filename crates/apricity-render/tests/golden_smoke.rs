//! Smoke test for the `golden-images` feature gate.
//!
//! Exists so the CI job that runs `cargo test -p apricity-render --features
//! golden-images` actually exercises a test target. Real golden-image
//! fixtures (CPU reference render → ΔE2000 vs checked-in PNG) land in M1.

#![cfg(feature = "golden-images")]

#[test]
fn golden_images_feature_compiles() {
    // The gate itself is the assertion. If this test target builds and runs
    // under `--features golden-images`, the feature is wired correctly.
}
