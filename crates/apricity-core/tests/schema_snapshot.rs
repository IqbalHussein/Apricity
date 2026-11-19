//! Schema-stability test: the JSON form of `GradeParameters::IDENTITY` and
//! of the parameter spec table must match the checked-in snapshot. The same
//! snapshot is read by the web shell's vitest suite, so a change here that
//! isn't mirrored across the FFI boundary fails CI on both sides.
//!
//! Regenerate deliberately with `APRICITY_REGEN_SNAPSHOT=1 cargo test -p
//! apricity-core --test schema_snapshot`.

use std::path::PathBuf;

use apricity_core::{param_spec, GradeParameters};

fn snapshot_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("schema_snapshot.json")
}

#[derive(serde::Serialize)]
struct Snapshot {
    identity: GradeParameters,
    scalar_paths: Vec<&'static str>,
    curve_paths: [&'static str; 4],
}

fn current() -> Snapshot {
    Snapshot {
        identity: GradeParameters::IDENTITY,
        scalar_paths: param_spec::all_scalar_specs().map(|s| s.path).collect(),
        curve_paths: param_spec::CURVE_PATHS,
    }
}

#[test]
fn schema_matches_snapshot() {
    let actual = serde_json::to_string_pretty(&current()).unwrap() + "\n";
    let path = snapshot_path();
    if std::env::var_os("APRICITY_REGEN_SNAPSHOT").is_some() {
        std::fs::write(&path, &actual).unwrap();
        return;
    }
    let expected =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert_eq!(
        actual, expected,
        "GradeParameters schema changed; regenerate the snapshot and bump SCHEMA_VERSION if breaking"
    );
}
