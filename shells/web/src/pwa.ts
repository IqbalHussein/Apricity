/**
 * Register the service worker (production builds only) so the app shell,
 * the WASM core and the fonts work offline. The worker is served from the
 * deployment base, so sub-path hosting (`/Apricity/`) works.
 */
export function registerServiceWorker(): void {
  if (!import.meta.env.PROD || !("serviceWorker" in navigator)) return;
  const base = import.meta.env.BASE_URL;
  window.addEventListener("load", () => {
    navigator.serviceWorker.register(`${base}sw.js`, { scope: base }).catch((e: unknown) => {
      console.warn("Offline support is unavailable", e);
    });
  });
}
