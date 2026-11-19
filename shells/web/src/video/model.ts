/**
 * Pure helpers for video import and export: codec and bitrate choice, size
 * caps, timestamp math, progress estimates and cleanup bookkeeping. Nothing
 * here touches the DOM, WebCodecs or the clock, so all of it is unit-tested.
 */

/** `accept` list for the import picker: images plus MP4/MOV video. */
export const MEDIA_ACCEPT =
  "image/jpeg,image/png,image/webp,image/heic,image/heif,video/mp4,video/quicktime,.jpg,.jpeg,.png,.webp,.heic,.heif,.mp4,.m4v,.mov";

/**
 * Whether a file should be opened as video.
 *
 * @param file - Name and MIME type of the picked or dropped file.
 * @returns `true` for MP4/MOV containers (by MIME type, falling back to the extension).
 */
export function isVideoFile(file: { name: string; type: string }): boolean {
  if (file.type.startsWith("video/")) return true;
  return /\.(mp4|m4v|mov)$/i.test(file.name);
}

/** Message for a video the browser cannot decode. */
export const UNSUPPORTED_VIDEO_MESSAGE =
  "This browser can't play this video. Use an H.264 MP4 or MOV.";

/** Message for the export tab when WebCodecs is missing. */
export const NO_WEBCODECS_MESSAGE =
  "Video export needs a browser with WebCodecs, like current Chrome or Edge.";

/** Message for the export tab on the CPU renderer. */
export const CPU_VIDEO_MESSAGE =
  "Video export needs WebGPU. The CPU renderer is too slow for video, so only the preview works here.";

/** Longest side of the scope and analysis proxy grabbed from a video frame, in pixels. */
export const VIDEO_PROXY_SIDE = 320;
/** Minimum gap between proxy grabs while a video plays, in milliseconds (5 Hz). */
export const VIDEO_PROXY_INTERVAL_MS = 200;
/** Most frames the export decoder and encoder may hold in flight each. */
export const MAX_QUEUE = 8;
/** Seconds between forced keyframes in exported video. */
export const KEYFRAME_INTERVAL_S = 2;

/**
 * Round a pixel size down to even numbers (H.264 needs even dimensions).
 *
 * @param width - Width in pixels.
 * @param height - Height in pixels.
 * @returns Each side rounded down to a multiple of 2, at least 2.
 */
export function evenSize(width: number, height: number): { width: number; height: number } {
  const even = (v: number): number => Math.max(2, Math.floor(v / 2) * 2);
  return { width: even(width), height: even(height) };
}

/**
 * Export size after the tier's long-edge cap, aspect ratio kept, even sides.
 *
 * @param width - Source width in pixels.
 * @param height - Source height in pixels.
 * @param maxLongEdge - Cap on the longer side in pixels, or `null` for none.
 * @returns Output size in pixels.
 */
export function videoExportSize(
  width: number,
  height: number,
  maxLongEdge: number | null,
): { width: number; height: number } {
  const long = Math.max(width, height);
  if (maxLongEdge === null || long <= maxLongEdge) return evenSize(width, height);
  const scale = maxLongEdge / long;
  return evenSize(Math.round(width * scale), Math.round(height * scale));
}

/** Codec family used for the exported video track. */
export type VideoCodecFamily = "h264" | "vp9";

/** One encoder configuration to try. */
export interface EncoderCandidate {
  /** WebCodecs codec string, e.g. `avc1.640028`. */
  codec: string;
  /** Family, which picks the muxer codec and the label. */
  family: VideoCodecFamily;
}

/** Display name for a codec family. */
export const CODEC_LABEL: Readonly<Record<VideoCodecFamily, string>> = {
  h264: "H.264",
  vp9: "VP9",
};

/** Muxer codec id for a family. */
export const MUXER_CODEC: Readonly<Record<VideoCodecFamily, "avc" | "vp9">> = {
  h264: "avc",
  vp9: "vp9",
};

