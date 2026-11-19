import { createSignal, onCleanup, Show } from "solid-js";
import type { JSX } from "solid-js";
import type { BakeScope } from "../core/wasm";
import { entitlements, requireFeature } from "../entitlements/store";
import { canEncode, exportImage, exportLut } from "../export/exporter";
import type { ExportResult } from "../export/exporter";
import {
  capSize,
  DEFAULT_QUALITY,
  IMAGE_FORMATS,
} from "../export/size";
import type { ImageFormat } from "../export/size";
import { downloadBlob } from "../lib/download";
import { engineBackend } from "../engine/bridge";
import { gradeParams } from "../state/grade";
import { activeLut } from "../state/lut";
import { currentSource } from "../state/project";
import { exportVideo, hasWebCodecs, resultMessage } from "../video/exportVideo";
import type { VideoExportResult, VideoProgress } from "../video/exportVideo";
import {
  CPU_VIDEO_MESSAGE,
  ExportCancelled,
  formatClock,
  NO_WEBCODECS_MESSAGE,
  videoExportSize,
} from "../video/model";
import { Dialog } from "./Dialog";
import { Segmented } from "./Segmented";
import { UpgradeNotice } from "./UpgradeNotice";

/** Controls whether the Export dialog is open. */
const [exportOpen, setExportOpen] = createSignal(false);

/** Whether the Export dialog is open (tracked). */
export const exportDialogOpen = exportOpen;
/** Open or close the Export dialog. */
export const setExportDialogOpen = setExportOpen;

type Tab = "image" | "video" | "lut";
type LutSize = "17" | "33" | "65";

