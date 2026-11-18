/** A decoded image reduced to a preview proxy. */
export interface ImageProxy {
  /** sRGB-encoded RGBA8, row-major, top row first, `width * height * 4` bytes. */
  rgba: Uint8Array;
  /** Proxy width in pixels. */
  width: number;
  /** Proxy height in pixels. */
  height: number;
  /** Full-resolution width in pixels. */
  fullWidth: number;
  /** Full-resolution height in pixels. */
  fullHeight: number;
}

/** Message shown for files that cannot be decoded (ui-spec copy rules). */
export const UNSUPPORTED_IMAGE_MESSAGE =
  "This file isn't a supported image. Use JPEG, PNG, WebP or HEIC.";

/** Longest side of the preview proxy, in pixels. */
export const PROXY_MAX_SIDE = 2048;

/**
 * Pixels of a decoded bitmap as RGBA8 via a 2D canvas.
 *
 * @param bitmap - Source bitmap (any size).
 * @returns Pixel buffer for the bitmap at its own size.
 */
function readPixels(bitmap: ImageBitmap): Uint8Array {
  const w = bitmap.width;
  const h = bitmap.height;
  const ctx =
    typeof OffscreenCanvas !== "undefined"
      ? new OffscreenCanvas(w, h).getContext("2d", { willReadFrequently: true })
      : Object.assign(document.createElement("canvas"), { width: w, height: h }).getContext("2d", {
          willReadFrequently: true,
        });
  if (!ctx) throw new Error("2D canvas unavailable");
  ctx.drawImage(bitmap, 0, 0);
  const data = ctx.getImageData(0, 0, w, h).data;
  return new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
}

/**
 * Decode an image file to an sRGB RGBA8 proxy no larger than `maxSide`.
 *
 * @param blob - JPEG, PNG, WebP or (where the browser supports it) HEIC.
 * @param maxSide - Longest proxy side in pixels (default 2048).
 * @returns The proxy and the full-resolution dimensions.
 * @throws Error with {@link UNSUPPORTED_IMAGE_MESSAGE} when the file cannot be decoded.
 */
export async function decodeToProxy(blob: Blob, maxSide = PROXY_MAX_SIDE): Promise<ImageProxy> {
  let full: ImageBitmap;
  try {
    full = await createImageBitmap(blob);
  } catch {
    throw new Error(UNSUPPORTED_IMAGE_MESSAGE);
  }
  const fullWidth = full.width;
  const fullHeight = full.height;
  const scale = Math.min(1, maxSide / Math.max(fullWidth, fullHeight));
  let proxy = full;
  if (scale < 1) {
    proxy = await createImageBitmap(full, {
      resizeWidth: Math.max(1, Math.round(fullWidth * scale)),
      resizeHeight: Math.max(1, Math.round(fullHeight * scale)),
      resizeQuality: "high",
    });
    full.close();
  }
  const rgba = readPixels(proxy);
  const out = { rgba, width: proxy.width, height: proxy.height, fullWidth, fullHeight };
  proxy.close();
  return out;
}
