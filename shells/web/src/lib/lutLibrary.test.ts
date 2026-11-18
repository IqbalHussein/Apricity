/// <reference types="node" />
import { readFileSync } from "node:fs";
import { beforeAll, describe, expect, it } from "vitest";
import { seededRng } from "../state/clock";
import { identityParams, initSync, lutBake, lutParse } from "../wasm/apricity_ffi.js";
import { INVALID_LUT_MESSAGE, LutLibrary, lutTitle, makeLutId } from "./lutLibrary";

beforeAll(() => {
  initSync({ module: readFileSync(new URL("../wasm/apricity_ffi_bg.wasm", import.meta.url)) });
});

describe("LUT state", () => {
  it("validates a baked LUT and takes its title", () => {
    const text = lutBake(identityParams(), 17, "creative", "My look");
    const info = lutParse(text);
    expect(info.size).toBe(17);
    expect(lutTitle(info, "file.cube")).toBe("My look");
  });

  it("falls back to the file name without extension", () => {
    expect(lutTitle({ title: null, size: 33 }, "warm.film.cube")).toBe("warm.film");
    expect(lutTitle({ title: "  ", size: 33 }, "x.cube")).toBe("x");
  });

  it("rejects text that is not a .cube", () => {
    expect(() => lutParse("not a lut")).toThrow();
    expect(INVALID_LUT_MESSAGE).toContain(".cube");
  });

  it("generates valid, reproducible ids", () => {
    const a = makeLutId(seededRng(1));
    expect(a).toBe(makeLutId(seededRng(1)));
    expect(a).toMatch(/^lut-[a-z0-9]+$/);
  });

  it("looks LUTs up by the grade's lut_id", () => {
    const lib = new LutLibrary();
    lib.add({ id: "lut-1", title: "A", size: 17, text: "t" });
    expect(lib.get("lut-1")?.title).toBe("A");
    expect(lib.get("missing")).toBeUndefined();
    expect(lib.get(null)).toBeUndefined();
  });
});
