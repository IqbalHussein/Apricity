import { createMemo, createSignal, For } from "solid-js";
import type { JSX } from "solid-js";
import type { CurvePoint } from "../core/wasm";
import {
  addPoint,
  editablePoints,
  identityPoints,
  movePoint,
  removePoint,
  sampleCurve,
} from "../lib/curve";
import { getPath } from "../lib/paramPath";
import { isAiSet } from "../state/aimarks";
import { gradeParams, setCurve } from "../state/grade";
import type { CurveChannel } from "../state/grade";
import { ui } from "../state/ui";
import { Segmented } from "./Segmented";

const SIZE = 240;
const PAD = 8;
const INNER = SIZE - PAD * 2;

const [channelSignal, setChannelSignal] = createSignal<CurveChannel>("master");

/** Selected curve channel tab (so the console can reveal a channel). */
export const curveChannel = channelSignal;
/**
 * Switch the visible curve channel.
 *
 * @param c - Channel to show.
 */
export const showCurveChannel = setChannelSignal;

const STROKE: Record<CurveChannel, string> = {
  master: "var(--text)",
  red: "#E8605A",
  green: "#3FB57A",
  blue: "#5B9BE8",
};

/**
 * Tone curve editor on a 240x240 neutral grid. Master, R, G and B tabs; click
 * to add a point, drag to move, double-click (or Delete) to remove. Writes
 * `stage2.tone_curve.*` (control points in the working encoding, [0, 1]).
 */
export function CurveEditor(): JSX.Element {
  let svg!: SVGSVGElement;
  const [drag, setDrag] = createSignal<number | null>(null);
  const channel = channelSignal;
  const points = createMemo<CurvePoint[]>(() =>
    editablePoints(
      getPath(gradeParams(), `stage2.tone_curve.${channel()}`) as CurvePoint[] | undefined,
    ),
  );
  const px = (x: number): number => PAD + x * INNER;
  const py = (y: number): number => PAD + (1 - y) * INNER;
  const toUnit = (e: PointerEvent): { x: number; y: number } => {
    const r = svg.getBoundingClientRect();
    const sx = ((e.clientX - r.left) / r.width) * SIZE;
    const sy = ((e.clientY - r.top) / r.height) * SIZE;
    return { x: (sx - PAD) / INNER, y: 1 - (sy - PAD) / INNER };
  };
  const path = (): string =>
    sampleCurve(points())
      .map((p, i) => `${i === 0 ? "M" : "L"}${px(p.x).toFixed(2)} ${py(p.y).toFixed(2)}`)
      .join(" ");
  const anyAi = (): boolean =>
    (["master", "red", "green", "blue"] as const).some((c) => isAiSet(`stage2.tone_curve.${c}`));
  const flashing = (): boolean => ui.flash()?.path.startsWith("stage2.tone_curve") === true;

  return (
    <div
      class="curve"
      classList={{ "row-flash": flashing() }}
      data-anchor="stage2.tone_curve.master stage2.tone_curve.red stage2.tone_curve.green stage2.tone_curve.blue"
    >
      <div class="curve-head">
        <Segmented<CurveChannel>
          label="Curve channel"
          value={channel()}
          onChange={setChannelSignal}
          options={[
            { value: "master", label: "Master" },
            { value: "red", label: "R", ariaLabel: "Red" },
            { value: "green", label: "G", ariaLabel: "Green" },
            { value: "blue", label: "B", ariaLabel: "Blue" },
          ]}
        />
        {anyAi() ? <span class="ai-ring" title="Set by the AI" /> : null}
      </div>
      <svg
        ref={svg}
        class="curve-svg"
        viewBox={`0 0 ${SIZE} ${SIZE}`}
        width={SIZE}
        height={SIZE}
        role="group"
        aria-label={`${channel()} tone curve editor. Click to add a point, drag to move, double-click a point to remove it.`}
        onPointerDown={(e) => {
          if (e.button !== 0 || (e.target as SVGElement).dataset.pt !== undefined) return;
          const u = toUnit(e);
          const res = addPoint(points(), u.x, u.y);
          setCurve(channel(), res.points);
          setDrag(res.index);
          svg.setPointerCapture(e.pointerId);
        }}
        onPointerMove={(e) => {
          const i = drag();
          if (i === null || !svg.hasPointerCapture(e.pointerId)) return;
          const u = toUnit(e);
          setCurve(channel(), movePoint(points(), i, u.x, u.y));
        }}
        onPointerUp={(e) => {
          setDrag(null);
          if (svg.hasPointerCapture(e.pointerId)) svg.releasePointerCapture(e.pointerId);
        }}
      >
        <rect x={PAD} y={PAD} width={INNER} height={INNER} class="curve-bg" />
        <For each={[0.25, 0.5, 0.75]}>
          {(t) => (
            <>
              <line x1={px(t)} y1={PAD} x2={px(t)} y2={PAD + INNER} class="curve-grid" />
              <line x1={PAD} y1={py(t)} x2={PAD + INNER} y2={py(t)} class="curve-grid" />
            </>
          )}
        </For>
        <line x1={px(0)} y1={py(0)} x2={px(1)} y2={py(1)} class="curve-diag" />
        <path d={path()} fill="none" stroke={STROKE[channel()]} stroke-width="2" />
        <For each={points()}>
          {(p, i) => (
            <circle
              data-pt={i()}
              cx={px(p.x)}
              cy={py(p.y)}
              r="5.5"
              class="curve-pt"
              tabindex="0"
              role="slider"
              aria-label={`Point ${i() + 1} of ${points().length}`}
              aria-valuetext={`Input ${Math.round(p.x * 100)} percent, output ${Math.round(p.y * 100)} percent`}
              aria-valuenow={Math.round(p.y * 100)}
              aria-valuemin={0}
              aria-valuemax={100}
              onPointerDown={(e) => {
                if (e.button !== 0) return;
                e.stopPropagation();
                setDrag(i());
                svg.setPointerCapture(e.pointerId);
              }}
              onDblClick={(e) => {
                e.stopPropagation();
                const next = removePoint(points(), i());
                setCurve(channel(), next);
              }}
              onKeyDown={(e) => {
                const step = e.shiftKey ? 0.1 : 0.01;
                const dx = e.key === "ArrowRight" ? step : e.key === "ArrowLeft" ? -step : 0;
                const dy = e.key === "ArrowUp" ? step : e.key === "ArrowDown" ? -step : 0;
                if (e.key === "Delete" || e.key === "Backspace") {
                  e.preventDefault();
                  setCurve(channel(), removePoint(points(), i()));
                } else if (dx !== 0 || dy !== 0) {
                  e.preventDefault();
                  setCurve(channel(), movePoint(points(), i(), p.x + dx, p.y + dy));
                }
              }}
            />
          )}
        </For>
      </svg>
      <button
        type="button"
        class="btn btn-quiet"
        onClick={() => setCurve(channel(), identityPoints())}
      >
        Reset curve
      </button>
    </div>
  );
}
