import type { CurvePoint } from "../core/wasm";

/** Minimum horizontal gap between neighbouring curve points (normalized). */
export const MIN_X_GAP = 0.01;
/** Largest number of control points the editor lets the user create. */
export const MAX_POINTS = 16;

/**
 * The identity curve used when a channel has no points.
 *
 * @returns Two points, (0,0) and (1,1), both in the working encoding [0, 1].
 */
export function identityPoints(): CurvePoint[] {
  return [
    { x: 0, y: 0 },
    { x: 1, y: 1 },
  ];
}

/**
 * Points to edit for a channel: the stored ones, or identity when empty.
 *
 * @param points - Stored control points (working encoding, [0, 1]).
 * @returns A sorted copy with at least two points.
 */
export function editablePoints(points: readonly CurvePoint[] | undefined): CurvePoint[] {
  if (!points || points.length < 2) return identityPoints();
  return points.map((p) => ({ ...p })).sort((a, b) => a.x - b.x);
}

/**
 * Whether a curve is the identity (so it can be stored as an empty list).
 *
 * @param points - Control points, [0, 1] in both axes.
 * @returns `true` when the curve is the straight diagonal.
 */
export function isIdentity(points: readonly CurvePoint[]): boolean {
  return (
    points.length === 2 &&
    points.every((p) => Math.abs(p.x - p.y) < 1e-6) &&
    points[0]?.x === 0 &&
    points[1]?.x === 1
  );
}

/**
 * Insert a point, keeping the list sorted.
 *
 * @param points - Sorted control points.
 * @param x - Input value, [0, 1].
 * @param y - Output value, [0, 1].
 * @returns A new list and the index of the new point; unchanged when full or too close to a neighbour.
 */
export function addPoint(
  points: readonly CurvePoint[],
  x: number,
  y: number,
): { points: CurvePoint[]; index: number } {
  const cx = Math.min(1, Math.max(0, x));
  const cy = Math.min(1, Math.max(0, y));
  const near = points.findIndex((p) => Math.abs(p.x - cx) < MIN_X_GAP);
  if (near >= 0 || points.length >= MAX_POINTS)
    return { points: points.map((p) => ({ ...p })), index: Math.max(near, 0) };
  const next = [...points.map((p) => ({ ...p })), { x: cx, y: cy }].sort((a, b) => a.x - b.x);
  return { points: next, index: next.findIndex((p) => p.x === cx && p.y === cy) };
}

/**
 * Move a point. End points keep their x; inner points stay between neighbours.
 *
 * @param points - Sorted control points.
 * @param index - Point to move.
 * @param x - Requested input value, [0, 1].
 * @param y - Requested output value, [0, 1].
 * @returns A new list.
 */
export function movePoint(
  points: readonly CurvePoint[],
  index: number,
  x: number,
  y: number,
): CurvePoint[] {
  const next = points.map((p) => ({ ...p }));
  const p = next[index];
  if (!p) return next;
  const last = next.length - 1;
  if (index !== 0 && index !== last) {
    const lo = (next[index - 1]?.x ?? 0) + MIN_X_GAP;
    const hi = (next[index + 1]?.x ?? 1) - MIN_X_GAP;
    p.x = Math.min(hi, Math.max(lo, x));
  }
  p.y = Math.min(1, Math.max(0, y));
  return next;
}

/**
 * Remove an inner point (the two end points always stay).
 *
 * @param points - Sorted control points.
 * @param index - Point to remove.
 * @returns A new list.
 */
export function removePoint(points: readonly CurvePoint[], index: number): CurvePoint[] {
  const next = points.map((p) => ({ ...p }));
  if (index > 0 && index < next.length - 1) next.splice(index, 1);
  return next;
}

/**
 * Display-only sampling of the monotone cubic (Fritsch-Carlson) through the
 * points, for drawing the editor path. The render core owns the real curve.
 *
 * @param points - Sorted control points, working encoding [0, 1].
 * @param samples - Number of samples across x.
 * @returns Polyline vertices, both axes [0, 1].
 */
export function sampleCurve(points: readonly CurvePoint[], samples = 64): CurvePoint[] {
  const n = points.length;
  if (n < 2) return identityPoints();
  const dx: number[] = [];
  const m: number[] = [];
  for (let i = 0; i < n - 1; i++) {
    const a = points[i] as CurvePoint;
    const b = points[i + 1] as CurvePoint;
    dx.push(b.x - a.x);
    m.push((b.y - a.y) / (b.x - a.x));
  }
  const t: number[] = [m[0] ?? 0];
  for (let i = 1; i < n - 1; i++) {
    const m0 = m[i - 1] as number;
    const m1 = m[i] as number;
    t.push(m0 * m1 <= 0 ? 0 : (m0 + m1) / 2);
  }
  t.push(m[n - 2] ?? 0);
  for (let i = 0; i < n - 1; i++) {
    const mi = m[i] as number;
    if (mi === 0) {
      t[i] = 0;
      t[i + 1] = 0;
      continue;
    }
    const a = (t[i] as number) / mi;
    const b = (t[i + 1] as number) / mi;
    const s = a * a + b * b;
    if (s > 9) {
      const k = 3 / Math.sqrt(s);
      t[i] = k * a * mi;
      t[i + 1] = k * b * mi;
    }
  }
  const out: CurvePoint[] = [];
  const first = points[0] as CurvePoint;
  const last = points[n - 1] as CurvePoint;
  let seg = 0;
  for (let s = 0; s <= samples; s++) {
    const x = s / samples;
    if (x <= first.x) {
      out.push({ x, y: first.y });
      continue;
    }
    if (x >= last.x) {
      out.push({ x, y: last.y });
      continue;
    }
    while (seg < n - 2 && x > (points[seg + 1] as CurvePoint).x) seg++;
    const p0 = points[seg] as CurvePoint;
    const p1 = points[seg + 1] as CurvePoint;
    const h = dx[seg] as number;
    const u = (x - p0.x) / h;
    const u2 = u * u;
    const u3 = u2 * u;
    const y =
      (2 * u3 - 3 * u2 + 1) * p0.y +
      (u3 - 2 * u2 + u) * h * (t[seg] as number) +
      (-2 * u3 + 3 * u2) * p1.y +
      (u3 - u2) * h * (t[seg + 1] as number);
    out.push({ x, y: Math.min(1, Math.max(0, y)) });
  }
  return out;
}
