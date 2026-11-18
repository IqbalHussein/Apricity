import type { UsageRecord } from "../storage/idb";

/**
 * Calendar month key used for usage counters.
 *
 * @param nowMs - Time in epoch milliseconds (UTC), from the injected clock.
 * @returns `YYYY-MM` in UTC.
 */
export function monthKey(nowMs: number): string {
  const d = new Date(nowMs);
  return `${d.getUTCFullYear()}-${String(d.getUTCMonth() + 1).padStart(2, "0")}`;
}

/** What is counted against monthly limits. */
export type UsageKind = "imports" | "prompts";

/**
 * Counters for the current calendar month. Pure and clock-agnostic: callers
 * pass the time, and a record from an earlier month reads as zero.
 */
export class UsageCounter {
  private record: UsageRecord;

  /**
   * @param nowMs - Current time, epoch milliseconds (UTC).
   * @param saved - Stored record, if any; ignored when it belongs to another month.
   */
  constructor(nowMs: number, saved?: UsageRecord) {
    const month = monthKey(nowMs);
    this.record =
      saved && saved.month === month ? { ...saved } : { month, imports: 0, prompts: 0 };
  }

  /**
   * @param nowMs - Current time, epoch milliseconds (UTC).
   * @param kind - Counter to read.
   * @returns Uses so far in the month containing `nowMs` (0 after a month rollover).
   */
  count(nowMs: number, kind: UsageKind): number {
    return this.record.month === monthKey(nowMs) ? this.record[kind] : 0;
  }

  /**
   * Add one use, rolling over to a fresh month first when the month changed.
   *
   * @param nowMs - Current time, epoch milliseconds (UTC).
   * @param kind - Counter to increment.
   * @returns The record to persist.
   */
  increment(nowMs: number, kind: UsageKind): UsageRecord {
    const month = monthKey(nowMs);
    if (this.record.month !== month) this.record = { month, imports: 0, prompts: 0 };
    this.record[kind] += 1;
    return { ...this.record };
  }
}
