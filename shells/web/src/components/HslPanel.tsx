import { createSignal, For, Show } from "solid-js";
import type { JSX } from "solid-js";
import { isAiSet } from "../state/aimarks";
import { getSpecs, specFor } from "../state/specs";
import { ui } from "../state/ui";
import { SliderRow } from "./Slider";

const [bandSignal, setBandSignal] = createSignal(0);

/** Index of the HSL band shown in the picker (into `SpecTable.hsl_bands`). */
export const hslBandIndex = bandSignal;
/**
 * Show a band in the HSL panel.
 *
 * @param i - Band index into `SpecTable.hsl_bands`.
 */
export const showHslBand = setBandSignal;

/**
 * HSL panel: an 8-band picker (swatches colored by each band's center hue)
 * and the hue, saturation and luminance sliders of the selected band, all
 * generated from the spec table.
 */
export function HslPanel(): JSX.Element {
  const specs = getSpecs();
  const band = (): string => specs.hsl_bands[bandSignal()] ?? "red";
  const bandAi = (name: string): boolean =>
    (["hue", "saturation", "luminance"] as const).some((k) => isAiSet(`stage2.hsl.${name}.${k}`));
  const flashing = (): boolean => ui.flash()?.path.startsWith("stage2.hsl.") === true;
  return (
    <div class="hsl" classList={{ "row-flash": flashing() }}>
      <div class="hsl-bands" role="radiogroup" aria-label="HSL band">
        <For each={specs.hsl_bands}>
          {(name, i) => (
            <button
              type="button"
              role="radio"
              class="hsl-swatch"
              classList={{ "has-ai": bandAi(name) }}
              aria-checked={bandSignal() === i()}
              aria-label={`${name}${bandAi(name) ? ", set by the AI" : ""}`}
              title={name}
              style={{ background: `hsl(${specs.hsl_centers_deg[i()] ?? 0} 75% 52%)` }}
              onClick={() => setBandSignal(i())}
            />
          )}
        </For>
      </div>
      <For each={["hue", "saturation", "luminance"] as const}>
        {(k) => {
          const spec = (): ReturnType<typeof specFor> => specFor(`stage2.hsl.${band()}.${k}`);
          return (
            <Show when={spec()} keyed>
              {(s) => (
                <SliderRow spec={s} label={`${band()[0]?.toUpperCase()}${band().slice(1)} ${k}`} />
              )}
            </Show>
          );
        }}
      </For>
    </div>
  );
}
