import { createEffect, createMemo, For, onCleanup, onMount, Show } from "solid-js";
import type { JSX } from "solid-js";
import { graticule } from "../core/wasm";
import type { Histogram, LegalReport, ScopeRaster } from "../core/wasm";
import { requestScopes, scopeData } from "../engine/bridge";
import type { ScopeTab } from "../state/ui";
import { ui } from "../state/ui";
import { Segmented } from "./Segmented";
import { CheckIcon, WarnIcon } from "./Icons";

const TABS: { value: ScopeTab; label: string }[] = [
  { value: "waveform", label: "Waveform" },
  { value: "parade", label: "Parade" },
  { value: "vectorscope", label: "Vectorscope" },
  { value: "histogram", label: "Histogram" },
];

/** Scope overlays sit on a scope background that is dark in both themes, so they use fixed light greys. */
const SCOPE_DIM = "#a3a3a3";
const SCOPE_TEXT = "#ededed";

/** Draw a scope raster scaled into the canvas, aspect-fit and centered; returns the target rect. */
function drawRaster(
  ctx: CanvasRenderingContext2D,
  r: ScopeRaster,
  w: number,
  h: number,
  square: boolean,
): { x: number; y: number; w: number; h: number } {
  const tmp = document.createElement("canvas");
  tmp.width = r.width;
  tmp.height = r.height;
  const tctx = tmp.getContext("2d");
  if (!tctx) return { x: 0, y: 0, w, h };
  tctx.putImageData(new ImageData(new Uint8ClampedArray(r.rgba), r.width, r.height), 0, 0);
  let rect = { x: 0, y: 0, w, h };
  if (square) {
    const s = Math.min(w, h);
    rect = { x: (w - s) / 2, y: (h - s) / 2, w: s, h: s };
  }
  ctx.imageSmoothingEnabled = true;
  ctx.drawImage(tmp, rect.x, rect.y, rect.w, rect.h);
  return rect;
}

function drawIreLines(ctx: CanvasRenderingContext2D, w: number, h: number): void {
  const line = SCOPE_DIM;
  ctx.save();
  ctx.font = "11px Archivo, system-ui, sans-serif";
  ctx.textBaseline = "bottom";
  for (const ire of [0, 50, 100]) {
    const y = Math.round(h - 1 - (ire / 100) * (h - 2)) + 0.5;
    ctx.strokeStyle = line;
    ctx.globalAlpha = ire === 50 ? 0.35 : 0.55;
    ctx.setLineDash(ire === 50 ? [3, 4] : []);
    ctx.beginPath();
    ctx.moveTo(0, y);
    ctx.lineTo(w, y);
    ctx.stroke();
    ctx.globalAlpha = 1;
    ctx.fillStyle = line;
    ctx.fillText(String(ire), 4, ire === 0 ? y - 2 : ire === 100 ? y + 13 : y - 2);
  }
  ctx.restore();
}

function drawVectorGraticule(
  ctx: CanvasRenderingContext2D,
  rect: { x: number; y: number; w: number; h: number },
): void {
  const g = graticule();
  const dim = SCOPE_DIM;
  const px = (x: number): number => rect.x + x * rect.w;
  const py = (y: number): number => rect.y + y * rect.h;
  ctx.save();
  ctx.strokeStyle = dim;
  ctx.lineWidth = 1;
  ctx.globalAlpha = 0.4;
  ctx.beginPath();
  ctx.arc(px(0.5), py(0.5), rect.w * 0.45, 0, Math.PI * 2);
  ctx.moveTo(rect.x, py(0.5));
  ctx.lineTo(rect.x + rect.w, py(0.5));
  ctx.moveTo(px(0.5), rect.y);
  ctx.lineTo(px(0.5), rect.y + rect.h);
  ctx.stroke();
  const poly = (pts: [number, number][], dash: number[], alpha: number): void => {
    ctx.beginPath();
    pts.forEach(([x, y], i) => (i === 0 ? ctx.moveTo(px(x), py(y)) : ctx.lineTo(px(x), py(y))));
    ctx.closePath();
    ctx.setLineDash(dash);
    ctx.globalAlpha = alpha;
    ctx.stroke();
  };
  poly(g.p3_gamut, [4, 3], 0.6);
  poly(g.rec709_gamut, [], 0.85);
  ctx.setLineDash([]);
  ctx.globalAlpha = 0.9;
  const a = (g.skin_tone_angle_deg * Math.PI) / 180;
  ctx.beginPath();
  ctx.moveTo(px(0.5), py(0.5));
  ctx.lineTo(px(0.5 + Math.cos(a) * 0.5), py(0.5 - Math.sin(a) * 0.5));
  ctx.stroke();
  ctx.font = "10px Archivo, system-ui, sans-serif";
  ctx.fillStyle = dim;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  for (const t of g.targets) {
    ctx.globalAlpha = 0.8;
    ctx.strokeRect(px(t.x) - 6, py(t.y) - 6, 12, 12);
    ctx.globalAlpha = 0.9;
    ctx.fillText(t.label, px(t.x), py(t.y) + (t.y < 0.5 ? 15 : -15));
  }
  ctx.restore();
}

