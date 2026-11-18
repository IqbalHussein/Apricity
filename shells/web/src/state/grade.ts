import { createSignal } from "solid-js";
import { identityParams, sanitizeParams } from "../core/wasm";
import type { CurvePoint, GradeParameters, LogProfile, Stage1Analysis } from "../core/wasm";
import { clampToSpec, getPath, setPath } from "../lib/paramPath";
import { isIdentity } from "../lib/curve";
import { clearAi, clearAllAi } from "./aimarks";
import { historyStack, resetHistory, touchHistory } from "./history";
import { specFor } from "./specs";

const [params, setParams] = createSignal<GradeParameters>({});

/**
 * The current grade (always sanitized by the Rust core). Units and color
 * spaces are those of `GradeParameters` in `apricity-core`.
 *
 * @returns The live grade; reading inside an effect tracks it.
 */
export const gradeParams = params;

/** Tone curve channel names, matching `ToneCurve`. */
export type CurveChannel = "master" | "red" | "green" | "blue";

const AUTO_FLAG: readonly (readonly [RegExp, string])[] = [
  [/^stage1\.exposure$/, "stage1.auto_exposure"],
  [/^stage1\.(temperature|tint)$/, "stage1.auto_white_balance"],
  [/^stage1\.black_point\./, "stage1.auto_black_point"],
];

function commit(
  next: GradeParameters,
  label: string,
  key: string | null,
  edited: readonly string[],
): void {
  // Touching an automatic Stage 1 control takes it out of Auto.
  for (const [re, flag] of AUTO_FLAG) {
    if (edited.some((p) => re.test(p)) && getPath(next, flag) === true)
      next = setPath(next, flag, false);
  }
  const clean = sanitizeParams(next);
  setParams(clean);
  historyStack().push(clean, label, key);
  touchHistory();
  clearAi(edited);
}

/**
 * Load the identity grade and a fresh history. Call once after the core is initialized.
 */
export function initGrade(): void {
  const id = identityParams();
  setParams(id);
  resetHistory(id);
}

/**
 * Reset for a newly opened image: identity grade with Stage 1 auto flags on
 * (exposure, white balance, black point), fresh history, no AI marks.
 *
 * @returns The new grade (analysis has not been applied yet).
 */
export function resetForNewSource(): GradeParameters {
  let next = identityParams();
  next = setPath(next, "stage1.auto_exposure", true);
  next = setPath(next, "stage1.auto_white_balance", true);
  next = setPath(next, "stage1.auto_black_point", true);
  const clean = sanitizeParams(next);
  setParams(clean);
  resetHistory(clean);
  clearAllAi();
  return clean;
}

/**
 * Set one scalar parameter. The value is clamped to the spec range, the
 * result is sanitized by the core, and consecutive edits of the same control
 * coalesce into one undo step.
 *
 * @param path - Dotted scalar path, e.g. `stage2.exposure`.
 * @param value - New value in the parameter's own unit (see its `ScalarSpec.unit`).
 * @param opts - `coalesce: false` forces a separate undo step (typed values, resets).
 * @returns The value actually stored (clamped), or `null` for an unknown path.
 */
export function setScalar(
  path: string,
  value: number,
  opts: { coalesce?: boolean } = {},
): number | null {
  const spec = specFor(path);
  if (!spec) return null;
  const v = clampToSpec(spec, value);
  const next = setPath(params(), path, v);
  commit(next, spec.label, opts.coalesce === false ? null : `scalar:${path}`, [path]);
  return v;
}

/**
 * Set several scalars as one undo step (a color wheel puck writes r, g, b together).
 *
 * @param entries - Path to value (own units); each is clamped to its spec.
 * @param label - Undo label.
 * @param key - Coalescing key, or `null` for a standalone step.
 */
export function setScalars(
  entries: Readonly<Record<string, number>>,
  label: string,
  key: string | null,
): void {
  let next = params();
  for (const [path, value] of Object.entries(entries)) {
    const spec = specFor(path);
    if (spec) next = setPath(next, path, clampToSpec(spec, value));
  }
  commit(next, label, key, Object.keys(entries));
}

/**
 * Replace one tone-curve channel.
 *
 * @param channel - Which curve.
 * @param points - Control points, both axes in the working encoding [0, 1]; the identity curve is stored as empty.
 */
export function setCurve(channel: CurveChannel, points: readonly CurvePoint[]): void {
  const stored = isIdentity(points) ? [] : points.map((p) => ({ x: p.x, y: p.y }));
  const path = `stage2.tone_curve.${channel}`;
  commit(
    setPath(params(), path, stored),
    `${channel[0]?.toUpperCase()}${channel.slice(1)} curve`,
    `curve:${channel}`,
    [path],
  );
}

/**
 * Set a non-scalar field (booleans, enums) as its own undo step.
 *
 * @param path - Dotted path, e.g. `stage1.auto_exposure`.
 * @param value - New value.
 * @param label - Undo label.
 */
export function setField(path: string, value: unknown, label: string): void {
  commit(setPath(params(), path, value), label, null, [path]);
}

/**
 * Choose the Stage 1 input profile.
 *
 * @param profile - Log profile id from `logProfiles()`.
 */
export function setInputProfile(profile: LogProfile): void {
  setField("stage1.input_profile", profile, "Input profile");
}

/**
 * Replace the whole grade (NGA result, recipe, undo target) as one step.
 *
 * @param next - Complete grade.
 * @param label - Undo label (the prompt for NGA results).
 * @returns The sanitized grade now current.
 */
export function applyParams(next: GradeParameters, label: string): GradeParameters {
  const clean = sanitizeParams(next);
  setParams(clean);
  historyStack().push(clean, label, null);
  touchHistory();
  return clean;
}

/**
 * Reset one scalar to its spec default as its own undo step.
 *
 * @param path - Dotted scalar path.
 */
export function resetPath(path: string): void {
  const spec = specFor(path);
  if (spec) setScalar(path, spec.default, { coalesce: false });
}

/**
 * Copy a Stage 1 analysis into the grade, honoring the `auto_*` flags
 * (mirrors `apricity_core::analysis::apply_analysis`).
 *
 * @param a - Analysis from `Engine.analyzeStage1()` (stops, mireds, tint units, scene-linear Rec.2020 black point).
 * @param label - Undo label; `null` updates the grade without an undo step (image load).
 * @param key - Undo coalescing key for repeated runs.
 */
export function applyAnalysis(
  a: Stage1Analysis,
  label: string | null,
  key: string | null = null,
): void {
  let next = params();
  const s1 = next.stage1 ?? {};
  if (s1.auto_exposure) next = setPath(next, "stage1.exposure", a.exposure);
  if (s1.auto_white_balance) {
    next = setPath(next, "stage1.temperature", a.temperature);
    next = setPath(next, "stage1.tint", a.tint);
  }
  if (s1.auto_black_point) next = setPath(next, "stage1.black_point", { ...a.black_point });
  const clean = sanitizeParams(next);
  setParams(clean);
  if (label === null) {
    resetHistory(clean);
  } else {
    historyStack().push(clean, label, key);
    touchHistory();
  }
}

/**
 * Undo one step and clear AI rings.
 *
 * @returns `true` when something was undone.
 */
export function undo(): boolean {
  const e = historyStack().undo();
  if (!e) return false;
  setParams(e.params);
  clearAllAi();
  touchHistory();
  return true;
}

/**
 * Redo one step and clear AI rings.
 *
 * @returns `true` when something was redone.
 */
export function redo(): boolean {
  const e = historyStack().redo();
  if (!e) return false;
  setParams(e.params);
  clearAllAi();
  touchHistory();
  return true;
}
