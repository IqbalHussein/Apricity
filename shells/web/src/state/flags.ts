/** Feature flags for experiments that are not settled product decisions. */
export interface Flags {
  /**
   * First-run onboarding (PRD §14 Q7 hypothesis, validated after the first
   * beta). Default on; `?onboarding=0` turns it off for a page load.
   */
  onboarding: boolean;
}

/** Default flag values. */
export const DEFAULT_FLAGS: Flags = { onboarding: true };

/**
 * Resolve flags, letting the query string override defaults.
 *
 * @param search - `location.search`, e.g. `?onboarding=0`.
 * @returns The flags in effect.
 */
export function resolveFlags(search: string): Flags {
  const q = new URLSearchParams(search);
  const o = q.get("onboarding");
  return { ...DEFAULT_FLAGS, onboarding: o === null ? DEFAULT_FLAGS.onboarding : o !== "0" };
}

/** Flags for this page load. */
export const flags: Flags = resolveFlags(typeof location === "undefined" ? "" : location.search);
