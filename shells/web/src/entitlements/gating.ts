import type { TierLimits } from "./limits";
import type { UsageKind } from "./usage";

/** A gated capability. `console:<group>` and `scope:<name>` carry their argument. */
export type Feature =
  | "import"
  | "prompt"
  | "recipe_save"
  | "lut_import"
  | "lut_export"
  | "ab"
  | `console:${string}`
  | `scope:${string}`;

/** Outcome of a gate check. */
export type GateResult = { allowed: true } | { allowed: false; reason: string };

/** Inputs a gate check needs besides the limits. */
export interface GateContext {
  /** Uses so far this month. */
  usage: Readonly<Record<UsageKind, number>>;
  /** Recipes currently saved (starters excluded). */
  recipeCount: number;
}

const ALLOWED: GateResult = { allowed: true };

function deny(reason: string): GateResult {
  return { allowed: false, reason };
}

function plural(n: number, one: string, many: string): string {
  return `${n} ${n === 1 ? one : many}`;
}

/**
 * Decide whether a feature is available. Pure: the same limits, usage and
 * count always give the same answer.
 *
 * @param feature - What the user is trying to do.
 * @param limits - The tier's limits.
 * @param ctx - Monthly usage and saved-recipe count.
 * @returns Allowed, or the reason it is not (copy for the notice).
 */
export function evaluate(feature: Feature, limits: TierLimits, ctx: GateContext): GateResult {
  if (feature === "import") {
    const cap = limits.importsPerMonth;
    return cap !== null && ctx.usage.imports >= cap
      ? deny(`Free includes ${plural(cap, "import", "imports")} a month, and this month's are used.`)
      : ALLOWED;
  }
  if (feature === "prompt") {
    const cap = limits.promptsPerMonth;
    return cap !== null && ctx.usage.prompts >= cap
      ? deny(`Free includes ${plural(cap, "AI prompt", "AI prompts")} a month, and this month's are used.`)
      : ALLOWED;
  }
  if (feature === "recipe_save") {
    const cap = limits.recipes;
    return cap !== null && ctx.recipeCount >= cap
      ? deny(`Free keeps up to ${plural(cap, "recipe", "recipes")}. Delete one to save another.`)
      : ALLOWED;
  }
  if (feature === "lut_import")
    return limits.lutImport ? ALLOWED : deny("Importing LUTs is a Pro feature.");
  if (feature === "lut_export")
    return limits.lutExport ? ALLOWED : deny("Exporting LUTs is a Pro feature.");
  if (feature === "ab") return limits.abGrade ? ALLOWED : deny("The A/B grade tool is a Pro feature.");
  if (feature.startsWith("console:")) {
    const group = feature.slice("console:".length);
    return limits.consoleGroups === null || limits.consoleGroups.includes(group)
      ? ALLOWED
      : deny(`Free includes the Light and Color sliders. ${group} needs Pro.`);
  }
  if (feature.startsWith("scope:")) {
    const scope = feature.slice("scope:".length);
    return (limits.scopes as readonly string[]).includes(scope)
      ? ALLOWED
      : deny("Free includes the histogram. The other scopes need Pro.");
  }
  return ALLOWED;
}

/** Copy shown in the beta next to a feature that Free would not have. */
export const BETA_NOTICE = "Pro feature. During the beta, every feature is unlocked.";
