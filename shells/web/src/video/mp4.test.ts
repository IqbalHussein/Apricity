import { describe, expect, it } from "vitest";
import { synthesizeAsc, toChunkInit, toSampleRefs, windowedReader } from "./mp4";

describe("mp4box sample mapping", () => {
  const sample = {
    offset: 100,
    size: 4,
    cts: 2002,
    dts: 1001,
    duration: 1001,
    timescale: 30000,
    is_sync: true,
  };
  it("copies the fields a decoder needs", () => {
    expect(toSampleRefs([sample])).toEqual([
      { offset: 100, size: 4, cts: 2002, dts: 1001, duration: 1001, timescale: 30000, isSync: true },
    ]);
  });
  it("maps a sync sample to a key chunk in microseconds", () => {
    const [ref] = toSampleRefs([sample]);
    const init = toChunkInit(ref!, new Uint8Array([1, 2, 3, 4]));
    expect(init.type).toBe("key");
    expect(init.timestamp).toBe(66733);
    expect(init.duration).toBe(33367);
    expect(Array.from(init.data)).toEqual([1, 2, 3, 4]);
  });
  it("maps a non-sync sample to a delta chunk", () => {
    const [ref] = toSampleRefs([{ ...sample, is_sync: false }]);
    expect(toChunkInit(ref!, new Uint8Array(0)).type).toBe("delta");
  });
});

describe("windowed reader", () => {
  it("serves neighbouring samples from one slice", async () => {
    const bytes = new Uint8Array(100).map((_, i) => i);
    let slices = 0;
    const blob = {
      slice: (a: number, b: number) => {
        slices += 1;
        return { arrayBuffer: async () => bytes.slice(a, b).buffer };
      },
    } as unknown as Blob;
    const read = windowedReader(blob);
    const ref = (offset: number, size: number) => ({
      offset,
      size,
      cts: 0,
      dts: 0,
      duration: 1,
      timescale: 1,
      isSync: true,
    });
    expect(Array.from(await read(ref(10, 3)))).toEqual([10, 11, 12]);
    expect(Array.from(await read(ref(40, 2)))).toEqual([40, 41]);
    expect(slices).toBe(1);
  });
});

describe("AAC config", () => {
  it("builds an AAC-LC AudioSpecificConfig", () => {
    // object type 2, 44.1 kHz (index 4), stereo: 0001 0010 0001 0000
    expect(Array.from(synthesizeAsc(44100, 2)!)).toEqual([0x12, 0x10]);
    expect(Array.from(synthesizeAsc(48000, 2)!)).toEqual([0x11, 0x90]);
  });
  it("returns null for a rate AAC does not list", () => {
    expect(synthesizeAsc(12345, 2)).toBeNull();
  });
});
