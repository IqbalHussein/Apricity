import { createMemo, createSignal, Show, untrack } from "solid-js";
import type { JSX } from "solid-js";
import { identityParams, recipeApply } from "../core/wasm";
import type { GradeParameters } from "../core/wasm";
import { flags } from "../state/flags";
import { gradeParams } from "../state/grade";
import { currentSource, stage1Analysis } from "../state/project";
import { renderCurrentThumbnail, starterRecipes } from "../state/recipes";
import { onboardingFinished, setOnboardingDone, settingsLoaded } from "../state/settings";

interface Step {
  title: string;
  body: string;
}

const STEPS: readonly Step[] = [
  {
    title: "Correction fixes the basics",
    body: "Exposure, white balance and black point are measured from your image and corrected first. Auto does the measuring, and every value stays editable.",
  },
  {
    title: "Grade sets the mood",
    body: "Contrast, color, wheels and curves come second, on top of the corrected image. The two stay separate, so you always know which one you're changing.",
  },
  {
    title: "Describe a look, then adjust anything",
    body: "Type what you want, like cold, desaturated noir. The AI lists every change it made, and each one is a control you can move yourself.",
  },
];

function Pair(props: { a: string | null; aLabel: string; b: string | null; bLabel: string }): JSX.Element {
  return (
    <div class="onb-pair">
      <figure>
        <Show when={props.a} fallback={<span class="onb-img onb-img-empty" />}>
          {(u) => <img class="onb-img" src={u()} alt="" />}
        </Show>
        <figcaption>{props.aLabel}</figcaption>
      </figure>
      <figure>
        <Show when={props.b} fallback={<span class="onb-img onb-img-empty" />}>
          {(u) => <img class="onb-img" src={u()} alt="" />}
        </Show>
        <figcaption>{props.bLabel}</figcaption>
      </figure>
    </div>
  );
}

/**
 * First-run walkthrough of the two-stage idea (PRD §14 Q7 hypothesis, behind
 * the `onboarding` flag). It uses the user's own image when one is open,
 * is dismissible at any step, and remembers that it was shown.
 */
export function Onboarding(): JSX.Element {
  const [step, setStep] = createSignal(0);
  const visible = (): boolean =>
    flags.onboarding && settingsLoaded() && !onboardingFinished() && currentSource() !== null;

  // Thumbnails depend only on the image, not on later edits.
  const shots = createMemo(() => {
    const src = currentSource();
    if (!src || !visible() || !stage1Analysis()) return null;
    const id = identityParams();
    const current: GradeParameters = untrack(gradeParams);
    const corrected: GradeParameters = { ...current, stage2: id.stage2 ?? {} };
    const moody = starterRecipes().find((r) => r.name === "Teal & orange");
    const noir = starterRecipes().find((r) => r.name === "Noir");
    return {
      original: renderCurrentThumbnail(id),
      corrected: renderCurrentThumbnail(corrected),
      graded: moody ? renderCurrentThumbnail(recipeApply(moody, current)) : null,
      noir: noir ? renderCurrentThumbnail(recipeApply(noir, current)) : null,
    };
  });

  const finish = (): void => setOnboardingDone(true);
  const last = (): boolean => step() === STEPS.length - 1;
  const s = (): Step => STEPS[step()] as Step;

  return (
    <Show when={visible()}>
      <section class="onboarding" role="region" aria-label="Welcome" data-testid="onboarding">
        <p class="onb-step" data-testid="onboarding-step">
          Step {step() + 1} of {STEPS.length}
        </p>
        <h2 class="onb-title">{s().title}</h2>
        <Show when={shots()}>
          {(sh) => (
            <>
              <Show when={step() === 0}>
                <Pair a={sh().original} aLabel="Original" b={sh().corrected} bLabel="Corrected" />
              </Show>
              <Show when={step() === 1}>
                <Pair a={sh().corrected} aLabel="Corrected" b={sh().graded} bLabel="Graded" />
              </Show>
              <Show when={step() === 2}>
                <p class="onb-prompt" aria-hidden="true">
                  cold, desaturated noir
                </p>
                <Pair a={sh().corrected} aLabel="Before" b={sh().noir} bLabel="After" />
              </Show>
            </>
          )}
        </Show>
        <p class="onb-body">{s().body}</p>
        <div class="onb-actions">
          <button type="button" class="link-btn" data-testid="onboarding-dismiss" onClick={finish}>
            Skip
          </button>
          <span class="onb-spacer" />
          <Show when={step() > 0}>
            <button type="button" class="btn" onClick={() => setStep(step() - 1)}>
              Back
            </button>
          </Show>
          <button
            type="button"
            class="btn btn-primary"
            data-testid="onboarding-next"
            onClick={() => (last() ? finish() : setStep(step() + 1))}
          >
            {last() ? "Start grading" : "Next"}
          </button>
        </div>
      </section>
    </Show>
  );
}
