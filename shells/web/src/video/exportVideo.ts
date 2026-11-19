import { ArrayBufferTarget, Muxer } from "mp4-muxer";
import { Engine } from "../core/wasm";
import type { GradeParameters } from "../core/wasm";
import { drawWatermark } from "../export/watermark";
import { getClock } from "../state/clock";
import type { Clock } from "../state/clock";
import { demux } from "./mp4";
import type { Demuxed } from "./mp4";
import {
  chooseBitrate,
  CODEC_LABEL,
  encoderCandidates,
  estimateProgress,
  ExportCancelled,
  KEYFRAME_INTERVAL_S,
  matrixToFloats,
  MAX_QUEUE,
  MUXER_CODEC,
  pickEncoder,
  ResourceBag,
  sampleTiming,
  throwIfAborted,
  UNSUPPORTED_VIDEO_MESSAGE,
  videoExportSize,
  videoFilename,
  waitForRoom,
} from "./model";
import type { ProgressEstimate, VideoCodecFamily } from "./model";
import { toChunkInit } from "./mp4";

/** Inputs of a video export. */
export interface VideoExportOptions {
  /** The original MP4/MOV file. */
  file: Blob;
  /** Display name of the file, used for the output name. */
  name: string;
  /** Aborting releases every codec and the engine and rejects with {@link ExportCancelled}. */
  signal: AbortSignal;
  /** Grade to apply (Stage 1 and Stage 2 parameters, as in the Rust core's schema). */
  params: GradeParameters;
  /** Text of the active `.cube` LUT, or `null`. */
  lutText: string | null;
  /** Cap on the longer output side in pixels (Free: 2048), or `null` for none. */
  longEdge: number | null;
  /** Draw the Free-tier mark. */
  watermark: boolean;
  /** Called after each frame with the progress numbers. */
  onProgress: (p: VideoProgress) => void;
  /** Time source for elapsed time and the estimate. */
  clock?: Clock;
}

/** Progress report during an export. */
export interface VideoProgress extends ProgressEstimate {
  /** Frames graded and handed to the encoder. */
  done: number;
  /** Frames in the source. */
  total: number;
  /** Milliseconds since the export started. */
  elapsedMs: number;
}

/** What a finished export produced. */
export interface VideoExportResult {
  /** File name, `<name>-graded.mp4`. */
  filename: string;
  /** The MP4 file. */
  blob: Blob;
  /** Output width in pixels. */
  width: number;
  /** Output height in pixels. */
  height: number;
  /** Frames written. */
  frames: number;
  /** Codec family that encoded the video. */
  codec: VideoCodecFamily;
  /** `copied` when the AAC track was passed through, `none` when the source has no audio, `dropped` when its audio is not AAC. */
  audio: "copied" | "none" | "dropped";
  /** Total export time in milliseconds. */
  elapsedMs: number;
  /** Whether the Free-tier mark was drawn. */
  watermarked: boolean;
}

/**
 * Whether this browser exposes WebCodecs video encode and decode.
 *
 * @returns `true` when `VideoEncoder` and `VideoDecoder` exist.
 */
export function hasWebCodecs(): boolean {
  return typeof VideoEncoder !== "undefined" && typeof VideoDecoder !== "undefined";
}

/**
 * One-sentence summary shown after a finished export.
 *
 * @param r - The export result.
 * @returns For example `Saved clip-graded.mp4, 1280 × 720 px, 30 frames, encoded as H.264.`
 */
export function resultMessage(r: VideoExportResult): string {
  const audio =
    r.audio === "copied"
      ? " Audio was copied as it was."
      : r.audio === "dropped"
        ? " The audio isn't AAC, so it was left out."
        : "";
  return `Saved ${r.filename}, ${r.width} × ${r.height} px, ${r.frames} frames, encoded as ${CODEC_LABEL[r.codec]}.${audio}`;
}

/**
 * Grade a video file frame by frame and write a new MP4.
 *
 * Pipeline: mp4box demux, `VideoDecoder`, a dedicated GPU `Engine` on a hidden
 * canvas at the output size, `VideoEncoder` (H.264, else VP9), `mp4-muxer`.
 * AAC audio is copied without re-encoding. Decoder and encoder queues stay
 * below {@link MAX_QUEUE}. Every WebGPU canvas read happens in the same task as
 * `render()`.
 *
 * @param o - Options.
 * @returns The finished file and facts about it.
 * @throws ExportCancelled when `o.signal` aborts; Error with user-facing text otherwise.
 */
