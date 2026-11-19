import { createSignal, onCleanup, onMount, Show } from "solid-js";
import type { JSX } from "solid-js";
import { pickMedia } from "../lib/filePicker";
import { importError, isLoading, openMediaFile, openSample } from "../state/project";
import { OpenIcon } from "./Icons";

/**
 * Empty state shown over the canvas until an image is open. Also installs
 * window-level drag-and-drop ("drop anywhere") for as long as it is mounted
 * with `global` set.
 *
 * @param props.empty - `true` shows the empty-state card; `false` only the drop handling and errors.
 */
export function ImportDropzone(props: { empty: boolean }): JSX.Element {
  const [dragging, setDragging] = createSignal(false);
  const hasFiles = (e: DragEvent): boolean =>
    Array.from(e.dataTransfer?.types ?? []).includes("Files");
  const onDragOver = (e: DragEvent): void => {
    if (!hasFiles(e)) return;
    e.preventDefault();
    setDragging(true);
  };
  const onDragLeave = (e: DragEvent): void => {
    if (e.relatedTarget === null) setDragging(false);
  };
  const onDrop = (e: DragEvent): void => {
    if (!hasFiles(e)) return;
    e.preventDefault();
    setDragging(false);
    const f = e.dataTransfer?.files[0];
    if (f) void openMediaFile(f);
  };
  onMount(() => {
    window.addEventListener("dragover", onDragOver);
    window.addEventListener("dragleave", onDragLeave);
    window.addEventListener("drop", onDrop);
  });
  onCleanup(() => {
    window.removeEventListener("dragover", onDragOver);
    window.removeEventListener("dragleave", onDragLeave);
    window.removeEventListener("drop", onDrop);
  });

  return (
    <>
      <Show when={props.empty}>
        <div class="empty">
          <p class="empty-text">Open a photo or video to start grading.</p>
          <div class="empty-actions">
            <button
              type="button"
              class="btn btn-primary"
              onClick={() => pickMedia((f) => void openMediaFile(f))}
              disabled={isLoading()}
            >
              <OpenIcon size={16} />
              Open file
            </button>
            <button type="button" class="link-btn" onClick={() => openSample()}>
              Try a sample image
            </button>
          </div>
          <Show when={importError()}>
            <p class="error-text" role="alert">
              {importError()}
            </p>
          </Show>
        </div>
      </Show>
      <Show when={!props.empty && importError()}>
        <p class="toast error-text" role="alert">
          {importError()}
        </p>
      </Show>
      <Show when={dragging()}>
        <div class="drop-veil" aria-hidden="true">
          <span>Drop to open</span>
        </div>
      </Show>
    </>
  );
}
