import { expect, test } from "@playwright/test";
import type { Page, TestInfo } from "@playwright/test";
import { readFileSync } from "node:fs";
import { meanColor } from "./helpers";

const MUXER_JS = "node_modules/mp4-muxer/build/mp4-muxer.js";
const WIDTH = 640;
const HEIGHT = 360;
const FRAMES = 60;
const FPS = 30;

interface Fixture {
  bytes: number[];
  codec: "h264" | "vp9";
  audio: boolean;
}

/** Open the app with onboarding off (the CPU project also forces the CPU renderer). */
async function open(page: Page, testInfo: TestInfo): Promise<void> {
  const cpu = testInfo.project.metadata.cpu === true;
  await page.goto(cpu ? "./?cpu=1&onboarding=0" : "./?onboarding=0");
  await expect(page.getByText("Open a photo or video to start grading.")).toBeVisible();
}

/**
 * Encode a short moving-gradient MP4 inside the page with WebCodecs and
 * mp4-muxer: H.264 when the browser can encode it, else VP9. An AAC track is
 * added when the browser can encode AAC.
 */
async function makeFixture(page: Page): Promise<Fixture> {
  await page.addScriptTag({ path: MUXER_JS });
  return page.evaluate(
    async ({ w, h, n, fps }) => {
      const lib = (window as unknown as { Mp4Muxer: typeof import("mp4-muxer") }).Mp4Muxer;
      const h264 = (
        await VideoEncoder.isConfigSupported({
          codec: "avc1.640028",
          width: w,
          height: h,
          bitrate: 2_000_000,
          framerate: fps,
          avc: { format: "avc" },
        })
      ).supported === true;
      const codecString = h264 ? "avc1.640028" : "vp09.00.10.08";
      const aac =
        typeof AudioEncoder !== "undefined" &&
        (
          await AudioEncoder.isConfigSupported({
            codec: "mp4a.40.2",
            sampleRate: 48000,
            numberOfChannels: 2,
            bitrate: 96000,
          })
        ).supported === true;
      const target = new lib.ArrayBufferTarget();
      const muxer = new lib.Muxer({
        target,
        video: { codec: h264 ? "avc" : "vp9", width: w, height: h },
        ...(aac ? { audio: { codec: "aac" as const, numberOfChannels: 2, sampleRate: 48000 } } : {}),
        fastStart: "in-memory",
        firstTimestampBehavior: "offset",
      });
      const venc = new VideoEncoder({
        output: (c, m) => muxer.addVideoChunk(c, m),
        error: (e) => {
          throw e;
        },
      });
      venc.configure({
        codec: codecString,
        width: w,
        height: h,
        bitrate: 2_000_000,
        framerate: fps,
        ...(h264 ? { avc: { format: "avc" as const } } : {}),
      });
      const canvas = document.createElement("canvas");
      canvas.width = w;
      canvas.height = h;
      const ctx = canvas.getContext("2d") as CanvasRenderingContext2D;
      for (let i = 0; i < n; i++) {
        const g = ctx.createLinearGradient(0, 0, w, h);
        const t = i / n;
        g.addColorStop(0, `hsl(${Math.round(t * 300)} 70% 45%)`);
        g.addColorStop(1, `hsl(${Math.round(t * 300 + 120)} 60% 30%)`);
        ctx.fillStyle = g;
        ctx.fillRect(0, 0, w, h);
        ctx.fillStyle = "#f0e0b0";
        ctx.fillRect(Math.round(t * (w - 60)), h / 2 - 30, 60, 60);
        const frame = new VideoFrame(canvas, {
          timestamp: Math.round((i * 1_000_000) / fps),
          duration: Math.round(1_000_000 / fps),
        });
        venc.encode(frame, { keyFrame: i % 15 === 0 });
        frame.close();
        if (venc.encodeQueueSize > 6) await new Promise((r) => setTimeout(r, 5));
      }
      await venc.flush();
      venc.close();
      if (aac) {
        const aenc = new AudioEncoder({
          output: (c, m) => muxer.addAudioChunk(c, m),
          error: (e) => {
            throw e;
          },
        });
        aenc.configure({ codec: "mp4a.40.2", sampleRate: 48000, numberOfChannels: 2, bitrate: 96000 });
        const seconds = n / fps;
        const total = Math.round(48000 * seconds);
        const block = 1024;
        for (let off = 0; off < total; off += block) {
          const len = Math.min(block, total - off);
          const data = new Float32Array(len * 2);
          for (let i = 0; i < len; i++) {
            const v = 0.2 * Math.sin((2 * Math.PI * 440 * (off + i)) / 48000);
            data[i] = v;
            data[len + i] = v;
          }
          const ad = new AudioData({
            format: "f32-planar",
            sampleRate: 48000,
            numberOfFrames: len,
            numberOfChannels: 2,
            timestamp: Math.round((off * 1_000_000) / 48000),
            data,
          });
          aenc.encode(ad);
          ad.close();
        }
        await aenc.flush();
        aenc.close();
      }
      muxer.finalize();
      return {
        bytes: Array.from(new Uint8Array(target.buffer)),
        codec: h264 ? ("h264" as const) : ("vp9" as const),
        audio: aac,
      };
    },
    { w: WIDTH, h: HEIGHT, n: FRAMES, fps: FPS },
  );
}

async function openFixture(page: Page): Promise<Fixture> {
  const fx = await makeFixture(page);
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Open file" }).first().click();
  await (
    await chooser
  ).setFiles({ name: "test-clip.mp4", mimeType: "video/mp4", buffer: Buffer.from(fx.bytes) });
  await expect(page.getByTestId("transport")).toBeVisible();
  return fx;
}

