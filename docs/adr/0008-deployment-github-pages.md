# ADR-0008: Deploy the Web Shell to GitHub Pages

- Status: accepted
- Date: 2026-11-17

## Context

v1.0 is a static site: WASM + JS + assets, with no server-side code (all
inference is on-device, and there is no payment backend yet; see ADR-0009). The
candidates were GitHub Pages, Cloudflare Pages, Vercel and Netlify. All except
GitHub Pages need a deploy token stored as a repository secret. GitHub Pages
can't set custom headers, so COOP/COEP cross-origin isolation is unavailable.

## Decision

Deploy with a GitHub Actions workflow (`.github/workflows/deploy.yml`) that
builds the WASM core and the Vite bundle and publishes it via
`actions/deploy-pages` on pushes to `main`. The app doesn't use
SharedArrayBuffer or WASM threads, so it doesn't need cross-origin isolation.
Vite's `base` is set from the repository name so asset URLs resolve under
`/<repo>/`.

## Consequences

- Zero extra credentials; one-time setup is Settings → Pages → Source:
  "GitHub Actions".
- The `dist/` output is host-agnostic; moving to Cloudflare Pages later is a
  workflow change only.
- If WASM threads are ever needed (e.g. a multi-threaded CPU fallback),
  revisit: they require COOP/COEP headers, or a service-worker shim on Pages.