/** H.264 levels (H.264 Annex A, Table A-1): id x10, max frame size and macroblock rate. */
const H264_LEVELS: readonly { level: number; maxFs: number; maxMbps: number }[] = [
  { level: 40, maxFs: 8192, maxMbps: 245760 },
  { level: 41, maxFs: 8192, maxMbps: 245760 },
  { level: 42, maxFs: 8704, maxMbps: 522240 },
  { level: 50, maxFs: 22080, maxMbps: 589824 },
  { level: 51, maxFs: 36864, maxMbps: 983040 },
  { level: 52, maxFs: 36864, maxMbps: 2073600 },
  { level: 60, maxFs: 139264, maxMbps: 4177920 },
  { level: 61, maxFs: 139264, maxMbps: 8355840 },
  { level: 62, maxFs: 139264, maxMbps: 16711680 },
];

/**
 * Smallest H.264 level (at least 4.0) that carries a picture size and rate.
 *
 * @param width - Width in pixels.
 * @param height - Height in pixels.
 * @param fps - Frame rate in frames per second.
 * @returns Level times ten (40 = level 4.0, 52 = level 5.2).
 */
export function h264Level(width: number, height: number, fps: number): number {
  const mbs = Math.ceil(width / 16) * Math.ceil(height / 16);
  const rate = mbs * Math.max(1, fps);
  const hit = H264_LEVELS.find((l) => mbs <= l.maxFs && rate <= l.maxMbps);
  return (hit ?? (H264_LEVELS[H264_LEVELS.length - 1] as { level: number })).level;
}

/**
 * WebCodecs string for H.264 High profile at a level.
 *
 * @param level - Level times ten (40, 42, 51 ...).
 * @returns For example `avc1.640028` for level 4.0.
 */
export function h264CodecString(level: number): string {
  return `avc1.6400${level.toString(16).toUpperCase().padStart(2, "0")}`;
}

/**
 * Encoder configurations in preference order: H.264 at the level the size
 * needs (then one level up), then VP9 in MP4 as the fallback.
 *
 * @param width - Output width in pixels.
 * @param height - Output height in pixels.
 * @param fps - Frame rate in frames per second.
 * @returns Candidates, best first.
 */
export function encoderCandidates(width: number, height: number, fps: number): EncoderCandidate[] {
  const level = h264Level(width, height, fps);
  const higher = H264_LEVELS.find((l) => l.level > level)?.level;
  const out: EncoderCandidate[] = [{ codec: h264CodecString(level), family: "h264" }];
  if (higher !== undefined) out.push({ codec: h264CodecString(higher), family: "h264" });
  out.push({ codec: "vp09.00.10.08", family: "vp9" });
  out.push({ codec: "vp09.00.40.08", family: "vp9" });
  out.push({ codec: "vp09.00.50.08", family: "vp9" });
  return out;
}

/**
 * First candidate the browser can encode.
 *
 * @param candidates - Configurations in preference order.
 * @param isSupported - Asks the browser (wraps `VideoEncoder.isConfigSupported`).
 * @returns The first supported candidate, or `null` when none is.
 */
export async function pickEncoder(
  candidates: readonly EncoderCandidate[],
  isSupported: (c: EncoderCandidate) => Promise<boolean>,
): Promise<EncoderCandidate | null> {
  for (const c of candidates) {
    try {
      if (await isSupported(c)) return c;
    } catch {
      // an unsupported string may throw instead of answering false
    }
  }
  return null;
}

/**
 * Target bitrate: the source's bitrate scaled by the pixel ratio, or 8 Mbps at
 * 1080p scaled by pixels when the source's rate is unknown.
 *
 * @param sourceBitsPerSecond - Source video bitrate in bits per second, or 0 / `null` when unknown.
 * @param sourcePixels - Source frame area in pixels.
 * @param outputPixels - Output frame area in pixels.
 * @returns Bits per second, clamped to 1 to 80 Mbps.
 */
