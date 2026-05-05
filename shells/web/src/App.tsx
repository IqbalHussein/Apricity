import { createSignal, onMount } from "solid-js";
import init, { identity_grade_json, version } from "./wasm/apricity_ffi";

/**
 * M0 smoke shell. Loads the WASM core, displays its version and the JSON for
 * the identity GradeParameters. Real editor UI lands in M4.
 */
export function App() {
  const [coreVersion, setCoreVersion] = createSignal<string | null>(null);
  const [identity, setIdentity] = createSignal<string | null>(null);

  onMount(async () => {
    await init();
    setCoreVersion(version());
    setIdentity(identity_grade_json());
  });

  return (
    <main style={{ "font-family": "system-ui", padding: "2rem" }}>
      <h1>Apricity</h1>
      <p>Cross-platform color grading. v0.0.0 — M0 scaffold.</p>
      <pre>core: {coreVersion() ?? "loading…"}</pre>
      <pre>identity grade: {identity() ?? "loading…"}</pre>
    </main>
  );
}
