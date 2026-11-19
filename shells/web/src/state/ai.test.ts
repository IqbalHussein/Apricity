import { describe, expect, it } from "vitest";
import { PROMPT_HISTORY_MAX, PromptHistory } from "./ai";
import type { PromptHistoryStore } from "./ai";

describe("PromptHistory", () => {
  it("keeps only the last 20 prompts, newest first", () => {
    const h = new PromptHistory();
    for (let i = 1; i <= 30; i++) h.add(`prompt ${i}`);
    expect(h.entries()).toHaveLength(PROMPT_HISTORY_MAX);
    expect(h.entries()[0]).toBe("prompt 30");
    expect(h.entries()[19]).toBe("prompt 11");
  });

  it("moves a repeated prompt to the front without duplicating it", () => {
    const h = new PromptHistory();
    h.add("cold noir");
    h.add("warm");
    h.add("Cold Noir");
    expect(h.entries()).toEqual(["Cold Noir", "warm"]);
  });

  it("ignores blank prompts", () => {
    const h = new PromptHistory();
    h.add("   ");
    expect(h.entries()).toHaveLength(0);
  });

  it("hydrates from and saves to a pluggable store", async () => {
    const saved: (readonly string[])[] = [];
    const store: PromptHistoryStore = {
      load: () => Promise.resolve(["b", "a"]),
      save: (e) => {
        saved.push(e);
        return Promise.resolve();
      },
    };
    const h = new PromptHistory(20, store);
    await h.hydrate();
    h.add("c");
    expect(h.entries()).toEqual(["c", "b", "a"]);
    expect(saved.at(-1)).toEqual(["c", "b", "a"]);
  });
});
