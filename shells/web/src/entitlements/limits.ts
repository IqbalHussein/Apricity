/** Which scopes a tier may open (matches `ScopeTab` in the UI state). */
export type ScopeName = "waveform" | "parade" | "vectorscope" | "histogram";

/** What a tier is allowed to do (PRD §9.1). `null` means unlimited. */
export interface TierLimits {
  /** Image imports per calendar month. */
  importsPerMonth: number | null;
  /** AI grading prompts per calendar month. */
  promptsPerMonth: number | null;
  /** Saved recipes (built-in starters do not count). */
  recipes: number | null;
  /** Longest edge of an exported image, in pixels. `null` exports at full resolution. */
  exportLongEdge: number | null;
  /** Whether exports carry the "Graded with Apricity" mark. */
  watermark: boolean;
  /** Manual Console groups (spec `group` names) that are usable; `null` means all. */
  consoleGroups: readonly string[] | null;
  /** Scopes that are usable. */
  scopes: readonly ScopeName[];
  /** Importing `.cube` LUTs. */
  lutImport: boolean;
  /** Exporting `.cube` LUTs. */
  lutExport: boolean;
  /** The A/B grade tool. */
  abGrade: boolean;
}

/** Subscription tier. */
export type Tier = "free" | "pro";

/** PRD §9.1, Free column. */
export const FREE_LIMITS: TierLimits = {
  importsPerMonth: 50,
  promptsPerMonth: 15,
  recipes: 5,
  exportLongEdge: 2048,
  watermark: true,
  consoleGroups: ["Light", "Color"],
  scopes: ["histogram"],
  lutImport: false,
  lutExport: false,
  abGrade: false,
};

/** PRD §9.1, Pro column (images and video export at full resolution). */
export const PRO_LIMITS: TierLimits = {
  importsPerMonth: null,
  promptsPerMonth: null,
  recipes: null,
  exportLongEdge: null,
  watermark: false,
  consoleGroups: null,
  scopes: ["waveform", "parade", "vectorscope", "histogram"],
  lutImport: true,
  lutExport: true,
  abGrade: true,
};

/**
 * @param tier - Subscription tier.
 * @returns The limits for the tier.
 */
export function limitsFor(tier: Tier): TierLimits {
  return tier === "pro" ? PRO_LIMITS : FREE_LIMITS;
}
