import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";
import { openApp, openSample } from "./helpers";

/** M6b UI (recipes, export, A/B, LUT) lands separately; skip until its hooks exist. */
async function requireM6b(page: Page, testId: string): Promise<void> {
  test.skip((await page.getByTestId(testId).count()) === 0, "M6b UI not present");
}

test("save recipe persists across reload", async ({ page }, testInfo) => {
  await openApp(page, testInfo);
  await openSample(page);
  await requireM6b(page, "save-recipe");
  await page.getByRole("textbox", { name: "Describe the look you want" }).fill("warm golden hour");
  await page.getByRole("button", { name: "Apply", exact: true }).click();
  await page.getByTestId("save-recipe").click();
  await expect(page.getByTestId("recipe-card").first()).toBeVisible();
  const count = await page.getByTestId("recipe-card").count();
  await page.reload();
  await expect(page.getByTestId("recipe-card")).toHaveCount(count);
});

test("export JPEG downloads a decodable image", async ({ page }, testInfo) => {
  await openApp(page, testInfo);
  await openSample(page);
  await requireM6b(page, "export-open");
  await page.getByTestId("export-open").click();
  const [download] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("export-confirm").click(),
  ]);
  const path = await download.path();
  expect(path).toBeTruthy();
  const { readFileSync } = await import("node:fs");
  const b64 = readFileSync(path).toString("base64");
  const dims = await page.evaluate(async (data) => {
    const img = new Image();
    img.src = `data:image/jpeg;base64,${data}`;
    await img.decode();
    return [img.naturalWidth, img.naturalHeight];
  }, b64);
  // Sample image is 1536x1024 (lib/sampleImage.ts); default export is full size.
  expect(dims).toEqual([1536, 1024]);
});

test("A/B toggle flips state", async ({ page }, testInfo) => {
  await openApp(page, testInfo);
  await openSample(page);
  await requireM6b(page, "ab-toggle");
  const toggle = page.getByTestId("ab-toggle");
  const before = await toggle.innerText();
  await toggle.click();
  await expect.poll(async () => toggle.innerText()).not.toBe(before);
});
