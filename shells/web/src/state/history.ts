import { createSignal } from "solid-js";
import type { GradeParameters } from "../core/wasm";
import { getClock } from "./clock";
import type { Clock } from "./clock";

/** One undo step. */
export interface HistoryEntry {
  /** Full grade after this step. */
  params: GradeParameters;
  /** Short label for the step ("Contrast", a prompt, "Surprise me"). */
  label: string;
  /** Coalescing key (e.g. `scalar:stage2.contrast`), or `null` for a standalone step. */
  key: string | null;
  /** Time of the latest change merged into this step, epoch milliseconds. */
  atMs: number;
}

/** Largest number of undo steps kept. */
export const HISTORY_MAX = 200;
/** Edits with the same key closer together than this merge into one step, in milliseconds. */
export const COALESCE_MS = 1000;

/**
 * Linear undo/redo stack of grade snapshots. Slider drags that share a key
 * and arrive within {@link COALESCE_MS} of each other collapse into one entry.
 * Pure data structure: no reactivity, clock injected.
 */
export class HistoryStack {
  private entries: HistoryEntry[];
  private index = 0;

  /**
   * @param initial - Baseline grade (entry 0, label "Start").
   * @param clock - Time source for coalescing.
   * @param max - Maximum entries kept (oldest dropped first).
   * @param coalesceMs - Coalescing window in milliseconds.
   */
  constructor(
    initial: GradeParameters,
    private readonly clock: Clock,
    private readonly max = HISTORY_MAX,
    private readonly coalesceMs = COALESCE_MS,
  ) {
    this.entries = [{ params: initial, label: "Start", key: null, atMs: clock.nowMs() }];
  }

  /**
   * Record a new grade.
   *
   * @param params - Grade after the edit.
   * @param label - Step label.
   * @param key - Coalescing key; consecutive pushes with the same key merge.
   */
  push(params: GradeParameters, label: string, key: string | null = null): void {
    const now = this.clock.nowMs();
    const top = this.entries[this.index];
    const atTop = this.index === this.entries.length - 1;
    if (
      key !== null &&
      top &&
      atTop &&
      this.index > 0 &&
      top.key === key &&
      now - top.atMs <= this.coalesceMs
    ) {
      this.entries[this.index] = { params, label, key, atMs: now };
      return;
    }
    this.entries = this.entries.slice(0, this.index + 1);
    this.entries.push({ params, label, key, atMs: now });
    if (this.entries.length > this.max) this.entries.splice(0, this.entries.length - this.max);
    this.index = this.entries.length - 1;
  }

  /** Start over with a new baseline (new image). */
  reset(params: GradeParameters): void {
    this.entries = [{ params, label: "Start", key: null, atMs: this.clock.nowMs() }];
    this.index = 0;
  }

  /**
   * Step back.
   *
   * @returns The entry now current, or `null` at the oldest step.
   */
  undo(): HistoryEntry | null {
    if (this.index === 0) return null;
    this.index -= 1;
    return this.entries[this.index] ?? null;
  }

  /**
   * Step forward.
   *
   * @returns The entry now current, or `null` at the newest step.
   */
  redo(): HistoryEntry | null {
    if (this.index >= this.entries.length - 1) return null;
    this.index += 1;
    return this.entries[this.index] ?? null;
  }

  /** Whether {@link undo} would change anything. */
  get canUndo(): boolean {
    return this.index > 0;
  }

  /** Whether {@link redo} would change anything. */
  get canRedo(): boolean {
    return this.index < this.entries.length - 1;
  }

  /** The current entry. */
  get current(): HistoryEntry {
    return this.entries[this.index] as HistoryEntry;
  }

  /** Number of entries including the baseline. */
  get size(): number {
    return this.entries.length;
  }

  /**
   * The grade before the latest step, for "Previous grade" comparison.
   *
   * @returns The previous entry's grade, or `null` at the baseline.
   */
  previousParams(): GradeParameters | null {
    return this.index > 0 ? (this.entries[this.index - 1]?.params ?? null) : null;
  }

  /** Label of the step {@link undo} would revert, or `null`. */
  get undoLabel(): string | null {
    return this.index > 0 ? this.current.label : null;
  }

  /** Label of the step {@link redo} would re-apply, or `null`. */
  get redoLabel(): string | null {
    return this.canRedo ? (this.entries[this.index + 1]?.label ?? null) : null;
  }
}

const [revision, setRevision] = createSignal(0);
let stack = new HistoryStack({}, getClock());

/**
 * Replace the app-wide stack (init and tests).
 *
 * @param initial - Baseline grade.
 * @param clock - Time source.
 * @returns The new stack.
 */
export function resetHistory(initial: GradeParameters, clock: Clock = getClock()): HistoryStack {
  stack = new HistoryStack(initial, clock);
  setRevision((r) => r + 1);
  return stack;
}

/**
 * The app-wide stack for imperative use; reactive reads go through {@link history}.
 *
 * @returns The live stack.
 */
export function historyStack(): HistoryStack {
  return stack;
}

/** Reactive read-only view of the app-wide history. */
export const history = {
  /** @returns `true` when undo is possible (tracked). */
  canUndo: (): boolean => (revision(), stack.canUndo),
  /** @returns `true` when redo is possible (tracked). */
  canRedo: (): boolean => (revision(), stack.canRedo),
  /** @returns Label of the step undo would revert (tracked). */
  undoLabel: (): string | null => (revision(), stack.undoLabel),
  /** @returns Label of the step redo would re-apply (tracked). */
  redoLabel: (): string | null => (revision(), stack.redoLabel),
  /** @returns Grade before the latest step, or `null` (tracked). */
  previousParams: (): GradeParameters | null => (revision(), stack.previousParams()),
  /** @returns Entry count (tracked). */
  size: (): number => (revision(), stack.size),
};

/** Notify readers after the stack was mutated. */
export function touchHistory(): void {
  setRevision((r) => r + 1);
}
