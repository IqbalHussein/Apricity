import { limitsFor } from "./limits";
import type { Tier, TierLimits } from "./limits";

/**
 * Source of the user's entitlements (ADR-0009). A payment provider later
 * implements this one interface; the gating logic does not change.
 */
export interface EntitlementProvider {
  /** @returns The tier the user currently holds. */
  tier(): Tier;
  /** @returns The limits that apply to that tier. */
  limits(): TierLimits;
  /** @returns `true` while every feature is unlocked for the beta, whatever the tier. */
  readonly beta: boolean;
}

/**
 * During the beta every user gets Pro, so nobody meets an unpurchasable tier.
 * Turning this off needs a decision on the payment provider (PRD §14 Q9, ADR-0009).
 */
export const BETA_ALL_PRO = true;

/** Entitlements held locally; no account or network involved. */
export class LocalBetaProvider implements EntitlementProvider {
  /**
   * @param betaAllPro - Grant Pro to everyone (default {@link BETA_ALL_PRO}).
   * @param baseTier - Tier held when `betaAllPro` is `false`.
   */
  constructor(
    private readonly betaAllPro: boolean = BETA_ALL_PRO,
    private readonly baseTier: Tier = "free",
  ) {}

  get beta(): boolean {
    return this.betaAllPro;
  }

  tier(): Tier {
    return this.betaAllPro ? "pro" : this.baseTier;
  }

  limits(): TierLimits {
    return limitsFor(this.tier());
  }
}

/**
 * Whether the beta unlock applies to this page load. `?beta=0` previews the
 * Free tier (a stricter state, so it grants nothing); anything else keeps {@link BETA_ALL_PRO}.
 *
 * @param search - `location.search`.
 * @returns `true` when every feature is unlocked.
 */
export function betaFromSearch(search: string): boolean {
  return new URLSearchParams(search).get("beta") === "0" ? false : BETA_ALL_PRO;
}
