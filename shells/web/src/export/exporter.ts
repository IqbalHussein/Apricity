import { Engine, lutBake, renderCpuRgba } from "../core/wasm";
import type { BakeScope } from "../core/wasm";
import { entitlements } from "../entitlements/store";
import { downloadBlob } from "../lib/download";
import { decodeFull } from "../lib/imageProxy";
import { downscaleRgba } from "../lib/thumbnail";
import { gradeParams } from "../state/grade";
import { activeLut } from "../state/lut";
import { currentSource } from "../state/project";
import type { Source } from "../state/project";
import {
  capSize,
  exportFilename,
  GPU_EXPORT_MAX_SIDE,
  IMAGE_FORMATS,
  lutFilename,
  qualityToUnit,
  watermarkLayout,
  WATERMARK_TEXT,
} from "./size";
import type { ImageFormat } from "./size";

/** Options of an image export. */
export interface ImageExportOptions {
  /** Output format. */
  format: ImageFormat;
  /** Quality for lossy formats, 1..100. */
  quality: number;
}

/** What an export produced. */
export interface ExportResult {
  /** Downloaded file name. */
  filename: string;
  /** Output width in pixels. */
  width: number;
  /** Output height in pixels. */
  height: number;
  /** Which renderer produced the pixels. */
  renderer: "gpu" | "cpu";
  /** File size in bytes. */
  bytes: number;
  /** `true` when the Free-tier watermark was drawn. */
  watermarked: boolean;
}

let exportEngine: Engine | null = null;
let exportCanvas: HTMLCanvasElement | null = null;
let gpuUnavailable = false;

/** Release the hidden export engine (tests, hot reload). */
export function disposeExportEngine(): void {
  exportEngine?.free();
  exportEngine = null;
  exportCanvas = null;
}

async function getExportEngine(): Promise<Engine | null> {
  // `?cpu=1` forces the CPU renderer for the preview and for export alike.
  if (gpuUnavailable || new URLSearchParams(location.search).has("cpu")) return null;
  if (exportEngine) return exportEngine;
  try {
    const canvas = document.createElement("canvas");
    const engine = await Engine.create(canvas, false);
    if (engine.backend() !== "webgpu") {
      engine.free();
      gpuUnavailable = true;
      return null;
    }
    exportCanvas = canvas;
    exportEngine = engine;
    return engine;
  } catch {
    gpuUnavailable = true;
    return null;
  }
}

function nextFrame(): Promise<void> {
  return new Promise((resolve) => {
    const t = window.setTimeout(resolve, 50);
    requestAnimationFrame(() => {
      window.clearTimeout(t);
      window.setTimeout(resolve, 0);
    });
  });
}

function canvasToBlob(canvas: HTMLCanvasElement, mime: string, quality: number): Promise<Blob> {
  return new Promise((resolve, reject) => {
    canvas.toBlob(
      (b) => {
        if (b) resolve(b);
        else reject(new Error("The browser could not encode this image."));
      },
      mime,
      quality,
    );
  });
}

