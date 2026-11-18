import { describe, expect, it } from "vitest";
import { resolveFlags } from "./flags";

describe("resolveFlags", () => {
  it("defaults the onboarding on", () => {
    expect(resolveFlags("").onboarding).toBe(true);
  });
  it("lets the query string turn it off or on", () => {
    expect(resolveFlags("?onboarding=0").onboarding).toBe(false);
    expect(resolveFlags("?onboarding=1").onboarding).toBe(true);
  });
});
