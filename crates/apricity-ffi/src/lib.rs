//! Single FFI surface for every shell.
//!
//! - [`api`]: plain-Rust functions holding all logic (serde-friendly, no
//!   platform types), natively unit-tested.
//! - [`compose`] and [`state`]: pure pieces of the web `Engine` (downscaling,
//!   compare resolution, CPU compositing, scopes, analysis).
//! - `wasm` feature: `wasm-bindgen` exports (thin serde adapters over the
//!   above) and, on `wasm32`, the GPU/canvas `Engine`.
//! - `uniffi` feature: reserved for Kotlin/Swift bindings.
//!
//! Shells consume only this crate. TypeScript declarations for the data types
//! are generated from the Rust definitions (`tsify-next`) into the
//! wasm-bindgen `.d.ts`; shells never hand-write them.

#![forbid(unsafe_code)]

pub mod api;
pub mod compose;
pub mod state;

#[cfg(feature = "wasm")]
mod wasm;

#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
mod engine;
