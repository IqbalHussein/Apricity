/**
 * Box-filter downscale of an sRGB RGBA8 image (averaging encoded values is
 * adequate for a small preview thumbnail).
 *
 * @param rgba - Source pixels, row-major, `w * h * 4` bytes, sRGB-encoded.
 * @param w - Source width in pixels.
 * @param h - Source height in pixels.
 * @param maxSide - Longest side of the result in pixels.
 * @returns The scaled pixels and their size; the source itself when it already fits.
 */
export function downscaleRgba(
  rgba: Uint8Array,
  w: number,
  h: number,
  maxSide: number,
): { rgba: Uint8Array; width: number; height: number } {
  const scale = Math.min(1, maxSide / Math.max(w, h));
  if (scale >= 1) return { rgba, width: w, height: h };
  const ow = Math.max(1, Math.round(w * scale));
  const oh = Math.max(1, Math.round(h * scale));
  const out = new Uint8Array(ow * oh * 4);
  for (let y = 0; y < oh; y++) {
    const y0 = Math.floor((y * h) / oh);
    const y1 = Math.max(y0 + 1, Math.floor(((y + 1) * h) / oh));
    for (let x = 0; x < ow; x++) {
      const x0 = Math.floor((x * w) / ow);
      const x1 = Math.max(x0 + 1, Math.floor(((x + 1) * w) / ow));
      let r = 0;
      let g = 0;
      let b = 0;
      let n = 0;
      for (let yy = y0; yy < y1; yy++) {
        for (let xx = x0; xx < x1; xx++) {
          const i = (yy * w + xx) * 4;
          r += rgba[i] as number;
          g += rgba[i + 1] as number;
          b += rgba[i + 2] as number;
          n++;
        }
      }
      const o = (y * ow + x) * 4;
      out[o] = Math.round(r / n);
      out[o + 1] = Math.round(g / n);
      out[o + 2] = Math.round(b / n);
      out[o + 3] = 255;
    }
  }
  return { rgba: out, width: ow, height: oh };
}

/**
 * Encode RGBA8 pixels as a JPEG `data:` URL through a 2D canvas.
 *
 * @param rgba - sRGB RGBA8 pixels, `w * h * 4` bytes.
 * @param w - Width in pixels.
 * @param h - Height in pixels.
 * @param quality - JPEG quality, 0..1.
 * @returns A `data:image/jpeg;base64,...` URL.
 */
export function rgbaToDataUrl(rgba: Uint8Array, w: number, h: number, quality = 0.82): string {
  const canvas = document.createElement("canvas");
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("2D canvas unavailable");
  ctx.putImageData(new ImageData(new Uint8ClampedArray(rgba), w, h), 0, 0);
  return canvas.toDataURL("image/jpeg", quality);
}
