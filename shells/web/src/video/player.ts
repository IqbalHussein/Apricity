import { createSignal } from "solid-js";
import type { ImageProxy } from "../lib/imageProxy";
import { PROXY_MAX_SIDE } from "../lib/imageProxy";
import { stepTime, UNSUPPORTED_VIDEO_MESSAGE } from "./model";

/** A frame the preview video presented. */
export interface FrameEvent {
  /** `true` when the video is paused: the frame came from a seek or a pause, not from playback. */
  settled: boolean;
}

const [time, setTime] = createSignal(0);
const [duration, setDuration] = createSignal(0);
const [playing, setPlaying] = createSignal(false);
const [muted, setMutedSignal] = createSignal(true);
const [looping, setLoopingSignal] = createSignal(false);
const [settledFrames, setSettledFrames] = createSignal(0);
const [hasAudio, setHasAudio] = createSignal(false);

let video: HTMLVideoElement | null = null;
let objectUrl: string | null = null;
let fps = 30;
let listener: ((e: FrameEvent) => void) | null = null;
let callbackId = 0;
let scratch: HTMLCanvasElement | null = null;

/** Playhead in seconds (tracked). */
export const videoTime = time;
/** Clip duration in seconds, 0 until a video is open (tracked). */
export const videoDuration = duration;
/** `true` while the preview plays (tracked). */
export const videoPlaying = playing;
/** `true` while the preview is muted (tracked). Muted by default. */
export const videoMuted = muted;
/** `true` when the clip loops (tracked). */
export const videoLooping = looping;
/** Counter that increases whenever a paused or seeked frame is presented (tracked); recipe thumbnails follow it. */
export const settledFrameCount = settledFrames;
/** `true` when the open clip has an audio track the preview can play (tracked). */
export const videoHasAudio = hasAudio;

/**
 * The hidden preview `<video>` element, or `null` when no video is open.
 *
 * @returns The element the engine reads frames from.
 */
export function videoElement(): HTMLVideoElement | null {
  return video;
}

/**
 * Receive an event for every frame the preview presents.
 *
 * @param fn - Handler, or `null` to stop.
 */
export function setFrameListener(fn: ((e: FrameEvent) => void) | null): void {
  listener = fn;
}

function emit(): void {
  if (!video) return;
  const settled = video.paused;
  if (settled) setSettledFrames((n) => n + 1);
  listener?.({ settled });
}

function scheduleFrameCallback(): void {
  if (!video || !("requestVideoFrameCallback" in video)) return;
  callbackId = video.requestVideoFrameCallback((_now, meta) => {
    setTime(meta.mediaTime);
    emit();
    scheduleFrameCallback();
  });
}

/**
 * Release the open video: pause, drop the element and revoke its URL.
 * Safe to call when nothing is open.
 */
export function closeVideo(): void {
  if (video) {
    if (callbackId && "cancelVideoFrameCallback" in video) video.cancelVideoFrameCallback(callbackId);
    video.pause();
    video.removeAttribute("src");
    video.load();
  }
  if (objectUrl) URL.revokeObjectURL(objectUrl);
  video = null;
  objectUrl = null;
  callbackId = 0;
  setTime(0);
  setDuration(0);
  setPlaying(false);
  setHasAudio(false);
}

/**
 * Draw the current frame to a scratch canvas and read it back.
 *
 * @param maxSide - Longest side of the result in pixels (never upscales).
 * @returns sRGB RGBA8 pixels, top row first, or `null` when no frame is available.
 */
export function grabFrame(
  maxSide: number,
): { rgba: Uint8Array; width: number; height: number } | null {
  const v = video;
  if (!v || v.readyState < 2 || v.videoWidth === 0) return null;
  const scale = Math.min(1, maxSide / Math.max(v.videoWidth, v.videoHeight));
  const w = Math.max(1, Math.round(v.videoWidth * scale));
  const h = Math.max(1, Math.round(v.videoHeight * scale));
  scratch ??= document.createElement("canvas");
  if (scratch.width !== w || scratch.height !== h) {
    scratch.width = w;
    scratch.height = h;
  }
  const ctx = scratch.getContext("2d", { willReadFrequently: true });
  if (!ctx) return null;
  ctx.drawImage(v, 0, 0, w, h);
  const data = ctx.getImageData(0, 0, w, h).data;
  return { rgba: new Uint8Array(data.buffer.slice(0)), width: w, height: h };
}

