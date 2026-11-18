import { onCleanup, Show } from "solid-js";
import type { JSX } from "solid-js";
import { clearNotice, currentNotice, proNote } from "../entitlements/store";
import type { Feature } from "../entitlements/gating";
import { CloseIcon, InfoIcon } from "./Icons";

/**
 * Inline note for a gated feature. During the beta it says every feature is
 * unlocked; on Free it states the limit. There is no checkout: no payment
 * provider exists yet (ADR-0009).
 *
 * @param props.feature - The gated capability the note is about.
 */
export function UpgradeNotice(props: { feature: Feature }): JSX.Element {
  return (
    <Show when={proNote(props.feature)}>
      {(text) => (
        <p class="upgrade-notice" data-testid="upgrade-notice">
          <InfoIcon size={14} />
          <span>{text()}</span>
        </p>
      )}
    </Show>
  );
}

/** Dismissible toast for blocked actions; hides itself after a few seconds. */
export function NoticeHost(): JSX.Element {
  let timer = 0;
  const arm = (): void => {
    window.clearTimeout(timer);
    timer = window.setTimeout(clearNotice, 7000);
  };
  onCleanup(() => window.clearTimeout(timer));
  return (
    <Show when={currentNotice()} keyed>
      {(text) => {
        arm();
        return (
          <div class="notice-toast" role="status" data-testid="notice-toast">
            <InfoIcon size={14} />
            <span>{text}</span>
            <button type="button" class="icon-btn" aria-label="Dismiss" onClick={clearNotice}>
              <CloseIcon size={14} />
            </button>
          </div>
        );
      }}
    </Show>
  );
}
