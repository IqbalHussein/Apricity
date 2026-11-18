/// <reference types="node" />
import { readFileSync } from "node:fs";
import { beforeAll, describe, expect, it } from "vitest";
import {
  identityParams,
  initSync,
  lutBake,
  lutParse,
  ngaApply,
  paramSpecs,
  recipeNew,
  recipeParse,
  recipeToJson,
  renderCpuRgba,
} from "../wasm/apricity_ffi.js";

const snapshot = JSON.parse(
  readFileSync(
    new URL("../../../../crates/apricity-core/tests/schema_snapshot.json", import.meta.url),
    "utf8",
  ),
) as { identity: unknown; scalar_paths: string[] };

beforeAll(() => {
  initSync({ module: readFileSync(new URL("../wasm/apricity_ffi_bg.wasm", import.meta.url)) });
});

describe("Rust core across the WASM boundary", () => {
  it("exposes exactly the schema snapshot's scalar paths", () => {
    const specs = paramSpecs();
    expect(specs.scalars.length).toBeGreaterThan(0);
    expect(specs.scalars.map((s) => s.path)).toEqual(snapshot.scalar_paths);
  });

  it("identityParams deep-equals the snapshot identity", () => {
    expect(identityParams()).toEqual(snapshot.identity);
  });

  it("ngaApply returns changes with rationale", () => {
    const r = ngaApply("warm golden hour", identityParams(), "auto");
    expect(r.changes.length).toBeGreaterThan(0);
    expect(r.changes.every((c) => c.rationale.length > 0)).toBe(true);
  });

  it("round-trips a recipe through JSON", () => {
    const r = recipeNew("r-1", "Warm", identityParams(), "warm", 1000);
    const back = recipeParse(recipeToJson(r));
    expect(back).toEqual(r);
    expect(() => recipeNew("bad id", "x", identityParams(), null, 0)).toThrow();
  });

  it("bakes an identity LUT that parses back at 33 points", () => {
    const text = lutBake(identityParams(), 33, "creative", "Identity");
    expect(lutParse(text)).toEqual({ title: "Identity", size: 33 });
    expect(() => lutParse("garbage")).toThrow();
  });

  it("renders identity as a no-op within one code value", () => {
    const w = 16;
    const h = 8;
    const src = new Uint8Array(w * h * 4);
    for (let y = 0; y < h; y++) {
      for (let x = 0; x < w; x++) {
        const i = (y * w + x) * 4;
        src.set([(x * 255) / (w - 1), (y * 255) / (h - 1), 128, 255].map(Math.round), i);
      }
    }
    const out = renderCpuRgba(src, w, h, identityParams(), null, w);
    expect(out.length).toBe(src.length);
    for (let i = 0; i < src.length; i++) {
      expect(Math.abs(out[i]! - src[i]!)).toBeLessThanOrEqual(1);
    }
  });
});
