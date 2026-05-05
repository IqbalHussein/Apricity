//! Recipe model and in-memory store.
//!
//! A Recipe wraps `GradeParameters` plus metadata (name, author, opt-in
//! sharing flag). Persistence is host-supplied via a trait so the same crate
//! drives `localStorage`/OPFS on the web, files on desktop, and Room/Core Data
//! on mobile.

#![forbid(unsafe_code)]
