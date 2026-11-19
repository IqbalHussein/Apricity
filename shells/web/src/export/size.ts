/** Image export formats. */
export type ImageFormat = "jpeg" | "png" | "webp";

/** Per-format export details. */
export interface FormatInfo {
  /** MIME type passed to `canvas.toBlob`. */
  mime: string;
  /** File extension without the dot. */
  ext: string;
  /** Label in the dialog. */
  label: string;
  /** Whether the quality slider applies. */
  lossy: boolean;
}

/** Export formats in display order. */
export const IMAGE_FORMATS: Readonly<Record<ImageFormat, FormatInfo>> = {
  jpeg: { mime: "image/jpeg", ext: "jpg", label: "JPEG", lossy: true },
  png: { mime: "image/png", ext: "png", label: "PNG", lossy: false },
  webp: { mime: "image/webp", ext: "webp", label: "WebP", lossy: true },
};

/** Default JPEG/WebP quality on a 1..100 scale. */
export const DEFAULT_QUALITY = 92;

/** Text of the Free-tier watermark. */
export const WATERMARK_TEXT = "Graded with Apricity";

/**
 * Largest texture side the GPU export path attempts, in pixels. Larger images
 * use the CPU reference renderer.
 */
export const GPU_EXPORT_MAX_SIDE = 8192;

/**
 * Export size after applying a tier's long-edge cap. Never upscales; the
 * aspect ratio is kept.
 *
 * @param width - Source width in pixels.
 * @param height - Source height in pixels.
 * @param maxLongEdge - Cap on the longer side in pixels, or `null` for no cap.
 * @returns Output size in whole pixels, each at least 1.
 */
export function capSize(
  width: number,
  height: number,
  maxLongEdge: number | null,
): { width: number; height: number } {
  const long = Math.max(width, height);
  if (maxLongEdge === null || long <= maxLongEdge) return { width, height };
  const scale = maxLongEdge / long;
  return {
    width: Math.max(1, Math.round(width * scale)),
    height: Math.max(1, Math.round(height * scale)),
  };
}

/**
 * File name for an exported image.
 *
 * @param sourceName - Name of the opened file, e.g. `IMG_0042.CR3.jpg`.
 * @param format - Export format.
 * @returns `<original-name>-graded.<ext>`, with the original extension removed.
 */
export function exportFilename(sourceName: string, format: ImageFormat): string {
  const base = sourceName.replace(/\.[^./\\]+$/, "").trim() || "image";
  return `${base}-graded.${IMAGE_FORMATS[format].ext}`;
}

/**
 * File name for an exported LUT.
 *
 * @param sourceName - Name of the opened file.
 * @returns `<original-name>-graded.cube`.
 */
export function lutFilename(sourceName: string): string {
  const base = sourceName.replace(/\.[^./\\]+$/, "").trim() || "image";
  return `${base}-graded.cube`;
}

/**
 * Where and how large to draw the watermark.
 *
 * @param width - Output width in pixels.
 * @param height - Output height in pixels.
 * @returns Font size and the bottom-right anchor (text is right-aligned, alphabetic baseline), in pixels.
 */
export function watermarkLayout(
  width: number,
  height: number,
): { fontPx: number; x: number; y: number } {
  const fontPx = Math.max(12, Math.round(Math.min(width, height * 1.5) * 0.026));
  const pad = Math.round(fontPx * 0.9);
  return { fontPx, x: width - pad, y: height - pad };
}

/**
 * Clamp a quality value to the encoder's range.
 *
 * @param q - Quality on a 1..100 scale (any number).
 * @returns Quality as the 0..1 value `toBlob` expects.
 */
export function qualityToUnit(q: number): number {
  const v = Number.isFinite(q) ? q : DEFAULT_QUALITY;
  return Math.min(100, Math.max(1, Math.round(v))) / 100;
}
