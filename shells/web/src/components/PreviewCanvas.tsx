import { onCleanup, onMount, Show } from "solid-js";
import type { JSX } from "solid-js";
import { disposeEngine, engineErrorText, mountEngine, previewSize } from "../engine/bridge";
import { clampView, zoomAt } from "../lib/view";
import type { View } from "../lib/view";
import { currentSource } from "../state/project";
import { ui } from "../state/ui";
import { FallbackBanner } from "./FallbackBanner";
import { ImportDropzone } from "./ImportDropzone";
import { WipeDivider } from "./WipeDivider";

/**
 * The preview area: hosts the engine canvas, handles zoom (wheel, pinch,
 * keys +/-/0), pan (drag, arrows) and double-click to fit, and layers the
 * wipe divider, empty state and fallback banner on top.
 */
export function PreviewCanvas(): JSX.Element {
  let host!: HTMLDivElement;
  const pointers = new Map<number, { x: number; y: number }>();
  let pinchStart: { dist: number; view: View } | null = null;
  let panStart: { x: number; y: number; view: View } | null = null;

  onMount(() => {
    void mountEngine(host).catch((e: unknown) => console.error("Engine failed to start", e));
    host.addEventListener("wheel", onWheel, { passive: false });
  });
  onCleanup(() => {
    host.removeEventListener("wheel", onWheel);
    disposeEngine();
  });

  const geometry = (): { aspect: number; w: number; h: number } | null => {
    const src = currentSource();
    const s = previewSize();
    if (!src || s.w === 0) return null;
    return { aspect: src.proxy.width / src.proxy.height, w: s.w, h: s.h };
  };
  const local = (e: { clientX: number; clientY: number }): { x: number; y: number } => {
    const r = host.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  };
  const zoomBy = (factor: number, at: { x: number; y: number }): void => {
    const g = geometry();
    if (!g) return;
    ui.setView(zoomAt(ui.view(), factor, at, g.aspect, g.w, g.h));
  };

  function onWheel(e: WheelEvent): void {
    if (!geometry()) return;
    e.preventDefault();
    const scale = e.ctrlKey ? 0.01 : 0.0015;
    zoomBy(Math.exp(-e.deltaY * scale), local(e));
  }

  const onPointerDown = (e: PointerEvent): void => {
    if (!geometry() || e.button !== 0) return;
    host.setPointerCapture(e.pointerId);
    pointers.set(e.pointerId, local(e));
    if (pointers.size === 2) {
      const [a, b] = [...pointers.values()] as [{ x: number; y: number }, { x: number; y: number }];
      pinchStart = { dist: Math.hypot(a.x - b.x, a.y - b.y), view: ui.view() };
      panStart = null;
    } else {
      panStart = { ...local(e), view: ui.view() };
    }
  };
  const onPointerMove = (e: PointerEvent): void => {
    const g = geometry();
    if (!g || !pointers.has(e.pointerId)) return;
    pointers.set(e.pointerId, local(e));
    if (pointers.size === 2 && pinchStart) {
      const [a, b] = [...pointers.values()] as [{ x: number; y: number }, { x: number; y: number }];
      const dist = Math.hypot(a.x - b.x, a.y - b.y);
      const mid = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
      const target = pinchStart.view.zoom * (dist / Math.max(1, pinchStart.dist));
      ui.setView(zoomAt(ui.view(), target / ui.view().zoom, mid, g.aspect, g.w, g.h));
      return;
    }
    if (panStart && ui.view().zoom > 1) {
      const p = local(e);
      const fitW = Math.min(g.w, g.h * g.aspect);
      const dw = fitW * panStart.view.zoom;
      const dh = (fitW / g.aspect) * panStart.view.zoom;
      ui.setView(
        clampView(g.aspect, g.w, g.h, {
          zoom: panStart.view.zoom,
          pan: [
            panStart.view.pan[0] + (p.x - panStart.x) / dw,
            panStart.view.pan[1] + (p.y - panStart.y) / dh,
          ],
        }),
      );
    }
  };
  const onPointerUp = (e: PointerEvent): void => {
    pointers.delete(e.pointerId);
    if (host.hasPointerCapture(e.pointerId)) host.releasePointerCapture(e.pointerId);
    if (pointers.size < 2) pinchStart = null;
    if (pointers.size === 0) panStart = null;
  };
  const onKey = (e: KeyboardEvent): void => {
    const g = geometry();
    if (!g) return;
    const c = { x: g.w / 2, y: g.h / 2 };
    if (e.key === "+" || e.key === "=") zoomBy(1.25, c);
    else if (e.key === "-" || e.key === "_") zoomBy(0.8, c);
    else if (e.key === "0") ui.setView({ zoom: 1, pan: [0, 0] });
    else if (e.key.startsWith("Arrow") && ui.view().zoom > 1) {
      const d = e.shiftKey ? 0.1 : 0.03;
      const v = ui.view();
      const dx = e.key === "ArrowLeft" ? d : e.key === "ArrowRight" ? -d : 0;
      const dy = e.key === "ArrowUp" ? d : e.key === "ArrowDown" ? -d : 0;
      ui.setView(
        clampView(g.aspect, g.w, g.h, { zoom: v.zoom, pan: [v.pan[0] + dx, v.pan[1] + dy] }),
      );
    } else return;
    e.preventDefault();
  };

  return (
    <div class="stage" classList={{ "stage-zoomed": ui.view().zoom > 1 }}>
      <div
        class="stage-host"
        ref={host}
        tabindex={currentSource() ? 0 : -1}
        aria-label="Image preview. Plus and minus zoom, zero fits, arrow keys pan when zoomed."
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
        onDblClick={() => ui.setView({ zoom: 1, pan: [0, 0] })}
        onKeyDown={onKey}
      />
      <WipeDivider />
      <ImportDropzone empty={currentSource() === null} />
      <FallbackBanner />
      <Show when={engineErrorText()}>
        <p class="toast error-text" role="alert">
          {engineErrorText()}
        </p>
      </Show>
    </div>
  );
}
