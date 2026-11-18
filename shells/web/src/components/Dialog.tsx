import { createUniqueId, onMount } from "solid-js";
import type { JSX } from "solid-js";
import { CloseIcon } from "./Icons";

/**
 * Modal dialog on the native `<dialog>` element (focus trap, Escape and the
 * top layer come from the browser). Clicking the backdrop closes it.
 *
 * @param props.title - Heading, also the accessible name.
 * @param props.onClose - Called after the dialog closed by any route.
 * @param props.testId - Value for `data-testid` on the dialog element.
 * @param props.wide - Use the wide layout (recipe grid).
 * @param props.children - Dialog body.
 */
export function Dialog(props: {
  title: string;
  onClose: () => void;
  testId?: string;
  wide?: boolean;
  children: JSX.Element;
}): JSX.Element {
  let el!: HTMLDialogElement;
  const titleId = createUniqueId();
  onMount(() => el.showModal());
  return (
    <dialog
      ref={el}
      class="dialog"
      classList={{ "dialog-wide": props.wide === true }}
      aria-labelledby={titleId}
      data-testid={props.testId}
      on:close={() => props.onClose()}
      onClick={(e) => {
        if (e.target === el) el.close();
      }}
    >
      <header class="dialog-head">
        <h2 id={titleId} class="dialog-title">
          {props.title}
        </h2>
        <button type="button" class="icon-btn" aria-label="Close" onClick={() => el.close()}>
          <CloseIcon size={16} />
        </button>
      </header>
      <div class="dialog-body">{props.children}</div>
    </dialog>
  );
}
