import { createSignal } from "solid-js";
import type { CompareSource, Overlay } from "../core/wasm";
import type { View } from "../lib/view";

/** Interaction mode (PRD §8.2). */
export type Mode = "quick" | "grade" | "review";
/** Theme choice; `system` follows `prefers-color-scheme`. */
export type ThemeChoice = "system" | "light" | "dark";
/** Scope tabs in Grade mode. */
export type ScopeTab = "waveform" | "parade" | "vectorscope" | "histogram";

const THEME_KEY = "apricity.theme";

function readTheme(): ThemeChoice {
  try {
    const v = localStorage.getItem(THEME_KEY);
    if (v === "light" || v === "dark") return v;
  } catch {
    // storage unavailable (private window, blocked): fall back to system
  }
  return "system";
}

const [mode, setMode] = createSignal<Mode>("quick");
const [theme, setThemeSignal] = createSignal<ThemeChoice>(readTheme());
const [scopeTab, setScopeTab] = createSignal<ScopeTab>("waveform");
const [explainOpen, setExplainOpen] = createSignal(false);
const [fallbackDismissed, setFallbackDismissed] = createSignal(false);
const [view, setView] = createSignal<View>({ zoom: 1, pan: [0, 0] });
const [wipeVertical, setWipeVertical] = createSignal(true);
const [wipePosition, setWipePosition] = createSignal(0.5);
const [compareSource, setCompareSource] = createSignal<CompareSource>("original");
const [overlay, setOverlay] = createSignal<Overlay>("none");
const [zebraThreshold, setZebraThreshold] = createSignal(0.95);
const [scrollTarget, setScrollTarget] = createSignal<{ path: string; token: number } | null>(null);
const [flash, setFlash] = createSignal<{ path: string; token: number } | null>(null);
const [mobile, setMobile] = createSignal(false);
const [sheetHeight, setSheetHeight] = createSignal(0);
const [stage1Open, setStage1Open] = createSignal(true);
const [scopesOpen, setScopesOpen] = createSignal(true);
let targetToken = 0;

/** Reactive UI state shared by components; no pipeline data lives here. */
export const ui = {
  /** Current mode. */
  mode,
  /**
   * Switch mode (instant; all other state is preserved).
   *
   * @param m - Target mode.
   */
  setMode,
  /** Theme choice. */
  theme,
  /**
   * Set and persist the theme choice (per-viewer convenience only).
   *
   * @param t - `system`, `light` or `dark`.
   */
  setTheme(t: ThemeChoice): void {
    setThemeSignal(t);
    try {
      if (t === "system") localStorage.removeItem(THEME_KEY);
      else localStorage.setItem(THEME_KEY, t);
    } catch {
      // not persisted; the choice still applies for this session
    }
  },
  /** Active scope tab. */
  scopeTab,
  setScopeTab,
  /** Whether the Explainability Panel is expanded. */
  explainOpen,
  setExplainOpen,
  /** Whether the user dismissed the WebGPU fallback banner. */
  fallbackDismissed,
  setFallbackDismissed,
  /** Zoom (1 = fit, up to 8) and pan (fraction of displayed image size). */
  view,
  setView,
  /** Wipe orientation: `true` = vertical divider (A left, B right). */
  wipeVertical,
  setWipeVertical,
  /** Wipe divider position as a fraction of the image, 0..1. */
  wipePosition,
  setWipePosition,
  /** What side B of the wipe shows. */
  compareSource,
  setCompareSource,
  /** Preview overlay. */
  overlay,
  setOverlay,
  /** Zebra threshold as output-encoded luma, 0..1. */
  zebraThreshold,
  setZebraThreshold,
  /** The console control to scroll to; `token` changes on every request. */
  scrollTarget,
  /** The console row currently flashing. */
  flash,
  /**
   * Ask the Manual Console to reveal and flash a control.
   *
   * @param path - Anchor path (`ParameterChange.console_anchor`).
   */
  revealControl(path: string): void {
    targetToken += 1;
    setScrollTarget({ path, token: targetToken });
    setFlash({ path, token: targetToken });
    setMode("grade");
  },
  /** Clear the flash highlight. */
  clearFlash(): void {
    setFlash(null);
  },
  /** `true` below 1024px viewport width. */
  mobile,
  setMobile,
  /** Whether the Correction panel is expanded. */
  stage1Open,
  setStage1Open,
  /** Whether the scopes dock in the Grade inspector is expanded. */
  scopesOpen,
  setScopesOpen,
  /** Mobile bottom-sheet height in CSS pixels; 0 means "default". */
  sheetHeight,
  setSheetHeight,
};
