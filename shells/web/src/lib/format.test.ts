import { describe, expect, it } from "vitest";
import { decimalsForStep, formatValue, parseNumber, sentenceCase, valueText } from "./format";

describe("value formatting", () => {
  it("derives decimals from the step", () => {
    expect(decimalsForStep(1)).toBe(0);
    expect(decimalsForStep(0.5)).toBe(1);
    expect(decimalsForStep(0.01)).toBe(2);
    expect(decimalsForStep(0.0005)).toBe(4);
  });

  it("signs bipolar values and keeps units separate", () => {
    expect(formatValue({ unit: "stops", step: 0.01, min: -4 }, 0.3)).toEqual({
      num: "+0.30",
      unit: "EV",
    });
    expect(formatValue({ unit: "percent", step: 1, min: -100 }, -25)).toEqual({
      num: "−25",
      unit: "%",
    });
    expect(formatValue({ unit: "percent", step: 1, min: 0 }, 40)).toEqual({ num: "40", unit: "%" });
  });

  it("speaks value and unit for aria-valuetext", () => {
    expect(valueText({ unit: "stops", step: 0.01, min: -4 }, 0.3)).toBe("+0.30 stops");
  });

  it("parses typed numbers", () => {
    expect(parseNumber("−2,5")).toBe(-2.5);
    expect(parseNumber("12 %")).toBe(12);
    expect(parseNumber("abc")).toBeNull();
  });

  it("sentence-cases group names", () => {
    expect(sentenceCase("Split Tone")).toBe("Split tone");
    expect(sentenceCase("HSL")).toBe("HSL");
  });
});
