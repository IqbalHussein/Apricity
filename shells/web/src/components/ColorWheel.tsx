import { createMemo, createUniqueId, Show, untrack } from "solid-js";
import type { JSX } from "solid-js";
import { getScalar } from "../lib/paramPath";
import { pointToWheel, rgbToWheel, wheelToPoint, wheelToRgb } from "../lib/wheel";
import type { WheelPos } from "../lib/wheel";
import { isAiSet } from "../state/aimarks";
import { gradeParams, setScalars } from "../state/grade";
import { specFor } from "../state/specs";
import { ui } from "../state/ui";
import { SliderRow } from "./Slider";

/** Wheel names, matching `GradeParameters.stage2`. */
export type WheelName = "lift" | "gamma" | "gain";

/**
 * A lift, gamma or gain color wheel: a 35%-opacity hue/saturation disc with a
 * puck, plus a master slider. The puck maps hue and saturation to r, g, b
 * offsets with zero mean (`wheelToRgb`); the master slider is separate.
 *
 * @param props.name - Which wheel.
 * @param props.title - Visible title.
 */
export function ColorWheel(props: { name: WheelName; title: string }): JSX.Element {
  let disc!: HTMLDivElement;
  const labelId = createUniqueId();
  const name = untrack(() => props.name);
  const paths = (["r", "g", "b", "master"] as const).map((c) => `stage2.${name}.${c}`);
  const maxOffset = specFor(`stage2.${name}.r`)?.max ?? 0.5;
  const pos = createMemo<WheelPos>(() => {
    const p = gradeParams();
    return rgbToWheel(
      getScalar(p, { path: paths[0] as string, default: 0 }),
      getScalar(p, { path: paths[1] as string, default: 0 }),
      getScalar(p, { path: paths[2] as string, default: 0 }),
      maxOffset,
    );
  });
  const radius = (): number => disc.clientWidth / 2;
  const write = (hue: number, sat: number, coalesce: boolean): void => {
    const [r, g, b] = wheelToRgb(hue, sat, maxOffset);
    setScalars(
      { [paths[0] as string]: r, [paths[1] as string]: g, [paths[2] as string]: b },
      props.title,
      coalesce ? `wheel:${name}` : null,
    );
  };
  const fromPointer = (e: PointerEvent): WheelPos => {
    const rect = disc.getBoundingClientRect();
    return pointToWheel(
      e.clientX - rect.left - rect.width / 2,
      e.clientY - rect.top - rect.height / 2,
      rect.width / 2,
    );
  };
  const puckStyle = (): JSX.CSSProperties => {
    const p = pos();
    const r = disc ? radius() : 60;
    const { x, y } = wheelToPoint(p, r);
    return {
      left: `calc(50% + ${x}px)`,
      top: `calc(50% + ${y}px)`,
      background:
        p.sat < 0.02 ? "var(--raised)" : `hsl(${p.hue} ${Math.round(40 + p.sat * 50)}% 55%)`,
    };
  };
  const onKey = (e: KeyboardEvent): void => {
    const step = e.shiftKey ? 10 : 2;
    const d: Record<string, [number, number]> = {
      ArrowLeft: [-step, 0],
      ArrowRight: [step, 0],
      ArrowUp: [0, -step],
      ArrowDown: [0, step],
    };
    const delta = d[e.key];
    if (e.key === "Delete" || e.key === "Backspace") {
      e.preventDefault();
      write(0, 0, false);
      return;
    }
    if (!delta) return;
    e.preventDefault();
    const r = radius();
    const cur = wheelToPoint(pos(), r);
    const next = pointToWheel(cur.x + delta[0], cur.y + delta[1], r);
    write(next.hue, next.sat, true);
  };
  const flashing = (): boolean => paths.some((p) => ui.flash()?.path === p);
  const aiSet = (): boolean => paths.some((p) => isAiSet(p));
  const master = specFor(`stage2.${name}.master`);

  return (
    <div class="wheel" classList={{ "row-flash": flashing() }} data-anchor={paths.join(" ")}>
      <span class="wheel-title" id={labelId}>
        <Show when={aiSet()}>
          <span class="ai-ring" title="Set by the AI" />
          <span class="sr-only">Set by the AI. </span>
        </Show>
        {props.title}
      </span>
      <div
        class="wheel-disc-wrap"
        ref={disc}
        onPointerDown={(e) => {
          if (e.button !== 0) return;
          disc.setPointerCapture(e.pointerId);
          const p = fromPointer(e);
          write(p.hue, p.sat, true);
        }}
        onPointerMove={(e) => {
          if (!disc.hasPointerCapture(e.pointerId)) return;
          const p = fromPointer(e);
          write(p.hue, p.sat, true);
        }}
        onPointerUp={(e) => {
          if (disc.hasPointerCapture(e.pointerId)) disc.releasePointerCapture(e.pointerId);
        }}
        onDblClick={() => write(0, 0, false)}
      >
        <div class="wheel-disc" />
        <div
          class="wheel-puck"
          tabindex="0"
          role="group"
          aria-roledescription="color wheel puck"
          aria-labelledby={labelId}
          aria-valuetext={`Hue ${Math.round(pos().hue)} degrees, strength ${Math.round(pos().sat * 100)} percent`}
          aria-label={`${props.title}: hue ${Math.round(pos().hue)} degrees, strength ${Math.round(pos().sat * 100)} percent. Arrow keys nudge, Delete resets.`}
          style={puckStyle()}
          onKeyDown={onKey}
        />
      </div>
      <Show when={master}>{(m) => <SliderRow spec={m()} label="Master" compact />}</Show>
    </div>
  );
}
