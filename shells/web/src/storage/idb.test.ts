/// <reference types="node" />
import { readFileSync } from "node:fs";
import { beforeAll, describe, expect, it } from "vitest";
import { identityParams, initSync, recipeNew } from "../wasm/apricity_ffi.js";
import {
  IdbPromptStore,
  LutStore,
  MemoryBackend,
  openBackend,
  RecipeStore,
  SettingsStore,
  UsageStore,
} from "./idb";

beforeAll(() => {
  initSync({ module: readFileSync(new URL("../wasm/apricity_ffi_bg.wasm", import.meta.url)) });
});

describe("openBackend", () => {
  it("falls back to memory when IndexedDB is unavailable", async () => {
    const b = await openBackend(undefined);
    expect(b.persistent).toBe(false);
  });

  it("falls back to memory when open fails (private window)", async () => {
    const broken = {
      open() {
        const req: Record<string, unknown> = {};
        queueMicrotask(() => {
          req.error = new Error("denied");
          (req.onerror as () => void)();
        });
        return req;
      },
    } as unknown as IDBFactory;
    expect((await openBackend(broken)).persistent).toBe(false);
  });
});

describe("RecipeStore", () => {
  it("round-trips recipes through recipeParse, newest first", async () => {
    const store = new RecipeStore(new MemoryBackend());
    const a = recipeNew("rcp-a", "First", identityParams(), "warm", 1000);
    const b = recipeNew("rcp-b", "Second", identityParams(), null, 2000);
    await store.put(a);
    await store.put(b);
    const list = await store.list();
    expect(list.map((r) => r.id)).toEqual(["rcp-b", "rcp-a"]);
    expect(list[1]?.prompt).toBe("warm");
    expect(list[0]?.params).toEqual(b.params);
  });

  it("deletes, and skips records that no longer parse", async () => {
    const backend = new MemoryBackend();
    const store = new RecipeStore(backend);
    await store.put(recipeNew("rcp-a", "Keep", identityParams(), null, 1));
    await store.put(recipeNew("rcp-b", "Drop", identityParams(), null, 2));
    await backend.put("recipes", "rcp-bad", { json: "{ not a recipe" });
    expect((await store.list()).map((r) => r.id)).toEqual(["rcp-b", "rcp-a"]);
    await store.delete("rcp-b");
    expect((await store.list()).map((r) => r.id)).toEqual(["rcp-a"]);
  });

  it("migrates and sanitizes on load", async () => {
    const backend = new MemoryBackend();
    const r = recipeNew("rcp-a", "Wild", identityParams(), null, 1);
    const wild = JSON.parse(JSON.stringify(r)) as { params: { stage2: { contrast: number } } };
    wild.params.stage2 = { contrast: 9999 };
    await backend.put("recipes", "rcp-a", { json: JSON.stringify(wild) });
    const [loaded] = await new RecipeStore(backend).list();
    expect(loaded?.params?.stage2?.contrast).toBeLessThanOrEqual(100);
  });
});

describe("other stores", () => {
  it("persists prompt history, usage, settings and LUTs", async () => {
    const backend = new MemoryBackend();
    const prompts = new IdbPromptStore(backend);
    expect(await prompts.load()).toEqual([]);
    await prompts.save(["b", "a"]);
    expect(await prompts.load()).toEqual(["b", "a"]);

    const usage = new UsageStore(backend);
    expect(await usage.get("2026-11")).toBeUndefined();
    await usage.put({ month: "2026-11", imports: 3, prompts: 2 });
    expect(await usage.get("2026-11")).toEqual({ month: "2026-11", imports: 3, prompts: 2 });

    const settings = new SettingsStore(backend);
    expect(await settings.load()).toEqual({});
    await settings.set("onboardingDone", true);
    expect(await settings.load()).toEqual({ onboardingDone: true });

    const luts = new LutStore(backend);
    await luts.put({ id: "lut-1", title: "T", size: 17, text: "x" });
    expect(await luts.list()).toHaveLength(1);
    await luts.delete("lut-1");
    expect(await luts.list()).toHaveLength(0);
  });
});
