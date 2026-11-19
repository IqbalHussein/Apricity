import { createSignal, untrack } from "solid-js";
import {
  identityParams,
  ngaApply,
  recipeApply,
  recipeNew,
  recipeParse,
  recipeRename,
  recipeToJson,
  renderCpuRgba,
} from "../core/wasm";
import type { GradeParameters, Recipe } from "../core/wasm";
import { entitlements, requireFeature } from "../entitlements/store";
import { downloadBlob, safeFileName } from "../lib/download";
import { getPath, setPath } from "../lib/paramPath";
import { downscaleRgba, rgbaToDataUrl } from "../lib/thumbnail";
import { grabFrame, settledFrameCount } from "../video/player";
import { RecipeStore, storage } from "../storage/idb";
import { clearAllAi } from "./aimarks";
import { lastPromptText } from "./ai";
import { getClock, getRng } from "./clock";
import { applyParams, gradeParams } from "./grade";
import { lutTextFor } from "./lut";
import { currentSource } from "./project";

/** Longest side of a recipe thumbnail, in pixels. */
export const THUMB_SIDE = 192;
/** Message for files that are not Apricity recipes. */
export const INVALID_RECIPE_MESSAGE = "This file isn't an Apricity recipe. Choose a recipe .json file.";

/** A built-in starter look: a name and the prompt the NGA turns into a grade. */
export interface StarterSpec {
  /** Display name. */
  name: string;
  /** Natural-language prompt. */
  prompt: string;
}

/** Built-in starter looks, generated at runtime by the NGA (not stored, not counted). */
export const STARTERS: readonly StarterSpec[] = [
  { name: "Golden hour", prompt: "golden hour, warm and soft" },
  { name: "Teal & orange", prompt: "teal and orange cinematic" },
  { name: "Faded film", prompt: "faded film, lifted blacks, muted" },
  { name: "Noir", prompt: "cold desaturated noir, high contrast" },
  { name: "Portra", prompt: "portra, warm skin tones, soft pastel" },
  { name: "Cool morning", prompt: "cool morning, blue and airy" },
];

const [saved, setSaved] = createSignal<readonly Recipe[]>([]);
const [error, setError] = createSignal<string | null>(null);
const [libraryOpen, setLibraryOpen] = createSignal(false);
const [saveOpen, setSaveOpen] = createSignal(false);
let starters: readonly Recipe[] | null = null;

/** Saved recipes, newest update first (tracked). */
export const savedRecipes = saved;
/** Last recipe error copy, or `null` (tracked). */
export const recipeError = error;
/** Whether the Recipe Library dialog is open (tracked). */
export const recipeLibraryOpen = libraryOpen;
/** Open or close the Recipe Library dialog. */
export const setRecipeLibraryOpen = setLibraryOpen;
/** Whether the Save recipe dialog is open (tracked). */
export const saveRecipeOpen = saveOpen;
/** Open or close the Save recipe dialog. */
export const setSaveRecipeOpen = setSaveOpen;

/**
 * The six built-in starter recipes. Generated once from NGA prompts on an
 * identity grade with Stage 1 on Auto (so applying one keeps the image's own
 * correction). They are never persisted and never count against the Free limit.
 *
 * @returns Starter recipes in display order.
 */
export function starterRecipes(): readonly Recipe[] {
  if (starters) return starters;
  let base: GradeParameters = identityParams();
  for (const f of ["auto_exposure", "auto_white_balance", "auto_black_point"])
    base = setPath(base, `stage1.${f}`, true);
  starters = STARTERS.map((s) => {
    const params = ngaApply(s.prompt, base, "fresh").params;
    return recipeNew(`starter-${safeId(s.name)}`, s.name, params, s.prompt, 0);
  });
  return starters;
}

function safeId(name: string): string {
  return name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
}

async function store(): Promise<RecipeStore> {
  return new RecipeStore(await storage());
}

