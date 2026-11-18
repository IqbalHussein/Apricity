import { describe, expect, it } from "vitest";
import {
  addPoint,
  editablePoints,
  identityPoints,
  isIdentity,
  MIN_X_GAP,
  movePoint,
  removePoint,
  sampleCurve,
} from "./curve";

describe("curve editing", () => {
  it("treats an empty curve as identity", () => {
    expect(editablePoints([])).toEqual(identityPoints());
    expect(isIdentity(identityPoints())).toBe(true);
  });

  it("adds points in sorted order and refuses near-duplicates", () => {
    const a = addPoint(identityPoints(), 0.5, 0.7);
    expect(a.points.map((p) => p.x)).toEqual([0, 0.5, 1]);
    expect(a.index).toBe(1);
    const b = addPoint(a.points, 0.505, 0.1);
    expect(b.points).toHaveLength(3);
  });

  it("keeps inner points between neighbours and end points at their x", () => {
    const pts = [
      { x: 0, y: 0 },
      { x: 0.5, y: 0.5 },
      { x: 1, y: 1 },
    ];
    expect(movePoint(pts, 1, 2, 2)[1]).toEqual({ x: 1 - MIN_X_GAP, y: 1 });
    expect(movePoint(pts, 0, 0.4, 0.2)[0]).toEqual({ x: 0, y: 0.2 });
  });

  it("removes only inner points", () => {
    const pts = [
      { x: 0, y: 0 },
      { x: 0.5, y: 0.5 },
      { x: 1, y: 1 },
    ];
    expect(removePoint(pts, 1)).toHaveLength(2);
    expect(removePoint(pts, 0)).toHaveLength(3);
  });

  it("samples a monotone curve without overshoot", () => {
    const pts = [
      { x: 0, y: 0 },
      { x: 0.2, y: 0.05 },
      { x: 0.8, y: 0.95 },
      { x: 1, y: 1 },
    ];
    const s = sampleCurve(pts, 100);
    for (let i = 1; i < s.length; i++) {
      expect((s[i] as { y: number }).y).toBeGreaterThanOrEqual(
        (s[i - 1] as { y: number }).y - 1e-9,
      );
    }
    expect(Math.max(...s.map((p) => p.y))).toBeLessThanOrEqual(1);
  });
});
