import { Show } from "solid-js";
import type { JSX } from "solid-js";
import { requireFeature } from "../entitlements/store";
import { pickFile } from "../lib/download";
import { activeLut, importLutFile, lutError, removeLut } from "../state/lut";
import { specFor } from "../state/specs";
import { SliderRow } from "./Slider";
import { UpgradeNotice } from "./UpgradeNotice";

/**
 * The LUT section of the Manual Console: import a `.cube`, then a strength
 * slider (`stage2.lut_strength`, percent) and "Remove LUT". The LUT is applied
 * inside Stage 2 in sRGB-encoded Rec.709.
 */
export function LutSection(): JSX.Element {
  const strength = specFor("stage2.lut_strength");
  return (
    <div class="lut-section" data-testid="lut-section">
      <Show
        when={activeLut()}
        fallback={
          <p class="dialog-note">Apply a 3D LUT on top of the grade. Only .cube files work.</p>
        }
      >
        {(lut) => (
          <p class="lut-active" data-testid="lut-active">
            <span class="lut-title" title={lut().title}>
              {lut().title}
            </span>
            <span class="lut-size">{lut().size} points</span>
          </p>
        )}
      </Show>
      <Show when={activeLut() && strength}>
        <SliderRow spec={strength as NonNullable<typeof strength>} />
      </Show>
      <div class="lut-actions">
        <button
          type="button"
          class="btn"
          data-testid="lut-import"
          onClick={() => {
            if (requireFeature("lut_import")) pickFile(".cube,text/plain", (f) => void importLutFile(f));
          }}
        >
          Import LUT (.cube)
        </button>
        <Show when={activeLut()}>
          <button type="button" class="btn" data-testid="lut-remove" onClick={() => removeLut()}>
            Remove LUT
          </button>
        </Show>
      </div>
      <Show when={lutError()}>
        <p class="error-text" role="alert">
          {lutError()}
        </p>
      </Show>
      <UpgradeNotice feature="lut_import" />
    </div>
  );
}
