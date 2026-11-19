import { describe, expect, it } from "vitest";
import { pointToWheel, rgbToWheel, wheelToPoint, wheelToRgb } from "./wheel";

describe("color wheel mapping", () => {
  it("produces zero-mean offsets within the range", () => {
    for (let hue = 0; hue < 360; hue += 15) {
      for (const sat of [0, 0.3, 1]) {
        const [r, g, b] = wheelToRgb(hue, sat, 0.5);
        expect(r + g + b).toBeCloseTo(0, 9);
        for (const c of [r, g, b]) expect(Math.abs(c)).toBeLessThanOrEqual(0.5 + 1e-9);
      }
    }
  });

  it("round-trips hue and saturation", () => {
    for (let hue = 1; hue < 360; hue += 7) {
      for (const sat of [0.1, 0.5, 0.95, 1]) {
        const [r, g, b] = wheelToRgb(hue, sat, 0.5);
        const back = rgbToWheel(r, g, b, 0.5);
        expect(back.sat).toBeCloseTo(sat, 6);
        expect(back.hue).toBeCloseTo(hue, 4);
      }
    }
  });

  it("points primary hues at their channel", () => {
    const [r, g, b] = wheelToRgb(0, 1, 0.5);
    expect(r).toBeCloseTo(0.5);
    expect(g).toBeCloseTo(-0.25);
    expect(b).toBeCloseTo(-0.25);
    expect(wheelToRgb(120, 1, 0.5)[1]).toBeCloseTo(0.5);
    expect(wheelToRgb(240, 1, 0.5)[2]).toBeCloseTo(0.5);
  });

  it("discards the common-mode part and reports zero at the center", () => {
    expect(rgbToWheel(0.1, 0.1, 0.1, 0.5).sat).toBeCloseTo(0);
    expect(rgbToWheel(0, 0, 0, 0.5)).toEqual({ hue: 0, sat: 0 });
  });

  it("maps pointer offsets to hue clockwise from the top and back", () => {
    expect(pointToWheel(0, -50, 100)).toEqual({ hue: 0, sat: 0.5 });
    expect(pointToWheel(100, 0, 100).hue).toBeCloseTo(90);
    expect(pointToWheel(500, 0, 100).sat).toBe(1);
    const p = wheelToPoint({ hue: 210, sat: 0.6 }, 60);
    const back = pointToWheel(p.x, p.y, 60);
    expect(back.hue).toBeCloseTo(210, 6);
    expect(back.sat).toBeCloseTo(0.6, 6);
  });
});
