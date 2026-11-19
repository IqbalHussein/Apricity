import type { GradeParameters } from "../core/wasm";

/** One of the two grade slots. */
export type AbSlot = "A" | "B";

/** Outcome of an A/B operation: the grade to make current, if it changes. */
export interface AbStep {
  /** Slot that is active after the step. */
  active: AbSlot;
  /** Grade to apply, or `null` when the live grade stays as it is. */
  apply: GradeParameters | null;
  /** Undo label for `apply`. */
  label: string;
}

/**
 * Two grades for one image (PRD §6.6). The active slot is the live grade; the
 * other slot holds the grade parked there. Pure data: the caller applies the
 * returned grade.
 */
export class AbSlots {
  private parked: Record<AbSlot, GradeParameters | null> = { A: null, B: null };
  private current: AbSlot = "A";

  /** @returns The slot that is live. */
  get active(): AbSlot {
    return this.current;
  }

  /** @returns `true` once the other slot holds a parked grade. */
  get otherParked(): boolean {
    return this.parked[other(this.current)] !== null;
  }

  /**
   * Switch to the other slot. The live grade is parked in the slot being left;
   * a slot that was never used starts as a copy of the live grade.
   *
   * @param live - The grade being edited now.
   * @returns The step to apply.
   */
  toggle(live: GradeParameters): AbStep {
    const from = this.current;
    const to = other(from);
    this.parked[from] = live;
    this.current = to;
    return { active: to, apply: this.parked[to] ?? live, label: `Switch to grade ${to}` };
  }

  /**
   * Make B equal to A.
   *
   * @param live - The grade being edited now.
   * @returns The step to apply; `apply` is `null` when A is the live slot (B just stores a copy).
   */
  copyAToB(live: GradeParameters): AbStep {
    if (this.current === "A") {
      this.parked.B = live;
      return { active: "A", apply: null, label: "Copy A to B" };
    }
    return { active: "B", apply: this.parked.A ?? live, label: "Copy A to B" };
  }

  /** Forget both slots (new image). */
  reset(): void {
    this.parked = { A: null, B: null };
    this.current = "A";
  }
}

function other(s: AbSlot): AbSlot {
  return s === "A" ? "B" : "A";
}
