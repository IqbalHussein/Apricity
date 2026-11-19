import { describe, expect, it } from "vitest";
import {
  averageFps,
  chooseBitrate,
  encoderCandidates,
  estimateProgress,
  evenSize,
  ExportCancelled,
  formatClock,
  frameStepSeconds,
  h264CodecString,
  h264Level,
  isVideoFile,
  matrixToFloats,
  pickEncoder,
  ResourceBag,
  sampleTiming,
  stepTime,
  throwIfAborted,
  videoExportSize,
  videoFilename,
  waitForRoom,
} from "./model";

describe("file detection and names", () => {
  it("detects video by MIME type or extension", () => {
    expect(isVideoFile({ name: "a.bin", type: "video/mp4" })).toBe(true);
    expect(isVideoFile({ name: "CLIP.MOV", type: "" })).toBe(true);
    expect(isVideoFile({ name: "a.jpg", type: "image/jpeg" })).toBe(false);
  });
  it("names the output after the source", () => {
    expect(videoFilename("beach day.mov")).toBe("beach day-graded.mp4");
    expect(videoFilename("")).toBe("video-graded.mp4");
  });
});

describe("size cap", () => {
  it("rounds to even sides", () => {
    expect(evenSize(1281, 721)).toEqual({ width: 1280, height: 720 });
    expect(evenSize(1, 1)).toEqual({ width: 2, height: 2 });
  });
  it("caps the long edge at 2048 for Free and keeps the aspect", () => {
    expect(videoExportSize(3840, 2160, 2048)).toEqual({ width: 2048, height: 1152 });
    expect(videoExportSize(2160, 3840, 2048)).toEqual({ width: 1152, height: 2048 });
  });
  it("does not upscale and leaves Pro uncapped", () => {
    expect(videoExportSize(1280, 720, 2048)).toEqual({ width: 1280, height: 720 });
    expect(videoExportSize(3840, 2160, null)).toEqual({ width: 3840, height: 2160 });
  });
});

describe("codec choice", () => {
  it("picks the H.264 level the size and rate need", () => {
    expect(h264Level(1280, 720, 30)).toBe(40);
    expect(h264Level(1920, 1080, 30)).toBe(40);
    expect(h264Level(1920, 1080, 60)).toBe(42);
    expect(h264Level(3840, 2160, 30)).toBe(51);
    expect(h264Level(3840, 2160, 60)).toBe(52);
  });
  it("formats the codec string", () => {
    expect(h264CodecString(40)).toBe("avc1.640028");
    expect(h264CodecString(51)).toBe("avc1.640033");
  });
  it("lists H.264 first and VP9 as fallback", () => {
    const c = encoderCandidates(1280, 720, 30);
    expect(c[0]).toEqual({ codec: "avc1.640028", family: "h264" });
    expect(c.some((x) => x.codec === "vp09.00.10.08")).toBe(true);
    expect(c.findIndex((x) => x.family === "vp9")).toBeGreaterThan(0);
  });
  it("prefers H.264 when supported", async () => {
    const pick = await pickEncoder(encoderCandidates(1280, 720, 30), async () => true);
    expect(pick?.family).toBe("h264");
  });
  it("falls back to VP9 when H.264 encode is unsupported", async () => {
    const pick = await pickEncoder(
      encoderCandidates(1280, 720, 30),
      async (c) => c.family === "vp9",
    );
    expect(pick).toEqual({ codec: "vp09.00.10.08", family: "vp9" });
  });
  it("treats a throwing check as unsupported and returns null when nothing works", async () => {
    const pick = await pickEncoder(encoderCandidates(1280, 720, 30), async () => {
      throw new Error("bad codec string");
    });
    expect(pick).toBeNull();
  });
});

describe("bitrate", () => {
  it("scales the source bitrate by pixel ratio", () => {
    expect(chooseBitrate(40_000_000, 3840 * 2160, 2048 * 1152)).toBe(
      Math.round(40_000_000 * ((2048 * 1152) / (3840 * 2160))),
    );
  });
  it("uses 8 Mbps at 1080p when the source rate is unknown", () => {
    expect(chooseBitrate(0, 1920 * 1080, 1920 * 1080)).toBe(8_000_000);
    expect(chooseBitrate(null, 3840 * 2160, 3840 * 2160)).toBe(32_000_000);
  });
  it("clamps to 1 to 80 Mbps", () => {
    expect(chooseBitrate(50_000, 1920 * 1080, 1920 * 1080)).toBe(1_000_000);
    expect(chooseBitrate(900_000_000, 1920 * 1080, 1920 * 1080)).toBe(80_000_000);
  });
});

