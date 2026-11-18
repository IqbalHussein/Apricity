import {
  createEffect,
  createMemo,
  createSignal,
  createUniqueId,
  on,
  onCleanup,
  Show,
  untrack,
} from "solid-js";
import type { JSX } from "solid-js";
import type { ScalarSpec } from "../core/wasm";
import { decimalsForStep, formatValue, parseNumber, valueText } from "../lib/format";
import { getScalar } from "../lib/paramPath";
import { trackBackground } from "../lib/tracks";
import { aiAnimation, isAiSet } from "../state/aimarks";
import { gradeParams, resetPath, setScalar } from "../state/grade";
import { getSpecs } from "../state/specs";
import { ui } from "../state/ui";

/** Long-press duration that resets a slider on touch, in milliseconds. */
const LONG_PRESS_MS = 550;

function prefersReducedMotion(): boolean {
  return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/** Props for {@link SliderTrack}. */
export interface SliderTrackProps {
  /** Element id of the visible label (for `aria-labelledby`). */
  labelledBy: string;
  /** Range minimum, in the control's own unit. */
  min: number;
  /** Range maximum, in the control's own unit. */
  max: number;
  /** Step in the control's own unit. */
  step: number;
  /** Value restored by reset, same unit. */
  defaultValue: number;
  /** Current value (tracked). */
  value: () => number;
  /** Called with a new, already stepped value; `coalesce` is true for drags and key repeats. */
  onChange: (v: number, coalesce: boolean) => void;
  /** Called on double-click or touch long-press. */
  onReset: () => void;
  /** Text for `aria-valuetext`. */
  valueText: (v: number) => string;
  /** Functional CSS background for the rail, or `null` for a neutral rail with a fill. */
  background: string | null;
  /** Before-value animation request (see `aiAnimation`), or `undefined`. */
  animateFrom?: () => { token: number; before: number } | undefined;
}

/**
 * The interactive rail of a slider: pointer drag, keyboard (arrows step,
 * Shift x10, PageUp/PageDown x10, Home/End), double-click and long-press
 * reset, and the 400 ms before-to-after thumb animation.
 */
export function SliderTrack(props: SliderTrackProps): JSX.Element {
  let rail!: HTMLDivElement;
  let thumb!: HTMLDivElement;
  let pressTimer: ReturnType<typeof setTimeout> | undefined;
  let longPressed = false;
  let startX = 0;
  const range = (): number => props.max - props.min;
  const pct = (v: number): number => ((v - props.min) / range()) * 100;
  const snap = (raw: number): number => {
    const d = decimalsForStep(props.step) + 1;
    const stepped = Math.round((raw - props.min) / props.step) * props.step + props.min;
    return Number(Math.min(props.max, Math.max(props.min, stepped)).toFixed(d));
  };
  const fromPointer = (e: PointerEvent): number => {
    const r = rail.getBoundingClientRect();
    return snap(props.min + ((e.clientX - r.left) / r.width) * range());
  };
  const clearPress = (): void => {
    if (pressTimer !== undefined) clearTimeout(pressTimer);
    pressTimer = undefined;
  };
  const bipolar = (): boolean => props.min < 0 && props.max > 0;

  const onPointerDown = (e: PointerEvent): void => {
    if (e.button !== 0) return;
    longPressed = false;
    startX = e.clientX;
    rail.setPointerCapture(e.pointerId);
    (e.currentTarget as HTMLElement).focus();
    if (e.pointerType !== "mouse") {
      pressTimer = setTimeout(() => {
        longPressed = true;
        if (typeof navigator.vibrate === "function") navigator.vibrate(10);
        props.onReset();
      }, LONG_PRESS_MS);
    }
    props.onChange(fromPointer(e), true);
  };
  const onPointerMove = (e: PointerEvent): void => {
    if (!rail.hasPointerCapture(e.pointerId) || longPressed) return;
    if (Math.abs(e.clientX - startX) > 6) clearPress();
    props.onChange(fromPointer(e), true);
  };
  const onPointerUp = (e: PointerEvent): void => {
    clearPress();
    if (rail.hasPointerCapture(e.pointerId)) rail.releasePointerCapture(e.pointerId);
  };
  const onKeyDown = (e: KeyboardEvent): void => {
    const big = e.shiftKey ? 10 : 1;
    const v = props.value();
    let next: number;
    switch (e.key) {
      case "ArrowRight":
      case "ArrowUp":
        next = v + props.step * big;
        break;
      case "ArrowLeft":
      case "ArrowDown":
        next = v - props.step * big;
        break;
      case "PageUp":
        next = v + props.step * 10;
        break;
      case "PageDown":
        next = v - props.step * 10;
        break;
      case "Home":
        next = props.min;
        break;
      case "End":
        next = props.max;
        break;
      case "Delete":
      case "Backspace":
        e.preventDefault();
        props.onReset();
        return;
      default:
        return;
    }
    e.preventDefault();
    props.onChange(snap(next), true);
  };

  createEffect(
    on(
      () => props.animateFrom?.(),
      (req) => {
        if (!req || prefersReducedMotion() || typeof thumb.animate !== "function") return;
        const now = props.value();
        if (Math.abs(req.before - now) < 1e-9) return;
        thumb.animate([{ left: `${pct(req.before)}%` }, { left: `${pct(now)}%` }], {
          duration: 400,
          easing: "cubic-bezier(0.22, 1, 0.36, 1)",
        });
      },
      { defer: true },
    ),
  );
  onCleanup(clearPress);

  const fillStyle = (): JSX.CSSProperties => {
    const origin = bipolar() ? 0 : props.min;
    const a = pct(Math.min(origin, props.value()));
    const b = pct(Math.max(origin, props.value()));
    return { left: `${a}%`, width: `${b - a}%` };
  };

  return (
    <div
      class="strack"
      classList={{ "strack-functional": props.background !== null }}
      ref={rail}
      role="slider"
      tabindex="0"
      aria-labelledby={props.labelledBy}
      aria-valuemin={props.min}
      aria-valuemax={props.max}
      aria-valuenow={props.value()}
      aria-valuetext={props.valueText(props.value())}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
      onKeyDown={onKeyDown}
      onDblClick={() => props.onReset()}
    >
      <div class="srail" style={props.background ? { background: props.background } : undefined} />
      <Show when={props.background === null}>
        <div class="sfill" style={fillStyle()} />
      </Show>
      <Show when={bipolar()}>
        <div class="szero" style={{ left: `${pct(0)}%` }} />
      </Show>
      <div class="sthumb" ref={thumb} style={{ left: `${pct(props.value())}%` }} />
    </div>
  );
}

/**
 * One console slider generated from a `ScalarSpec`: label, functional track,
 * value with unit (click to type a number), AI-set ring, and flash target.
 *
 * @param props.spec - Parameter spec from `paramSpecs()`.
 * @param props.label - Override for the spec label.
 * @param props.compact - Stack label above the rail (narrow columns).
 * @param props.onChange - Called after the grade changed.
 */
export function SliderRow(props: {
  spec: ScalarSpec;
  label?: string;
  compact?: boolean;
  onChange?: (v: number) => void;
}): JSX.Element {
  const labelId = createUniqueId();
  const value = createMemo(() => getScalar(gradeParams(), props.spec));
  const [editing, setEditing] = createSignal(false);
  const specs = getSpecs();
  const bg = untrack(() => trackBackground(props.spec, specs.hsl_centers_deg, specs.hsl_bands));
  const text = (): { num: string; unit: string } => formatValue(props.spec, value());
  const flashing = (): boolean => ui.flash()?.path === props.spec.path;
  const label = (): string => props.label ?? props.spec.label;

  const commit = (raw: string): void => {
    if (!editing()) return; // Enter removes the input, which then blurs and would commit twice
    setEditing(false);
    const n = parseNumber(raw);
    if (n === null) return;
    const stored = setScalar(props.spec.path, n, { coalesce: false });
    if (stored !== null) props.onChange?.(stored);
  };

  return (
    <div
      class="srow"
      classList={{
        "srow-compact": props.compact === true,
        "row-flash": flashing(),
        "srow-ai": isAiSet(props.spec.path),
      }}
      data-anchor={props.spec.path}
    >
      <span class="slabel" id={labelId} title={label()}>
        <Show when={isAiSet(props.spec.path)}>
          <span class="ai-ring" title="Set by the AI" />
          <span class="sr-only">Set by the AI. </span>
        </Show>
        {label()}
      </span>
      <SliderTrack
        labelledBy={labelId}
        min={props.spec.min}
        max={props.spec.max}
        step={props.spec.step}
        defaultValue={props.spec.default}
        value={value}
        background={bg}
        valueText={(v) => valueText(props.spec, v)}
        onChange={(v, coalesce) => {
          const stored = setScalar(props.spec.path, v, { coalesce });
          if (stored !== null) props.onChange?.(stored);
        }}
        onReset={() => {
          resetPath(props.spec.path);
          props.onChange?.(props.spec.default);
        }}
        animateFrom={() => {
          const a = aiAnimation();
          const before = a?.before[props.spec.path];
          return a && before !== undefined ? { token: a.token, before } : undefined;
        }}
      />
      <Show
        when={!editing()}
        fallback={
          <input
            class="svalue-input"
            type="text"
            inputmode="decimal"
            aria-label={`${label()} value`}
            value={String(value())}
            ref={(el) => queueMicrotask(() => (el.focus(), el.select()))}
            onBlur={(e) => commit(e.currentTarget.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") commit(e.currentTarget.value);
              else if (e.key === "Escape") setEditing(false);
            }}
          />
        }
      >
        <button
          type="button"
          class="svalue"
          onClick={() => setEditing(true)}
          aria-label={`${label()}: ${valueText(props.spec, value())}. Edit value`}
        >
          <span class="svalue-num">{text().num}</span>
          <Show when={text().unit}>
            <span class="svalue-unit" classList={{ "svalue-unit-word": text().unit.length > 1 }}>
              {text().unit}
            </span>
          </Show>
        </button>
      </Show>
    </div>
  );
}

/**
 * A slider that is not backed by a `GradeParameters` field (view settings
 * such as the zebra threshold).
 *
 * @param props.label - Visible label.
 * @param props.min - Minimum, in `unit`.
 * @param props.max - Maximum, in `unit`.
 * @param props.step - Step, in `unit`.
 * @param props.value - Current value (tracked).
 * @param props.onChange - Receives the new value.
 * @param props.unit - Suffix shown after the value (e.g. `%`).
 */
export function PlainSliderRow(props: {
  label: string;
  min: number;
  max: number;
  step: number;
  value: () => number;
  onChange: (v: number) => void;
  unit: string;
}): JSX.Element {
  const labelId = createUniqueId();
  return (
    <div class="srow">
      <span class="slabel" id={labelId}>
        {props.label}
      </span>
      <SliderTrack
        labelledBy={labelId}
        min={props.min}
        max={props.max}
        step={props.step}
        defaultValue={props.max}
        value={props.value}
        background={null}
        valueText={(v) => `${v} ${props.unit}`}
        onChange={(v) => props.onChange(v)}
        onReset={() => props.onChange(props.max)}
      />
      <span class="svalue svalue-static">
        <span class="svalue-num">{props.value()}</span>
        <span class="svalue-unit">{props.unit}</span>
      </span>
    </div>
  );
}