function drawWatermark(ctx: CanvasRenderingContext2D, w: number, h: number): void {
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

/**
 * Render the open image with the current grade to a 2D canvas.
 *
 * GPU path: a WebGPU canvas is readable only in the task that called
 * `render()`, so the 2D copy is made synchronously right after it.
 */
async function renderToCanvas(
  src: Source,
  onProgress: (message: string) => void,
): Promise<{ canvas: HTMLCanvasElement; renderer: "gpu" | "cpu" }> {
  const limits = entitlements.limits();
  const target = capSize(src.proxy.fullWidth, src.proxy.fullHeight, limits.exportLongEdge);
  onProgress("Decoding the original image");
  await nextFrame();
  let img: { rgba: Uint8Array; width: number; height: number };
  if (src.file) {
    img = await decodeFull(src.file, target);
  } else {
    // The generated sample has no file: its proxy is the full image.
    const p = src.proxy;
    const t = capSize(p.width, p.height, limits.exportLongEdge);
    img = downscaleRgba(p.rgba, p.width, p.height, Math.max(t.width, t.height));
  }
  const params = gradeParams();
  const lutText = activeLut()?.text ?? null;

  const out = document.createElement("canvas");
  out.width = img.width;
  out.height = img.height;
  const ctx = out.getContext("2d");
  if (!ctx) throw new Error("2D canvas unavailable");

  const fits = img.width <= GPU_EXPORT_MAX_SIDE && img.height <= GPU_EXPORT_MAX_SIDE;
  if (fits) {
    onProgress("Rendering on the GPU");
    const engine = await getExportEngine();
    if (engine && exportCanvas) {
      try {
        engine.setSourceRgba(img.rgba, img.width, img.height, img.width, img.height);
        engine.setParams(params);
        engine.setLut(lutText);
        engine.setOverlay("none", 0.95);
        engine.resize(img.width, img.height);
        engine.setView({ zoom: 1, pan: [0, 0], exact: true });
        engine.render();
        // Same task as render(): the WebGPU canvas is unreadable after any await.
        ctx.drawImage(exportCanvas, 0, 0);
        engine.setSourceRgba(new Uint8Array(4), 1, 1, 1, 1);
        return { canvas: out, renderer: "gpu" };
      } catch (e) {
        console.warn("GPU export failed, using the CPU renderer", e);
      }
    }
  }

  onProgress(
    `Rendering ${img.width} × ${img.height} on the CPU. This can take a while for large images.`,
  );
  await nextFrame();
  const px = renderCpuRgba(img.rgba, img.width, img.height, params, lutText, img.width);
  const frame = ctx.createImageData(img.width, img.height);
  frame.data.set(px);
  ctx.putImageData(frame, 0, 0);
  return { canvas: out, renderer: "cpu" };
}

/**
 * Export the open image at full resolution (capped per tier) with the current
 * grade, and download it as `<original-name>-graded.<ext>`.
 *
 * @param opts - Format and quality.
 * @param onProgress - Called with a short status sentence as the export advances.
 * @returns What was produced.
 * @throws Error with user-facing copy when nothing is open or the browser cannot encode the format.
 */
export async function exportImage(
  opts: ImageExportOptions,
  onProgress: (message: string) => void = () => undefined,
): Promise<ExportResult> {
  const src = currentSource();
  if (!src || src.kind !== "image") throw new Error("Open an image to export it.");
  const info = IMAGE_FORMATS[opts.format];
  const { canvas, renderer } = await renderToCanvas(src, onProgress);
  const ctx = canvas.getContext("2d");
  const watermarked = entitlements.limits().watermark;
  if (ctx && watermarked) drawWatermark(ctx, canvas.width, canvas.height);
  onProgress(`Encoding ${info.label}`);
  const blob = await canvasToBlob(canvas, info.mime, qualityToUnit(opts.quality));
  if (blob.type !== info.mime)
    throw new Error(`This browser can't encode ${info.label}. Choose JPEG or PNG.`);
  const filename = exportFilename(src.name, opts.format);
  downloadBlob(blob, filename);
  return {
    filename,
    width: canvas.width,
    height: canvas.height,
    renderer,
    bytes: blob.size,
    watermarked,
  };
}

/**
 * Whether the browser can encode a format with `canvas.toBlob`.
 *
 * @param format - Export format.
 * @returns `true` when it can (PNG and JPEG always; WebP is missing in Safari).
 */
export function canEncode(format: ImageFormat): boolean {
  if (format !== "webp") return true;
  try {
    return document.createElement("canvas").toDataURL("image/webp").startsWith("data:image/webp");
  } catch {
    return false;
  }
}

/** Options of a LUT export. */
export interface LutExportOptions {
  /** Grid points per axis. */
  size: 17 | 33 | 65;
  /** `creative` bakes Stage 2 only, `full` the whole pipeline. */
  scope: BakeScope;
}

/**
 * Bake the current grade into a `.cube` 3D LUT and download it.
 *
 * @param opts - Grid size and scope.
 * @returns The downloaded file name.
 * @throws Error when nothing is open or the core rejects the request.
 */
export function exportLut(opts: LutExportOptions): string {
  const src = currentSource();
  if (!src) throw new Error("Open an image to export a LUT.");
  const title = src.name.replace(/\.[^./\\]+$/, "") || "Apricity grade";
  const text = lutBake(gradeParams(), opts.size, opts.scope, title);
  const filename = lutFilename(src.name);
  downloadBlob(new Blob([text], { type: "text/plain" }), filename);
  return filename;
}