export async function exportVideo(o: VideoExportOptions): Promise<VideoExportResult> {
  const clock = o.clock ?? getClock();
  const startMs = clock.nowMs();
  const bag = new ResourceBag();
  try {
    throwIfAborted(o.signal);
    const dm = await demux(o.file);
    throwIfAborted(o.signal);
    return await run(o, dm, bag, clock, startMs);
  } finally {
    bag.closeAll();
  }
}

async function run(
  o: VideoExportOptions,
  dm: Demuxed,
  bag: ResourceBag,
  clock: Clock,
  startMs: number,
): Promise<VideoExportResult> {
  const v = dm.video;
  const out = videoExportSize(v.width, v.height, o.longEdge);
  const bitrate = chooseBitrate(v.bitrate, v.width * v.height, out.width * out.height);
  const total = dm.videoSamples.length;
  if (total === 0) throw new Error("This file has no video frames.");

  const decoderConfig: VideoDecoderConfig = {
    codec: v.codec,
    codedWidth: v.width,
    codedHeight: v.height,
    ...(v.description ? { description: v.description } : {}),
  };
  const decSupport = await VideoDecoder.isConfigSupported(decoderConfig).catch(() => null);
  if (!decSupport?.supported) throw new Error(UNSUPPORTED_VIDEO_MESSAGE);

  const encoderBase = (codec: string): VideoEncoderConfig => ({
    codec,
    width: out.width,
    height: out.height,
    bitrate,
    framerate: v.fps,
    latencyMode: "quality",
  });
  const choice = await pickEncoder(encoderCandidates(out.width, out.height, v.fps), async (c) => {
    const cfg =
      c.family === "h264"
        ? { ...encoderBase(c.codec), avc: { format: "avc" as const } }
        : encoderBase(c.codec);
    return (await VideoEncoder.isConfigSupported(cfg)).supported === true;
  });
  if (!choice) {
    throw new Error("This browser can't encode H.264 or VP9 video. Use current Chrome or Edge.");
  }
  throwIfAborted(o.signal);

  // Dedicated engine on a hidden canvas at the output size (GPU only).
  const gpuCanvas = document.createElement("canvas");
  const engine = await Engine.create(gpuCanvas, false);
  bag.add(() => engine.free());
  if (engine.backend() !== "webgpu") throw new Error("Video export needs WebGPU.");
  engine.setParams(o.params);
  engine.setLut(o.lutText);
  engine.setOverlay("none", 0.95);
  engine.resize(out.width, out.height);
  engine.setView({ zoom: 1, pan: [0, 0], exact: true });
  throwIfAborted(o.signal);

  let flat: HTMLCanvasElement | null = null;
  let flatCtx: CanvasRenderingContext2D | null = null;
  if (o.watermark) {
    flat = document.createElement("canvas");
    flat.width = out.width;
    flat.height = out.height;
    flatCtx = flat.getContext("2d");
    if (!flatCtx) throw new Error("2D canvas unavailable");
  }

  const audioInfo = dm.audio;
  const copyAudio = audioInfo !== null && audioInfo.description !== null;
  const identity = v.matrix === null || isIdentity(v.matrix);
  const target = new ArrayBufferTarget();
  const muxer = new Muxer({
    target,
    video: {
      codec: MUXER_CODEC[choice.family],
      width: out.width,
      height: out.height,
      ...(identity ? {} : { rotation: matrixToFloats(v.matrix as number[]) }),
    },
    ...(copyAudio && audioInfo
      ? {
          audio: {
            codec: "aac" as const,
            numberOfChannels: audioInfo.channels,
            sampleRate: audioInfo.sampleRate,
          },
        }
      : {}),
    fastStart: "in-memory",
    firstTimestampBehavior: "cross-track-offset",
  });

  const state: { fatal: Error | null } = { fatal: null };
  let wake: (() => void) | null = null;
  const waitForChange = (): Promise<void> =>
    new Promise<void>((resolve) => {
      const t = window.setTimeout(resolve, 25);
      wake = () => {
        window.clearTimeout(t);
        resolve();
      };
    });
  const poke = (): void => wake?.();

  const encoder = new VideoEncoder({
    output: (chunk, meta) => muxer.addVideoChunk(chunk, meta),
    error: (e) => {
      state.fatal = new Error(`The video encoder failed: ${e.message}`);
      poke();
    },
  });
  bag.add(() => {
    if (encoder.state !== "closed") encoder.close();
  });
  encoder.addEventListener("dequeue", poke);
  encoder.configure(
    choice.family === "h264"
      ? { ...encoderBase(choice.codec), avc: { format: "avc" } }
      : encoderBase(choice.codec),
  );

  const pending: VideoFrame[] = [];
  const decoder = new VideoDecoder({
    output: (f) => {
      pending.push(f);
      poke();
    },
    error: (e) => {
      state.fatal = new Error(`The video decoder failed: ${e.message}`);
      poke();
    },
  });
  bag.add(() => {
    for (const f of pending.splice(0)) f.close();
    if (decoder.state !== "closed") decoder.close();
  });
  decoder.addEventListener("dequeue", poke);
  decoder.configure(decoderConfig);

  const check = (): void => {
    throwIfAborted(o.signal);
    if (state.fatal) throw state.fatal;
  };
  const gop = Math.max(1, Math.round(v.fps * KEYFRAME_INTERVAL_S));
  const frameDurationUs = Math.round(1_000_000 / v.fps);
  const readVideo = dm.openReader();
  let nextSample = 0;
  let done = 0;

  const report = (): void => {
    const elapsedMs = clock.nowMs() - startMs;
    o.onProgress({ done, total, elapsedMs, ...estimateProgress({ done, total, elapsedMs }) });
  };

  const gradeAndEncode = async (frame: VideoFrame): Promise<void> => {
    const timestamp = frame.timestamp;
    const duration = frame.duration ?? frameDurationUs;
    let bitmap: ImageBitmap;
    try {
      bitmap = await createImageBitmap(frame);
    } finally {
      frame.close();
    }
    check();
    // From here to `encode` everything is synchronous: the WebGPU canvas is only readable in render()'s task.
    engine.setSourceBitmap(bitmap);
    bitmap.close();
    engine.render();
    let source: CanvasImageSource = gpuCanvas;
    if (flat && flatCtx) {
      flatCtx.drawImage(gpuCanvas, 0, 0);
      drawWatermark(flatCtx, out.width, out.height);
      source = flat;
    }
    const graded = new VideoFrame(source, { timestamp, duration });
    try {
      encoder.encode(graded, { keyFrame: done % gop === 0 });
    } finally {
      graded.close();
    }
    done += 1;
    report();
  };

  report();
  while (done < total) {
    check();
    while (
      nextSample < total &&
      decoder.decodeQueueSize < MAX_QUEUE &&
      pending.length < MAX_QUEUE
    ) {
      const s = dm.videoSamples[nextSample] as (typeof dm.videoSamples)[number];
      nextSample += 1;
      decoder.decode(new EncodedVideoChunk(toChunkInit(s, await readVideo(s))));
      check();
    }
    const frame = pending.shift();
    if (frame) {
      await waitForRoom(() => encoder.encodeQueueSize, MAX_QUEUE, waitForChange, o.signal);
      await gradeAndEncode(frame);
    } else if (nextSample >= total) {
      await decoder.flush();
      check();
      if (pending.length === 0) break; // the decoder produced fewer frames than samples
    } else {
      await waitForChange();
    }
  }
  check();
  await encoder.flush();
  check();

  let audio: VideoExportResult["audio"] = "none";
  if (audioInfo) {
    if (copyAudio) {
      const readAudio = dm.openReader();
      const cfg = {
        decoderConfig: {
          codec: audioInfo.codec,
          numberOfChannels: audioInfo.channels,
          sampleRate: audioInfo.sampleRate,
          description: audioInfo.description as Uint8Array,
        },
      };
      for (let i = 0; i < dm.audioSamples.length; i++) {
        const s = dm.audioSamples[i] as (typeof dm.audioSamples)[number];
        const t = sampleTiming(s.cts, s.dts, s.duration, s.timescale);
        muxer.addAudioChunkRaw(await readAudio(s), "key", t.timestamp, t.duration, i === 0 ? cfg : undefined);
        if (i % 256 === 255) check();
      }
      audio = "copied";
    } else {
      audio = "dropped";
    }
  }
  check();
  muxer.finalize();
  const blob = new Blob([target.buffer], { type: "video/mp4" });
  return {
    filename: videoFilename(o.name),
    blob,
    width: out.width,
    height: out.height,
    frames: done,
    codec: choice.family,
    audio,
    elapsedMs: clock.nowMs() - startMs,
    watermarked: o.watermark,
  };
}

function isIdentity(m: readonly number[]): boolean {
  return m[0] === 0x10000 && m[1] === 0 && m[3] === 0 && m[4] === 0x10000 && m[6] === 0 && m[7] === 0;
}

export { ExportCancelled };