export function chooseBitrate(
  sourceBitsPerSecond: number | null,
  sourcePixels: number,
  outputPixels: number,
): number {
  const ratio = outputPixels / Math.max(1, sourcePixels);
  const raw =
    sourceBitsPerSecond && sourceBitsPerSecond > 0
      ? sourceBitsPerSecond * ratio
      : 8_000_000 * (outputPixels / (1920 * 1080));
  return Math.round(Math.min(80_000_000, Math.max(1_000_000, raw)));
}

/** Timing of one sample in microseconds, as WebCodecs wants it. */
export interface SampleTiming {
  /** Presentation time in microseconds. */
  timestamp: number;
  /** Decode time in microseconds. */
  dts: number;
  /** Duration in microseconds. */
  duration: number;
}

/**
 * Convert a container sample's timing to microseconds.
 *
 * @param cts - Composition (presentation) time in track timescale units.
 * @param dts - Decode time in track timescale units.
 * @param duration - Duration in track timescale units.
 * @param timescale - Track timescale in units per second.
 * @returns Timing in microseconds, rounded to whole microseconds.
 */
export function sampleTiming(
  cts: number,
  dts: number,
  duration: number,
  timescale: number,
): SampleTiming {
  const us = (v: number): number => Math.round((v * 1_000_000) / timescale);
  return { timestamp: us(cts), dts: us(dts), duration: us(duration) };
}

/**
 * Average frame rate of a track.
 *
 * @param frames - Number of samples.
 * @param durationUnits - Track duration in timescale units.
 * @param timescale - Track timescale in units per second.
 * @returns Frames per second; 30 when the inputs do not describe a duration.
 */
export function averageFps(frames: number, durationUnits: number, timescale: number): number {
  if (frames <= 0 || durationUnits <= 0 || timescale <= 0) return 30;
  return (frames * timescale) / durationUnits;
}

/** Export progress as shown in the dialog. */
export interface ProgressSnapshot {
  /** Frames graded and encoded so far. */
  done: number;
  /** Frames in the source. */
  total: number;
  /** Milliseconds since the export started (injected clock). */
  elapsedMs: number;
}

/** Derived progress numbers. */
export interface ProgressEstimate {
  /** Completed fraction, 0..1. */
  fraction: number;
  /** Throughput in frames per second; 0 before the first frame. */
  fps: number;
  /** Estimated milliseconds remaining, or `null` before there is a rate to extrapolate. */
  etaMs: number | null;
}

/**
 * Fraction, rate and time remaining from a progress snapshot.
 *
 * @param p - Frames done, total and elapsed milliseconds.
 * @returns The estimate; the ETA is `null` until a frame completed and some time passed.
 */
export function estimateProgress(p: ProgressSnapshot): ProgressEstimate {
  const total = Math.max(1, p.total);
  const done = Math.min(total, Math.max(0, p.done));
  const fraction = done / total;
  if (done === 0 || p.elapsedMs <= 0) return { fraction, fps: 0, etaMs: null };
  const fps = (done * 1000) / p.elapsedMs;
  return { fraction, fps, etaMs: Math.round(((total - done) * 1000) / fps) };
}

/**
 * Clock text for transport and progress, e.g. `0:07.4` or `1:02:03`.
 *
 * @param seconds - Time in seconds (negative and non-finite values show zero).
 * @param withTenths - Append one decimal for tenths of a second (ignored past an hour).
 * @returns `m:ss`, `m:ss.t` or `h:mm:ss`.
 */
