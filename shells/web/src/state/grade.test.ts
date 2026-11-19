/// <reference types="node" />
import { readFileSync } from "node:fs";
import { beforeAll, beforeEach, describe, expect, it } from "vitest";
import { initSync, paramSpecs } from "../wasm/apricity_ffi.js";
import { manualClock, setClock } from "./clock";
import {
  applyAnalysis,
  gradeParams,
  initGrade,
  redo,
  resetForNewSource,
  resetPath,
  setCurve,
  setScalar,
  setScalars,
  undo,
} from "./grade";
import { history, historyStack } from "./history";

beforeAll(() => {
  initSync({ module: readFileSync(new URL("../wasm/apricity_ffi_bg.wasm", import.meta.url)) });
});

beforeEach(() => {
  setClock(manualClock(1_000));
  initGrade();
});

describe("grade store", () => {
  it("clamps scalars to the spec range through the core", () => {
    const spec = paramSpecs().scalars.find((s) => s.path === "stage2.contrast");
    expect(spec).toBeDefined();
    expect(setScalar("stage2.contrast", 9999)).toBe(spec?.max);
    expect(gradeParams().stage2?.contrast).toBe(spec?.max);
    expect(setScalar("stage2.contrast", -9999)).toBe(spec?.min);
    expect(gradeParams().stage2?.contrast).toBe(spec?.min);
  });

  it("rejects unknown paths and non-finite values fall back to the default", () => {
    expect(setScalar("stage2.nope", 1)).toBeNull();
    expect(setScalar("stage2.contrast", Number.NaN)).toBe(0);
  });

  it("writes nested paths (hsl bands, stage 1 black point)", () => {
    setScalar("stage2.hsl.blue.hue", 12);
    setScalar("stage1.black_point.g", 0.01);
    expect(gradeParams().stage2?.hsl?.blue?.hue).toBe(12);
    expect(gradeParams().stage1?.black_point?.g).toBeCloseTo(0.01);
  });

  it("coalesces a drag into one undo step and undo/redo restore values", () => {
    for (let v = 1; v <= 20; v++) setScalar("stage2.exposure", v / 10);
    expect(historyStack().size).toBe(2);
    expect(gradeParams().stage2?.exposure).toBeCloseTo(2);
    expect(undo()).toBe(true);
    expect(gradeParams().stage2?.exposure).toBe(0);
    expect(history.canRedo()).toBe(true);
    expect(redo()).toBe(true);
    expect(gradeParams().stage2?.exposure).toBeCloseTo(2);
  });

  it("writes a wheel puck as one undo step", () => {
    setScalars(
      { "stage2.lift.r": 0.2, "stage2.lift.g": -0.1, "stage2.lift.b": -0.1 },
      "Lift",
      "wheel:lift",
    );
    expect(gradeParams().stage2?.lift?.r).toBeCloseTo(0.2);
    undo();
    expect(gradeParams().stage2?.lift?.r).toBe(0);
  });

  it("stores the identity curve as empty and edited curves as points", () => {
    setCurve("master", [
      { x: 0, y: 0 },
      { x: 0.5, y: 0.6 },
      { x: 1, y: 1 },
    ]);
    expect(gradeParams().stage2?.tone_curve?.master).toHaveLength(3);
  });

  it("resets one control as its own step", () => {
    setScalar("stage2.contrast", 40);
    resetPath("stage2.contrast");
    expect(gradeParams().stage2?.contrast).toBe(0);
    undo();
    expect(gradeParams().stage2?.contrast).toBe(40);
  });

  it("applies a Stage 1 analysis per auto flag, and a manual edit leaves Auto", () => {
    const analysis = {
      exposure: 0.7,
      temperature: 30,
      tint: -4,
      black_point: { r: 0.01, g: 0.01, b: 0.01 },
      median_luminance: 0.2,
      estimated_kelvin: 3200,
      illuminant: "tungsten" as const,
    };
    applyAnalysis(analysis, null);
    expect(gradeParams().stage1?.exposure).toBe(0);

    resetForNewSource();
    expect(gradeParams().stage1?.auto_exposure).toBe(true);
    applyAnalysis(analysis, null);
    expect(gradeParams().stage1?.exposure).toBeCloseTo(0.7);
    expect(gradeParams().stage1?.temperature).toBeCloseTo(30);
    expect(gradeParams().stage1?.black_point?.r).toBeCloseTo(0.01);

    setScalar("stage1.exposure", 1.2);
    expect(gradeParams().stage1?.auto_exposure).toBe(false);
    expect(gradeParams().stage1?.auto_white_balance).toBe(true);
    applyAnalysis(analysis, null);
    expect(gradeParams().stage1?.exposure).toBeCloseTo(1.2);
  });
});
