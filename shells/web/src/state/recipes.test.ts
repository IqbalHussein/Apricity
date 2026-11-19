/// <reference types="node" />
import { readFileSync } from "node:fs";
import { beforeAll, beforeEach, describe, expect, it } from "vitest";
import { LocalBetaProvider } from "../entitlements/provider";
import { clearNotice, setEntitlementProvider } from "../entitlements/store";
import { MemoryBackend, setStorageBackend } from "../storage/idb";
import { initSync } from "../wasm/apricity_ffi.js";
import { manualClock, seededRng, setClock, setRng } from "./clock";
import { gradeParams, initGrade, setScalar } from "./grade";
import {
  applyRecipe,
  deleteRecipe,
  hydrateRecipes,
  recipeFileName,
  renameRecipe,
  saveRecipe,
  savedRecipes,
  starterRecipes,
} from "./recipes";

const settle = (): Promise<void> => new Promise((r) => setTimeout(r, 0));

beforeAll(() => {
  initSync({ module: readFileSync(new URL("../wasm/apricity_ffi_bg.wasm", import.meta.url)) });
});

beforeEach(async () => {
  setStorageBackend(new MemoryBackend());
  setClock(manualClock(1_000_000));
  setRng(seededRng(7));
  setEntitlementProvider(new LocalBetaProvider(true));
  clearNotice();
  initGrade();
  for (const r of [...savedRecipes()]) deleteRecipe(r.id);
  await settle();
});

describe("starter recipes", () => {
  it("builds six distinct looks from NGA prompts, each with Stage 1 on Auto", () => {
    const starters = starterRecipes();
    expect(starters.map((r) => r.name)).toEqual([
      "Golden hour",
      "Teal & orange",
      "Faded film",
      "Noir",
      "Portra",
      "Cool morning",
    ]);
    const looks = new Set(starters.map((r) => JSON.stringify(r.params?.stage2)));
    expect(looks.size).toBe(6);
    for (const r of starters) {
      expect(r.params?.stage1?.auto_exposure).toBe(true);
      expect(JSON.stringify(r.params?.stage2)).not.toBe(JSON.stringify(gradeParams().stage2));
    }
  });

  it("are not stored", async () => {
    starterRecipes();
    await hydrateRecipes();
    expect(savedRecipes()).toHaveLength(0);
  });
});

describe("saving, persisting and applying", () => {
  it("round-trips a saved recipe through storage", async () => {
    setScalar("stage2.contrast", 25, { coalesce: false });
    const r = saveRecipe("  Dusk  ");
    expect(r?.name).toBe("Dusk");
    expect(r?.created_at_ms).toBe(1_000_000);
    expect(r?.id).toMatch(/^rcp-[a-z0-9]+$/);
    await settle();

    await hydrateRecipes();
    expect(savedRecipes().map((x) => x.name)).toEqual(["Dusk"]);
    expect(savedRecipes()[0]?.params?.stage2?.contrast).toBe(25);

    setScalar("stage2.contrast", -10, { coalesce: false });
    applyRecipe(savedRecipes()[0] as NonNullable<ReturnType<typeof saveRecipe>>);
    expect(gradeParams().stage2?.contrast).toBe(25);
  });

  it("renames and deletes", async () => {
    const r = saveRecipe("One");
    expect(r).not.toBeNull();
    expect(renameRecipe(r as NonNullable<typeof r>, "Two")).toBe(true);
    expect(savedRecipes()[0]?.name).toBe("Two");
    expect(renameRecipe(r as NonNullable<typeof r>, "  ")).toBe(false);
    await settle();
    await hydrateRecipes();
    expect(savedRecipes()[0]?.name).toBe("Two");
    deleteRecipe(savedRecipes()[0]?.id as string);
    await settle();
    await hydrateRecipes();
    expect(savedRecipes()).toHaveLength(0);
  });

  it("stops at five recipes on the Free tier", () => {
    setEntitlementProvider(new LocalBetaProvider(false));
    for (let i = 0; i < 5; i++) expect(saveRecipe(`R${i}`)).not.toBeNull();
    expect(saveRecipe("R5")).toBeNull();
    expect(savedRecipes()).toHaveLength(5);
  });

  it("names recipe files safely", () => {
    expect(recipeFileName({ name: "Teal / orange: v2" })).toBe("Teal - orange- v2.recipe.json");
  });
});
