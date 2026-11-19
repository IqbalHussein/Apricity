import { describe, expect, it } from "vitest";
import type { GradeParameters } from "../core/wasm";
import { COALESCE_MS, HISTORY_MAX, HistoryStack } from "./history";
import { manualClock } from "./clock";

const g = (n: number): GradeParameters => ({ stage2: { contrast: n } });

describe("HistoryStack", () => {
  it("starts at the baseline with nothing to undo", () => {
    const h = new HistoryStack(g(0), manualClock());
    expect(h.canUndo).toBe(false);
    expect(h.canRedo).toBe(false);
    expect(h.previousParams()).toBeNull();
  });

  it("undoes and redoes whole steps", () => {
    const clock = manualClock();
    const h = new HistoryStack(g(0), clock);
    h.push(g(1), "A");
    clock.advance(5000);
    h.push(g(2), "B");
    expect(h.undo()?.params).toEqual(g(1));
    expect(h.undo()?.params).toEqual(g(0));
    expect(h.undo()).toBeNull();
    expect(h.redo()?.params).toEqual(g(1));
    expect(h.redoLabel).toBe("B");
  });

  it("coalesces a slider drag into one entry", () => {
    const clock = manualClock();
    const h = new HistoryStack(g(0), clock);
    for (let i = 1; i <= 30; i++) {
      h.push(g(i), "Contrast", "scalar:stage2.contrast");
      clock.advance(16);
    }
    expect(h.size).toBe(2);
    expect(h.current.params).toEqual(g(30));
    expect(h.previousParams()).toEqual(g(0));
    h.undo();
    expect(h.current.params).toEqual(g(0));
  });

  it("does not coalesce different keys or edits after the window", () => {
    const clock = manualClock();
    const h = new HistoryStack(g(0), clock);
    h.push(g(1), "a", "k1");
    h.push(g(2), "b", "k2");
    expect(h.size).toBe(3);
    clock.advance(COALESCE_MS + 1);
    h.push(g(3), "b", "k2");
    expect(h.size).toBe(4);
  });

  it("drops the redo branch on a new edit", () => {
    const clock = manualClock();
    const h = new HistoryStack(g(0), clock);
    h.push(g(1), "a");
    h.push(g(2), "b");
    h.undo();
    h.push(g(9), "c");
    expect(h.canRedo).toBe(false);
    expect(h.current.params).toEqual(g(9));
  });

  it("never merges into an entry that has a redo branch", () => {
    const clock = manualClock();
    const h = new HistoryStack(g(0), clock);
    h.push(g(1), "a", "k");
    h.push(g(2), "b", "other");
    h.undo();
    h.push(g(3), "a", "k");
    expect(h.size).toBe(3);
  });

  it("caps the stack at 200 entries, dropping the oldest", () => {
    const clock = manualClock();
    const h = new HistoryStack(g(0), clock);
    for (let i = 1; i <= HISTORY_MAX + 50; i++) {
      h.push(g(i), `s${i}`);
      clock.advance(5000);
    }
    expect(h.size).toBe(HISTORY_MAX);
    expect(h.current.params).toEqual(g(HISTORY_MAX + 50));
    let steps = 0;
    while (h.undo()) steps++;
    expect(steps).toBe(HISTORY_MAX - 1);
  });
});
