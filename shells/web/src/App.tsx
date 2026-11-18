import { createSignal, onMount } from "solid-js";
import { identityParams, initCore, version } from "./core/wasm";

/**
 * Smoke shell: loads the WASM core and shows its version and the identity
 * grade. The editor UI lands in M6.
 */
export function App() {
  const [coreVersion, setCoreVersion] = createSignal<string | null>(null);
  const [identity, setIdentity] = createSignal<string | null>(null);

  onMount(async () => {
    await initCore();
    setCoreVersion(version());
    setIdentity(JSON.stringify(identityParams(), null, 2));
  });

  return (
    <main style={{ "font-family": "system-ui", padding: "2rem" }}>
      <h1>Apricity</h1>
      <p>Cross-platform color grading. Core smoke page.</p>
      <pre>core: {coreVersion() ?? "loading…"}</pre>
      <pre>identity grade: {identity() ?? "loading…"}</pre>
    </main>
  );
}
