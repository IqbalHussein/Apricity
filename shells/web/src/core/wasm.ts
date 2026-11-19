import init from "../wasm/apricity_ffi";

export * from "../wasm/apricity_ffi";

let loading: Promise<void> | null = null;

/**
 * Load and instantiate the Rust core (WASM) exactly once.
 *
 * Idempotent: concurrent and repeated calls share one load. Must resolve
 * before any other export of this module is called. In the browser this
 * fetches `apricity_ffi_bg.wasm` relative to the generated glue module; Node
 * tests instead call `initSync` with the file's bytes.
 *
 * All data types re-exported here (`GradeParameters`, `Recipe`, `NgaResult`,
 * `SpecTable`, ...) are generated from the Rust definitions by `tsify-next`;
 * never declare them by hand.
 *
 * @returns A promise that resolves once the module is ready.
 */
export function initCore(): Promise<void> {
  loading ??= init().then(() => undefined);
  return loading;
}
