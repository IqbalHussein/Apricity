import type { JSX } from "solid-js";
import { abActive, abBStored, copyAToB, toggleAb } from "../state/ab";
import { currentSource } from "../state/project";

/**
 * A/B grade toggle for the top bar (PRD §6.6): one click or the `\` key swaps
 * the live grade between slots A and B; "A → B" copies A into B.
 */
export function AbToggle(): JSX.Element {
  const off = (): boolean => currentSource() === null;
  return (
    <div class="ab-slot" data-slot="ab" role="group" aria-label="A/B grades">
      <button
        type="button"
        class="ab-toggle"
        data-testid="ab-toggle"
        data-active={abActive()}
        disabled={off()}
        aria-label={`Grade ${abActive()} is live. Switch to grade ${abActive() === "A" ? "B" : "A"}`}
        title="Switch between grades A and B (\)"
        onClick={() => toggleAb()}
      >
        <span class="ab-cell" classList={{ on: abActive() === "A" }}>
          A
        </span>
        <span class="ab-cell" classList={{ on: abActive() === "B" }}>
          B
        </span>
      </button>
      <button
        type="button"
        class="btn btn-quiet ab-copy"
        data-testid="ab-copy"
        disabled={off()}
        aria-label="Copy A to B"
        title={abBStored() ? "Replace B with A" : "Copy A to B"}
        onClick={() => copyAToB()}
      >
        A → B
      </button>
    </div>
  );
}
