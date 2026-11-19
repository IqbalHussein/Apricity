import { createFile, DataStream, Endianness, MP4BoxBuffer } from "mp4box";
import type { ISOFile, Movie, Sample, Track } from "mp4box";
import { sampleTiming } from "./model";

/** One container sample (a video frame or an audio packet), located in the source file. */
export interface SampleRef {
  /** Byte offset of the sample data in the file. */
  offset: number;
  /** Byte length of the sample data. */
  size: number;
  /** Composition (presentation) time, in track timescale units. */
  cts: number;
  /** Decode time, in track timescale units. */
  dts: number;
  /** Duration, in track timescale units. */
  duration: number;
  /** Track timescale, in units per second. */
  timescale: number;
  /** `true` for keyframes (video) and every audio packet. */
  isSync: boolean;
}

/** Facts about the video track. */
export interface VideoTrackInfo {
  /** Track id in the container. */
  id: number;
  /** WebCodecs codec string from the sample entry, e.g. `avc1.640028`. */
  codec: string;
  /** Coded width in pixels. */
  width: number;
  /** Coded height in pixels. */
  height: number;
  /** Average bitrate in bits per second; 0 when unknown. */
  bitrate: number;
  /** Frames per second. */
  fps: number;
  /** Number of frames. */
  frames: number;
  /** Track duration in seconds. */
  durationS: number;
  /** Nine-entry display matrix as floats (rotation), in file order. */
  matrix: readonly number[] | null;
  /** `avcC` / `hvcC` payload for `VideoDecoder.configure`, without the box header. */
  description: Uint8Array | null;
}

/** Facts about the audio track. */
export interface AudioTrackInfo {
  /** Track id in the container. */
  id: number;
  /** Codec string, e.g. `mp4a.40.2`. */
  codec: string;
  /** Number of channels. */
  channels: number;
  /** Sample rate in Hz. */
  sampleRate: number;
  /** AudioSpecificConfig bytes for AAC; `null` when the codec is not AAC. */
  description: Uint8Array | null;
}

/** An opened MP4/MOV file ready for decoding. */
export interface Demuxed {
  /** The first video track. */
  video: VideoTrackInfo;
  /** Its samples in decode order. */
  videoSamples: SampleRef[];
  /** The first audio track, or `null`. */
  audio: AudioTrackInfo | null;
  /** Its samples in decode order. */
  audioSamples: SampleRef[];
  /**
   * A reader for sample bytes. Use one per track: each keeps its own read-ahead window.
   *
   * @returns A function returning a sample's compressed bytes.
   */
  openReader(): (s: SampleRef) => Promise<Uint8Array>;
}

/** Size of the windows read from the file, in bytes. */
const READ_WINDOW = 4 * 1024 * 1024;

/**
 * Map a container sample to the arguments of `new EncodedVideoChunk`.
 *
 * @param s - Sample reference.
 * @param data - The sample's bytes.
 * @returns Chunk init: key/delta type, timestamp and duration in microseconds, and data.
 */
export function toChunkInit(
  s: SampleRef,
  data: Uint8Array,
): { type: "key" | "delta"; timestamp: number; duration: number; data: Uint8Array } {
  const t = sampleTiming(s.cts, s.dts, s.duration, s.timescale);
  return { type: s.isSync ? "key" : "delta", timestamp: t.timestamp, duration: t.duration, data };
}

/**
 * Convert mp4box samples to {@link SampleRef}s.
 *
 * @param samples - Samples from `getTrackSamplesInfo`.
 * @returns References in the same order.
 */
export function toSampleRefs(
  samples: readonly Pick<
    Sample,
    "offset" | "size" | "cts" | "dts" | "duration" | "timescale" | "is_sync"
  >[],
): SampleRef[] {
  return samples.map((s) => ({
    offset: s.offset,
    size: s.size,
    cts: s.cts,
    dts: s.dts,
    duration: s.duration,
    timescale: s.timescale,
    isSync: s.is_sync,
  }));
}

/**
 * Windowed reader: loads a few megabytes around a sample and serves the
 * following samples from that window, so decode order does not mean one
 * `Blob.slice` per frame.
 *
 * @param blob - The source file.
 * @returns A function returning a sample's bytes.
 */
export function windowedReader(blob: Blob): (s: SampleRef) => Promise<Uint8Array> {
  let start = 0;
  let bytes = new Uint8Array(0);
  return async (s) => {
    const end = s.offset + s.size;
    if (s.offset < start || end > start + bytes.byteLength) {
      const len = Math.max(READ_WINDOW, s.size);
      start = s.offset;
      bytes = new Uint8Array(await blob.slice(start, start + len).arrayBuffer());
    }
    return bytes.subarray(s.offset - start, end - start);
  };
}