function seconds(text: string): number {
  const m = /^(\d+):(\d{2})(?:\.(\d))?$/.exec(text.trim());
  if (!m) return Number.NaN;
  return Number(m[1]) * 60 + Number(m[2]) + Number(m[3] ?? 0) / 10;
}

test("video loads and the transport shows the duration", async ({ page }, testInfo) => {
  await open(page, testInfo);
  await openFixture(page);
  // 60 frames at 30 fps is 2.0 s.
  await expect(page.getByTestId("transport-duration")).toHaveText("0:02.0");
  await expect(page.getByTestId("transport-time")).toHaveText("0:00.0");
  await expect(page.getByRole("img", { name: "Image preview" })).toBeVisible();
});

test("play advances time, scrubbing and frame stepping move the playhead", async ({
  page,
}, testInfo) => {
  await open(page, testInfo);
  await openFixture(page);
  const time = page.getByTestId("transport-time");
  await page.getByTestId("transport-play").click();
  await expect
    .poll(async () => seconds(await time.innerText()), { timeout: 10_000 })
    .toBeGreaterThan(0.2);
  await page.getByTestId("transport-play").click();
  await expect(page.getByTestId("transport-play")).toHaveAccessibleName("Play");

  const scrub = page.getByTestId("transport-scrub");
  await scrub.evaluate((el: HTMLInputElement) => {
    el.value = "1.5";
    el.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await expect
    .poll(async () => seconds(await time.innerText()), { timeout: 10_000 })
    .toBeCloseTo(1.5, 0);

  const before = seconds(await time.innerText());
  await page.keyboard.press(".");
  await page.keyboard.press(".");
  await expect
    .poll(async () => seconds(await time.innerText()), { timeout: 10_000 })
    .toBeGreaterThanOrEqual(before);
  await page.keyboard.press(" ");
  await expect(page.getByTestId("transport-play")).toHaveAccessibleName("Pause");
  await page.keyboard.press(" ");
  await expect(page.getByTestId("transport-play")).toHaveAccessibleName("Play");
});

test("a prompt grades the video preview", async ({ page }, testInfo) => {
  await open(page, testInfo);
  await openFixture(page);
  const canvas = page.getByRole("img", { name: "Image preview" });
  await page.waitForTimeout(500);
  const before = await meanColor(page, await canvas.screenshot());
  await page
    .getByRole("textbox", { name: "Describe the look you want" })
    .fill("moody teal and orange, lifted blacks");
  await page.getByRole("button", { name: "Apply", exact: true }).click();
  await expect(
    page.getByRole("region", { name: "Explainability" }).getByText(/The AI made \d+ adjustments?\./),
  ).toBeVisible();
  await expect
    .poll(
      async () => {
        const after = await meanColor(page, await canvas.screenshot());
        return Math.abs(after[0] - before[0]) + Math.abs(after[1] - before[1]) + Math.abs(after[2] - before[2]);
      },
      { timeout: 15_000 },
    )
    .toBeGreaterThan(3);
});

test("export produces an MP4 with the source duration and size", async ({ page }, testInfo) => {
  test.skip(testInfo.project.metadata.cpu === true, "video export needs WebGPU");
  test.setTimeout(120_000);
  await open(page, testInfo);
  const fx = await openFixture(page);
  await page.getByRole("textbox", { name: "Describe the look you want" }).fill("warm golden hour");
  await page.getByRole("button", { name: "Apply", exact: true }).click();
  await page.getByTestId("export-open").click();
  const dialog = page.getByTestId("export-dialog");
  await expect(dialog.getByTestId("export-video")).toBeVisible();
  // The image tab is hidden for video sources; the LUT tab stays.
  await expect(dialog.getByRole("radio", { name: "Image" })).toHaveCount(0);
  await expect(dialog.getByRole("radio", { name: "LUT (.cube)" })).toHaveCount(1);

  const [download] = await Promise.all([
    page.waitForEvent("download", { timeout: 90_000 }),
    dialog.getByTestId("export-confirm").click(),
  ]);
  expect(download.suggestedFilename()).toBe("test-clip-graded.mp4");
  const result = dialog.getByTestId("export-result");
  await expect(result).toBeVisible();
  const message = await result.innerText();
  expect(message).toContain(fx.codec === "h264" ? "encoded as H.264" : "encoded as VP9");
  if (fx.audio) expect(message).toContain("Audio was copied");

  const b64 = readFileSync(await download.path()).toString("base64");
  const info = await page.evaluate(async (data) => {
    const bytes = Uint8Array.from(atob(data), (c) => c.charCodeAt(0));
    const url = URL.createObjectURL(new Blob([bytes], { type: "video/mp4" }));
    const v = document.createElement("video");
    v.muted = true;
    await new Promise<void>((resolve, reject) => {
      v.onloadedmetadata = () => resolve();
      v.onerror = () => reject(new Error("the exported file does not decode"));
      v.src = url;
    });
    return { w: v.videoWidth, h: v.videoHeight, d: v.duration };
  }, b64);
  expect([info.w, info.h]).toEqual([WIDTH, HEIGHT]);
  expect(Math.abs(info.d - FRAMES / FPS)).toBeLessThanOrEqual(1 / FPS + 0.001);
});

test("the CPU renderer previews video but explains that export is unavailable", async ({
  page,
}, testInfo) => {
  test.skip(testInfo.project.metadata.cpu !== true, "CPU renderer only");
  await open(page, testInfo);
  await openFixture(page);
  await page.getByTestId("export-open").click();
  await expect(page.getByTestId("export-video-blocked")).toContainText("needs WebGPU");
  await expect(page.getByTestId("export-confirm")).toBeDisabled();
});
