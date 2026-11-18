import { For } from "solid-js";
import type { JSX } from "solid-js";

/** One option of a {@link Segmented} control. */
export interface SegmentOption<T extends string> {
  /** Value reported on selection. */
  value: T;
  /** Visible label. */
  label: JSX.Element;
  /** Accessible name when the label is an icon. */
  ariaLabel?: string | undefined;
  /** Greys the option out. */
  disabled?: boolean | undefined;
  /** Tooltip text. */
  title?: string | undefined;
}

/**
 * Segmented control with radiogroup semantics and arrow-key navigation.
 *
 * @param props.options - Options in display order.
 * @param props.value - Selected value.
 * @param props.onChange - Called with the newly selected value.
 * @param props.label - Accessible name of the group.
 * @param props.class - Extra CSS classes.
 */
export function Segmented<T extends string>(props: {
  options: readonly SegmentOption<T>[];
  value: T;
  onChange: (v: T) => void;
  label: string;
  class?: string;
}): JSX.Element {
  const onKey = (e: KeyboardEvent): void => {
    const enabled = props.options.filter((o) => !o.disabled);
    const i = enabled.findIndex((o) => o.value === props.value);
    let next = -1;
    if (e.key === "ArrowRight" || e.key === "ArrowDown") next = (i + 1) % enabled.length;
    else if (e.key === "ArrowLeft" || e.key === "ArrowUp")
      next = (i - 1 + enabled.length) % enabled.length;
    else return;
    e.preventDefault();
    const target = enabled[next];
    if (target) {
      props.onChange(target.value);
      queueMicrotask(() => {
        const btn = (e.currentTarget as HTMLElement).querySelector<HTMLElement>(
          '[aria-checked="true"]',
        );
        btn?.focus();
      });
    }
  };
  return (
    <div
      class={`segmented ${props.class ?? ""}`}
      role="radiogroup"
      aria-label={props.label}
      onKeyDown={onKey}
    >
      <For each={props.options}>
        {(o) => (
          <button
            type="button"
            role="radio"
            class="segment"
            aria-checked={o.value === props.value}
            aria-label={o.ariaLabel}
            tabindex={o.value === props.value ? 0 : -1}
            disabled={o.disabled}
            title={o.title}
            onClick={() => props.onChange(o.value)}
          >
            {o.label}
          </button>
        )}
      </For>
    </div>
  );
}
