/* Apricity service worker (PRD §7.3: the core workflow works offline).
 *
 * - Static assets: cache-first (Vite names them with content hashes, so a hit never goes stale).
 * - Google Fonts: cache-first, including the cross-origin stylesheet and font files.
 * - Navigation: network-first; offline falls back to the cached index.html.
 * - Everything is relative to the registration scope, so it works under a sub-path
 *   such as https://<user>.github.io/Apricity/.
 * Bump VERSION to drop old caches.
 */
const VERSION = "apricity-v1";
const FONT_HOSTS = ["fonts.googleapis.com", "fonts.gstatic.com"];
const SCOPE = self.registration.scope;
const INDEX = new URL("index.html", SCOPE).href;

self.addEventListener("install", (event) => {
  event.waitUntil(precache().then(() => self.skipWaiting()));
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k !== VERSION).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  );
});

self.addEventListener("fetch", (event) => {
  const req = event.request;
  if (req.method !== "GET") return;
  const url = new URL(req.url);
  if (req.mode === "navigate") {
    event.respondWith(networkFirstNavigation(req));
  } else if (url.origin === self.location.origin || FONT_HOSTS.includes(url.hostname)) {
    event.respondWith(cacheFirst(req));
  }
});

async function put(request, response) {
  // Opaque (no-cors) responses are cached too: Google Fonts' stylesheet link is one.
  if (response && (response.ok || response.type === "opaque")) {
    const cache = await caches.open(VERSION);
    await cache.put(request, response.clone());
  }
  return response;
}

async function cacheFirst(request) {
  const cached = await caches.match(request, { ignoreVary: true });
  if (cached) return cached;
  try {
    return await put(request, await fetch(request));
  } catch (e) {
    return new Response("", { status: 504, statusText: "Offline" });
  }
}

async function networkFirstNavigation(request) {
  try {
    const response = await fetch(request);
    if (response.ok) await put(INDEX, response.clone());
    return response;
  } catch (e) {
    const cached = (await caches.match(INDEX)) || (await caches.match(SCOPE));
    return cached || new Response("Apricity is offline and has not been cached yet.", { status: 503 });
  }
}

/** Fetch the app shell and everything it references so the first offline visit works. */
async function precache() {
  const cache = await caches.open(VERSION);
  const res = await fetch(new Request(INDEX, { cache: "reload" }));
  if (!res.ok) return;
  await cache.put(INDEX, res.clone());
  const html = await res.text();
  const urls = new Set();
  for (const m of html.matchAll(/(?:src|href)="([^"]+)"/g)) {
    const u = new URL(m[1], INDEX);
    if (u.origin === self.location.origin || FONT_HOSTS.includes(u.hostname)) urls.add(u.href);
  }
  await Promise.all([...urls].map((u) => cacheUrl(cache, u)));
  // Scripts reference the WASM, styles and fonts by file name; fetch those too.
  for (const u of urls) {
    if (!/\.(js|css)(\?|$)/.test(u) && !u.includes("fonts.googleapis.com")) continue;
    const r = await cache.match(u, { ignoreVary: true });
    if (!r || r.type === "opaque") continue;
    const text = await r.clone().text();
    const found = new Set();
    for (const m of text.matchAll(/(?:url\(|["'`])((?:https:\/\/fonts\.gstatic\.com\/|\.{0,2}\/|assets\/)?[A-Za-z0-9_./%-]+\.(?:wasm|woff2?|css))/g)) {
      try {
        const abs = new URL(m[1], u);
        if (abs.origin === self.location.origin || FONT_HOSTS.includes(abs.hostname)) found.add(abs.href);
      } catch (e) {
        /* not a URL */
      }
    }
    await Promise.all([...found].map((f) => cacheUrl(cache, f)));
  }
}

async function cacheUrl(cache, url) {
  try {
    if (await cache.match(url, { ignoreVary: true })) return;
    const sameOrigin = new URL(url).origin === self.location.origin;
    const res = await fetch(url, sameOrigin ? undefined : { mode: "cors" });
    if (res.ok) await cache.put(url, res);
  } catch (e) {
    /* best effort: runtime caching picks it up later */
  }
}
