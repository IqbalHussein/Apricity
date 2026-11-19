import { createSignal, For, onCleanup, Show } from "solid-js";
import type { JSX } from "solid-js";
import { lastNgaResult, ngaError, promptHistory, runPrompt, runSurprise } from "../state/ai";
import { currentSource } from "../state/project";
import { ArrowUpIcon, ShuffleIcon } from "./Icons";

/**
 * The Neural Grading Agent prompt bar: free-text input, Apply (auto mode),
 * Refine (keeps the grade), Surprise me (seed from the injected RNG), and a
 * history popover with the last 20 prompts.
 */
export function NgaPromptBar(): JSX.Element {
  let input!: HTMLInputElement;
  const [text, setText] = createSignal("");
  const [open, setOpen] = createSignal(false);
  const disabled = (): boolean => currentSource() === null;
  const unmatched = (): readonly string[] => lastNgaResult()?.unmatched_words ?? [];
  const submit = (mode: "auto" | "refine"): void => {
    if (runPrompt(text(), mode)) setOpen(false);
  };
  const onDocClick = (e: MouseEvent): void => {
    if (!(e.target as Element).closest(".prompt-history, .history-btn")) setOpen(false);
  };
  document.addEventListener("click", onDocClick);
  onCleanup(() => document.removeEventListener("click", onDocClick));

  return (
    <div class="prompt" classList={{ "prompt-disabled": disabled() }}>
      <div class="prompt-row">
        <div class="prompt-field">
          <input
            ref={input}
            class="prompt-input"
            type="text"
            enterkeyhint="go"
            autocomplete="off"
            spellcheck={false}
            placeholder="Describe the look you want, like cold, desaturated noir"
            aria-label="Describe the look you want"
            disabled={disabled()}
            value={text()}
            onInput={(e) => setText(e.currentTarget.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                submit("auto");
              } else if (e.key === "ArrowUp" && text() === "" && promptHistory().length > 0) {
                e.preventDefault();
                setOpen(true);
              } else if (e.key === "Escape") setOpen(false);
            }}
          />
          <button
            type="button"
            class="icon-btn history-btn"
            aria-label="Prompt history"
            aria-expanded={open()}
            aria-haspopup="listbox"
            disabled={promptHistory().length === 0}
            onClick={() => setOpen(!open())}
          >
            <ArrowUpIcon size={16} />
          </button>
          <Show when={open()}>
            <ul class="prompt-history" role="listbox" aria-label="Recent prompts">
              <For each={promptHistory()}>
                {(p) => (
                  <li role="option" aria-selected="false">
                    <button
                      type="button"
                      onClick={() => {
                        setText(p);
                        setOpen(false);
                        input.focus();
                      }}
                    >
                      {p}
                    </button>
                  </li>
                )}
              </For>
            </ul>
          </Show>
        </div>
        <div class="prompt-actions">
          <button
            type="button"
            class="btn btn-primary"
            disabled={disabled()}
            onClick={() => submit("auto")}
          >
            Apply
          </button>
          <button
            type="button"
            class="btn"
            disabled={disabled()}
            onClick={() => submit("refine")}
            title="Keep the current grade and add this change"
          >
            Refine
          </button>
          <button
            type="button"
            class="btn btn-icon-text"
            disabled={disabled()}
            onClick={() => runSurprise()}
          >
            <ShuffleIcon size={14} />
            Surprise me
          </button>
        </div>
      </div>
      <Show when={unmatched().length > 0}>
        <p class="prompt-note">Didn&apos;t recognize: {unmatched().join(", ")}</p>
      </Show>
      <Show when={ngaError()}>
        <p class="prompt-note error-text" role="alert">
          {ngaError()}
        </p>
      </Show>
    </div>
  );
}
