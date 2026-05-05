# apricity-ffi

The single FFI surface for every shell.

| Feature flag | Target | Tooling |
|---|---|---|
| `wasm` | Web shell | `wasm-bindgen` + `wasm-pack` |
| `uniffi` | Android, iOS | `uniffi-rs` |

Shells depend only on this crate. They do **not** depend directly on
`apricity-core`, `apricity-render`, etc. The FFI types are the contract.

## Build

Web (from repo root):

```
wasm-pack build crates/apricity-ffi --target web --features wasm --out-dir ../../shells/web/src/wasm
```
