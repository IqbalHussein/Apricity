import { defineConfig, devices } from "@playwright/test";

const PORT = 4173;
const BASE = "/Apricity/";

const WEBGPU_ARGS = [
  "--enable-unsafe-webgpu",
  "--enable-features=Vulkan",
  "--use-vulkan=swiftshader",
  "--use-angle=vulkan",
  "--disable-vulkan-surface",
];

/**
 * End-to-end suite. Builds the production bundle under the `/Apricity/` sub-path
 * (as on GitHub Pages, ADR-0008) and serves it with `vite preview`. The WASM
 * core must already be built (`pnpm run build:wasm`).
 */
export default defineConfig({
  testDir: "./e2e",
  timeout: 60_000,
  expect: { timeout: 15_000 },
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: process.env.CI ? 2 : undefined,
  reporter: process.env.CI ? [["list"], ["html", { open: "never" }]] : "list",
  use: {
    baseURL: `http://localhost:${PORT}${BASE}`,
    trace: "off",
  },
  webServer: {
    command: `pnpm exec vite build && pnpm exec vite preview --port ${PORT} --strictPort`,
    url: `http://localhost:${PORT}${BASE}`,
    env: { APRICITY_BASE: BASE },
    reuseExistingServer: !process.env.CI,
    timeout: 180_000,
  },
  projects: [
    {
      name: "webgpu",
      use: { ...devices["Desktop Chrome"], launchOptions: { args: WEBGPU_ARGS } },
    },
    {
      // Forces the CPU renderer via `?cpu=1` (see engine/bridge.ts).
      name: "cpu-fallback",
      use: { ...devices["Desktop Chrome"], launchOptions: { args: WEBGPU_ARGS } },
      metadata: { cpu: true },
    },
  ],
});
