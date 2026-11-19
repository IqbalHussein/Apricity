/** Pan/zoom state, matching the engine's `ViewParams` (zoom 1 = fit). */
export interface View {
  /** Zoom relative to fit, 1..8. */
  zoom: number;
  /** Pan as a fraction of the displayed image size; (0,0) is centered. */
  pan: [number, number];
}

/** Axis-aligned rectangle in CSS pixels. */
export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** Lowest zoom (fit). */
export const MIN_ZOOM = 1;
/** Highest zoom. */
export const MAX_ZOOM = 8;

/**
 * Where the image lands in the canvas; mirrors the Rust `place` function.
 *
 * @param aspect - Image width / height.
 * @param cw - Canvas width in CSS pixels.
 * @param ch - Canvas height in CSS pixels.
 * @param view - Current view.
 * @returns The image rectangle in CSS pixels (may extend beyond the canvas).
 */
export function imageRect(aspect: number, cw: number, ch: number, view: View): Rect {
  const fitW = Math.min(cw, ch * aspect);
  const w = fitW * view.zoom;
  const h = (fitW / aspect) * view.zoom;
  const cx = cw / 2 + view.pan[0] * w;
  const cy = ch / 2 + view.pan[1] * h;
  return { x: cx - w / 2, y: cy - h / 2, w, h };
}

/**
 * Clamp the pan so the image keeps covering the canvas along any axis where
 * it is larger than the canvas, and stays centered otherwise.
 *
 * @param aspect - Image width / height.
 * @param cw - Canvas width in CSS pixels.
 * @param ch - Canvas height in CSS pixels.
 * @param view - View to clamp.
 * @returns A new, clamped view.
 */
export function clampView(aspect: number, cw: number, ch: number, view: View): View {
  const zoom = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, view.zoom));
  const fitW = Math.min(cw, ch * aspect);
  const dims: [number, number] = [fitW * zoom, (fitW / aspect) * zoom];
  const canvas: [number, number] = [cw, ch];
  const pan: [number, number] = [0, 0];
  for (const i of [0, 1] as const) {
    const d = dims[i];
    const limit = d > canvas[i] ? (d - canvas[i]) / (2 * d) : 0;
    pan[i] = Math.min(limit, Math.max(-limit, view.pan[i])) + 0; // + 0 turns -0 into 0
  }
  return { zoom, pan };
}

/**
 * Zoom by `factor` keeping the image point under `cursor` fixed.
 *
 * @param view - Current view.
 * @param factor - Multiplier applied to zoom (clamped to 1..8 overall).
 * @param cursor - Cursor position in CSS pixels relative to the canvas.
 * @param aspect - Image width / height.
 * @param cw - Canvas width in CSS pixels.
 * @param ch - Canvas height in CSS pixels.
 * @returns The new, clamped view.
 */
export function zoomAt(
  view: View,
  factor: number,
  cursor: { x: number; y: number },
  aspect: number,
  cw: number,
  ch: number,
): View {
  const zoom = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, view.zoom * factor));
  const fitW = Math.min(cw, ch * aspect);
  const before: [number, number] = [fitW * view.zoom, (fitW / aspect) * view.zoom];
  const after: [number, number] = [fitW * zoom, (fitW / aspect) * zoom];
  const rel: [number, number] = [cursor.x - cw / 2, cursor.y - ch / 2];
  const pan: [number, number] = [0, 0];
  for (const i of [0, 1] as const) {
    const n = rel[i] / before[i] - view.pan[i];
    pan[i] = rel[i] / after[i] - n;
  }
  return clampView(aspect, cw, ch, { zoom, pan });
}
