import { describe, expect, it } from "vitest";
import type { GradeParameters } from "../core/wasm";
import { AbSlots } from "./abSlots";

const g = (contrast: number): GradeParameters => ({ stage2: { contrast } });

describe("AbSlots", () => {
  it("starts on A and copies the live grade into a never-used B", () => {
    const s = new AbSlots();
    expect(s.active).toBe("A");
    const step = s.toggle(g(10));
    expect(step.active).toBe("B");
    expect(step.apply).toEqual(g(10));
    expect(step.label).toBe("Switch to grade B");
  });

  it("swaps between two independent grades", () => {
    const s = new AbSlots();
    s.toggle(g(10)); // now B, edit B to 40
    const back = s.toggle(g(40));
    expect(back.active).toBe("A");
    expect(back.apply).toEqual(g(10));
    expect(s.toggle(g(10)).apply).toEqual(g(40));
  });

  it("copies A to B from either slot", () => {
    const s = new AbSlots();
    expect(s.copyAToB(g(5))).toMatchObject({ active: "A", apply: null });
    expect(s.toggle(g(5)).apply).toEqual(g(5)); // B got A's grade
    s.toggle(g(5)); // back to A, B parked
    s.toggle(g(5)); // to B
    const copy = s.copyAToB(g(99));
    expect(copy.apply).toEqual(g(5));
  });

  it("forgets everything on reset", () => {
    const s = new AbSlots();
    s.toggle(g(1));
    s.reset();
    expect(s.active).toBe("A");
    expect(s.otherParked).toBe(false);
  });
});