function ImageSection(): JSX.Element {
  const [format, setFormat] = createSignal<ImageFormat>("jpeg");
  const [quality, setQuality] = createSignal(DEFAULT_QUALITY);
  const [busy, setBusy] = createSignal<string | null>(null);
  const [done, setDone] = createSignal<ExportResult | null>(null);
  const [failure, setFailure] = createSignal<string | null>(null);
  const webp = canEncode("webp");
  const outSize = (): { width: number; height: number } | null => {
    const s = currentSource();
    return s ? capSize(s.proxy.fullWidth, s.proxy.fullHeight, entitlements.limits().exportLongEdge) : null;
  };
  const run = async (): Promise<void> => {
    setDone(null);
    setFailure(null);
    setBusy("Preparing the export");
    try {
      setDone(await exportImage({ format: format(), quality: quality() }, setBusy));
    } catch (e) {
      setFailure(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(null);
    }
  };
  const lossy = (): boolean => IMAGE_FORMATS[format()].lossy;
  return (
    <section class="export-section" data-testid="export-image">
      <div class="field">
        <span class="field-label" id="export-format-label">
          Format
        </span>
        <Segmented<ImageFormat>
          label="Format"
          value={format()}
          onChange={setFormat}
          options={[
            { value: "jpeg", label: "JPEG" },
            { value: "png", label: "PNG" },
            {
              value: "webp",
              label: "WebP",
              disabled: !webp,
              title: webp ? undefined : "This browser can't encode WebP",
            },
          ]}
        />
      </div>
      <Show when={lossy()}>
        <label class="field">
          <span class="field-label">
            Quality <span class="field-value">{quality()}</span>
          </span>
          <input
            class="range"
            type="range"
            min="1"
            max="100"
            step="1"
            data-testid="export-quality"
            aria-label="Quality"
            value={quality()}
            onInput={(e) => setQuality(Number(e.currentTarget.value))}
          />
        </label>
      </Show>
      <Show when={outSize()}>
        {(s) => (
          <p class="dialog-note" data-testid="export-size">
            Output: {s().width} × {s().height} px
            {entitlements.limits().watermark ? ", with the Apricity mark in the corner" : ""}.
          </p>
        )}
      </Show>
      <Show when={entitlements.tier() === "free"}>
        <p class="dialog-note">
          Free exports are limited to {entitlements.limits().exportLongEdge} px on the long edge.
        </p>
      </Show>
      <Show when={entitlements.beta()}>
        <p class="dialog-note">
          Full resolution without the mark is a Pro feature. During the beta, every feature is
          unlocked.
        </p>
      </Show>
      <Show when={busy()}>
        <p class="export-progress" role="status" data-testid="export-progress">
          <span class="spinner" aria-hidden="true" />
          {busy()}
        </p>
      </Show>
      <Show when={done()}>
        {(r) => (
          <p class="export-done" role="status" data-testid="export-result">
            Saved {r().filename}, {r().width} × {r().height} px.
          </p>
        )}
      </Show>
      <Show when={failure()}>
        <p class="error-text" role="alert" data-testid="export-error">
          {failure()}
        </p>
      </Show>
      <div class="dialog-actions">
        <button
          type="button"
          class="btn btn-primary"
          data-testid="export-confirm"
          disabled={busy() !== null || !currentSource()}
          onClick={() => void run()}
        >
          Export {IMAGE_FORMATS[format()].label}
        </button>
      </div>
    </section>
  );
}

function VideoSection(): JSX.Element {
  const [progress, setProgress] = createSignal<VideoProgress | null>(null);
  const [done, setDone] = createSignal<VideoExportResult | null>(null);
  const [failure, setFailure] = createSignal<string | null>(null);
  const [running, setRunning] = createSignal(false);
  let abort: AbortController | null = null;
  onCleanup(() => abort?.abort());

  const blocked = (): string | null => {
    if (!hasWebCodecs()) return NO_WEBCODECS_MESSAGE;
    if (engineBackend() !== "webgpu") return CPU_VIDEO_MESSAGE;
    return null;
  };
  const outSize = (): { width: number; height: number } | null => {
    const s = currentSource();
    return s
      ? videoExportSize(s.proxy.fullWidth, s.proxy.fullHeight, entitlements.limits().exportLongEdge)
      : null;
  };
  const run = async (): Promise<void> => {
    const src = currentSource();
    if (!src?.file || src.kind !== "video") return;
    setDone(null);
    setFailure(null);
    setProgress(null);
    setRunning(true);
    abort = new AbortController();
    const limits = entitlements.limits();
    try {
      const result = await exportVideo({
        file: src.file,
        name: src.name,
        signal: abort.signal,
        params: gradeParams(),
        lutText: activeLut()?.text ?? null,
        longEdge: limits.exportLongEdge,
        watermark: limits.watermark,
        onProgress: setProgress,
      });
      downloadBlob(result.blob, result.filename);
      setDone(result);
    } catch (e) {
      if (e instanceof ExportCancelled) setFailure("Export cancelled. Nothing was saved.");
      else setFailure(e instanceof Error ? e.message : String(e));
    } finally {
      abort = null;
      setRunning(false);
      setProgress(null);
    }
  };
  return (
    <section class="export-section" data-testid="export-video">
      <Show when={outSize()}>
        {(s) => (
          <p class="dialog-note" data-testid="export-size">
            Output: {s().width} × {s().height} px, MP4 (H.264 where the browser can encode it, otherwise
            VP9)
            {entitlements.limits().watermark ? ", with the Apricity mark in the corner" : ""}.
            Audio is copied as it is.
          </p>
        )}
      </Show>
      <Show when={entitlements.tier() === "free"}>
        <p class="dialog-note">
          Free exports are limited to {entitlements.limits().exportLongEdge} px on the long edge.
        </p>
      </Show>
      <Show when={entitlements.beta()}>
        <p class="dialog-note">
          Full resolution without the mark is a Pro feature. During the beta, every feature is
          unlocked.
        </p>
      </Show>
      <Show when={blocked()}>
        <p class="error-text" role="alert" data-testid="export-video-blocked">
          {blocked()}
        </p>
      </Show>
      <Show when={running()}>
        <div class="export-video-progress" role="status" data-testid="export-progress">
          <progress
            class="export-meter"
            max="1"
            value={progress()?.fraction ?? 0}
            aria-label="Export progress"
          />
          <div class="export-stats">
            <span data-testid="export-frames">
              Frame {progress()?.done ?? 0} of {progress()?.total ?? 0}
            </span>
            <span data-testid="export-elapsed">
              {formatClock((progress()?.elapsedMs ?? 0) / 1000)} elapsed
            </span>
            <span data-testid="export-eta">
              {progress()?.etaMs == null
                ? "Estimating"
                : `${formatClock((progress()?.etaMs ?? 0) / 1000)} left`}
            </span>
          </div>
        </div>
      </Show>
      <Show when={done()}>
        {(r) => (
          <p class="export-done" role="status" data-testid="export-result">
            {resultMessage(r())}
          </p>
        )}
      </Show>
      <Show when={failure()}>
        <p class="error-text" role="alert" data-testid="export-error">
          {failure()}
        </p>
      </Show>
      <div class="dialog-actions">
        <Show when={running()}>
          <button
            type="button"
            class="btn"
            data-testid="export-cancel"
            onClick={() => abort?.abort()}
          >
            Cancel
          </button>
        </Show>
        <button
          type="button"
          class="btn btn-primary"
          data-testid="export-confirm"
          disabled={running() || blocked() !== null || !currentSource()}
          onClick={() => void run()}
        >
          Export MP4
        </button>
      </div>
    </section>
  );
}

function LutSectionExport(): JSX.Element {
  const [size, setSize] = createSignal<LutSize>("33");
  const [scope, setScope] = createSignal<BakeScope>("creative");
  const [done, setDone] = createSignal<string | null>(null);
  const [failure, setFailure] = createSignal<string | null>(null);
  const run = (): void => {
    setDone(null);
    setFailure(null);
    if (!requireFeature("lut_export")) return;
    try {
      setDone(exportLut({ size: Number(size()) as 17 | 33 | 65, scope: scope() }));
    } catch (e) {
      setFailure(e instanceof Error ? e.message : String(e));
    }
  };
  return (
    <section class="export-section" data-testid="export-lut">
      <div class="field">
        <span class="field-label">Size</span>
        <Segmented<LutSize>
          label="LUT size"
          value={size()}
          onChange={setSize}
          options={[
            { value: "17", label: "17" },
            { value: "33", label: "33" },
            { value: "65", label: "65" },
          ]}
        />
      </div>
      <div class="field">
        <span class="field-label">Contents</span>
        <Segmented<BakeScope>
          label="LUT contents"
          value={scope()}
          onChange={setScope}
          options={[
            { value: "creative", label: "Creative (Stage 2 only)" },
            { value: "full", label: "Full pipeline" },
          ]}
        />
      </div>
      <p class="dialog-note">
        Creative bakes the Grade only, so it works on footage you correct elsewhere. Full pipeline
        also bakes Correction.
      </p>
      <UpgradeNotice feature="lut_export" />
      <Show when={done()}>
        <p class="export-done" role="status" data-testid="export-lut-result">
          Saved {done()}.
        </p>
      </Show>
      <Show when={failure()}>
        <p class="error-text" role="alert">
          {failure()}
        </p>
      </Show>
      <div class="dialog-actions">
        <button
          type="button"
          class="btn btn-primary"
          data-testid="export-lut-confirm"
          disabled={!currentSource()}
          onClick={run}
        >
          Export LUT (.cube)
        </button>
      </div>
    </section>
  );
}

/**
 * Export dialog: image export (format, quality, full resolution capped per
 * tier, watermark on Free) for images, MP4 export with progress and Cancel for
 * video, and `.cube` LUT export for both.
 */
export function ExportDialog(): JSX.Element {
  const isVideo = (): boolean => currentSource()?.kind === "video";
  const [tab, setTab] = createSignal<Tab>(isVideo() ? "video" : "image");
  return (
    <Dialog title="Export" testId="export-dialog" onClose={() => setExportOpen(false)}>
      <Show when={currentSource()}>
        <Segmented<Tab>
          class="export-tabs"
          label="Export type"
          value={tab()}
          onChange={setTab}
          options={[
            isVideo() ? { value: "video", label: "Video" } : { value: "image", label: "Image" },
            { value: "lut", label: "LUT (.cube)" },
          ]}
        />
        <Show when={tab() === "image" && !isVideo()}>
          <ImageSection />
        </Show>
        <Show when={tab() === "video" && isVideo()}>
          <VideoSection />
        </Show>
        <Show when={tab() === "lut"}>
          <LutSectionExport />
        </Show>
      </Show>
    </Dialog>
  );
}
