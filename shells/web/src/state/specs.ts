import { logProfiles, paramSpecs } from "../core/wasm";
import type { LogProfileInfo, ScalarSpec, SpecTable } from "../core/wasm";

let table: SpecTable | null = null;
let byPath: Map<string, ScalarSpec> | null = null;
let profiles: LogProfileInfo[] | null = null;

/**
 * The parameter spec table from the Rust core (cached). The core must be
 * initialized first.
 *
 * @returns Every scalar spec plus curve and HSL band names.
 */
export function getSpecs(): SpecTable {
  table ??= paramSpecs();
  return table;
}

/**
 * Look up one scalar spec.
 *
 * @param path - Dotted path such as `stage2.contrast`.
 * @returns The spec, or `undefined` for unknown paths.
 */
export function specFor(path: string): ScalarSpec | undefined {
  byPath ??= new Map(getSpecs().scalars.map((s) => [s.path, s]));
  return byPath.get(path);
}

/**
 * Input color profiles supported by Stage 1 (cached).
 *
 * @returns Profile ids and labels from the core.
 */
export function getLogProfiles(): LogProfileInfo[] {
  profiles ??= logProfiles();
  return profiles;
}
