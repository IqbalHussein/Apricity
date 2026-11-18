import { createSignal } from "solid-js";

const [marked, setMarked] = createSignal<ReadonlySet<string>>(new Set<string>());
const [animation, setAnimation] = createSignal<{
  token: number;
  before: Readonly<Record<string, number>>;
} | null>(null);
let animationToken = 0;

/**
 * Paths of controls set by the last AI result and not yet touched by the user.
 *
 * @returns A set of dotted parameter paths.
 */
export const aiPaths = marked;

/**
 * The before-values of the latest AI apply, used to animate slider thumbs.
 *
 * @returns Token (changes on every apply) and before-values by path, in each parameter's own unit.
 */
export const aiAnimation = animation;

/**
 * Whether a control currently carries the AI-set ring.
 *
 * @param path - Dotted parameter path.
 * @returns `true` when the AI set it and the user has not edited it since.
 */
export function isAiSet(path: string): boolean {
  return marked().has(path);
}

/**
 * Record an AI apply.
 *
 * @param paths - Changed parameter paths.
 * @param before - Values before the change, by path (own units); numeric paths only.
 * @param replace - `true` to drop earlier marks (a fresh grade), `false` to add.
 */
export function markAi(
  paths: readonly string[],
  before: Readonly<Record<string, number>>,
  replace: boolean,
): void {
  setMarked((prev) => new Set<string>([...(replace ? [] : prev), ...paths]));
  animationToken += 1;
  setAnimation({ token: animationToken, before });
}

/**
 * Remove the ring from controls the user edited.
 *
 * @param paths - Edited parameter paths.
 */
export function clearAi(paths: readonly string[]): void {
  const cur = marked();
  if (!paths.some((p) => cur.has(p))) return;
  setMarked((prev) => new Set<string>([...prev].filter((p) => !paths.includes(p))));
}

/** Remove every AI ring (new image, undo, redo). */
export function clearAllAi(): void {
  if (marked().size > 0) setMarked(new Set<string>());
  setAnimation(null);
}
