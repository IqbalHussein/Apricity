import { describe, expect, it } from "vitest";
import {
  capSize,
  exportFilename,
  lutFilename,
  qualityToUnit,
  watermarkLayout,
} from "./size";

describe("capSize", () => {
  it("leaves images within the cap alone", () => {
    expect(capSize(1536, 1024, 2048)).toEqual({ width: 1536, height: 1024 });
    expect(capSize(6000, 4000, null)).toEqual({ width: 6000, height: 4000 });
  });
  it("scales the long edge down to the cap, keeping the aspect ratio", () => {
    expect(capSize(6000, 4000, 2048)).toEqual({ width: 2048, height: 1365 });
    expect(capSize(3000, 6000, 2048)).toEqual({ width: 1024, height: 2048 });
  });
  it("never produces an empty image", () => {
    expect(capSize(10000, 1, 2048).height).toBe(1);
  });
});

describe("filenames", () => {
  it("replaces the original extension", () => {
    expect(exportFilename("IMG_0042.jpeg", "jpeg")).toBe("IMG_0042-graded.jpg");
    expect(exportFilename("holiday.final.PNG", "webp")).toBe("holiday.final-graded.webp");
    expect(exportFilename("Sample image", "png")).toBe("Sample image-graded.png");
    expect(exportFilename("", "jpeg")).toBe("image-graded.jpg");
  });
  it("names LUTs after the image", () => {
    expect(lutFilename("shot.png")).toBe("shot-graded.cube");
  });
});

describe("quality and watermark", () => {
  it("maps quality to the encoder range", () => {
    expect(qualityToUnit(92)).toBe(0.92);
    expect(qualityToUnit(0)).toBe(0.01);
    expect(qualityToUnit(500)).toBe(1);
    expect(qualityToUnit(Number.NaN)).toBe(0.92);
  });
  it("anchors the watermark inside the bottom-right corner", () => {
    const l = watermarkLayout(2048, 1365);
    expect(l.x).toBeLessThan(2048);
    expect(l.y).toBeLessThan(1365);
    expect(l.fontPx).toBeGreaterThanOrEqual(12);
    expect(watermarkLayout(100, 60).fontPx).toBe(12);
  });
});
