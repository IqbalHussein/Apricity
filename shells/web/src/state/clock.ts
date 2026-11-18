/** Source of wall-clock time, injected so pipeline-adjacent code stays deterministic. */
export interface Clock {
  /**
   * Current time.
   *
   * @returns Milliseconds since the Unix epoch (UTC).
   */
  nowMs(): number;
}

/** Seedable random source, injected so "Surprise me" and tests are reproducible. */
export interface Rng {
  /**
   * Next uniform value.
   *
   * @returns A float in [0, 1).
   */
  next(): number;
  /**
   * Next seed for the Rust core (`ngaSurprise`, grain).
   *
   * @returns An integer in [0, 2^53).
   */
  nextSeed(): number;
}

/**
 * Deterministic clock for tests.
 *
 * @param startMs - Initial time in epoch milliseconds.
 * @returns A clock plus a function that advances it.
 */
export function manualClock(startMs = 0): Clock & { advance(ms: number): void } {
  let t = startMs;
  return {
    nowMs: () => t,
    advance: (ms: number) => {
      t += ms;
    },
  };
}

/**
 * mulberry32 generator seeded with a 32-bit integer.
 *
 * @param seed - Any integer; the same seed yields the same sequence.
 * @returns A reproducible {@link Rng}.
 */
export function seededRng(seed: number): Rng {
  let s = seed >>> 0;
  const next = (): number => {
    s = (s + 0x6d2b79f5) >>> 0;
    let t = s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  return {
    next,
    // 21 + 32 = 53 bits is the largest exact integer range of a double.
    nextSeed: () => Math.floor(next() * 2 ** 21) * 2 ** 32 + Math.floor(next() * 2 ** 32),
  };
}

/**
 * The production clock. This module is the only place allowed to read the
 * system time: every other module receives a {@link Clock}.
 */
export const systemClock: Clock = {
  // eslint-disable-next-line no-restricted-syntax -- the single sanctioned wall-clock read; everything else injects a Clock
  nowMs: () => Date.now(),
};

/**
 * RNG seeded from the platform CSPRNG. This module is the only place allowed
 * to touch `crypto`; callers receive an {@link Rng}.
 *
 * @returns A seeded {@link Rng} whose seed is unpredictable per page load.
 */
export function systemRng(): Rng {
  const buf = new Uint32Array(1);
  globalThis.crypto.getRandomValues(buf);
  return seededRng(buf[0] as number);
}

let activeClock: Clock = systemClock;
let activeRng: Rng | null = null;

/**
 * Replace the app-wide clock (tests, or a future replay tool).
 *
 * @param clock - New clock.
 */
export function setClock(clock: Clock): void {
  activeClock = clock;
}

/**
 * Replace the app-wide RNG.
 *
 * @param rng - New RNG.
 */
export function setRng(rng: Rng): void {
  activeRng = rng;
}

/**
 * The app-wide clock.
 *
 * @returns The injected clock (system clock by default).
 */
export function getClock(): Clock {
  return activeClock;
}

/**
 * The app-wide RNG, created lazily from the system CSPRNG.
 *
 * @returns The injected RNG.
 */
export function getRng(): Rng {
  activeRng ??= systemRng();
  return activeRng;
}