/** Load saved recipes (re-parsed by the core on every load). Never throws. */
export async function hydrateRecipes(): Promise<void> {
  try {
    setSaved(await (await store()).list());
  } catch {
    // the library starts empty for this session
  }
}

/**
 * Suggested name for a new recipe.
 *
 * @returns The last prompt with a capital letter (at most 40 characters), or "My recipe".
 */
export function defaultRecipeName(): string {
  const p = lastPromptText()?.trim();
  if (!p) return "My recipe";
  const cut = p.length > 40 ? `${p.slice(0, 40).trimEnd()}` : p;
  return cut.charAt(0).toUpperCase() + cut.slice(1);
}

function newId(): string {
  return `rcp-${getRng().nextSeed().toString(36)}`;
}

function persist(r: Recipe): void {
  void store()
    .then((s) => s.put(r))
    .catch(() => undefined);
}

function upsert(r: Recipe): void {
  setSaved((list) =>
    [r, ...list.filter((x) => x.id !== r.id)].sort((a, b) => b.updated_at_ms - a.updated_at_ms),
  );
  persist(r);
}

function thumbnailOf(params: GradeParameters, lutId: string | null | undefined): string | null {
  const small = smallProxy();
  if (!small) return null;
  try {
    const px = renderCpuRgba(
      small.rgba,
      small.width,
      small.height,
      params,
      lutTextFor(lutId),
      small.width,
    );
    return rgbaToDataUrl(px, small.width, small.height);
  } catch {
    return null;
  }
}

/**
 * Save the current grade (Stage 1 and Stage 2) as a recipe.
 *
 * @param name - Name, 1 to 64 characters after trimming.
 * @returns The saved recipe, or `null` when blocked by the Free limit or when the core rejects the name
 *   (then {@link recipeError} holds the copy).
 */
export function saveRecipe(name: string): Recipe | null {
  if (!requireFeature("recipe_save", { recipeCount: saved().length })) return null;
  try {
    const params = gradeParams();
    const r = recipeNew(newId(), name, params, lastPromptText(), getClock().nowMs());
    const thumb = thumbnailOf(params, getPath(params, "stage2.lut_id") as string | null);
    upsert(thumb ? { ...r, thumbnail: thumb } : r);
    setError(null);
    return thumb ? { ...r, thumbnail: thumb } : r;
  } catch (e) {
    setError(e instanceof Error ? e.message : String(e));
    return null;
  }
}

/**
 * Apply a recipe to the current image as one undo step ("Apply recipe: name").
 * Stage 1 values the recipe marks as automatic keep the image's own analysis.
 *
 * @param r - Saved or starter recipe.
 */
export function applyRecipe(r: Recipe): void {
  applyParams(recipeApply(r, gradeParams()), `Apply recipe: ${r.name}`);
  clearAllAi();
}

/**
 * Rename a saved recipe.
 *
 * @param r - Saved recipe.
 * @param name - New name, 1 to 64 characters after trimming.
 * @returns `true` on success; otherwise {@link recipeError} holds the copy.
 */
export function renameRecipe(r: Recipe, name: string): boolean {
  try {
    upsert({ ...recipeRename(r, name, getClock().nowMs()) });
    setError(null);
    return true;
  } catch (e) {
    setError(e instanceof Error ? e.message : String(e));
    return false;
  }
}

/**
 * Delete a saved recipe.
 *
 * @param id - Recipe id.
 */
export function deleteRecipe(id: string): void {
  setSaved((list) => list.filter((r) => r.id !== id));
  void store()
    .then((s) => s.delete(id))
    .catch(() => undefined);
}

/**
 * File name for an exported recipe.
 *
 * @param r - Recipe.
 * @returns `<name>.recipe.json`, safe for file systems.
 */
export function recipeFileName(r: Pick<Recipe, "name">): string {
  return `${safeFileName(r.name)}.recipe.json`;
}

