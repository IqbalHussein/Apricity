import { readFileSync } from "node:fs";
import { expect, test } from "@playwright/test";
import type { Download, Page } from "@playwright/test";
import { openApp, openSample } from "./helpers";

const SAMPLE_W = 1536;
const SAMPLE_H = 1024;

async function applyPrompt(page: Page, text: string): Promise<void> {
  await page.getByRole("textbox", { name: "Describe the look you want" }).fill(text);
  await page.getByRole("button", { name: "Apply", exact: true }).click();
  await expect(page.getByRole("region", { name: "Explainability" })).toBeVisible();
}

async function imageSize(page: Page, download: Download, mime: string): Promise<number[]> {
  const path = await download.path();
  const b64 = readFileSync(path).toString("base64");
  return page.evaluate(
    async ([data, type]) => {
      const img = new Image();
      img.src = `data:${type};base64,${data}`;
      await img.decode();
      return [img.naturalWidth, img.naturalHeight];
    },
    [b64, mime],
  );
}

async function exportJpeg(page: Page): Promise<Download> {
  await page.getByTestId("export-open").click();
  const [download] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("export-confirm").click(),
  ]);
  return download;
}

test("onboarding card appears, steps and dismisses", async ({ page }, testInfo) => {
  await openApp(page, testInfo, { onboarding: "1" });
  await openSample(page);
  const card = page.getByTestId("onboarding");
  await expect(card).toBeVisible();
  await expect(page.getByTestId("onboarding-step")).toHaveText("Step 1 of 3");
  await page.getByTestId("onboarding-next").click();
  await expect(page.getByTestId("onboarding-step")).toHaveText("Step 2 of 3");
  await page.getByTestId("onboarding-dismiss").click();
  await expect(card).toBeHidden();
});

test("onboarding is off with ?onboarding=0", async ({ page }, testInfo) => {
  await openApp(page, testInfo);
  await openSample(page);
  await expect(page.getByTestId("onboarding")).toBeHidden();
});

test("save recipe persists across reload", async ({ page }, testInfo) => {
  await openApp(page, testInfo);
  await openSample(page);
  await applyPrompt(page, "warm golden hour");
  await page.getByTestId("save-recipe").click();
  await page.getByTestId("recipe-name-input").fill("E2E look");
  await page.getByTestId("save-recipe-confirm").click();

  await page.getByTestId("recipe-library-open").click();
  const library = page.getByTestId("recipe-library");
  await expect(library.getByTestId("recipe-card")).toHaveCount(1);
  await expect(library.getByTestId("recipe-name")).toHaveText("E2E look");
  await expect(library.getByTestId("starter-card").first()).toBeVisible();

  await page.reload();
  await expect(page.getByText("Open a photo or video to start grading.")).toBeVisible();
  await page.getByTestId("recipe-library-open").click();
  await expect(page.getByTestId("recipe-library").getByTestId("recipe-card")).toHaveCount(1);
  await expect(page.getByTestId("recipe-library").getByTestId("recipe-name")).toHaveText("E2E look");
});

test("Quick strip shows recipe chips", async ({ page }, testInfo) => {
  await openApp(page, testInfo);
  await openSample(page);
  await expect(page.getByTestId("recipe-chip").first()).toBeVisible();
});

test("export JPEG downloads a decodable image at full size", async ({ page }, testInfo) => {
  await openApp(page, testInfo);
  await openSample(page);
  const download = await exportJpeg(page);
  expect(download.suggestedFilename()).toMatch(/\.jpe?g$/i);
  expect(await imageSize(page, download, "image/jpeg")).toEqual([SAMPLE_W, SAMPLE_H]);
  await expect(page.getByTestId("export-result")).toBeVisible();
});

test("A/B toggle flips, copies A to B, and responds to the backslash key", async ({ page }, testInfo) => {
  await openApp(page, testInfo);
  await openSample(page);
  const toggle = page.getByTestId("ab-toggle");
  await expect(toggle).toHaveAttribute("data-active", "A");
  await toggle.click();
  await expect(toggle).toHaveAttribute("data-active", "B");
  await toggle.click();
  await expect(toggle).toHaveAttribute("data-active", "A");

  await page.getByRole("img", { name: "Image preview" }).click({ position: { x: 5, y: 5 } }).catch(() => undefined);
  await page.locator("body").press("\\");
  await expect(toggle).toHaveAttribute("data-active", "B");
  await page.locator("body").press("\\");
  await expect(toggle).toHaveAttribute("data-active", "A");

  await page.getByTestId("ab-copy").click();
  await expect(toggle).toHaveAttribute("data-active", /^[AB]$/);
});

