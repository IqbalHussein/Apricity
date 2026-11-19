import { For, onCleanup, onMount, Show } from "solid-js";
import type { JSX } from "solid-js";
import { planckianGradient } from "../lib/kelvin";
import { pickMedia } from "../lib/filePicker";
import { redo, undo } from "../state/grade";
import { history } from "../state/history";
import { currentSource, openMediaFile } from "../state/project";
import type { Mode } from "../state/ui";
import { ui } from "../state/ui";
import { toggleAb } from "../state/ab";
import { setRecipeLibraryOpen } from "../state/recipes";
import { AbToggle } from "./AbToggle";
import { setExportDialogOpen } from "./ExportDialog";
import { BookmarkIcon, GridIcon, OpenIcon, RedoIcon, ThemeIcon, UndoIcon } from "./Icons";
import { openSaveRecipe } from "./Recipes";
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
 * redo, the A/B toggle, recipe save and library, theme toggle and Export.
 */
export function TopBar(): JSX.Element {
  const onKey = (e: KeyboardEvent): void => {
    const mod = e.ctrlKey || e.metaKey;
    const t = e.target as HTMLElement | null;
    const typing =
      t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.tagName === "SELECT" || t.isContentEditable);
    if (e.key === "\\" && !mod && !e.altKey && !typing && !document.querySelector("dialog[open]")) {
      e.preventDefault();
      toggleAb();
      return;
    }
    if (!mod || e.key.toLowerCase() !== "z") return;
    if (typing) return;
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
        <AbToggle />
        <button
          type="button"
          class="icon-btn"
          data-testid="save-recipe"
          aria-label="Save recipe"
          title="Save recipe"
          disabled={currentSource() === null}
          onClick={() => openSaveRecipe()}
        >
          <BookmarkIcon />
        </button>
        <button
          type="button"
          class="icon-btn"
          data-testid="recipe-library-open"
          aria-label="Recipe library"
          title="Recipe library"
          onClick={() => setRecipeLibraryOpen(true)}
        >
          <GridIcon />
        </button>
        <button
          type="button"
          class="icon-btn tb-open"
          aria-label="Open file"
          title="Open file"
          onClick={() => pickMedia((f) => void openMediaFile(f))}
          disabled={currentSource() === null}
          classList={{ "hide-empty": currentSource() === null }}
        >
          <OpenIcon />
        </button>
        <button
          type="button"
          class="icon-btn tb-theme"
          aria-label={`Theme: ${ui.theme()}. Switch to ${THEME_NEXT[ui.theme()]}`}
          title={`Theme: ${ui.theme()}`}
          onClick={() => ui.setTheme(THEME_NEXT[ui.theme()])}
        >
          <ThemeIcon />
        </button>
        <button
          type="button"
          class="btn btn-export"
          data-testid="export-open"
          disabled={currentSource() === null}
          onClick={() => setExportDialogOpen(true)}
        >
          Export
        </button>
      </div>
    </header>
  );
}
