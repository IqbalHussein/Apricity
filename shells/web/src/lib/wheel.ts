/** Phase of each RGB channel on the hue circle, degrees (R=0, G=120, B=240). */
const PHASE = [0, 120, 240] as const;
const RAD = Math.PI / 180;

/** Puck position on a color wheel. */
export interface WheelPos {
  /** Hue in **degrees** 0..360, clockwise from the top (red), HSV-style hue. */
  hue: number;
  /** Normalized radius 0..1 (0 = center, 1 = rim). */
  sat: number;
}

/**
 * Map a puck position to per-channel offsets with zero mean.
 *
 * `c_i = A * cos(hue - phase_i)` with `A = sat * maxOffset`; the three cosines
 * sum to zero, so the wheel never changes overall brightness (that is the
 * `master` channel's job). Units are the wheel channels' own normalized
 * offsets (see `ColorWheel`).
 *
 * @param hue - Hue in degrees.
 * @param sat - Normalized radius 0..1 (clamped).
 * @param maxOffset - Largest per-channel offset at the rim (the spec max, e.g. 0.5).
 * @returns `[r, g, b]` offsets.
 */
export function wheelToRgb(hue: number, sat: number, maxOffset: number): [number, number, number] {
  const a = Math.min(1, Math.max(0, sat)) * maxOffset;
  return [
    a * Math.cos((hue - PHASE[0]) * RAD),
    a * Math.cos((hue - PHASE[1]) * RAD),
    a * Math.cos((hue - PHASE[2]) * RAD),
  ];
}

/**
 * Inverse of {@link wheelToRgb}: project offsets onto the wheel. Any mean
 * (common-mode) component of the input is discarded.
 *
 * @param r - Red offset, normalized.
 * @param g - Green offset, normalized.
 * @param b - Blue offset, normalized.
 * @param maxOffset - Offset at the rim (the spec max, e.g. 0.5).
 * @returns Hue in degrees [0, 360) and radius 0..1.
 */
export function rgbToWheel(r: number, g: number, b: number, maxOffset: number): WheelPos {
  const x = (2 * r - g - b) / 3;
  const y = (g - b) / Math.sqrt(3);
  const amp = Math.hypot(x, y);
  let hue = Math.atan2(y, x) / RAD;
  if (hue < 0) hue += 360;
  return { hue: amp < 1e-9 ? 0 : hue, sat: maxOffset > 0 ? Math.min(1, amp / maxOffset) : 0 };
}

/**
 * Convert a pointer offset from the wheel center to a puck position.
 *
 * @param dx - Pixels right of center.
 * @param dy - Pixels below center.
 * @param radius - Wheel radius in pixels.
 * @returns Hue (clockwise from top) and radius clamped to the rim.
 */
export function pointToWheel(dx: number, dy: number, radius: number): WheelPos {
  const sat = Math.min(1, Math.hypot(dx, dy) / radius);
  let hue = Math.atan2(dx, -dy) / RAD;
  if (hue < 0) hue += 360;
  return { hue, sat };
}

/**
 * Inverse of {@link pointToWheel}.
 *
 * @param pos - Puck position.
 * @param radius - Wheel radius in pixels.
 * @returns Offset in pixels from the wheel center (x right, y down).
 */
export function wheelToPoint(pos: WheelPos, radius: number): { x: number; y: number } {
  return {
    x: Math.sin(pos.hue * RAD) * pos.sat * radius,
    y: -Math.cos(pos.hue * RAD) * pos.sat * radius,
  };
}