test("LUT import, active state and removal", async ({ page }, testInfo) => {
  await openApp(page, testInfo);
  await openSample(page);
  await page.getByRole("radio", { name: "Grade" }).or(page.getByRole("tab", { name: "Grade" })).first().click();
  const lines = ['TITLE "E2E tint"', "LUT_3D_SIZE 2"];
  for (let b = 0; b < 2; b++)
    for (let g = 0; g < 2; g++)
      for (let r = 0; r < 2; r++) lines.push(`${r * 0.9} ${g * 0.8} ${b}`);
  const toggle = page.locator('section[data-group="LUT"] .section-toggle');
  await toggle.scrollIntoViewIfNeeded();
  if ((await toggle.getAttribute("aria-expanded")) !== "true") await toggle.click();
  const lutButton = page.getByTestId("lut-import");
  await lutButton.scrollIntoViewIfNeeded();
  const [chooser] = await Promise.all([page.waitForEvent("filechooser"), lutButton.click()]);
  await chooser.setFiles({
    name: "e2e.cube",
    mimeType: "text/plain",
    buffer: Buffer.from(lines.join("\n") + "\n"),
  });
  await expect(page.getByTestId("lut-active")).toBeVisible();
  await expect(page.getByTestId("lut-active")).toContainText("E2E tint");
  await page.getByTestId("lut-remove").click();
  await expect(page.getByTestId("lut-active")).toBeHidden();
});

test("LUT export from the Export dialog writes a 33-point .cube", async ({ page }, testInfo) => {
  await openApp(page, testInfo);
  await openSample(page);
  await applyPrompt(page, "moody teal and orange, lifted blacks");
  await page.getByTestId("export-open").click();
  await page.getByRole("radio", { name: "LUT (.cube)" }).click();
  const [download] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("export-lut-confirm").click(),
  ]);
  expect(download.suggestedFilename()).toMatch(/\.cube$/i);
  const text = readFileSync(await download.path(), "utf8");
  expect(text).toContain("LUT_3D_SIZE 33");
  await expect(page.getByTestId("export-lut-result")).toBeVisible();
});

test("Free tier (?beta=0) caps export at 2048 px on the long edge", async ({ page }, testInfo) => {
  await openApp(page, testInfo, { beta: "0" });
  // A 3200x2000 PNG, generated in the page, so the cap actually bites.
  const b64 = await page.evaluate(async () => {
    const c = document.createElement("canvas");
    c.width = 3200;
    c.height = 2000;
    const ctx = c.getContext("2d")!;
    const g = ctx.createLinearGradient(0, 0, 3200, 2000);
    g.addColorStop(0, "#335577");
    g.addColorStop(1, "#ddaa66");
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, 3200, 2000);
    const blob: Blob = await new Promise((res) => c.toBlob((b) => res(b!), "image/png"));
    const buf = new Uint8Array(await blob.arrayBuffer());
    let s = "";
    for (let i = 0; i < buf.length; i += 0x8000) s += String.fromCharCode(...buf.subarray(i, i + 0x8000));
    return btoa(s);
  });
  const [chooser] = await Promise.all([
    page.waitForEvent("filechooser"),
    page.locator(".empty").getByRole("button", { name: "Open file" }).click(),
  ]);
  await chooser.setFiles({ name: "big.png", mimeType: "image/png", buffer: Buffer.from(b64, "base64") });
  await expect(page.getByRole("img", { name: "Image preview" })).toBeVisible();
  await expect(page.getByTestId("export-open")).toBeEnabled();

  const download = await exportJpeg(page);
  const [w, h] = await imageSize(page, download, "image/jpeg");
  expect(Math.max(w, h)).toBeLessThanOrEqual(2048);
  expect(Math.max(w, h)).toBeGreaterThan(1500);
  await expect(page.getByText(/Free exports are limited to 2048 px/)).toBeVisible();
});
