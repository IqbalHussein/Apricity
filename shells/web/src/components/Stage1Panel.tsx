import { For, Show } from "solid-js";
import type { JSX } from "solid-js";
import type { Illuminant, LogProfile } from "../core/wasm";
import { reanalyze } from "../engine/bridge";
import { getPath } from "../lib/paramPath";
import { gradeParams, setField, setInputProfile } from "../state/grade";
import { stage1Analysis } from "../state/project";
import { getLogProfiles, specFor } from "../state/specs";
import { ui } from "../state/ui";
import { ChevronIcon } from "./Icons";
import { SliderRow } from "./Slider";

const ILLUMINANT: Record<Illuminant, string> = {
  tungsten: "tungsten",
  warm_mixed: "warm mixed light",
  daylight: "daylight",
  shade: "shade",
  unknown: "unclear light",
};

interface AutoGroup {
  /** Heading. */
  title: string;
  /** `CorrectionParameters` flag name. */
  flag: "auto_exposure" | "auto_white_balance" | "auto_black_point";
  /** Undo label for the toggle. */
  label: string;
  /** Scalar paths shown in the group. */
  paths: string[];
}

const GROUPS: AutoGroup[] = [
  {
    title: "Exposure",
    flag: "auto_exposure",
    label: "Auto exposure",
    paths: ["stage1.exposure", "stage1.exposure_target"],
  },
  {
    title: "White balance",
    flag: "auto_white_balance",
    label: "Auto white balance",
    paths: ["stage1.temperature", "stage1.tint"],
  },
  {
    title: "Black point",
    flag: "auto_black_point",
    label: "Auto black point",
    paths: ["stage1.black_point.r", "stage1.black_point.g", "stage1.black_point.b"],
  },
];

function Toggle(props: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
  text: string;
}): JSX.Element {
  return (
    <button
      type="button"
      role="switch"
      class="switch"
      aria-checked={props.checked}
      aria-label={props.label}
      onClick={() => props.onChange(!props.checked)}
    >
      <span class="switch-track">
        <span class="switch-knob" />
      </span>
      <span class="switch-text">{props.text}</span>
    </button>
  );
}

/**
 * Stage 1 (Correction) panel: input profile, per-group Auto toggles that
 * re-run the analysis, the detected light, and the Correction sliders.
 * Compact and technical by design (ui-spec principle 3).
 */
export function Stage1Panel(): JSX.Element {
  const flag = (f: AutoGroup["flag"]): boolean => getPath(gradeParams(), `stage1.${f}`) === true;
  const profile = (): LogProfile =>
    (getPath(gradeParams(), "stage1.input_profile") as LogProfile | undefined) ?? "none";
  const detected = (): string | null => {
    const a = stage1Analysis();
    if (!a) return null;
    const k = Math.round(a.estimated_kelvin / 100) * 100;
    return a.illuminant === "unknown"
      ? "Detected: light source unclear"
      : `Detected: ${ILLUMINANT[a.illuminant]}, ≈${k} K`;
  };
  return (
    <section class="stage1" aria-labelledby="stage1-title">
      <h2 class="stage-title" id="stage1-title">
        <button
          type="button"
          class="section-toggle stage-toggle"
          aria-expanded={ui.stage1Open()}
          onClick={() => ui.setStage1Open(!ui.stage1Open())}
        >
          <span class="chev" classList={{ open: ui.stage1Open() }}>
            <ChevronIcon size={12} />
          </span>
          Correction
        </button>
      </h2>
      <div class="s1-body" hidden={!ui.stage1Open()}>
        <div class="s1-row">
          <label class="slabel" for="input-profile">
            Input profile
          </label>
          <select
            id="input-profile"
            class="select"
            value={profile()}
            onChange={(e) => {
              setInputProfile(e.currentTarget.value as LogProfile);
              if (GROUPS.some((g) => flag(g.flag))) reanalyze("Auto correction");
            }}
          >
            <For each={getLogProfiles()}>{(p) => <option value={p.id}>{p.label}</option>}</For>
          </select>
        </div>
        <Show when={profile() !== "none"}>
          <div class="s1-row">
            <Toggle
              checked={getPath(gradeParams(), "stage1.log_to_display") === true}
              onChange={(v) => setField("stage1.log_to_display", v, "Log to display")}
              label="Log to display"
              text="Log to display"
            />
          </div>
        </Show>
        <Show when={detected()}>
          <p class="detected" aria-live="polite">
            {detected()}
          </p>
        </Show>
        <For each={GROUPS}>
          {(g) => (
            <div class="s1-group">
              <div class="s1-group-head">
                <span class="s1-group-title">{g.title}</span>
                <Toggle
                  checked={flag(g.flag)}
                  label={`${g.label}`}
                  text="Auto"
                  onChange={(v) => {
                    setField(`stage1.${g.flag}`, v, g.label);
                    if (v) reanalyze(g.label);
                  }}
                />
              </div>
              <For each={g.paths}>
                {(p) => {
                  const spec = specFor(p);
                  return (
                    <Show
                      when={
                        spec && (p !== "stage1.exposure_target" || flag("auto_exposure"))
                          ? spec
                          : undefined
                      }
                      keyed
                    >
                      {(s) => (
                        <SliderRow
                          spec={s}
                          onChange={() => {
                            if (s.path === "stage1.exposure_target" && flag("auto_exposure"))
                              reanalyze("Exposure target", "target");
                          }}
                        />
                      )}
                    </Show>
                  );
                }}
              </For>
            </div>
          )}
        </For>
      </div>
    </section>
  );
}
