import { watermarkLayout, WATERMARK_TEXT } from "./size";

/**
 * Draw the Free-tier "Graded with Apricity" mark in the bottom-right corner.
 * Shared by image and video export so both look the same.
 *
 * @param ctx - 2D context of the output canvas.
 * @param w - Output width in pixels.
 * @param h - Output height in pixels.
 */
export function drawWatermark(
  ctx: CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D,
  w: number,
  h: number,
): void {
  const { fontPx, x, y } = watermarkLayout(w, h);
  ctx.save();
  ctx.font = `500 ${fontPx}px Archivo, system-ui, sans-serif`;
  ctx.textAlign = "right";
  ctx.textBaseline = "alphabetic";
  ctx.shadowColor = "rgba(0,0,0,0.55)";
  ctx.shadowBlur = fontPx * 0.35;
  ctx.fillStyle = "rgba(255,255,255,0.88)";
  ctx.fillText(WATERMARK_TEXT, x, y);
  ctx.restore();
}
