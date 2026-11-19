import { describe, expect, it } from "vitest";
import { downscaleRgba } from "./thumbnail";

describe("downscaleRgba", () => {
  it("returns the source when it already fits", () => {
    const px = new Uint8Array(4 * 4 * 4);
    expect(downscaleRgba(px, 4, 4, 8).rgba).toBe(px);
  });

  it("averages boxes and keeps the aspect ratio", () => {
    const w = 8;
    const h = 4;
    const px = new Uint8Array(w * h * 4);
    for (let y = 0; y < h; y++)
      for (let x = 0; x < w; x++) px.set(x < 4 ? [200, 100, 0, 255] : [0, 100, 200, 255], (y * w + x) * 4);
    const out = downscaleRgba(px, w, h, 4);
    expect([out.width, out.height]).toEqual([4, 2]);
    expect(Array.from(out.rgba.slice(0, 4))).toEqual([200, 100, 0, 255]);
    expect(Array.from(out.rgba.slice(12, 16))).toEqual([0, 100, 200, 255]);
  });
});
