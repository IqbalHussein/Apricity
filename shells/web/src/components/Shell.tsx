import { createEffect, createSignal, onCleanup, onMount, Show } from "solid-js";
import type { JSX } from "solid-js";
import { currentSource } from "../state/project";
import { ui } from "../state/ui";
import { ExplainabilityPanel } from "./ExplainabilityPanel";
import { ExportDialog, exportDialogOpen } from "./ExportDialog";
import { Inspector } from "./Inspector";
import { NgaPromptBar } from "./NgaPromptBar";
import { Onboarding } from "./Onboarding";
import { PreviewCanvas } from "./PreviewCanvas";
import { RecipeDialogs, RecipeStrip } from "./Recipes";
import { ReviewToolbar } from "./ReviewToolbar";
import { ScopesPanel } from "./ScopesPanel";
import { Transport } from "./Transport";
import { ModeSwitch, TopBar } from "./TopBar";
import { NoticeHost } from "./UpgradeNotice";

/** Viewport width at which the desktop two-column layout starts, in CSS pixels. */
export const DESKTOP_MIN_WIDTH = 1024;

/**
 * Application layout. Desktop (>= 1024 px): image and prompt on the left, a
 * 360 px inspector on the right. Mobile: image on top, prompt bar, a bottom
 * sheet with a drag handle, and a bottom tab bar. Mode-dependent panels are
 * mounted per mode; all state lives in stores so switching is instant.
 */
export function Shell(): JSX.Element {
  const [dragStart, setDragStart] = createSignal<{ y: number; h: number } | null>(null);
  onMount(() => {
    const mq = matchMedia(`(max-width: ${DESKTOP_MIN_WIDTH - 1}px)`);
    const sync = (): void => {
      ui.setMobile(mq.matches);
    };
    sync();
    mq.addEventListener("change", sync);
    onCleanup(() => mq.removeEventListener("change", sync));
  });
  createEffect(() => {
    if (ui.mode() === "grade" && ui.mobile()) ui.setExplainOpen(false);
  });
  createEffect(() => {
    const t = ui.theme();
    if (t === "system") document.documentElement.removeAttribute("data-theme");
    else document.documentElement.setAttribute("data-theme", t);
  });

  const mode = ui.mode;
  const sheetStyle = (): JSX.CSSProperties =>
    ui.sheetHeight() > 0 ? { "--sheet-h": `${ui.sheetHeight()}px` } : {};

  return (
    <div class="app" classList={{ mobile: ui.mobile(), [`mode-${mode()}`]: true }}>
      <TopBar />
      <main class="main">
        <PreviewCanvas />
        <Transport />
        <Onboarding />
        <Show when={mode() === "quick"}>
          <div class="quick-hist">
            <ScopesPanel histogramOnly />
          </div>
        </Show>
        <Show when={mode() === "review"}>
          <ReviewToolbar />
        </Show>
        <Show when={mode() !== "review"}>
          <div class="dock">
            <Show when={mode() === "quick"}>
              <RecipeStrip />
            </Show>
            <NgaPromptBar />
            <ExplainabilityPanel />
          </div>
        </Show>
      </main>
      <Show when={mode() === "grade" && currentSource() !== null}>
        <aside class="side" aria-label="Inspector" style={sheetStyle()}>
          <Show when={ui.mobile()}>
            <div
              class="sheet-handle"
              role="separator"
              aria-orientation="horizontal"
              aria-label="Resize panel"
              tabindex="0"
              onPointerDown={(e) => {
                (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
                setDragStart({
                  y: e.clientY,
                  h: (e.currentTarget as HTMLElement).parentElement?.clientHeight ?? 300,
                });
              }}
              onPointerMove={(e) => {
                const s = dragStart();
                if (!s) return;
                ui.setSheetHeight(
                  Math.round(
                    Math.min(window.innerHeight * 0.75, Math.max(140, s.h + (s.y - e.clientY))),
                  ),
                );
              }}
              onPointerUp={() => setDragStart(null)}
              onKeyDown={(e) => {
                const cur = ui.sheetHeight() || window.innerHeight * 0.36;
                if (e.key === "ArrowUp")
                  ui.setSheetHeight(Math.min(window.innerHeight * 0.75, cur + 24));
                else if (e.key === "ArrowDown") ui.setSheetHeight(Math.max(140, cur - 24));
                else return;
                e.preventDefault();
              }}
            >
              <span class="sheet-grip" />
            </div>
          </Show>
          <Inspector mobile={ui.mobile()} />
        </aside>
      </Show>
      <Show when={ui.mobile()}>
        <ModeSwitch variant="tabs" />
      </Show>
      <RecipeDialogs />
      <Show when={exportDialogOpen()}>
        <ExportDialog />
      </Show>
      <NoticeHost />
    </div>
  );
}
