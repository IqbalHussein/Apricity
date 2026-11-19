import { createMemo, Show } from "solid-js";
import type { JSX } from "solid-js";
import { previewSize } from "../engine/bridge";
import { imageRect } from "../lib/view";
import { currentSource } from "../state/project";
import { ui } from "../state/ui";

const SOURCE_NAME = {
  original: "Original",
  stage1: "Stage 1 only",
  previous: "Previous grade",
  reference: "Reference",
} as const;

/**
 * Review-mode wipe divider drawn over the image: draggable by pointer and
 * keyboard (arrow keys move 1%, Shift 10%). Position is a fraction of the
 * image, 0..1; side A (left or top) is the current grade, side B shows the
 * selected compare source.
 */
export function WipeDivider(): JSX.Element {
  let handle!: HTMLDivElement;
  const rect = createMemo(() => {
    const src = currentSource();
    const s = previewSize();
    if (!src || s.w === 0) return null;
    return imageRect(src.proxy.width / src.proxy.height, s.w, s.h, ui.view());
  });
  const vertical = ui.wipeVertical;
  const set = (v: number): void => {
    ui.setWipePosition(Math.min(1, Math.max(0, v)));
  };
  const fromPointer = (e: PointerEvent): number => {
    const r = rect();
    const host = handle.closest(".stage")?.getBoundingClientRect();
    if (!r || !host) return 0.5;
    return vertical() ? (e.clientX - host.left - r.x) / r.w : (e.clientY - host.top - r.y) / r.h;
  };
  const line = (): JSX.CSSProperties => {
    const r = rect();
    if (!r) return { display: "none" };
    const p = ui.wipePosition();
    const s = previewSize();
    if (vertical()) {
      const top = Math.max(0, r.y);
      return {
        left: `${r.x + p * r.w}px`,
        top: `${top}px`,
        height: `${Math.max(0, Math.min(s.h, r.y + r.h) - top)}px`,
      };
    }
    const left = Math.max(0, r.x);
    return {
      top: `${r.y + p * r.h}px`,
      left: `${left}px`,
      width: `${Math.max(0, Math.min(s.w, r.x + r.w) - left)}px`,
    };
  };
  const onKey = (e: KeyboardEvent): void => {
    const step = e.shiftKey ? 0.1 : 0.01;
    const back = vertical() ? "ArrowLeft" : "ArrowUp";
    const fwd = vertical() ? "ArrowRight" : "ArrowDown";
    if (e.key === back) set(ui.wipePosition() - step);
    else if (e.key === fwd) set(ui.wipePosition() + step);
    else if (e.key === "Home") set(0);
    else if (e.key === "End") set(1);
    else return;
    e.preventDefault();
  };
  return (
    <Show when={ui.mode() === "review" && currentSource() !== null}>
      <div
        class="wipe"
        classList={{ vertical: vertical(), horizontal: !vertical() }}
        style={line()}
      >
        <div
          ref={handle}
          class="wipe-handle"
          role="slider"
          tabindex="0"
          aria-label="Wipe divider"
          aria-orientation={vertical() ? "horizontal" : "vertical"}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={Math.round(ui.wipePosition() * 100)}
          aria-valuetext={`${Math.round(ui.wipePosition() * 100)} percent, graded on the ${vertical() ? "left" : "top"}`}
          onPointerDown={(e) => {
            e.stopPropagation();
            handle.setPointerCapture(e.pointerId);
            set(fromPointer(e));
          }}
          onPointerMove={(e) => {
            if (handle.hasPointerCapture(e.pointerId)) set(fromPointer(e));
          }}
          onPointerUp={(e) => {
            if (handle.hasPointerCapture(e.pointerId)) handle.releasePointerCapture(e.pointerId);
          }}
          onKeyDown={onKey}
        >
          <span class="wipe-grip" />
        </div>
      </div>
      <div
        class="wipe-tags"
        classList={{ vertical: vertical(), horizontal: !vertical() }}
        aria-hidden="true"
      >
        <span class="chip">Graded</span>
        <span class="chip">{SOURCE_NAME[ui.compareSource()]}</span>
      </div>
    </Show>
  );
}