describe("timestamps", () => {
  it("converts timescale units to microseconds", () => {
    expect(sampleTiming(1001, 0, 1001, 30000)).toEqual({
      timestamp: 33367,
      dts: 0,
      duration: 33367,
    });
    expect(sampleTiming(90000, 90000, 3000, 90000)).toEqual({
      timestamp: 1_000_000,
      dts: 1_000_000,
      duration: 33333,
    });
  });
  it("computes the average frame rate", () => {
    expect(averageFps(30, 30000, 1000)).toBe(1);
    expect(averageFps(300, 10 * 30000, 30000)).toBe(30);
    expect(averageFps(0, 0, 0)).toBe(30);
  });
  it("steps frames from the middle of a frame", () => {
    const fps = 10;
    expect(frameStepSeconds(fps)).toBeCloseTo(0.1);
    expect(stepTime(0, 1, fps, 3)).toBeCloseTo(0.15);
    expect(stepTime(0.15, 1, fps, 3)).toBeCloseTo(0.25);
    expect(stepTime(0.25, -1, fps, 3)).toBeCloseTo(0.15);
    expect(stepTime(0, -1, fps, 3)).toBe(0);
    expect(stepTime(2.95, 1, fps, 3)).toBeLessThan(3);
  });
  it("converts a display matrix to floats", () => {
    expect(matrixToFloats([0x10000, 0, 0, 0, 0x10000, 0, 0, 0, 0x40000000])).toEqual([
      1, 0, 0, 0, 1, 0, 0, 0, 1,
    ]);
  });
});

describe("clock text", () => {
  it("formats minutes, tenths and hours", () => {
    expect(formatClock(0)).toBe("0:00");
    expect(formatClock(7.46, true)).toBe("0:07.4");
    expect(formatClock(65)).toBe("1:05");
    expect(formatClock(3723)).toBe("1:02:03");
    expect(formatClock(Number.NaN)).toBe("0:00");
  });
});

describe("progress estimate", () => {
  it("has no estimate before the first frame", () => {
    expect(estimateProgress({ done: 0, total: 100, elapsedMs: 500 })).toEqual({
      fraction: 0,
      fps: 0,
      etaMs: null,
    });
  });
  it("extrapolates from elapsed time", () => {
    const p = estimateProgress({ done: 30, total: 120, elapsedMs: 3000 });
    expect(p.fraction).toBe(0.25);
    expect(p.fps).toBeCloseTo(10);
    expect(p.etaMs).toBe(9000);
  });
  it("clamps overshoot", () => {
    const p = estimateProgress({ done: 130, total: 120, elapsedMs: 1000 });
    expect(p.fraction).toBe(1);
    expect(p.etaMs).toBe(0);
  });
});

describe("cancellation cleanup", () => {
  it("releases resources newest first, once", () => {
    const order: string[] = [];
    const bag = new ResourceBag();
    bag.add(() => order.push("engine"));
    bag.add(() => order.push("decoder"));
    bag.add(() => order.push("encoder"));
    bag.closeAll();
    bag.closeAll();
    expect(order).toEqual(["encoder", "decoder", "engine"]);
  });
  it("keeps releasing when one closer throws", () => {
    const order: string[] = [];
    const bag = new ResourceBag();
    bag.add(() => order.push("a"));
    bag.add(() => {
      throw new Error("already closed");
    });
    bag.add(() => order.push("c"));
    bag.closeAll();
    expect(order).toEqual(["c", "a"]);
  });
  it("closes a resource added after cleanup at once", () => {
    const bag = new ResourceBag();
    bag.closeAll();
    let closed = false;
    bag.add(() => {
      closed = true;
    });
    expect(closed).toBe(true);
  });
  it("throws ExportCancelled for an aborted signal", () => {
    const c = new AbortController();
    expect(() => throwIfAborted(c.signal)).not.toThrow();
    c.abort();
    expect(() => throwIfAborted(c.signal)).toThrow(ExportCancelled);
  });
});

describe("queue backpressure", () => {
  it("waits until the queue drops below the limit", async () => {
    let size = 8;
    let waits = 0;
    await waitForRoom(
      () => size,
      8,
      async () => {
        waits += 1;
        size -= 1;
      },
      new AbortController().signal,
    );
    expect(waits).toBe(1);
    expect(size).toBe(7);
  });
  it("does not wait when there is room", async () => {
    let waits = 0;
    await waitForRoom(() => 3, 8, async () => void (waits += 1), new AbortController().signal);
    expect(waits).toBe(0);
  });
  it("stops waiting when aborted", async () => {
    const c = new AbortController();
    await expect(
      waitForRoom(
        () => 99,
        8,
        async () => c.abort(),
        c.signal,
      ),
    ).rejects.toBeInstanceOf(ExportCancelled);
  });
});
