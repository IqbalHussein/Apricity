import { createSignal, onMount, Show } from "solid-js";
import type { JSX } from "solid-js";
import { Shell } from "./components/Shell";
import { initCore } from "./core/wasm";
import { initGrade } from "./state/grade";
import "./styles/app.css";

/**
 * Application root: loads the Rust core (WASM), seeds the stores from it and
 * renders the shell once ready.
 */
export function App(): JSX.Element {
  const [ready, setReady] = createSignal(false);
  const [failure, setFailure] = createSignal<string | null>(null);

  onMount(async () => {
    try {
      await initCore();
      initGrade();
      setReady(true);
    } catch (e) {
      setFailure(e instanceof Error ? e.message : String(e));
    }
  });

  return (
    <Show
      when={ready()}
      fallback={
        <div class="boot" role={failure() ? "alert" : "status"}>
          {failure()
            ? `Apricity could not start the color engine. ${failure()}`
            : "Loading the color engine"}
        </div>
      }
    >
      <Shell />
    </Show>
  );
}
