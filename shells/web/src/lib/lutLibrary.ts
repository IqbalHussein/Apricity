import type { LutInfo } from "../core/wasm";
import type { LutRecord } from "../storage/idb";
import type { Rng } from "../state/clock";

/** Largest `.cube` file accepted, in bytes (a 65-point LUT is about 6 MB). */
export const MAX_LUT_BYTES = 32 * 1024 * 1024;

/** Message for files that are not usable `.cube` LUTs. */
export const INVALID_LUT_MESSAGE =
  "This file isn't a valid .cube LUT. Use a 3D LUT with 2 to 256 points per axis.";

/**
 * Display title for an imported LUT.
 *
 * @param info - Parsed header (`TITLE` and size).
 * @param fileName - Original file name, used when the file has no title.
 * @returns The title, or the file name without its extension.
 */
export function lutTitle(info: LutInfo, fileName: string): string {
  const t = info.title?.trim();
  return t ? t : fileName.replace(/\.[^.]+$/, "") || "LUT";
}

/**
 * Generate an id for an imported LUT (`[A-Za-z0-9_-]`, safe for `stage2.lut_id`).
 *
 * @param rng - Injected random source.
 * @returns A fresh id such as `lut-k3j2h1`.
 */
export function makeLutId(rng: Rng): string {
  return `lut-${rng.nextSeed().toString(36)}`;
}

/** The imported LUTs known on this device, by id. Pure data structure. */
export class LutLibrary {
  private readonly byId = new Map<string, LutRecord>();

  /**
   * @param rec - LUT to add or replace.
   */
  add(rec: LutRecord): void {
    this.byId.set(rec.id, rec);
  }

  /**
   * @param id - A `stage2.lut_id` value, or `null`/`undefined` for none.
   * @returns The LUT, or `undefined` when none is set or the id is unknown.
   */
  get(id: string | null | undefined): LutRecord | undefined {
    return id ? this.byId.get(id) : undefined;
  }

  /** @returns All LUTs, in insertion order. */
  list(): LutRecord[] {
    return [...this.byId.values()];
  }
}
