/// <reference types="vitest" />
import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

// ADR-0008: the site is served from `/<repo>/` on GitHub Pages, which cannot
// set COOP/COEP headers. The app uses no SharedArrayBuffer or WASM threads, so
// dev and production both run without cross-origin isolation.
export default defineConfig({
  base: process.env.APRICITY_BASE ?? "/",
  plugins: [solid()],
  build: {
    target: "esnext",
  },
  test: {
    environment: "node",
    passWithNoTests: true,
    exclude: ["e2e/**", "node_modules/**", "dist/**"],
  },
});
