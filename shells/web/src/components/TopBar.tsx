import { For, onCleanup, onMount, Show } from "solid-js";
import type { JSX } from "solid-js";
import { planckianGradient } from "../lib/kelvin";
import { pickImage } from "../lib/filePicker";
import { redo, undo } from "../state/grade";
import { history } from "../state/history";
import { currentSource, openImageFile } from "../state/project";
import type { Mode } from "../state/ui";
import { ui } from "../state/ui";
import { OpenIcon, RedoIcon, ThemeIcon, UndoIcon } from "./Icons";
import { Segmented } from "./Segmented";

const MODES: { value: Mode; label: string }[] = [
  { value: "quick", label: "Quick" },
  { value: "grade", label: "Grade" },
  { value: "review", label: "Review" },
];

function ModeTabs(): JSX.Element {
  const move = (delta: number): void => {
    const i = MODES.findIndex((x) => x.value === ui.mode());
    const next = MODES[(i + delta + MODES.length) % MODES.length];
    if (next) ui.setMode(next.value);
    queueMicrotask(() =>
      document.querySelector<HTMLElement>('.tab[aria-selected="true"]')?.focus(),
    );
  };
  return (
    <nav class="tabbar" aria-label="Mode">
      <div role="tablist" class="tabbar-list" aria-label="Mode">
        <For each={MODES}>
          {(m) => (
            <button
              type="button"
              role="tab"
              class="tab"
              aria-selected={ui.mode() === m.value}
              tabindex={ui.mode() === m.value ? 0 : -1}
              onClick={() => ui.setMode(m.value)}
              onKeyDown={(e) => {
                if (e.key === "ArrowRight") move(1);
                else if (e.key === "ArrowLeft") move(-1);
                else return;
                e.preventDefault();
              }}
            >
              {m.label}
            </button>
          )}
        </For>
      </div>
    </nav>
  );
}

/**
 * Mode switch: a segmented control on desktop (top bar) or a bottom tab bar
 * on mobile.
 *
 * @param props.variant - `bar` for the top bar, `tabs` for the mobile tab bar.
 */
export function ModeSwitch(props: { variant: "bar" | "tabs" }): JSX.Element {
  return (
    <Show when={props.variant === "bar"} fallback={<ModeTabs />}>
      <Segmented<Mode>
        label="Mode"
        value={ui.mode()}
        onChange={ui.setMode}
        options={MODES}
        class="mode-switch"
      />
    </Show>
  );
}

const THEME_NEXT = { system: "light", light: "dark", dark: "system" } as const;

/**
 * Top bar: wordmark with its Planckian strip, mode switch (desktop), undo and
 * redo, the A/B slot, theme toggle and a disabled Export placeholder.
 */
export function TopBar(): JSX.Element {
  const onKey = (e: KeyboardEvent): void => {
    const mod = e.ctrlKey || e.metaKey;
    if (!mod || e.key.toLowerCase() !== "z") return;
    const t = e.target as HTMLElement | null;
    if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)) return;
    e.preventDefault();
    if (e.shiftKey) redo();
    else undo();
  };
  onMount(() => window.addEventListener("keydown", onKey));
  onCleanup(() => window.removeEventListener("keydown", onKey));

  return (
    <header class="topbar">
      <div class="wordmark">
        <span class="wordmark-text">Apricity</span>
        <span
          class="wordmark-strip"
          style={{ background: planckianGradient("to right", 16) }}
          aria-hidden="true"
        />
      </div>
      <div class="topbar-mode">
        <ModeSwitch variant="bar" />
      </div>
      <div class="topbar-actions">
        <button
          type="button"
          class="icon-btn"
          aria-label={history.undoLabel() ? `Undo ${history.undoLabel()}` : "Undo"}
          title="Undo (Ctrl+Z)"
          disabled={!history.canUndo()}
          onClick={() => undo()}
        >
          <UndoIcon />
        </button>
        <button
          type="button"
          class="icon-btn"
          aria-label={history.redoLabel() ? `Redo ${history.redoLabel()}` : "Redo"}
          title="Redo (Shift+Ctrl+Z)"
          disabled={!history.canRedo()}
          onClick={() => redo()}
        >
          <RedoIcon />
        </button>
        <div class="ab-slot" data-slot="ab" />
        <button
          type="button"
          class="icon-btn"
          aria-label="Open file"
          title="Open file"
          onClick={() => pickImage((f) => void openImageFile(f))}
          disabled={currentSource() === null}
          classList={{ "hide-empty": currentSource() === null }}
        >
          <OpenIcon />
        </button>
        <button
          type="button"
          class="icon-btn"
          aria-label={`Theme: ${ui.theme()}. Switch to ${THEME_NEXT[ui.theme()]}`}
          title={`Theme: ${ui.theme()}`}
          onClick={() => ui.setTheme(THEME_NEXT[ui.theme()])}
        >
          <ThemeIcon />
        </button>
        <button type="button" class="btn btn-export" disabled title="Export is not available yet">
          Export
        </button>
      </div>
    </header>
  );
}