function boxPayload(box: { write(stream: never): void }): Uint8Array {
  const ds = new DataStream(undefined, 0, Endianness.BIG_ENDIAN);
  box.write(ds as never);
  // The first 8 bytes are the box header (size + fourcc); decoders take the payload.
  return new Uint8Array(ds.buffer, 8, ds.getPosition() - 8).slice();
}

interface EntryWithBoxes {
  avcC?: { write(stream: never): void };
  hvcC?: { write(stream: never): void };
  esds?: { esd?: { descs?: { tag: number; data?: Uint8Array; descs?: { tag: number; data?: Uint8Array }[] }[] } };
}

/**
 * Audio decoder-specific info (AudioSpecificConfig) from an `esds` box.
 *
 * @param esd - The `esds` box's descriptor tree.
 * @returns The config bytes, or `null` when absent.
 */
function ascFromEsds(entry: EntryWithBoxes): Uint8Array | null {
  const decoderConfig = entry.esds?.esd?.descs?.find((d) => d.tag === 0x04);
  const specific = decoderConfig?.descs?.find((d) => d.tag === 0x05);
  return specific?.data ? specific.data.slice() : null;
}

/**
 * Build a two-byte AAC-LC AudioSpecificConfig when the file has no usable `esds`.
 *
 * @param sampleRate - Sample rate in Hz.
 * @param channels - Channel count.
 * @returns Config bytes, or `null` for a sample rate AAC does not list.
 */
export function synthesizeAsc(sampleRate: number, channels: number): Uint8Array | null {
  const rates = [96000, 88200, 64000, 48000, 44100, 32000, 24000, 22050, 16000, 12000, 11025, 8000, 7350];
  const idx = rates.indexOf(sampleRate);
  if (idx < 0) return null;
  const aot = 2;
  const value = (aot << 11) | (idx << 7) | ((channels & 15) << 3);
  return new Uint8Array([(value >> 8) & 255, value & 255]);
}

function entryOf(file: ISOFile, track: Track): EntryWithBoxes | null {
  const trak = file.getTrackById(track.id) as unknown as {
    mdia?: { minf?: { stbl?: { stsd?: { entries?: EntryWithBoxes[] } } } };
  };
  return trak.mdia?.minf?.stbl?.stsd?.entries?.[0] ?? null;
}

async function readMovie(blob: Blob): Promise<{ file: ISOFile; info: Movie }> {
  const file = createFile();
  let info: Movie | null = null;
  let failure: string | null = null;
  file.onReady = (m) => {
    info = m;
  };
  file.onError = (_module, message) => {
    failure = message;
  };
  let pos = 0;
  while (info === null && failure === null && pos < blob.size) {
    const ab = await blob.slice(pos, pos + READ_WINDOW).arrayBuffer();
    const next = file.appendBuffer(MP4BoxBuffer.fromArrayBuffer(ab, pos));
    pos = next > pos ? next : pos + ab.byteLength;
  }
  if (info === null) throw new Error(failure ?? "No movie header found.");
  return { file, info };
}

/**
 * Open an MP4 or MOV file: read its movie header and sample tables.
 *
 * @param blob - The file.
 * @returns Track facts, sample references and a reader for sample bytes.
 * @throws Error when the file has no video track or cannot be parsed.
 */
export async function demux(blob: Blob): Promise<Demuxed> {
  const { file, info } = await readMovie(blob);
  const v = info.videoTracks[0];
  if (!v?.video) throw new Error("This file has no video track.");
  const videoSamples = toSampleRefs(file.getTrackSamplesInfo(v.id));
  const entry = entryOf(file, v);
  const descBox = entry?.avcC ?? entry?.hvcC ?? null;
  const frames = videoSamples.length;
  const last = videoSamples[frames - 1];
  const durationUnits = last ? last.cts + last.duration : v.duration;
  const video: VideoTrackInfo = {
    id: v.id,
    codec: v.codec,
    width: v.video.width,
    height: v.video.height,
    bitrate: v.bitrate,
    fps: frames > 0 && durationUnits > 0 ? (frames * v.timescale) / durationUnits : 30,
    frames,
    durationS: durationUnits / v.timescale,
    matrix: v.matrix ? Array.from(v.matrix as ArrayLike<number>) : null,
    description: descBox ? boxPayload(descBox) : null,
  };

  const a = info.audioTracks[0];
  let audio: AudioTrackInfo | null = null;
  let audioSamples: SampleRef[] = [];
  if (a?.audio) {
    audioSamples = toSampleRefs(file.getTrackSamplesInfo(a.id));
    const aacEntry = entryOf(file, a);
    audio = {
      id: a.id,
      codec: a.codec,
      channels: a.audio.channel_count,
      sampleRate: a.audio.sample_rate,
      description: a.codec.startsWith("mp4a.40")
        ? (aacEntry ? ascFromEsds(aacEntry) : null) ??
          synthesizeAsc(a.audio.sample_rate, a.audio.channel_count)
        : null,
    };
  }
  return { video, videoSamples, audio, audioSamples, openReader: () => windowedReader(blob) };
}
