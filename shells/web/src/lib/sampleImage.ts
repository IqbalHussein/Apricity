import type { ImageProxy } from "./imageProxy";

/** Width of the generated sample, in pixels. */
export const SAMPLE_WIDTH = 1536;
/** Height of the generated sample, in pixels. */
export const SAMPLE_HEIGHT = 1024;

/** Deterministic hash noise in [0, 1); keeps the sample reproducible without an RNG. */
function hashNoise(x: number, y: number): number {
  let h = Math.imul(x, 374761393) ^ Math.imul(y, 668265263);
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
}

/**
 * Paint a procedural test scene (dusk sky, hills, lake, sun, skin tones and a
 * patch strip) in sRGB. Deliberately a little flat and warm so grades show.
 *
 * @param ctx - 2D context of a canvas sized {@link SAMPLE_WIDTH} by {@link SAMPLE_HEIGHT}.
 */
function paintScene(ctx: CanvasRenderingContext2D): void {
  const W = SAMPLE_WIDTH;
  const H = SAMPLE_HEIGHT;
  const horizon = Math.round(H * 0.5);
  const sky = ctx.createLinearGradient(0, 0, 0, horizon);
  sky.addColorStop(0, "#5f7ea6");
  sky.addColorStop(0.55, "#c4a99c");
  sky.addColorStop(1, "#f0c48e");
  ctx.fillStyle = sky;
  ctx.fillRect(0, 0, W, horizon);

  const sun = ctx.createRadialGradient(W * 0.68, horizon - 40, 4, W * 0.68, horizon - 40, 260);
  sun.addColorStop(0, "rgba(255,246,222,1)");
  sun.addColorStop(0.12, "rgba(255,226,170,0.95)");
  sun.addColorStop(1, "rgba(255,200,140,0)");
  ctx.fillStyle = sun;
  ctx.fillRect(0, 0, W, horizon + 40);

  const ridge = (base: number, amp: number, phase: number, color: string): void => {
    ctx.fillStyle = color;
    ctx.beginPath();
    ctx.moveTo(0, horizon);
    for (let x = 0; x <= W; x += 8) {
      const y =
        base -
        amp *
          (0.55 * Math.sin(x / 190 + phase) +
            0.3 * Math.sin(x / 71 + phase * 2) +
            0.15 * Math.sin(x / 29));
      ctx.lineTo(x, y);
    }
    ctx.lineTo(W, horizon);
    ctx.closePath();
    ctx.fill();
  };
  ridge(horizon - 70, 55, 0.4, "#7d7f8c");
  ridge(horizon - 28, 38, 2.1, "#56655f");
  ridge(horizon - 4, 16, 4.0, "#3b4a3d");

  const lake = ctx.createLinearGradient(0, horizon, 0, H * 0.82);
  lake.addColorStop(0, "#c9a58a");
  lake.addColorStop(1, "#46596f");
  ctx.fillStyle = lake;
  ctx.fillRect(0, horizon, W, H * 0.82 - horizon);
  ctx.fillStyle = "rgba(255,226,170,0.35)";
  for (let i = 0; i < 26; i++) {
    const y = horizon + 14 + i * 9;
    const half = 120 - i * 3.2;
    ctx.fillRect(W * 0.68 - half, y, half * 2, 2);
  }

  ctx.fillStyle = "#2f3b30";
  ctx.fillRect(0, H * 0.82, W, H * 0.18);

  const stripY = Math.round(H * 0.84);
  const stripH = Math.round(H * 0.12);
  const patches = [
    "#734f3d",
    "#c39279",
    "#657a9d",
    "#5a7a45",
    "#8f84b8",
    "#6dbfa8",
    "#d98a2c",
    "#4b5ea6",
    "#bd5360",
    "#5b3c6a",
    "#9bba3c",
    "#e3a82e",
  ];
  const pw = Math.floor((W * 0.62) / patches.length);
  patches.forEach((c, i) => {
    ctx.fillStyle = c;
    ctx.fillRect(40 + i * pw, stripY, pw - 6, stripH);
  });
  const ramp = ctx.createLinearGradient(W * 0.68, 0, W - 40, 0);
  ramp.addColorStop(0, "#050505");
  ramp.addColorStop(1, "#fafafa");
  ctx.fillStyle = ramp;
  ctx.fillRect(W * 0.68, stripY, W * 0.32 - 40, stripH);

  const skin = ctx.createRadialGradient(W * 0.2, H * 0.62, 6, W * 0.2, H * 0.62, 70);
  skin.addColorStop(0, "#e1ad90");
  skin.addColorStop(1, "#b97f63");
  ctx.fillStyle = skin;
  ctx.beginPath();
  ctx.arc(W * 0.2, H * 0.62, 70, 0, Math.PI * 2);
  ctx.fill();
}

/**
 * Generate the built-in sample image entirely in code (works offline).
 *
 * @returns A preview proxy in sRGB RGBA8; full-resolution equals proxy size.
 */
export function generateSampleImage(): ImageProxy {
  const canvas = document.createElement("canvas");
  canvas.width = SAMPLE_WIDTH;
  canvas.height = SAMPLE_HEIGHT;
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) throw new Error("2D canvas unavailable");
  paintScene(ctx);
  const img = ctx.getImageData(0, 0, SAMPLE_WIDTH, SAMPLE_HEIGHT);
  const d = img.data;
  for (let y = 0; y < SAMPLE_HEIGHT; y++) {
    for (let x = 0; x < SAMPLE_WIDTH; x++) {
      const i = (y * SAMPLE_WIDTH + x) * 4;
      const n = (hashNoise(x, y) - 0.5) * 5;
      d[i] = Math.min(255, Math.max(0, (d[i] as number) + n));
      d[i + 1] = Math.min(255, Math.max(0, (d[i + 1] as number) + n));
      d[i + 2] = Math.min(255, Math.max(0, (d[i + 2] as number) + n));
    }
  }
  return {
    rgba: new Uint8Array(d.buffer, d.byteOffset, d.byteLength),
    width: SAMPLE_WIDTH,
    height: SAMPLE_HEIGHT,
    fullWidth: SAMPLE_WIDTH,
    fullHeight: SAMPLE_HEIGHT,
  };
}
