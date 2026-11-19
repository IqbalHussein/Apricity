import { For, Show } from "solid-js";
import type { JSX } from "solid-js";
import type { CompareSource, Overlay } from "../core/wasm";
import { pickImage } from "../lib/filePicker";
import { history } from "../state/history";
import { currentReference, openReferenceFile } from "../state/project";
import { ui } from "../state/ui";
import { Segmented } from "./Segmented";
import { PlainSliderRow } from "./Slider";

/** False-color zones, output-encoded Rec.709 luma percent (mirrors `output.wgsl`). */
const FALSE_COLOR_LEGEND = [
  { color: "rgb(128 0 153)", label: "Crushed, below 2%" },
  { color: "rgb(26 64 230)", label: "Near black, 2 to 8%" },
  { color: "rgb(51 191 77)", label: "Middle grey, 42 to 50%" },
  { color: "rgb(242 140 179)", label: "Skin tones, 62 to 72%" },
  { color: "rgb(255 230 26)", label: "Near clipping, 97 to 99.5%" },
  { color: "rgb(230 26 26)", label: "Clipped, above 99.5%" },
];

/**
 * Review-mode controls: compare source, wipe orientation, and overlays with
 * their legend.
 */
export function ReviewToolbar(): JSX.Element {
  const onSource = (v: CompareSource): void => {
    if (v === "reference" && !currentReference()) {
      pickImage((f) => void openReferenceFile(f));
      return;
    }
    ui.setCompareSource(v);
  };
  return (
    <div class="review-bar" role="toolbar" aria-label="Review tools">
      <div class="review-group">
        <span class="review-label">Compare with</span>
        <Segmented<CompareSource>
          label="Compare source"
          value={ui.compareSource()}
          onChange={onSource}
          options={[
            { value: "original", label: "Original" },
            { value: "stage1", label: "Stage 1 only" },
            {
              value: "previous",
              label: "Previous grade",
              disabled: history.previousParams() === null,
              title: history.previousParams() === null ? "Make a change first" : undefined,
            },
            {
              value: "reference",
              label: "Reference",
              title: currentReference() ? currentReference()?.name : "Choose a reference image",
            },
          ]}
        />
      </div>
      <div class="review-group">
        <span class="review-label">Divider</span>
        <Segmented<"vertical" | "horizontal">
          label="Wipe orientation"
          value={ui.wipeVertical() ? "vertical" : "horizontal"}
          onChange={(v) => ui.setWipeVertical(v === "vertical")}
          options={[
            { value: "vertical", label: "Vertical" },
            { value: "horizontal", label: "Horizontal" },
          ]}
        />
      </div>
      <div class="review-group">
        <span class="review-label">Overlay</span>
        <Segmented<Overlay>
          label="Overlay"
          value={ui.overlay()}
          onChange={ui.setOverlay}
          options={[
            { value: "none", label: "None" },
            { value: "false_color", label: "False color" },
            { value: "zebra", label: "Zebra" },
          ]}
        />
      </div>
      <Show when={ui.overlay() === "zebra"}>
        <div class="review-group review-wide">
          <PlainSliderRow
            label="Threshold"
            min={50}
            max={100}
            step={1}
            unit="%"
            value={() => Math.round(ui.zebraThreshold() * 100)}
            onChange={(v) => ui.setZebraThreshold(v / 100)}
          />
          <p class="legend-note">
            <span class="zebra-swatch" aria-hidden="true" />
            Stripes mark luma above {Math.round(ui.zebraThreshold() * 100)}%.
          </p>
        </div>
      </Show>
      <Show when={ui.overlay() === "false_color"}>
        <ul class="legend" aria-label="False color legend">
          <For each={FALSE_COLOR_LEGEND}>
            {(z) => (
              <li>
                <span class="legend-swatch" style={{ background: z.color }} aria-hidden="true" />
                {z.label}
              </li>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
}