/**
 * Download a recipe as a `.json` file (the backup path of ADR-0010).
 *
 * @param r - Recipe to export.
 */
export function exportRecipeFile(r: Recipe): void {
  downloadBlob(new Blob([recipeToJson(r)], { type: "application/json" }), recipeFileName(r));
}

/**
 * Import a recipe `.json` file. The file is validated and migrated by the
 * core; it always gets a fresh id so it cannot overwrite an existing recipe.
 *
 * @param file - Recipe file.
 * @returns The imported recipe, or `null` (see {@link recipeError}).
 */
export async function importRecipeFile(file: File): Promise<Recipe | null> {
  if (!requireFeature("recipe_save", { recipeCount: saved().length })) return null;
  try {
    const parsed = recipeParse(await file.text());
    const thumbnail =
      typeof parsed.thumbnail === "string" && parsed.thumbnail.startsWith("data:image/")
        ? parsed.thumbnail
        : null;
    const r = recipeParse(
      recipeToJson({ ...parsed, id: newId(), thumbnail, updated_at_ms: getClock().nowMs() }),
    );
    upsert(r);
    setError(null);
    return r;
  } catch {
    setError(INVALID_RECIPE_MESSAGE);
    return null;
  }
}

// ---- thumbnails rendered on the current image ----

interface SmallProxy {
  sourceId: number;
  frame: number;
  rgba: Uint8Array;
  width: number;
  height: number;
}

let small: SmallProxy | null = null;
const thumbCache = new Map<string, string | null>();
let thumbSource = -1;

function smallProxy(): SmallProxy | null {
  const src = untrack(currentSource);
  if (!src) return null;
  // Video thumbnails follow the paused or seeked frame; images never change.
  const frame = src.kind === "video" ? untrack(settledFrameCount) : 0;
  if (small?.sourceId !== src.id || small.frame !== frame) {
    const grabbed = src.kind === "video" ? grabFrame(THUMB_SIDE) : null;
    if (grabbed) small = { sourceId: src.id, frame, ...grabbed };
    else {
      const p = src.proxy;
      small = { sourceId: src.id, frame, ...downscaleRgba(p.rgba, p.width, p.height, THUMB_SIDE) };
    }
  }
  return small;
}

/**
 * Thumbnail of a recipe applied to the current image, rendered with the CPU
 * reference renderer on a {@link THUMB_SIDE} px copy and cached per image.
 *
 * @param r - Saved or starter recipe.
 * @returns A `data:` URL, the recipe's stored thumbnail when no image is open, or `null`.
 */
export function recipeThumbnail(r: Recipe): string | null {
  const src = untrack(currentSource);
  if (!src) return r.thumbnail ?? null;
  const stamp = src.kind === "video" ? src.id * 1e6 + untrack(settledFrameCount) : src.id;
  if (thumbSource !== stamp) {
    thumbCache.clear();
    thumbSource = stamp;
  }
  const key = `${r.id}:${r.updated_at_ms}`;
  if (thumbCache.has(key)) return thumbCache.get(key) ?? null;
  const applied = recipeApply(r, untrack(gradeParams));
  const url = thumbnailOf(applied, getPath(applied, "stage2.lut_id") as string | null);
  thumbCache.set(key, url);
  return url;
}

/**
 * The thumbnail for the current grade's own look, used by onboarding.
 *
 * @param params - Grade to render on the current image.
 * @returns A `data:` URL, or `null` when no image is open.
 */
export function renderCurrentThumbnail(params: GradeParameters): string | null {
  return thumbnailOf(params, getPath(params, "stage2.lut_id") as string | null);
}

/** @returns Free-tier recipe slots used, e.g. for a count label (tracked). */
export function recipeLimitLabel(): string | null {
  const cap = entitlements.limits().recipes;
  return cap === null ? null : `${saved().length} of ${cap} saved`;
}
