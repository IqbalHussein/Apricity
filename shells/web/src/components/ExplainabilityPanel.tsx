import { For, Show } from "solid-js";
import type { JSX } from "solid-js";
import { lastNgaResult } from "../state/ai";
import { ui } from "../state/ui";
import { ChevronIcon } from "./Icons";

/**
 * Explainability Panel (PRD G2, P0): slides up after a prompt with the
 * summary sentence and one row per change (label, before to after,
 * rationale, "Show in console"). Collapsible.
 */
export function ExplainabilityPanel(): JSX.Element {
  return (
    <Show when={lastNgaResult()} keyed>
      {(result) => (
        <section
          class="explain"
          classList={{ collapsed: !ui.explainOpen() }}
          aria-label="Explainability"
        >
          <header class="explain-head">
            <button
              type="button"
              class="explain-toggle"
              aria-expanded={ui.explainOpen()}
              aria-controls="explain-body"
              onClick={() => ui.setExplainOpen(!ui.explainOpen())}
            >
              <span class="chev" classList={{ open: ui.explainOpen() }}>
                <ChevronIcon size={14} />
              </span>
              <span class="explain-summary" aria-live="polite">
                {result.summary}
              </span>
            </button>
          </header>
          <div class="explain-body" id="explain-body" hidden={!ui.explainOpen()}>
            <Show when={result.changes.length === 0}>
              <p class="explain-empty">
                Nothing changed. Try describing the mood, the light or the color.
              </p>
            </Show>
            <ul class="explain-list">
              <For each={result.changes}>
                {(c) => (
                  <li class="explain-row" classList={{ "row-flash": ui.flash()?.path === c.path }}>
                    <div class="explain-main">
                      <span class="explain-label">{c.label}</span>
                      <span class="explain-delta">
                        {c.before_text} <span aria-label="to">&rarr;</span> {c.after_text}
                      </span>
                      <p class="explain-why">{c.rationale}</p>
                    </div>
                    <button
                      type="button"
                      class="icon-btn"
                      aria-label={`Show ${c.label} in console`}
                      title="Show in console"
                      onClick={() => {
                        ui.revealControl(c.console_anchor);
                        if (ui.mobile()) ui.setExplainOpen(false);
                      }}
                    >
                      <ChevronIcon size={16} />
                    </button>
                  </li>
                )}
              </For>
            </ul>
          </div>
        </section>
      )}
    </Show>
  );
}
