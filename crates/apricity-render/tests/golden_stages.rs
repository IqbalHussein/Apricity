//! Per-stage golden fixtures rendered by the CPU reference.
//!
//! `APRICITY_REGEN_GOLDEN=1 cargo test -p apricity-render --features
//! golden-images --test golden_stages` rewrites the PNGs after a deliberate
//! pipeline change. Review the diff images before committing.

#![cfg(feature = "golden-images")]

use std::path::PathBuf;

use apricity_render::golden::{fixtures, max_rgb_delta, PATCH_SIZE};
use image::{ImageBuffer, Rgba};

const MAX_DELTA_8BIT: u8 = 1;

fn path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(format!("{name}.png"))
}

#[test]
fn every_stage_matches_its_fixture() {
    let regen = std::env::var_os("APRICITY_REGEN_GOLDEN").is_some();
    let mut failures = Vec::new();
    for fx in fixtures() {
        let actual = fx.render_cpu();
        let p = path(fx.name);
        if regen {
            ImageBuffer::<Rgba<u8>, _>::from_raw(PATCH_SIZE, PATCH_SIZE, actual)
                .expect("patch size")
                .save(&p)
                .expect("write fixture");
            continue;
        }
        let reference = image::open(&p)
            .unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
            .to_rgba8();
        let d = max_rgb_delta(&actual, reference.as_raw());
        if d > MAX_DELTA_8BIT {
            failures.push(format!("{}: max delta {d}", fx.name));
        }
    }
    assert!(
        failures.is_empty(),
        "golden drift:\n{}",
        failures.join("\n")
    );
}

#[test]
fn every_fixture_differs_from_identity() {
    // Guards against a stage silently becoming a no-op.
    let identity = apricity_render::golden::Fixture {
        name: "identity",
        params: apricity_core::GradeParameters::IDENTITY,
        overlay: apricity_render::Overlay::None,
        lut: None,
    }
    .render_cpu();
    for fx in fixtures() {
        if fx.name == "overlay_zebra" {
            continue; // Zebra only marks the brightest band; checked below.
        }
        assert!(
            max_rgb_delta(&fx.render_cpu(), &identity) > 4,
            "{} is a no-op",
            fx.name
        );
    }
}