function drawHistogram(ctx: CanvasRenderingContext2D, h: Histogram, w: number, ht: number): void {
  const series: [number[], string][] = [
    [h.red, "rgba(232,96,90,0.75)"],
    [h.green, "rgba(63,181,122,0.75)"],
    [h.blue, "rgba(91,155,232,0.75)"],
  ];
  const all = [...h.red, ...h.green, ...h.blue].sort((a, b) => a - b);
  const cap = Math.max(1, all[Math.floor(all.length * 0.995)] ?? 1);
  const scale = (v: number): number => Math.sqrt(Math.min(v, cap) / cap) * (ht - 6);
  ctx.save();
  ctx.globalCompositeOperation = "lighter";
  for (const [bins, color] of series) {
    ctx.fillStyle = color;
    ctx.beginPath();
    ctx.moveTo(0, ht);
    bins.forEach((v, i) => ctx.lineTo((i / (bins.length - 1)) * w, ht - scale(v)));
    ctx.lineTo(w, ht);
    ctx.closePath();
    ctx.globalAlpha = 0.55;
    ctx.fill();
  }
  ctx.globalCompositeOperation = "source-over";
  ctx.globalAlpha = 1;
  ctx.strokeStyle = SCOPE_TEXT;
  ctx.lineWidth = 1;
  ctx.beginPath();
  h.luma.forEach((v, i) => {
    const x = (i / (h.luma.length - 1)) * w;
    const y = ht - scale(v);
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  });
  ctx.stroke();
  ctx.restore();
}

/** A canvas that redraws when its size or `draw` dependencies change. */
function ScopeCanvas(props: {
  draw: (ctx: CanvasRenderingContext2D, w: number, h: number) => void;
  label: string;
  class?: string;
}): JSX.Element {
  let canvas!: HTMLCanvasElement;
  let wrap!: HTMLDivElement;
  const redraw = (): void => {
    const dpr = Math.max(1, window.devicePixelRatio || 1);
    const w = wrap.clientWidth;
    const h = wrap.clientHeight;
    if (w < 2 || h < 2) return;
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(h * dpr);
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);
    props.draw(ctx, w, h);
  };
  createEffect(redraw);
  onMount(() => {
    const ro = new ResizeObserver(redraw);
    ro.observe(wrap);
    onCleanup(() => ro.disconnect());
  });
  return (
    <div class={`scope-canvas ${props.class ?? ""}`} ref={wrap}>
      <canvas ref={canvas} role="img" aria-label={props.label} />
    </div>
  );
}

function LegalBadge(props: { legal: LegalReport | undefined }): JSX.Element {
  const issue = (l: LegalReport): string => {
    const parts: string[] = [];
    if (l.luma_over_pct >= 0.1) parts.push(`${l.luma_over_pct.toFixed(1)}% over 100 IRE`);
    if (l.luma_under_pct >= 0.1) parts.push(`${l.luma_under_pct.toFixed(1)}% under 0 IRE`);
    if (l.gamut_excursion_pct >= 0.1)
      parts.push(`${l.gamut_excursion_pct.toFixed(1)}% out of gamut`);
    return parts.join(", ");
  };
  return (
    <Show when={props.legal} keyed>
      {(l) => (
        <span
          class="legal"
          classList={{ "legal-ok": l.legal, "legal-warn": !l.legal }}
          role="status"
        >
          {l.legal ? <CheckIcon size={14} /> : <WarnIcon size={14} />}
          {l.legal ? "Broadcast legal" : `Not broadcast legal: ${issue(l)}`}
        </span>
      )}
    </Show>
  );
}

/**
 * Scopes panel: Waveform / Parade / Vectorscope / Histogram tabs over a
 * 256 px scope area, with IRE lines, the vectorscope graticule from the core
 * and a broadcast-legal badge. In Quick mode (`histogramOnly`) it shows the
 * histogram alone.
 *
 * @param props.histogramOnly - Show only the histogram, without tabs (Quick mode).
 */
export function ScopesPanel(props: { histogramOnly?: boolean }): JSX.Element {
  const tab = (): ScopeTab => (props.histogramOnly ? "histogram" : ui.scopeTab());
  const kinds = createMemo(() => (props.histogramOnly ? ["histogram"] : [tab(), "legal"]));
  createEffect(() => {
    const dispose = requestScopes(kinds());
    onCleanup(dispose);
  });
  const data = scopeData;

  return (
    <section
      class="scopes"
      classList={{ "scopes-mini": props.histogramOnly === true }}
      aria-label="Scopes"
    >
      <Show when={!props.histogramOnly}>
        <Segmented<ScopeTab>
          class="scope-tabs"
          label="Scope type"
          value={ui.scopeTab()}
          onChange={ui.setScopeTab}
          options={TABS.map((t) => ({ value: t.value, label: t.label }))}
        />
      </Show>
      <div class="scope-area">
        <For each={[tab()]}>
          {(t) => (
            <>
              <Show when={t === "waveform" || t === "parade"}>
                <ScopeCanvas
                  label={t === "waveform" ? "Luma waveform" : "RGB parade"}
                  draw={(ctx, w, h) => {
                    const r = t === "waveform" ? data()?.waveform : data()?.parade;
                    if (r) drawRaster(ctx, r, w, h, false);
                    drawIreLines(ctx, w, h);
                  }}
                />
              </Show>
              <Show when={t === "vectorscope"}>
                <ScopeCanvas
                  label="Vectorscope"
                  draw={(ctx, w, h) => {
                    const r = data()?.vectorscope;
                    const rect = r
                      ? drawRaster(ctx, r, w, h, true)
                      : {
                          x: (w - Math.min(w, h)) / 2,
                          y: (h - Math.min(w, h)) / 2,
                          w: Math.min(w, h),
                          h: Math.min(w, h),
                        };
                    drawVectorGraticule(ctx, rect);
                  }}
                />
              </Show>
              <Show when={t === "histogram"}>
                <ScopeCanvas
                  label="Histogram"
                  draw={(ctx, w, h) => {
                    const hist = data()?.histogram;
                    if (hist) drawHistogram(ctx, hist, w, h);
                  }}
                />
              </Show>
            </>
          )}
        </For>
      </div>
      <Show when={!props.histogramOnly}>
        <LegalBadge legal={data()?.legal} />
      </Show>
    </section>
  );
}