/** Facts about an opened clip. */
export interface OpenedVideo {
  /** First-frame proxy at most 2048 px on the longest side, with the video's display size as full size. */
  proxy: ImageProxy;
  /** Clip duration in seconds. */
  durationS: number;
}

/**
 * Open a video file in the hidden preview element and wait for its first frame.
 *
 * @param file - MP4 or MOV.
 * @param frameRate - Frames per second, used for frame stepping.
 * @returns The first-frame proxy and the duration.
 * @throws Error with {@link UNSUPPORTED_VIDEO_MESSAGE} when the browser cannot decode the video.
 */
export async function openVideo(file: Blob, frameRate: number): Promise<OpenedVideo> {
  closeVideo();
  fps = frameRate > 0 ? frameRate : 30;
  const el = document.createElement("video");
  el.muted = true;
  el.playsInline = true;
  el.preload = "auto";
  el.loop = looping();
  setMutedSignal(true);
  const url = URL.createObjectURL(file);
  objectUrl = url;
  video = el;
  try {
    await new Promise<void>((resolve, reject) => {
      el.addEventListener("loadeddata", () => resolve(), { once: true });
      el.addEventListener("error", () => reject(new Error(UNSUPPORTED_VIDEO_MESSAGE)), {
        once: true,
      });
      el.src = url;
    });
    // loadeddata can fire before the first frame is presentable; a zero seek settles it.
    if (el.readyState < 2) throw new Error(UNSUPPORTED_VIDEO_MESSAGE);
  } catch (e) {
    closeVideo();
    throw e;
  }
  setDuration(Number.isFinite(el.duration) ? el.duration : 0);
  setTime(0);
  setPlaying(false);
  setHasAudio(audioPresent(el));
  el.addEventListener("play", () => setPlaying(true));
  el.addEventListener("pause", () => {
    setPlaying(false);
    setTime(el.currentTime);
    emit();
  });
  el.addEventListener("ended", () => setPlaying(false));
  el.addEventListener("seeked", () => {
    setTime(el.currentTime);
    emit();
  });
  scheduleFrameCallback();
  const frame = grabFrame(PROXY_MAX_SIDE);
  if (!frame) {
    closeVideo();
    throw new Error(UNSUPPORTED_VIDEO_MESSAGE);
  }
  return {
    proxy: {
      rgba: frame.rgba,
      width: frame.width,
      height: frame.height,
      fullWidth: el.videoWidth,
      fullHeight: el.videoHeight,
    },
    durationS: el.duration,
  };
}

function audioPresent(el: HTMLVideoElement): boolean {
  const anyEl = el as HTMLVideoElement & {
    mozHasAudio?: boolean;
    webkitAudioDecodedByteCount?: number;
    audioTracks?: { length: number };
  };
  if (anyEl.audioTracks) return anyEl.audioTracks.length > 0;
  if (typeof anyEl.mozHasAudio === "boolean") return anyEl.mozHasAudio;
  return true;
}

/** Play when paused, pause when playing. Restarts from the beginning at the end of a clip. */
export function togglePlay(): void {
  const v = video;
  if (!v) return;
  if (v.paused) {
    if (v.ended || v.currentTime >= v.duration - 0.001) v.currentTime = 0;
    void v.play().catch(() => setPlaying(false));
  } else {
    v.pause();
  }
}

/**
 * Move the playhead.
 *
 * @param seconds - Target time in seconds; clamped to the clip.
 */
export function seekTo(seconds: number): void {
  const v = video;
  if (!v) return;
  const t = Math.min(Math.max(0, seconds), Math.max(0, v.duration));
  v.currentTime = t;
  setTime(t);
}

/**
 * Step one frame, pausing first.
 *
 * @param direction - `1` forward, `-1` back.
 */
export function stepFrame(direction: 1 | -1): void {
  const v = video;
  if (!v) return;
  v.pause();
  seekTo(stepTime(v.currentTime, direction, fps, v.duration));
}

/**
 * Mute or unmute the preview audio.
 *
 * @param on - `true` to mute.
 */
export function setMuted(on: boolean): void {
  setMutedSignal(on);
  if (video) video.muted = on;
}

/**
 * Turn looping on or off.
 *
 * @param on - `true` to loop.
 */
export function setLooping(on: boolean): void {
  setLoopingSignal(on);
  if (video) video.loop = on;
}

/**
 * Frame rate of the open clip.
 *
 * @returns Frames per second (30 when unknown).
 */
export function videoFps(): number {
  return fps;
}
