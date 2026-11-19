import { hydrateEntitlements } from "../entitlements/store";
import { attachPromptStore } from "../state/ai";
import { hydrateLuts } from "../state/lut";
import { hydrateRecipes } from "../state/recipes";
import { hydrateSettings } from "../state/settings";
import { IdbPromptStore, storage } from "./idb";

/**
 * Open local storage and load everything persisted: settings, usage counters,
 * recipes, imported LUTs and prompt history. Never throws and never shows an
 * error: when IndexedDB is unavailable the app simply runs without persisting.
 */
export async function initPersistence(): Promise<void> {
  const backend = await storage();
  await Promise.allSettled([
    hydrateSettings(),
    hydrateEntitlements(),
    hydrateRecipes(),
    hydrateLuts(),
    attachPromptStore(new IdbPromptStore(backend)),
  ]);
}
