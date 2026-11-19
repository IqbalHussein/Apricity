import { expect, test } from "@playwright/test";
import { distinctColors, meanColor, openApp, openSample } from "./helpers";

const PROMPT = "moody teal and orange, lifted blacks";

test("loads with no console errors and shows the empty state", async ({ page }, testInfo) => {
  const errors: string[] = [];
  page.on("console", (m) => {
    if (m.type() === "error") errors.push(m.text());
  });
  page.on("pageerror", (e) => errors.push(String(e)));
  await openApp(page, testInfo);
  await expect(page.getByRole("button", { name: "Try a sample image" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Open file" }).first()).toBeVisible();
  // Third-party font requests may fail offline; they are not app errors.
  expect(errors.filter((e) => !/fonts\.(googleapis|gstatic)|ERR_|Failed to load resource/.test(e))).toEqual([]);
});

test("sample image renders non-blank pixels", async ({ page }, testInfo) => {
  await openApp(page, testInfo);
  await openSample(page);
  const canvas = page.getByRole("img", { name: "Image preview" });
  await expect
    .poll(async () => distinctColors(page, await canvas.screenshot()), { timeout: 20_000 })
    .toBeGreaterThan(20);
});

test("NGA prompt shows the Explainability summary, rows, and console link", async ({ page }, testInfo) => {
  await openApp(page, testInfo);
  await openSample(page);
  const canvas = page.getByRole("img", { name: "Image preview" });
  const before = await meanColor(page, await canvas.screenshot());

  await page.getByRole("textbox", { name: "Describe the look you want" }).fill(PROMPT);
  await page.getByRole("button", { name: "Apply", exact: true }).click();

  const panel = page.getByRole("region", { name: "Explainability" });
  await expect(panel.getByText(/The AI made \d+ adjustments?\./)).toBeVisible();
  await expect(panel.locator(".explain-row")).not.toHaveCount(0);
  expect(await panel.locator(".explain-row").count()).toBeGreaterThanOrEqual(5);

  // The grade must visibly change the image.
  await expect
    .poll(async () => {
      const after = await meanColor(page, await canvas.screenshot());
      return Math.abs(after[0] - before[0]) + Math.abs(after[1] - before[1]) + Math.abs(after[2] - before[2]);
    })
    .toBeGreaterThan(1);

  await panel.getByRole("button", { name: /^Show .* in console$/ }).first().click();
  await expect(page.getByRole("radio", { name: "Grade" })).toBeChecked();
  await expect(page.getByRole("complementary", { name: "Inspector" })).toBeVisible();
});

test("keyboard changes a slider value and undo restores it", async ({ page }, testInfo) => {
  await openApp(page, testInfo);
  await openSample(page);
  await page.getByRole("radio", { name: "Grade" }).or(page.getByRole("tab", { name: "Grade" })).first().click();
  const inspector = page.getByRole("complementary", { name: "Inspector" });
  await expect(inspector).toBeVisible();
  const slider = inspector.getByRole("slider").first();
  await slider.scrollIntoViewIfNeeded();
  const initial = await slider.getAttribute("aria-valuetext");
  await slider.focus();
  for (let i = 0; i < 5; i++) await slider.press("ArrowRight");
  await expect(slider).not.toHaveAttribute("aria-valuetext", initial ?? "");
  const changed = await slider.getAttribute("aria-valuetext");
  expect(changed).not.toBe(initial);

  // Keyboard steps may coalesce into one history entry or not; undo until restored.
  const undo = page.getByRole("button", { name: /^Undo/ });
  for (let i = 0; i < 6 && (await slider.getAttribute("aria-valuetext")) !== initial; i++) {
    await expect(undo).toBeEnabled();
    await undo.click();
  }
  await expect(slider).toHaveAttribute("aria-valuetext", initial ?? "");
});

test("review mode wipe divider is present and draggable", async ({ page }, testInfo) => {
  await openApp(page, testInfo);
  await openSample(page);
  await page.getByRole("radio", { name: "Review" }).click();
  const wipe = page.getByRole("slider", { name: "Wipe divider" });
  await expect(wipe).toBeVisible();
  const start = Number(await wipe.getAttribute("aria-valuenow"));
  const box = (await wipe.boundingBox())!;
  const x = box.x + box.width / 2;
  const y = box.y + box.height / 2;
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x + 120, y, { steps: 6 });
  await page.mouse.up();
  await expect.poll(async () => Number(await wipe.getAttribute("aria-valuenow"))).not.toBe(start);
});

test("false-color overlay toggles", async ({ page }, testInfo) => {
  await openApp(page, testInfo);
  await openSample(page);
  await page.getByRole("radio", { name: "Review" }).click();
  const legend = page.getByRole("list", { name: "False color legend" });
  await expect(legend).toBeHidden();
  await page.getByRole("radio", { name: "False color" }).click();
  await expect(legend).toBeVisible();
  await page.getByRole("radio", { name: "None" }).click();
  await expect(legend).toBeHidden();
});

test.describe("mobile", () => {
  test.use({ viewport: { width: 390, height: 844 }, hasTouch: true });
  test("shows the bottom tab bar", async ({ page }, testInfo) => {
    await openApp(page, testInfo);
    await openSample(page);
    const tabs = page.getByRole("tablist", { name: "Mode" });
    await expect(tabs).toBeVisible();
    await expect(tabs.getByRole("tab")).toHaveCount(3);
    await tabs.getByRole("tab", { name: "Grade" }).click();
    await expect(tabs.getByRole("tab", { name: "Grade" })).toHaveAttribute("aria-selected", "true");
    await expect(page.getByRole("complementary", { name: "Inspector" })).toBeVisible();
  });
});
