import { createSignal } from "solid-js";
import type { Stage1Analysis } from "../core/wasm";
import { decodeToProxy } from "../lib/imageProxy";
import type { ImageProxy } from "../lib/imageProxy";
import { generateSampleImage } from "../lib/sampleImage";
import { entitlements, requireFeature } from "../entitlements/store";
import { resetAi } from "./ai";
import { resetAb } from "./ab";
import { resetForNewSource } from "./grade";
import { ui } from "./ui";

/** What kind of media a {@link Source} holds. Only images exist until M7 adds video. */
export type SourceKind = "image" | "video";

/** The image being graded. */
export interface Source {
  /** Media kind; export UI sections are chosen by it. */
  kind: SourceKind;
  /**
   * The original file, kept so export can decode full resolution on demand
   * (no full-resolution pixels are held between exports). `null` for the
   * generated sample, whose proxy is already its full resolution.
   */
  file: File | null;
  /** Increments for every opened image so effects can detect a new one. */
  id: number;
  /** Display name (file name). */
  name: string;
  /** Preview proxy (sRGB RGBA8, at most 2048 px on the longest side) and full-resolution size. */
  proxy: ImageProxy;
}

/** A reference image for the Review wipe. */
export interface Reference {
  /** Increments for every loaded reference. */
  id: number;
  /** File name. */
  name: string;
  /** Reference proxy, at most 1024 px on the longest side. */
  proxy: ImageProxy;
}

const [source, setSource] = createSignal<Source | null>(null);
const [reference, setReference] = createSignal<Reference | null>(null);
const [analysis, setAnalysis] = createSignal<Stage1Analysis | null>(null);
const [loading, setLoading] = createSignal(false);
const [loadError, setLoadError] = createSignal<string | null>(null);
let nextId = 0;

/** The open image, or `null` (tracked). */
export const currentSource = source;
/** The loaded reference, or `null` (tracked). */
export const currentReference = reference;
/** Latest Stage 1 analysis, or `null` (tracked). */
export const stage1Analysis = analysis;
/** `true` while a file decodes (tracked). */
export const isLoading = loading;
/** User-facing import error, or `null` (tracked). */
export const importError = loadError;

/**
 * Record the Stage 1 analysis shown in the Correction panel.
 *
 * @param a - Analysis, or `null` to clear.
 */
export function setStage1Analysis(a: Stage1Analysis | null): void {
  // The first analysis of an image folds Correction into its summary line.
  if (a && analysis() === null) ui.setStage1Open(false);
  setAnalysis(a);
}

/**
 * Make a decoded proxy the open image. Resets the grade (Stage 1 auto flags
 * on), history, AI state and view.
 *
 * @param proxy - Decoded preview proxy.
 * @param name - Display name.
 * @param file - Original file for full-resolution export; `null` when the proxy is the full image.
 */
export function openProxy(proxy: ImageProxy, name: string, file: File | null = null): void {
  nextId += 1;
  resetForNewSource();
  resetAi();
  setAnalysis(null);
  setReference(null);
  if (ui.compareSource() === "reference") ui.setCompareSource("original");
  ui.setView({ zoom: 1, pan: [0, 0] });
  resetAb();
  setLoadError(null);
  setSource({ id: nextId, kind: "image", file, name, proxy });
}

/**
 * Decode and open an image file.
 *
 * @param file - JPEG, PNG, WebP or HEIC (where the browser decodes it).
 * @returns `true` on success; on failure {@link importError} holds the copy.
 */
export async function openImageFile(file: File): Promise<boolean> {
  if (!requireFeature("import")) return false;
  setLoading(true);
  try {
    openProxy(await decodeToProxy(file), file.name, file);
    entitlements.record("imports");
    return true;
  } catch (e) {
    setLoadError(e instanceof Error ? e.message : String(e));
    return false;
  } finally {
    setLoading(false);
  }
}

/** Open the built-in procedural sample image. */
export function openSample(): void {
  openProxy(generateSampleImage(), "Sample image");
}

/**
 * Load a reference image for the Review wipe.
 *
 * @param file - Image file.
 * @returns `true` on success.
 */
export async function openReferenceFile(file: File): Promise<boolean> {
  try {
    const proxy = await decodeToProxy(file, 1024);
    nextId += 1;
    setReference({ id: nextId, name: file.name, proxy });
    ui.setCompareSource("reference");
    setLoadError(null);
    return true;
  } catch (e) {
    setLoadError(e instanceof Error ? e.message : String(e));
    return false;
  }
}
