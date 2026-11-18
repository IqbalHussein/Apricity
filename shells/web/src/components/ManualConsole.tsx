import { createEffect, createMemo, createSignal, For, on, Show } from "solid-js";
import type { JSX } from "solid-js";
import { sentenceCase } from "../lib/format";
import { getSpecs } from "../state/specs";
import { ui } from "../state/ui";
import { ColorWheel } from "./ColorWheel";
import { CurveEditor, showCurveChannel } from "./CurveEditor";
import { ChevronIcon } from "./Icons";
import { HslPanel, showHslBand } from "./HslPanel";
import { SliderRow } from "./Slider";

/** Console sections in display order; each is a spec `group` or the virtual "Curves". */
const ORDER = ["Light", "Color", "Wheels", "HSL", "Curves", "Split Tone", "Effects"] as const;
/** Spec groups that never appear in the Manual Console. */
const HIDDEN = new Set(["Correction", "LUT"]);

function groupOf(path: string): string {
  if (path.startsWith("stage2.tone_curve")) return "Curves";
  return (
    getSpecs().scalars.find((s) => s.path === path)?.group ??
    (path.startsWith("stage2.lift") ||
    path.startsWith("stage2.gamma") ||
    path.startsWith("stage2.gain")
      ? "Wheels"
      : "")
  );
}

/**
 * The Manual Console: every Stage 2 control, generated from `paramSpecs()`
 * and grouped by `group`. Wheels, HSL and Curves get dedicated editors;
 * everything else is a generic slider row. Stage 1 (Correction) lives in its
 * own panel. Reveals and flashes the control named by `ui.scrollTarget`.
 */
export function ManualConsole(): JSX.Element {
  let root!: HTMLDivElement;
  const specs = getSpecs();
  const groups = createMemo(() => {
    const present = new Set(
      specs.scalars
        .filter((s) => s.stage === "creative" && !HIDDEN.has(s.group))
        .map((s) => s.group),
    );
    const list: string[] = ORDER.filter((g) => g === "Curves" || present.has(g));
    for (const g of present) if (!list.includes(g)) list.push(g);
    return list;
  });
  const [closed, setClosed] = createSignal<ReadonlySet<string>>(new Set(["Split Tone", "Effects"]));
  const toggle = (g: string): void => {
    setClosed((c) => (c.has(g) ? new Set([...c].filter((x) => x !== g)) : new Set([...c, g])));
  };

  createEffect(
    on(
      ui.scrollTarget,
      (t) => {
        if (!t) return;
        const group = groupOf(t.path);
        setClosed((c) => new Set([...c].filter((x) => x !== group)));
        const hsl = /^stage2\.hsl\.(\w+)\./.exec(t.path);
        if (hsl) showHslBand(Math.max(0, specs.hsl_bands.indexOf(hsl[1] as string)));
        const curve = /^stage2\.tone_curve\.(master|red|green|blue)$/.exec(t.path);
        if (curve) showCurveChannel(curve[1] as "master" | "red" | "green" | "blue");
        // wait for the section to open and the HSL band to render before measuring
        setTimeout(() => {
          const el =
            root
              .closest(".inspector-scroll")
              ?.querySelector<HTMLElement>(`[data-anchor~="${CSS.escape(t.path)}"]`) ??
            document.querySelector<HTMLElement>(`[data-anchor~="${CSS.escape(t.path)}"]`);
          const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
          el?.scrollIntoView({ block: "center", behavior: reduce ? "auto" : "smooth" });
          setTimeout(() => ui.clearFlash(), 1600);
        }, 60);
      },
      { defer: true },
    ),
  );

  return (
    <div class="console" ref={root}>
      <For each={groups()}>
        {(g) => (
          <section class="section" data-group={g}>
            <h3 class="section-title">
              <button
                type="button"
                class="section-toggle"
                aria-expanded={!closed().has(g)}
                onClick={() => toggle(g)}
              >
                <span class="chev" classList={{ open: !closed().has(g) }}>
                  <ChevronIcon size={12} />
                </span>
                {sentenceCase(g)}
              </button>
            </h3>
            <Show when={!closed().has(g)}>
              <div class="section-body">
                <Show when={g === "Wheels"}>
                  <div class="wheels">
                    <ColorWheel name="lift" title="Lift" />
                    <ColorWheel name="gamma" title="Gamma" />
                    <ColorWheel name="gain" title="Gain" />
                  </div>
                </Show>
                <Show when={g === "HSL"}>
                  <HslPanel />
                </Show>
                <Show when={g === "Curves"}>
                  <CurveEditor />
                </Show>
                <Show when={g !== "Wheels" && g !== "HSL" && g !== "Curves"}>
                  <For each={specs.scalars.filter((s) => s.stage === "creative" && s.group === g)}>
                    {(s) => <SliderRow spec={s} />}
                  </For>
                </Show>
              </div>
            </Show>
          </section>
        )}
      </For>
    </div>
  );
}
