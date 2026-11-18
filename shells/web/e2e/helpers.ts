import { expect } from "@playwright/test";
import type { Page, TestInfo } from "@playwright/test";

/** Open the app; the `cpu-fallback` project appends `?cpu=1` to force the CPU renderer. */
export async function openApp(page: Page, testInfo: TestInfo): Promise<void> {
  const cpu = testInfo.project.metadata.cpu === true;
  await page.goto(cpu ? "./?cpu=1" : "./");
  await expect(page.getByText("Open a photo or video to start grading.")).toBeVisible();
}

/** Load the built-in sample and wait until the preview canvas exists. */
export async function openSample(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Try a sample image" }).click();
  await expect(page.getByRole("img", { name: "Image preview" })).toBeVisible();
  await expect(page.getByText("Open a photo or video to start grading.")).toBeHidden();
}

/**
 * Number of distinct quantized colors in a PNG, decoded in the page. A blank or
 * uniform canvas yields 1.
 */
export async function distinctColors(page: Page, png: Buffer): Promise<number> {
  return page.evaluate(async (b64) => {
    const img = new Image();
    img.src = `data:image/png;base64,${b64}`;
    await img.decode();
    const c = document.createElement("canvas");
    c.width = img.naturalWidth;
    c.height = img.naturalHeight;
    const ctx = c.getContext("2d");
    if (!ctx) return 0;
    ctx.drawImage(img, 0, 0);
    const d = ctx.getImageData(0, 0, c.width, c.height).data;
    const seen = new Set<number>();
    for (let i = 0; i < d.length; i += 16) {
      seen.add(((d[i] >> 3) << 10) | ((d[i + 1] >> 3) << 5) | (d[i + 2] >> 3));
    }
    return seen.size;
  }, png.toString("base64"));
}

/** Mean RGB of a PNG, decoded in the page. */
export async function meanColor(page: Page, png: Buffer): Promise<[number, number, number]> {
  return page.evaluate(async (b64) => {
    const img = new Image();
    img.src = `data:image/png;base64,${b64}`;
    await img.decode();
    const c = document.createElement("canvas");
    c.width = img.naturalWidth;
    c.height = img.naturalHeight;
    const ctx = c.getContext("2d")!;
    ctx.drawImage(img, 0, 0);
    const d = ctx.getImageData(0, 0, c.width, c.height).data;
    let r = 0, g = 0, b = 0, n = 0;
    for (let i = 0; i < d.length; i += 16) { r += d[i]; g += d[i + 1]; b += d[i + 2]; n++; }
    return [r / n, g / n, b / n] as [number, number, number];
  }, png.toString("base64"));
}
