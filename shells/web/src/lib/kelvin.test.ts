import { describe, expect, it } from "vitest";
import { kelvinToRgb, planckianGradient, planckianStops } from "./kelvin";

describe("Planckian ramp", () => {
  it("is monotonic: kelvin rises and the color moves from amber to blue", () => {
    const stops = planckianStops(24);
    expect(stops[0]?.kelvin).toBeCloseTo(2000, 3);
    expect(stops.at(-1)?.kelvin).toBeCloseTo(10000, 3);
    for (let i = 1; i < stops.length; i++) {
      const a = stops[i - 1];
      const b = stops[i];
      if (!a || !b) throw new Error("missing stop");
      expect(b.kelvin).toBeGreaterThan(a.kelvin);
      expect(b.pos).toBeGreaterThan(a.pos);
      expect(b.rgb.b - b.rgb.r).toBeGreaterThanOrEqual(a.rgb.b - a.rgb.r);
    }
  });

  it("spaces stops evenly in mireds", () => {
    const m = planckianStops(8).map((s) => 1e6 / s.kelvin);
    const d = m.map((v, i) => (i === 0 ? 0 : (m[i - 1] as number) - v)).slice(1);
    for (const x of d) expect(x).toBeCloseTo(d[0] as number, 6);
  });

  it("gives amber for 2000 K and bluish for 10000 K", () => {
    const warm = kelvinToRgb(2000);
    const cool = kelvinToRgb(10000);
    expect(warm.r).toBeGreaterThan(warm.b + 100);
    expect(cool.b).toBeGreaterThan(cool.r);
  });

  it("emits a CSS gradient in the requested direction", () => {
    expect(planckianGradient("to right")).toMatch(/^linear-gradient\(to right, rgb\(/);
    expect(planckianGradient("to left", 5).match(/rgb\(/g)).toHaveLength(5);
  });
});
