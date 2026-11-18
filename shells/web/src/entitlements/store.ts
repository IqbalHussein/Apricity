import { createSignal } from "solid-js";
import { getClock } from "../state/clock";
import { storage, UsageStore } from "../storage/idb";
import type { UsageRecord } from "../storage/idb";
import { BETA_NOTICE, evaluate } from "./gating";
import type { Feature, GateContext, GateResult } from "./gating";
import { FREE_LIMITS } from "./limits";
import { LocalBetaProvider } from "./provider";
import type { EntitlementProvider } from "./provider";
import { monthKey, UsageCounter } from "./usage";
import type { UsageKind } from "./usage";

const [provider, setProviderSignal] = createSignal<EntitlementProvider>(new LocalBetaProvider());
const [record, setRecord] = createSignal<UsageRecord | undefined>(undefined);
const [notice, setNotice] = createSignal<string | null>(null);

/**
 * @param kind - Counter to read.
 * @returns Uses so far this calendar month (tracked).
 */
function used(kind: UsageKind): number {
  return counter().count(getClock().nowMs(), kind);
}

function counter(): UsageCounter {
  return new UsageCounter(getClock().nowMs(), record());
}

async function usageStore(): Promise<UsageStore> {
  return new UsageStore(await storage());
}

/**
 * Replace the entitlement provider (a payment integration, or tests).
 *
 * @param p - Provider to use from now on.
 */
export function setEntitlementProvider(p: EntitlementProvider): void {
  setProviderSignal(p);
}

/** Load this month's usage counters from storage. Never throws. */
export async function hydrateEntitlements(): Promise<void> {
  try {
    const saved = await (await usageStore()).get(monthKey(getClock().nowMs()));
    if (saved) setRecord(saved);
  } catch {
    // counters stay at zero for this session
  }
}

/** Reactive entitlement state and gate checks (ADR-0009). */
export const entitlements = {
  /** @returns The tier in effect (tracked). */
  tier: () => provider().tier(),
  /** @returns The limits in effect (tracked). */
  limits: () => provider().limits(),
  /** @returns `true` while every feature is unlocked for the beta (tracked). */
  beta: () => provider().beta,
  /** Uses so far this calendar month for a counter (tracked). */
  used,
  /**
   * Check a feature without side effects.
   *
   * @param feature - Capability to check.
   * @param ctx - Extra context; `recipeCount` is needed for `recipe_save`.
   * @returns Allowed, or the reason it is not (tracked).
   */
  gate(feature: Feature, ctx: { recipeCount?: number } = {}): GateResult {
    const full: GateContext = {
      usage: {
        imports: used("imports"),
        prompts: used("prompts"),
      },
      recipeCount: ctx.recipeCount ?? 0,
    };
    return evaluate(feature, provider().limits(), full);
  },
  /**
   * Count one use against a monthly limit and persist it.
   *
   * @param kind - Counter to increment.
   */
  record(kind: UsageKind): void {
    const next = counter().increment(getClock().nowMs(), kind);
    setRecord(next);
    void usageStore()
      .then((s) => s.put(next))
      .catch(() => undefined);
  },
};

/**
 * Check a feature and, when it is blocked, show the reason as a notice.
 *
 * @param feature - Capability the user is trying to use.
 * @param ctx - Extra context (see {@link entitlements.gate}).
 * @returns `true` when the action may proceed.
 */
export function requireFeature(feature: Feature, ctx: { recipeCount?: number } = {}): boolean {
  const g = entitlements.gate(feature, ctx);
  if (!g.allowed) setNotice(g.reason);
  return g.allowed;
}

/**
 * Honest note for a feature that Free lacks.
 *
 * @param feature - Capability to describe.
 * @returns The beta note while every feature is unlocked, the block reason when it is
 *   blocked, or `null` when nothing needs saying (tracked).
 */
export function proNote(feature: Feature): string | null {
  const g = entitlements.gate(feature);
  if (!g.allowed) return g.reason;
  const onFree = evaluate(feature, FREE_LIMITS, { usage: { imports: 0, prompts: 0 }, recipeCount: 0 });
  return entitlements.beta() && !onFree.allowed ? BETA_NOTICE : null;
}

/** The notice currently shown, or `null` (tracked). */
export const currentNotice = notice;
/**
 * Show a short notice (toast).
 *
 * @param text - Plain sentence.
 */
export const showNotice = setNotice;
/** Hide the notice. */
export function clearNotice(): void {
  setNotice(null);
}
