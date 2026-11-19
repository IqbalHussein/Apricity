import { onCleanup, onMount, Show } from "solid-js";
import type { JSX } from "solid-js";
import { currentSource } from "../state/project";
import { formatClock } from "../video/model";
import {
  seekTo,
  setLooping,
  setMuted,
  stepFrame,
  togglePlay,
  videoDuration,
  videoHasAudio,
  videoLooping,
  videoMuted,
  videoPlaying,
  videoTime,
} from "../video/player";
import {
  FrameNextIcon,
  FramePrevIcon,
  LoopIcon,
  MutedIcon,
  PauseIcon,
  PlayIcon,
  VolumeIcon,
} from "./Icons";

function typingTarget(t: EventTarget | null): boolean {
  const el = t as HTMLElement | null;
  if (!el) return false;
  return (
    el.tagName === "INPUT" ||
    el.tagName === "TEXTAREA" ||
    el.tagName === "SELECT" ||
    el.isContentEditable
  );
}

/**
 * Transport bar under the preview for video sources: play/pause (Space),
 * scrubber with current time and duration, frame step (comma and period),
 * loop and mute. Renders nothing for images.
 */
export function Transport(): JSX.Element {
  const onKey = (e: KeyboardEvent): void => {
    if (currentSource()?.kind !== "video") return;
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    if (document.querySelector("dialog[open]")) return;
    const t = e.target as HTMLElement | null;
    if (typingTarget(t)) return;
    if (e.key === " ") {
      // A focused button or slider already reacts to Space itself.
      if (t && (t.tagName === "BUTTON" || t.getAttribute("role") === "slider")) return;
      togglePlay();
    } else if (e.key === ",") stepFrame(-1);
    else if (e.key === ".") stepFrame(1);
    else return;
    e.preventDefault();
  };
  onMount(() => window.addEventListener("keydown", onKey));
  onCleanup(() => window.removeEventListener("keydown", onKey));

  return (
    <Show when={currentSource()?.kind === "video"}>
      <div class="transport" role="group" aria-label="Video transport" data-testid="transport">
        <button
          type="button"
          class="icon-btn"
          aria-label={videoPlaying() ? "Pause" : "Play"}
          title={videoPlaying() ? "Pause (Space)" : "Play (Space)"}
          data-testid="transport-play"
          onClick={togglePlay}
        >
          <Show when={videoPlaying()} fallback={<PlayIcon size={18} />}>
            <PauseIcon size={18} />
          </Show>
        </button>
        <button
          type="button"
          class="icon-btn"
          aria-label="Previous frame"
          title="Previous frame (,)"
          onClick={() => stepFrame(-1)}
        >
          <FramePrevIcon />
        </button>
        <button
          type="button"
          class="icon-btn"
          aria-label="Next frame"
          title="Next frame (.)"
          onClick={() => stepFrame(1)}
        >
          <FrameNextIcon />
        </button>
        <span class="transport-time" data-testid="transport-time" aria-label="Current time">
          {formatClock(videoTime(), true)}
        </span>
        <input
          class="range transport-scrub"
          type="range"
          min="0"
          max={Math.max(videoDuration(), 0.001)}
          step="any"
          value={videoTime()}
          aria-label="Seek"
          aria-valuetext={`${formatClock(videoTime(), true)} of ${formatClock(videoDuration(), true)}`}
          data-testid="transport-scrub"
          onInput={(e) => seekTo(Number(e.currentTarget.value))}
        />
        <span class="transport-time transport-dur" data-testid="transport-duration" aria-label="Duration">
          {formatClock(videoDuration(), true)}
        </span>
        <button
          type="button"
          class="icon-btn"
          classList={{ "is-on": videoLooping() }}
          aria-label="Loop"
          aria-pressed={videoLooping()}
          title="Loop"
          data-testid="transport-loop"
          onClick={() => setLooping(!videoLooping())}
        >
          <LoopIcon />
        </button>
        <button
          type="button"
          class="icon-btn"
          aria-label={videoMuted() ? "Unmute" : "Mute"}
          aria-pressed={videoMuted()}
          title={videoHasAudio() ? (videoMuted() ? "Unmute" : "Mute") : "This clip has no audio"}
          disabled={!videoHasAudio()}
          data-testid="transport-mute"
          onClick={() => setMuted(!videoMuted())}
        >
          <Show when={videoMuted()} fallback={<VolumeIcon />}>
            <MutedIcon />
          </Show>
        </button>
      </div>
    </Show>
  );
}
