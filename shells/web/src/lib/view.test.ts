import { describe, expect, it } from "vitest";
import { clampView, imageRect, MAX_ZOOM, zoomAt } from "./view";

const A = 1.5;
const [W, H] = [900, 600];

describe("view math", () => {
  it("fits the image at zoom 1 and centers it", () => {
    const r = imageRect(A, W, H, { zoom: 1, pan: [0, 0] });
    expect(r).toEqual({ x: 0, y: 0, w: 900, h: 600 });
    const wide = imageRect(2, W, H, { zoom: 1, pan: [0, 0] });
    expect(wide.h).toBe(450);
    expect(wide.y).toBe(75);
  });

  it("keeps the image point under the cursor fixed while zooming", () => {
    const cursor = { x: 700, y: 200 };
    const v0 = { zoom: 2, pan: [0.05, -0.02] as [number, number] };
    const before = imageRect(A, W, H, v0);
    const n = [(cursor.x - before.x) / before.w, (cursor.y - before.y) / before.h];
    const v1 = zoomAt(v0, 1.5, cursor, A, W, H);
    const after = imageRect(A, W, H, v1);
    expect((cursor.x - after.x) / after.w).toBeCloseTo(n[0] as number, 6);
    expect((cursor.y - after.y) / after.h).toBeCloseTo(n[1] as number, 6);
  });

  it("clamps zoom to 1..8 and returns to center at fit", () => {
    const up = zoomAt({ zoom: 1, pan: [0, 0] }, 100, { x: 100, y: 100 }, A, W, H);
    expect(up.zoom).toBe(MAX_ZOOM);
    const down = zoomAt({ zoom: 2, pan: [0.1, 0.1] }, 0.01, { x: 100, y: 100 }, A, W, H);
    expect(down.zoom).toBe(1);
    expect(down.pan).toEqual([0, 0]);
  });

  it("limits pan so the image keeps covering the canvas", () => {
    const v = clampView(A, W, H, { zoom: 2, pan: [5, -5] });
    const r = imageRect(A, W, H, v);
    expect(r.x).toBeCloseTo(0);
    expect(r.y + r.h).toBeCloseTo(H);
  });
});
