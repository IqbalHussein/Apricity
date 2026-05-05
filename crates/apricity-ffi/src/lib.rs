//! Single FFI surface for every shell.
//!
//! - `wasm` feature: `wasm-bindgen` exports for the web shell.
//! - `uniffi` feature: UniFFI exports for Kotlin (Android) and Swift (iOS).
//!
//! Both features re-export the same logical API. The shells consume only this
//! crate; they never depend directly on `apricity-core`, `apricity-render`,
//! etc.

#![forbid(unsafe_code)]

#[cfg(feature = "wasm")]
mod wasm {
    use apricity_core::GradeParameters;
    use wasm_bindgen::prelude::*;

    /// Returns the identity `GradeParameters` as JSON. Smoke test for the
    /// WASM build; real surface lands in M1.
    #[wasm_bindgen]
    pub fn identity_grade_json() -> String {
        serde_json::to_string(&GradeParameters::IDENTITY)
            .expect("IDENTITY serializes")
    }

    /// Library version, for the web shell to confirm it loaded the right WASM.
    #[wasm_bindgen]
    pub fn version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}
