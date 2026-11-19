import { createSignal } from "solid-js";
import { lutParse } from "../core/wasm";
import { INVALID_LUT_MESSAGE, LutLibrary, lutTitle, makeLutId, MAX_LUT_BYTES } from "../lib/lutLibrary";
import { getPath, setPath } from "../lib/paramPath";
import { requireFeature } from "../entitlements/store";
import { LutStore, storage } from "../storage/idb";
import type { LutRecord } from "../storage/idb";
import { getRng } from "./clock";
import { applyParams, gradeParams, setField } from "./grade";
import { clearAllAi } from "./aimarks";

const library = new LutLibrary();
const [revision, setRevision] = createSignal(0);
const [error, setError] = createSignal<string | null>(null);

/** Last LUT import error copy, or `null` (tracked). */
export const lutError = error;

/**
 * The LUT referenced by the current grade.
 *
 * @returns The stored LUT, or `null` when `stage2.lut_id` is unset or unknown on this device (tracked).
 */
export function activeLut(): LutRecord | null {
  revision();
  return library.get(getPath(gradeParams(), "stage2.lut_id") as string | null | undefined) ?? null;
}

/**
 * `.cube` text of a LUT by id, for rendering recipe thumbnails.
 *
 * @param id - A `stage2.lut_id` value.
 * @returns The file text, or `null` when unknown.
 */
export function lutTextFor(id: string | null | undefined): string | null {
  return library.get(id)?.text ?? null;
}

/** Load stored LUTs. Never throws. */
export async function hydrateLuts(): Promise<void> {
  try {
    for (const rec of await new LutStore(await storage()).list()) library.add(rec);
    setRevision((r) => r + 1);
  } catch {
    // LUTs imported earlier stay unavailable for this session
  }
}

/**
 * Import a `.cube` file: validate it with the core, store it, and make it the
 * grade's LUT at full strength as one undo step. Gated as a Pro feature.
 *
 * @param file - A `.cube` 3D LUT.
 * @returns `true` on success; on failure {@link lutError} holds the copy.
 */
export async function importLutFile(file: File): Promise<boolean> {
  if (!requireFeature("lut_import")) return false;
  try {
    if (file.size > MAX_LUT_BYTES) throw new Error(INVALID_LUT_MESSAGE);
    const text = await file.text();
    let info;
    try {
      info = lutParse(text);
    } catch {
      throw new Error(INVALID_LUT_MESSAGE);
    }
    const rec: LutRecord = {
      id: makeLutId(getRng()),
      title: lutTitle(info, file.name),
      size: info.size,
      text,
    };
    library.add(rec);
    setRevision((r) => r + 1);
    void storage()
      .then((b) => new LutStore(b).put(rec))
      .catch(() => undefined);
    let next = setPath(gradeParams(), "stage2.lut_id", rec.id);
    next = setPath(next, "stage2.lut_strength", 100);
    applyParams(next, `Import LUT: ${rec.title}`);
    clearAllAi();
    setError(null);
    return true;
  } catch (e) {
    setError(e instanceof Error ? e.message : String(e));
    return false;
  }
}

/** Remove the LUT from the current grade as one undo step. */
export function removeLut(): void {
  setField("stage2.lut_id", null, "Remove LUT");
}
