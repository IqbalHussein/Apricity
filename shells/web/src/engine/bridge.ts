import {
  createEffect,
  createMemo,
  createRoot,
  createSignal,
  on,
  onCleanup,
  untrack,
} from "solid-js";
import { Engine } from "../core/wasm";
import type { BackendKind, ScopesResult } from "../core/wasm";
import { getClock } from "../state/clock";
import { applyAnalysis, gradeParams } from "../state/grade";
import { history } from "../state/history";
import { currentReference, currentSource, setStage1Analysis } from "../state/project";
import { ui } from "../state/ui";

/** Minimum gap between two scope computations, in milliseconds (10 Hz). */
export const SCOPES_MIN_INTERVAL_MS = 100;
/** Largest drawing surface side in device pixels. */
const MAX_SURFACE = 4096;

const [backend, setBackend] = createSignal<BackendKind | null>(null);
const [canvasSize, setCanvasSize] = createSignal<{ w: number; h: number }>({ w: 0, h: 0 });
const [scopes, setScopes] = createSignal<ScopesResult | null>(null);
const [engineError, setEngineError] = createSignal<string | null>(null);
const [requests, setRequests] = createSignal<readonly (readonly string[])[]>([]);

let engine: Engine | null = null;
let host: HTMLElement | null = null;
let canvas: HTMLCanvasElement | null = null;
let dirty = true;
let scopesDirty = true;
let lastScopesAt = -Infinity;
let rafId = 0;
let observer: ResizeObserver | null = null;
let disposeRoot: (() => void) | null = null;
let appliedSourceId = -1;
let appliedReferenceId = -1;

/** Active rendering backend once the engine exists (tracked). */
export const engineBackend = backend;
/** Canvas size in CSS pixels (tracked). */
export const previewSize = canvasSize;
/** Latest scope data (tracked). */
export const scopeData = scopes;
/** Last engine error text (tracked). */
export const engineErrorText = engineError;

/** Request a re-render on the next animation frame. */
export function invalidate(): void {
  dirty = true;
}

/**
 * Ask for scopes to be computed while a panel needs them. The bridge computes
 * the union of all requests, at most {@link SCOPES_MIN_INTERVAL_MS} apart.
 *
 * @param kinds - Scope kinds: `waveform`, `parade`, `vectorscope`, `histogram`, `legal`.
 * @returns A function that withdraws the request.
 */
export function requestScopes(kinds: readonly string[]): () => void {
  setRequests((r) => [...r, kinds]);
  scopesDirty = true;
  return () => setRequests((r) => r.filter((k) => k !== kinds));
}

function safe<T>(label: string, fn: () => T): T | undefined {
  try {
    return fn();
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    console.error(`engine: ${label}`, e);
    setEngineError(`${label}: ${msg}`);
    return undefined;
  }
}

function makeCanvas(): HTMLCanvasElement {
  const c = document.createElement("canvas");
  c.className = "preview-canvas";
  c.setAttribute("aria-label", "Image preview");
  c.setAttribute("role", "img");
  return c;
}

/**
 * Create the engine inside `hostEl`, preferring WebGPU. If WebGPU setup throws
 * after the canvas received a webgpu context, retry on a fresh canvas with the
 * CPU renderer.
 *
 * @param hostEl - Element that will contain the canvas.
 * @param forceCpu - Skip WebGPU (also set by `?cpu=1`).
 * @returns The active backend.
 */
export async function mountEngine(hostEl: HTMLElement, forceCpu = false): Promise<BackendKind> {
  disposeEngine();
  host = hostEl;
  const wantCpu = forceCpu || new URLSearchParams(location.search).has("cpu");
  canvas = makeCanvas();
  host.appendChild(canvas);
  try {
    engine = await Engine.create(canvas, wantCpu);
  } catch (e) {
    console.warn("WebGPU setup failed, retrying with the CPU renderer", e);
    canvas.remove();
    canvas = makeCanvas();
    host.appendChild(canvas);
    engine = await Engine.create(canvas, true);
  }
  const kind = engine.backend();
  setBackend(kind);
  observer = new ResizeObserver(() => resizeSurface());
  observer.observe(host);
  resizeSurface();
  disposeRoot = createRoot((dispose) => {
    installEffects();
    return dispose;
  });
  rafId = requestAnimationFrame(tick);
  return kind;
}

/** Stop the render loop and release the engine. */
export function disposeEngine(): void {
  cancelAnimationFrame(rafId);
  observer?.disconnect();
  observer = null;
  disposeRoot?.();
  disposeRoot = null;
  engine?.free();
  engine = null;
  canvas?.remove();
  canvas = null;
  appliedSourceId = -1;
  appliedReferenceId = -1;
  setBackend(null);
}

