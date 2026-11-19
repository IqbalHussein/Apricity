import { createSignal, Show } from "solid-js";
import type { JSX } from "solid-js";
import { ui } from "../state/ui";
import { ChevronIcon } from "./Icons";
import { ManualConsole } from "./ManualConsole";
import { ScopesPanel } from "./ScopesPanel";
import { Segmented } from "./Segmented";
import { Stage1Panel } from "./Stage1Panel";

/**
 * Grade-mode inspector: Stage 1 (Correction), a 1px divider, then Stage 2
 * (the Manual Console), with scopes docked below on desktop. On mobile a
 * Controls / Scopes switch shares the sheet.
 *
 * @param props.mobile - `true` for the bottom-sheet layout.
 */
export function Inspector(props: { mobile: boolean }): JSX.Element {
  const [pane, setPane] = createSignal<"controls" | "scopes">("controls");
  return (
    <div class="inspector" classList={{ "inspector-mobile": props.mobile }}>
      <Show when={props.mobile}>
        <Segmented<"controls" | "scopes">
          class="inspector-switch"
          label="Inspector view"
          value={pane()}
          onChange={setPane}
          options={[
            { value: "controls", label: "Controls" },
            { value: "scopes", label: "Scopes" },
          ]}
        />
      </Show>
      <Show when={!props.mobile || pane() === "controls"}>
        <div class="inspector-scroll">
          <Stage1Panel />
          <hr class="stage-divider" />
          <h2 class="stage-title stage-title-creative">Grade</h2>
          <ManualConsole />
        </div>
      </Show>
      <Show when={!props.mobile || pane() === "scopes"}>
        <div class="inspector-scopes" classList={{ collapsed: !props.mobile && !ui.scopesOpen() }}>
          <Show when={!props.mobile}>
            <button
              type="button"
              class="section-toggle scopes-toggle"
              data-testid="scopes-toggle"
              aria-expanded={ui.scopesOpen()}
              onClick={() => ui.setScopesOpen(!ui.scopesOpen())}
            >
              <span class="chev" classList={{ open: ui.scopesOpen() }}>
                <ChevronIcon size={12} />
              </span>
              Scopes
            </button>
          </Show>
          <Show when={props.mobile || ui.scopesOpen()}>
            <ScopesPanel />
          </Show>
        </div>
      </Show>
    </div>
  );
}
