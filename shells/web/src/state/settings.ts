import { createSignal } from "solid-js";
import { SettingsStore, storage } from "../storage/idb";

const [onboardingDone, setOnboardingDoneSignal] = createSignal(false);
const [loaded, setLoaded] = createSignal(false);

/** Whether the user finished or dismissed the onboarding (tracked). */
export const onboardingFinished = onboardingDone;
/** `true` once stored settings were read (tracked); the onboarding waits for it. */
export const settingsLoaded = loaded;

/** Load persisted settings. Never throws; on failure defaults apply. */
export async function hydrateSettings(): Promise<void> {
  try {
    const s = await new SettingsStore(await storage()).load();
    setOnboardingDoneSignal(s.onboardingDone === true);
  } catch {
    // defaults apply for this session
  } finally {
    setLoaded(true);
  }
}

/**
 * Remember that the onboarding is done.
 *
 * @param done - `true` once finished or dismissed.
 */
export function setOnboardingDone(done: boolean): void {
  setOnboardingDoneSignal(done);
  void storage()
    .then((b) => new SettingsStore(b).set("onboardingDone", done))
    .catch(() => undefined);
}
