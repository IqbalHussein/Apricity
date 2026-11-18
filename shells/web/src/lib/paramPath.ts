import type { GradeParameters, ScalarSpec } from "../core/wasm";

/**
 * Read a dotted path such as `stage2.hsl.red.hue` from a parameter tree.
 *
 * @param obj - Any JSON-like tree (normally a `GradeParameters`).
 * @param path - Dotted path as used by `ScalarSpec.path`.
 * @returns The value at the path, or `undefined` when any segment is missing.
 */
export function getPath(obj: unknown, path: string): unknown {
  let cur: unknown = obj;
  for (const seg of path.split(".")) {
    if (cur === null || typeof cur !== "object") return undefined;
    cur = (cur as Record<string, unknown>)[seg];
  }
  return cur;
}

/**
 * Read a scalar parameter in its own unit (see `ScalarSpec.unit`).
 *
 * @param params - Current grade.
 * @param spec - Spec naming the path and supplying the identity default.
 * @returns The stored value, or `spec.default` when the field is absent.
 */
export function getScalar(
  params: GradeParameters,
  spec: Pick<ScalarSpec, "path" | "default">,
): number {
  const v = getPath(params, spec.path);
  return typeof v === "number" && Number.isFinite(v) ? v : spec.default;
}

/**
 * Return a copy of `params` with the value at `path` replaced. Intermediate
 * objects are created when missing. The input is never mutated.
 *
 * @param params - Grade to copy.
 * @param path - Dotted path, e.g. `stage2.lift.r`.
 * @param value - New value in the parameter's own unit.
 * @returns A new `GradeParameters` (not yet sanitized).
 */
export function setPath(params: GradeParameters, path: string, value: unknown): GradeParameters {
  const next = structuredClone(params) as Record<string, unknown>;
  const segs = path.split(".");
  let cur = next;
  for (const seg of segs.slice(0, -1)) {
    const child = cur[seg];
    if (child === null || typeof child !== "object") cur[seg] = {};
    cur = cur[seg] as Record<string, unknown>;
  }
  cur[segs[segs.length - 1] as string] = value;
  return next as GradeParameters;
}

/**
 * Clamp a value to the inclusive range of its spec. Non-finite input returns
 * the spec default.
 *
 * @param spec - Range holder, in the parameter's own unit.
 * @param value - Candidate value in the same unit.
 * @returns The clamped value.
 */
export function clampToSpec(
  spec: Pick<ScalarSpec, "min" | "max" | "default">,
  value: number,
): number {
  if (!Number.isFinite(value)) return spec.default;
  return Math.min(spec.max, Math.max(spec.min, value));
}
