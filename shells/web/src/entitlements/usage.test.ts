import { describe, expect, it } from "vitest";
import { manualClock } from "../state/clock";
import { monthKey, UsageCounter } from "./usage";

describe("monthKey", () => {
  it("uses the UTC calendar month", () => {
    expect(monthKey(Date.UTC(2026, 0, 31, 23, 59, 59))).toBe("2026-01");
    expect(monthKey(Date.UTC(2026, 1, 1, 0, 0, 0))).toBe("2026-02");
    expect(monthKey(Date.UTC(2026, 11, 31, 23, 59))).toBe("2026-12");
  });
});

describe("UsageCounter", () => {
  it("counts within a month and rolls over to zero in the next", () => {
    const clock = manualClock(Date.UTC(2026, 10, 30, 12));
    const c = new UsageCounter(clock.nowMs());
    c.increment(clock.nowMs(), "imports");
    c.increment(clock.nowMs(), "imports");
    c.increment(clock.nowMs(), "prompts");
    expect(c.count(clock.nowMs(), "imports")).toBe(2);
    expect(c.count(clock.nowMs(), "prompts")).toBe(1);

    clock.advance(24 * 3600 * 1000); // 1 December
    expect(c.count(clock.nowMs(), "imports")).toBe(0);
    const next = c.increment(clock.nowMs(), "imports");
    expect(next).toEqual({ month: "2026-12", imports: 1, prompts: 0 });
  });

  it("ignores a saved record from an earlier month", () => {
    const c = new UsageCounter(Date.UTC(2026, 5, 1), { month: "2026-05", imports: 49, prompts: 14 });
    expect(c.count(Date.UTC(2026, 5, 1), "imports")).toBe(0);
  });

  it("resumes a saved record from the same month", () => {
    const c = new UsageCounter(Date.UTC(2026, 4, 20), { month: "2026-05", imports: 49, prompts: 14 });
    expect(c.count(Date.UTC(2026, 4, 20), "imports")).toBe(49);
    expect(c.increment(Date.UTC(2026, 4, 20), "imports").imports).toBe(50);
  });
});
