import { describe, expect, it } from "vitest";
import { manualClock, seededRng } from "./clock";

describe("injected clock and rng", () => {
  it("is reproducible for a seed and yields integer seeds below 2^53", () => {
    const a = seededRng(42);
    const b = seededRng(42);
    for (let i = 0; i < 50; i++) {
      const s = a.nextSeed();
      expect(s).toBe(b.nextSeed());
      expect(Number.isSafeInteger(s)).toBe(true);
      expect(s).toBeLessThan(2 ** 53);
    }
    const c = seededRng(43);
    expect(c.nextSeed()).not.toBe(seededRng(42).nextSeed());
  });

  it("advances the manual clock only when told", () => {
    const c = manualClock(10);
    expect(c.nowMs()).toBe(10);
    c.advance(5);
    expect(c.nowMs()).toBe(15);
  });
});