export function formatClock(seconds: number, withTenths = false): string {
  const s = Number.isFinite(seconds) && seconds > 0 ? seconds : 0;
  const whole = Math.floor(s);
  const h = Math.floor(whole / 3600);
  const m = Math.floor((whole % 3600) / 60);
  const sec = whole % 60;
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(sec).padStart(2, "0")}`;
  const tenths = withTenths ? `.${Math.floor((s - whole) * 10)}` : "";
  return `${m}:${String(sec).padStart(2, "0")}${tenths}`;
}

/**
 * File name for an exported video.
 *
 * @param sourceName - Name of the opened file.
 * @returns `<original-name>-graded.mp4`.
 */
export function videoFilename(sourceName: string): string {
  const base = sourceName.replace(/\.[^./\\]+$/, "").trim() || "video";
  return `${base}-graded.mp4`;
}

/**
 * Seconds one frame step moves the playhead.
 *
 * @param fps - Frame rate in frames per second (falls back to 30 when unknown).
 * @returns Seconds per frame.
 */
export function frameStepSeconds(fps: number): number {
  return 1 / (Number.isFinite(fps) && fps > 0 ? fps : 30);
}

/**
 * Target time of a frame step.
 *
 * @param current - Playhead in seconds.
 * @param direction - `1` forward, `-1` back.
 * @param fps - Frame rate in frames per second.
 * @param duration - Clip duration in seconds.
 * @returns New playhead in seconds, clamped to the clip. Lands at the middle of the target frame so it never rounds back into the old one.
 */
export function stepTime(current: number, direction: 1 | -1, fps: number, duration: number): number {
  const step = frameStepSeconds(fps);
  const index = Math.round(current / step - 0.5) + direction;
  return Math.min(Math.max(0, duration - step / 2), Math.max(0, (index + 0.5) * step));
}

/**
 * Convert an MP4 `tkhd`/`mvhd` matrix (16.16 and 2.30 fixed point) to the
 * floats the muxer expects.
 *
 * @param m - Nine raw matrix entries in file order.
 * @returns Nine floats in the same order.
 */
export function matrixToFloats(m: ArrayLike<number>): [number, number, number, number, number, number, number, number, number] {
  const f = (i: number, shift: number): number => (m[i] as number) / shift;
  const a = 65536;
  const b = 1073741824;
  return [f(0, a), f(1, a), f(2, b), f(3, a), f(4, a), f(5, b), f(6, a), f(7, a), f(8, b)];
}

/**
 * Collects cleanup callbacks so a cancelled or failed export releases every
 * codec, frame and engine exactly once, in reverse order of acquisition.
 */
export class ResourceBag {
  private readonly closers: (() => void)[] = [];
  private closed = false;

  /**
   * Register a resource.
   *
   * @param closer - Releases the resource; errors are swallowed so the rest still run.
   * @returns The same function, so it can be stored.
   */
  add(closer: () => void): () => void {
    if (this.closed) closer();
    else this.closers.push(closer);
    return closer;
  }

  /** Run every registered closer, newest first. Safe to call more than once. */
  closeAll(): void {
    if (this.closed) return;
    this.closed = true;
    while (this.closers.length > 0) {
      try {
        (this.closers.pop() as () => void)();
      } catch {
        // keep releasing the remaining resources
      }
    }
  }
}

/** Thrown when an export is cancelled. */
export class ExportCancelled extends Error {
  /** Creates the cancellation error. */
  constructor() {
    super("Export cancelled.");
    this.name = "ExportCancelled";
  }
}

/**
 * Throw when a signal has been aborted.
 *
 * @param signal - Cancellation signal.
 * @throws ExportCancelled when aborted.
 */
export function throwIfAborted(signal: AbortSignal): void {
  if (signal.aborted) throw new ExportCancelled();
}

/**
 * Wait until a queue has room, waking on dequeue events.
 *
 * @param queue - Reads the current queue length.
 * @param limit - Largest length that still counts as room is `limit - 1`.
 * @param waitForChange - Resolves when the queue may have changed (a dequeue event or a short timer).
 * @param signal - Cancellation signal.
 * @throws ExportCancelled when aborted while waiting.
 */
export async function waitForRoom(
  queue: () => number,
  limit: number,
  waitForChange: () => Promise<void>,
  signal: AbortSignal,
): Promise<void> {
  while (queue() >= limit) {
    throwIfAborted(signal);
    await waitForChange();
  }
  throwIfAborted(signal);
}
