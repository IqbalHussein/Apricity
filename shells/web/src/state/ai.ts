import { createSignal } from "solid-js";
import { ngaApply, ngaSurprise } from "../core/wasm";
import type { NgaResult, PromptMode } from "../core/wasm";
import { entitlements } from "../entitlements/store";
import { markAi } from "./aimarks";
import { getRng } from "./clock";
import type { Rng } from "./clock";
import { applyParams, gradeParams } from "./grade";
import { ui } from "./ui";

/** How many prompts the history keeps. */
export const PROMPT_HISTORY_MAX = 20;

/**
 * Persistence seam for prompt history; {@link attachPromptStore} plugs the
 * IndexedDB store in (ADR-0010).
 */
export interface PromptHistoryStore {
  /** @returns Saved prompts, newest first. */
  load(): Promise<readonly string[]>;
  /**
   * @param entries - Prompts to persist, newest first.
   */
  save(entries: readonly string[]): Promise<void>;
}

/** In-memory prompt history, newest first, capped, de-duplicated (case-insensitive). */
export class PromptHistory {
  private list: string[] = [];

  /**
   * @param cap - Maximum entries kept.
   * @param store - Optional persistence; failures are ignored.
   */
  constructor(
    private readonly cap = PROMPT_HISTORY_MAX,
    private readonly store?: PromptHistoryStore,
  ) {}

  /**
   * Add a prompt; a repeat moves to the front.
   *
   * @param prompt - Prompt text; blank text is ignored.
   * @returns The updated list, newest first.
   */
  add(prompt: string): readonly string[] {
    const text = prompt.trim();
    if (text === "") return this.list;
    this.list = [text, ...this.list.filter((p) => p.toLowerCase() !== text.toLowerCase())].slice(
      0,
      this.cap,
    );
    void this.store?.save(this.list).catch(() => undefined);
    return this.list;
  }

  /**
   * Replace the list from storage.
   *
   * @returns The loaded list, newest first.
   */
  async hydrate(): Promise<readonly string[]> {
    if (this.store) {
      try {
        this.list = [...(await this.store.load())].slice(0, this.cap);
      } catch {
        // storage failure leaves the in-memory list as is
      }
    }
    return this.list;
  }

  /** @returns Prompts, newest first. */
  entries(): readonly string[] {
    return this.list;
  }
}

let history = new PromptHistory();
const [prompts, setPrompts] = createSignal<readonly string[]>([]);
const [lastResult, setLastResult] = createSignal<NgaResult | null>(null);
const [lastPrompt, setLastPrompt] = createSignal<string | null>(null);
const [error, setError] = createSignal<string | null>(null);

/**
 * Persist prompt history through `store` and load what was saved earlier.
 *
 * @param store - Storage seam; a failing store leaves the in-memory history in use.
 */
export async function attachPromptStore(store: PromptHistoryStore): Promise<void> {
  history = new PromptHistory(PROMPT_HISTORY_MAX, store);
  setPrompts(await history.hydrate());
}

/** Prompt history, newest first (tracked). */
export const promptHistory = prompts;
/** The last NGA result, or `null` (tracked). */
export const lastNgaResult = lastResult;
/** Last prompt text sent (tracked). */
export const lastPromptText = lastPrompt;
/** Last NGA error message (tracked). */
export const ngaError = error;

/**
 * Run the NGA on a prompt and apply the result as one undo step.
 *
 * @param prompt - Natural-language look description.
 * @param mode - `auto` for Apply, `refine` for Refine.
 * @returns The result, or `null` when the prompt was empty or the core rejected it.
 */
export function runPrompt(prompt: string, mode: PromptMode): NgaResult | null {
  const text = prompt.trim();
  if (text === "") {
    setError("Describe the look you want first.");
    return null;
  }
  const gate = entitlements.gate("prompt");
  if (!gate.allowed) {
    setError(gate.reason);
    return null;
  }
  try {
    const result = ngaApply(text, gradeParams(), mode);
    finish(result, text);
    setPrompts(history.add(text));
    entitlements.record("prompts");
    return result;
  } catch (e) {
    setError(e instanceof Error ? e.message : String(e));
    return null;
  }
}

/**
 * Apply a seeded random variation ("Surprise me").
 *
 * @param rng - Seed source (injected so tests are reproducible).
 * @returns The result, or `null` on failure.
 */
export function runSurprise(rng: Rng = getRng()): NgaResult | null {
  try {
    const result = ngaSurprise(gradeParams(), lastPrompt(), rng.nextSeed());
    finish(result, "Surprise me");
    return result;
  } catch (e) {
    setError(e instanceof Error ? e.message : String(e));
    return null;
  }
}

function finish(result: NgaResult, label: string): void {
  const before: Record<string, number> = {};
  for (const c of result.changes) if (c.unit !== "curve") before[c.path] = c.before;
  applyParams(result.params, label);
  markAi(
    result.changes.map((c) => c.path),
    before,
    result.mode_used === "fresh",
  );
  setError(null);
  setLastResult(result);
  if (label !== "Surprise me") setLastPrompt(label);
  ui.setExplainOpen(true);
}

/** Clear NGA state when a new image opens. */
export function resetAi(): void {
  setLastResult(null);
  setError(null);
  ui.setExplainOpen(false);
}
