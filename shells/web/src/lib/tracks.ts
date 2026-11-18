import type { ScalarSpec } from "../core/wasm";
import { planckianGradient } from "./kelvin";

const hsl = (h: number, s: number, l: number): string =>
  `hsl(${((h % 360) + 360) % 360} ${s}% ${l}%)`;

/**
 * Functional slider track: a CSS background that shows what the slider does
 * (ui-spec principle 2). Left is the minimum, right the maximum.
 *
 * @param spec - Parameter spec; its path decides the track.
 * @param hslCentersDeg - Band center hues in degrees, from the spec table, in band order.
 * @param hslBands - Band names in the same order as `hslCentersDeg`.
 * @returns A CSS `background` value, or `null` for a neutral track.
 */
export function trackBackground(
  spec: Pick<ScalarSpec, "path">,
  hslCentersDeg: readonly number[],
  hslBands: readonly string[],
): string | null {
  const p = spec.path;
  if (p.endsWith(".temperature")) return planckianGradient("to left");
  if (p.endsWith(".tint")) {
    return "linear-gradient(to right, rgb(52 168 83), rgb(128 128 128) 50%, rgb(204 62 168))";
  }
  if (p === "stage2.saturation" || p === "stage2.vibrance") {
    return "linear-gradient(to right, rgb(128 128 128), rgb(214 120 90) 45%, rgb(176 92 200) 72%, rgb(72 152 232))";
  }
  if (p === "stage1.exposure" || p === "stage2.exposure") {
    return "linear-gradient(to right, rgb(16 16 16), rgb(238 238 238))";
  }
  if (p === "stage1.black_point.r") return "linear-gradient(to right, rgb(0 0 0), rgb(220 70 70))";
  if (p === "stage1.black_point.g") return "linear-gradient(to right, rgb(0 0 0), rgb(60 170 90))";
  if (p === "stage1.black_point.b") return "linear-gradient(to right, rgb(0 0 0), rgb(80 120 230))";
  if (p.endsWith("_hue") && p.startsWith("stage2.split_tone")) {
    const stops = Array.from(
      { length: 7 },
      (_, i) => `${hsl(i * 60, 70, 55)} ${((i / 6) * 100).toFixed(1)}%`,
    );
    return `linear-gradient(to right, ${stops.join(", ")})`;
  }
  const m = /^stage2\.hsl\.(\w+)\.(hue|saturation|luminance)$/.exec(p);
  if (m) {
    const idx = hslBands.indexOf(m[1] as string);
    const c = hslCentersDeg[idx];
    if (c === undefined) return null;
    if (m[2] === "hue")
      return `linear-gradient(to right, ${hsl(c - 30, 75, 52)}, ${hsl(c, 75, 52)}, ${hsl(c + 30, 75, 52)})`;
    if (m[2] === "saturation")
      return `linear-gradient(to right, rgb(128 128 128), ${hsl(c, 80, 52)})`;
    return `linear-gradient(to right, ${hsl(c, 70, 16)}, ${hsl(c, 80, 50)} 50%, ${hsl(c, 80, 86)})`;
  }
  return null;
}