function resizeSurface(): void {
  if (!engine || !host) return;
  const dpr = Math.max(1, window.devicePixelRatio || 1);
  const cssW = host.clientWidth;
  const cssH = host.clientHeight;
  if (cssW < 2 || cssH < 2) return;
  const w = Math.min(MAX_SURFACE, Math.round(cssW * dpr));
  const h = Math.min(MAX_SURFACE, Math.round(cssH * dpr));
  setCanvasSize({ w: cssW, h: cssH });
  safe("resize", () => engine?.resize(w, h));
  dirty = true;
}

/**
 * Run Stage 1 analysis on the loaded image and apply it to the grade
 * according to the `auto_*` flags.
 *
 * @param label - Undo label, or `null` to fold into the baseline (image load).
 * @param key - Undo coalescing key for repeated runs (e.g. dragging the exposure target).
 */
export function reanalyze(label: string | null, key: string | null = null): void {
  if (!engine || !currentSource()) return;
  const e = engine;
  safe("analyze", () => {
    e.setParams(untrack(gradeParams));
    const a = e.analyzeStage1();
    setStage1Analysis(a);
    applyAnalysis(a, label, key);
  });
}

/** Letterbox color follows the `--surround` token so the canvas never shows a foreign black. */
function syncBackground(): void {
  const hex = getComputedStyle(document.documentElement).getPropertyValue("--surround").trim();
  const m = /^#([0-9a-f]{6})$/i.exec(hex);
  if (!m || !engine) return;
  const n = parseInt(m[1] as string, 16);
  // The surface is sRGB-encoded, so the token's channel values are used as they are.
  safe("background", () =>
    engine?.setBackground(((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255),
  );
  dirty = true;
}

function installEffects(): void {
  createEffect(() => {
    const src = currentSource();
    if (!engine || !src || src.id === appliedSourceId) return;
    appliedSourceId = src.id;
    const p = src.proxy;
    safe("source", () =>
      engine?.setSourceRgba(p.rgba, p.width, p.height, p.fullWidth, p.fullHeight),
    );
    untrack(() => reanalyze(null));
    scopesDirty = true;
    dirty = true;
  });

  createEffect(() => {
    const ref = currentReference();
    if (!engine) return;
    if (!ref) {
      if (appliedReferenceId !== -1) safe("reference", () => engine?.clearReference());
      appliedReferenceId = -1;
    } else if (ref.id !== appliedReferenceId) {
      appliedReferenceId = ref.id;
      safe("reference", () =>
        engine?.setReferenceRgba(ref.proxy.rgba, ref.proxy.width, ref.proxy.height),
      );
    }
    dirty = true;
  });

  createEffect(() => {
    const p = gradeParams();
    safe("params", () => engine?.setParams(p));
    dirty = true;
    scopesDirty = true;
  });

  createEffect(() => {
    const review = ui.mode() === "review";
    const previous = history.previousParams();
    const source = ui.compareSource();
    const cfg = {
      enabled: review,
      vertical: ui.wipeVertical(),
      position: ui.wipePosition(),
      source,
      previous,
    };
    safe("compare", () => engine?.setCompare(cfg));
    dirty = true;
  });

  createEffect(() => {
    const kind = ui.mode() === "review" ? ui.overlay() : "none";
    const t = ui.zebraThreshold();
    safe("overlay", () => engine?.setOverlay(kind, t));
    dirty = true;
  });

  createEffect(() => {
    const v = ui.view();
    safe("view", () => engine?.setView({ zoom: v.zoom, pan: v.pan }));
    dirty = true;
  });

  createEffect(
    on(ui.theme, () => {
      queueMicrotask(syncBackground);
    }),
  );
  const mq = matchMedia("(prefers-color-scheme: light)");
  mq.addEventListener("change", syncBackground);
  onCleanup(() => mq.removeEventListener("change", syncBackground));

  const kinds = createMemo(() => [...new Set(requests().flat())].sort().join(","));
  createEffect(
    on(kinds, () => {
      scopesDirty = true;
    }),
  );
}

function computeScopes(): void {
  if (!engine || !currentSource()) return;
  const list = kindsNow();
  if (list.length === 0) {
    return;
  }
  const result = safe("scopes", () => engine?.scopes(list));
  if (result) setScopes(result);
}

function kindsNow(): string[] {
  return [...new Set(untrack(requests).flat())];
}

function tick(): void {
  rafId = requestAnimationFrame(tick);
  if (!engine) return;
  if (dirty && currentSource()) {
    dirty = false;
    safe("render", () => engine?.render());
  }
  if (scopesDirty) {
    const now = getClock().nowMs();
    if (now - lastScopesAt >= SCOPES_MIN_INTERVAL_MS) {
      scopesDirty = false;
      lastScopesAt = now;
      computeScopes();
    }
  }
}

/**
 * The canvas element, for tests and screenshots.
 *
 * @returns The preview canvas or `null` before mount.
 */
export function previewCanvas(): HTMLCanvasElement | null {
  return canvas;
}
