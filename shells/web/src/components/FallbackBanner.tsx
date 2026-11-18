import { Show } from "solid-js";
import type { JSX } from "solid-js";
import { engineBackend } from "../engine/bridge";
import { ui } from "../state/ui";
import { CloseIcon, InfoIcon } from "./Icons";

/**
 * Banner shown when the engine runs on the CPU renderer instead of WebGPU
 * (ADR-0007). Copy is fixed by ui-spec.
 */
export function FallbackBanner(): JSX.Element {
  return (
    <Show when={engineBackend() === "cpu" && !ui.fallbackDismissed()}>
      <div class="banner" role="status">
        <span class="banner-icon">
          <InfoIcon size={16} />
        </span>
        <p>
          Your browser doesn&apos;t support WebGPU, so the preview runs at reduced resolution.
          Grading, scopes and export still work.
        </p>
        <button
          type="button"
          class="icon-btn"
          aria-label="Dismiss"
          onClick={() => ui.setFallbackDismissed(true)}
        >
          <CloseIcon size={14} />
        </button>
      </div>
    </Show>
  );
}
