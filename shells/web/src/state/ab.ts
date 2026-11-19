import { createSignal } from "solid-js";
import { AbSlots } from "../lib/abSlots";
import type { AbSlot } from "../lib/abSlots";
import { requireFeature } from "../entitlements/store";
import { clearAllAi } from "./aimarks";
import { applyParams, gradeParams } from "./grade";
import { currentSource } from "./project";

const slots = new AbSlots();
const [active, setActive] = createSignal<AbSlot>("A");
const [bStored, setBStored] = createSignal(false);

/** The live slot (tracked). */
export const abActive = active;
/** `true` once B holds a grade (tracked). */
export const abBStored = bStored;

/** Forget both slots when a new image opens. */
export function resetAb(): void {
  slots.reset();
  setActive("A");
  setBStored(false);
}

/**
 * Swap to the other grade as one undo step. Gated as a Pro feature.
 *
 * @returns `true` when the grade switched.
 */
export function toggleAb(): boolean {
  if (!currentSource() || !requireFeature("ab")) return false;
  const step = slots.toggle(gradeParams());
  setActive(step.active);
  setBStored(true);
  if (step.apply) {
    applyParams(step.apply, step.label);
    clearAllAi();
  }
  return true;
}

/**
 * Copy the A grade into B.
 *
 * @returns `true` when something was copied.
 */
export function copyAToB(): boolean {
  if (!currentSource() || !requireFeature("ab")) return false;
  const step = slots.copyAToB(gradeParams());
  setBStored(true);
  if (step.apply) {
    applyParams(step.apply, step.label);
    clearAllAi();
  }
  return true;
}
