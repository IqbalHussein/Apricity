import type { ScalarSpec, Unit } from "../core/wasm";

/** Display suffix and spoken unit for a parameter unit. */
interface UnitFormat {
  /** Short suffix shown after the number (may be empty). */
  suffix: string;
  /** Word used in `aria-valuetext`. */
  spoken: string;
}

const UNIT_FORMATS: Record<Unit, UnitFormat> = {
  stops: { suffix: "EV", spoken: "stops" },
  mireds: { suffix: "mired", spoken: "mireds" },
  tint: { suffix: "", spoken: "tint" },
  percent: { suffix: "%", spoken: "percent" },
  degrees: { suffix: "°", spoken: "degrees" },
  normalized: { suffix: "", spoken: "" },
  linear: { suffix: "", spoken: "" },
  pixels: { suffix: "px", spoken: "pixels" },
};

/**
 * Number of decimals needed to show multiples of `step` exactly.
 *
 * @param step - Slider step in the parameter's own unit.
 * @returns Decimal places, 0..4.
 */
export function decimalsForStep(step: number): number {
  for (let d = 0; d < 4; d++) {
    const scaled = step * 10 ** d;
    if (Math.abs(scaled - Math.round(scaled)) < 1e-6) return d;
  }
  return 4;
}

/**
 * Format a parameter value for display.
 *
 * @param spec - Supplies unit, step and sign convention (bipolar ranges get an explicit `+`).
 * @param value - Value in the parameter's own unit (stops, mireds, percent, ...).
 * @returns The number text (U+2212 for negatives) and the unit suffix.
 */
export function formatValue(
  spec: Pick<ScalarSpec, "unit" | "step" | "min">,
  value: number,
): { num: string; unit: string } {
  const d = decimalsForStep(spec.step);
  const rounded = Number(value.toFixed(d));
  let num = Math.abs(rounded).toFixed(d);
  if (rounded < 0) num = `−${num}`;
  else if (rounded > 0 && spec.min < 0) num = `+${num}`;
  return { num, unit: UNIT_FORMATS[spec.unit].suffix };
}

/**
 * Text for `aria-valuetext`, e.g. "+0.30 stops".
 *
 * @param spec - Spec supplying unit and step.
 * @param value - Value in the parameter's own unit.
 * @returns Value followed by its spoken unit.
 */
export function valueText(spec: Pick<ScalarSpec, "unit" | "step" | "min">, value: number): string {
  const { num } = formatValue(spec, value);
  const spoken = UNIT_FORMATS[spec.unit].spoken;
  return spoken ? `${num} ${spoken}` : num;
}

/**
 * Parse user-typed numeric text (accepts `-`, U+2212 and a decimal comma).
 *
 * @param text - Raw input text; a trailing unit suffix is ignored.
 * @returns The number, or `null` when the text is not numeric.
 */
export function parseNumber(text: string): number | null {
  const cleaned = text
    .trim()
    .replace(/−/g, "-")
    .replace(",", ".")
    .replace(/[^0-9eE+.-]/g, "");
  if (cleaned === "") return null;
  const n = Number(cleaned);
  return Number.isFinite(n) ? n : null;
}

/**
 * Sentence-case a console group name ("Split Tone" becomes "Split tone"); all-caps names such as "HSL" are kept.
 *
 * @param s - Group heading from the spec table.
 * @returns The heading with only the first letter capitalized.
 */
export function sentenceCase(s: string): string {
  if (s.length === 0 || s === s.toUpperCase()) return s;
  return s.charAt(0).toUpperCase() + s.slice(1).toLowerCase();
}
