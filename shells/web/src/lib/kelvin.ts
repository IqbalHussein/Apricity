/** An sRGB color, each channel 0..255. */
export interface Rgb8 {
  r: number;
  g: number;
  b: number;
}

/** One gradient stop of the Planckian ramp. */
export interface PlanckianStop {
  /** Color temperature in kelvin. */
  kelvin: number;
  /** Position along the ramp, 0..1 (uniform in mireds, so the ramp looks even). */
  pos: number;
  /** Approximate display color of a blackbody at `kelvin`, sRGB-encoded 0..255. */
  rgb: Rgb8;
}

const clamp255 = (v: number): number => Math.min(255, Math.max(0, Math.round(v)));

/**
 * Approximate sRGB color of a blackbody radiator (display swatch only, not
 * used for any color math).
 *
 * @param kelvin - Correlated color temperature in kelvin; clamped to 1000..40000.
 * @returns sRGB-encoded color, channels 0..255.
 */
export function kelvinToRgb(kelvin: number): Rgb8 {
  const t = Math.min(40000, Math.max(1000, kelvin)) / 100;
  const r = t <= 66 ? 255 : 329.698727446 * (t - 60) ** -0.1332047592;
  const g =
    t <= 66
      ? 99.4708025861 * Math.log(t) - 161.1195681661
      : 288.1221695283 * (t - 60) ** -0.0755148492;
  const b = t >= 66 ? 255 : t <= 19 ? 0 : 138.5177312231 * Math.log(t - 10) - 305.0447927307;
  return { r: clamp255(r), g: clamp255(g), b: clamp255(b) };
}

/**
 * Sample the Planckian ramp, evenly spaced in mireds (1e6 / kelvin).
 *
 * @param count - Number of stops, at least 2.
 * @param kelvinMin - Warm end in kelvin (default 2000).
 * @param kelvinMax - Cool end in kelvin (default 10000).
 * @returns Stops ordered by ascending kelvin (amber first); `pos` runs 0..1.
 */
export function planckianStops(count = 12, kelvinMin = 2000, kelvinMax = 10000): PlanckianStop[] {
  const n = Math.max(2, Math.floor(count));
  const miredWarm = 1e6 / kelvinMin;
  const miredCool = 1e6 / kelvinMax;
  const out: PlanckianStop[] = [];
  for (let i = 0; i < n; i++) {
    const pos = i / (n - 1);
    const mired = miredWarm + (miredCool - miredWarm) * pos;
    const kelvin = 1e6 / mired;
    out.push({ kelvin, pos, rgb: kelvinToRgb(kelvin) });
  }
  return out;
}

/**
 * CSS `linear-gradient` for the Planckian ramp.
 *
 * @param direction - CSS direction keyword, e.g. `"to right"` (amber to blue)
 *   or `"to left"` (amber on the right).
 * @param count - Number of stops.
 * @returns A CSS background-image value.
 */
export function planckianGradient(
  direction: "to right" | "to left" = "to right",
  count = 12,
): string {
  const parts = planckianStops(count).map(
    (s) => `rgb(${s.rgb.r} ${s.rgb.g} ${s.rgb.b}) ${(s.pos * 100).toFixed(1)}%`,
  );
  return `linear-gradient(${direction}, ${parts.join(", ")})`;
}
