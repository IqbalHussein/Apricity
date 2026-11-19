import { expect, test } from "@playwright/test";
import { openApp, openSample } from "./helpers";

test("fallback banner reflects the active renderer", async ({ page }, testInfo) => {
  await openApp(page, testInfo);
  await openSample(page);
  const banner = page.getByRole("status").filter({ hasText: "doesn't support WebGPU" });
  if (testInfo.project.metadata.cpu === true) {
    await expect(banner).toBeVisible();
    return;
  }
  const hasAdapter = await page.evaluate(async () => {
    const gpu = (navigator as Navigator & { gpu?: { requestAdapter(): Promise<unknown> } }).gpu;
    return gpu ? (await gpu.requestAdapter()) !== null : false;
  });
  testInfo.annotations.push({ type: "webgpu-adapter", description: String(hasAdapter) });
  // With an adapter the app must stay on WebGPU; without one it must show the banner.
  if (hasAdapter) await expect(banner).toBeHidden();
  else await expect(banner).toBeVisible();
});
