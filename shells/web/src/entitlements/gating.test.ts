import { beforeEach, describe, expect, it } from "vitest";
import { manualClock, setClock } from "../state/clock";
import { BETA_NOTICE, evaluate } from "./gating";
import { FREE_LIMITS, PRO_LIMITS } from "./limits";
import { BETA_ALL_PRO, betaFromSearch, LocalBetaProvider } from "./provider";
import {
  clearNotice,
  currentNotice,
  entitlements,
  proNote,
  requireFeature,
  setEntitlementProvider,
} from "./store";

const none = { usage: { imports: 0, prompts: 0 }, recipeCount: 0 };

describe("PRD 9.1 limits as data", () => {
  it("matches the Free column", () => {
    expect(FREE_LIMITS).toMatchObject({
      importsPerMonth: 50,
      promptsPerMonth: 15,
      recipes: 5,
      exportLongEdge: 2048,
      watermark: true,
      consoleGroups: ["Light", "Color"],
      scopes: ["histogram"],
      lutImport: false,
      lutExport: false,
      abGrade: false,
    });
  });
  it("leaves Pro unlimited", () => {
    expect(PRO_LIMITS.importsPerMonth).toBeNull();
    expect(PRO_LIMITS.exportLongEdge).toBeNull();
    expect(PRO_LIMITS.watermark).toBe(false);
  });
});

describe("evaluate on the Free tier", () => {
  it("blocks the 51st import and the 16th prompt, not the 50th or 15th", () => {
    expect(evaluate("import", FREE_LIMITS, { ...none, usage: { imports: 49, prompts: 0 } }).allowed).toBe(true);
    expect(evaluate("import", FREE_LIMITS, { ...none, usage: { imports: 50, prompts: 0 } }).allowed).toBe(false);
    expect(evaluate("prompt", FREE_LIMITS, { ...none, usage: { imports: 0, prompts: 14 } }).allowed).toBe(true);
    expect(evaluate("prompt", FREE_LIMITS, { ...none, usage: { imports: 0, prompts: 15 } }).allowed).toBe(false);
  });
  it("blocks the sixth recipe", () => {
    expect(evaluate("recipe_save", FREE_LIMITS, { ...none, recipeCount: 4 }).allowed).toBe(true);
    const g = evaluate("recipe_save", FREE_LIMITS, { ...none, recipeCount: 5 });
    expect(g.allowed).toBe(false);
    if (!g.allowed) expect(g.reason).toContain("5 recipes");
  });
  it("allows Light and Color only", () => {
    expect(evaluate("console:Light", FREE_LIMITS, none).allowed).toBe(true);
    expect(evaluate("console:Color", FREE_LIMITS, none).allowed).toBe(true);
    for (const g of ["Wheels", "HSL", "Curves", "Split Tone", "Effects"])
      expect(evaluate(`console:${g}`, FREE_LIMITS, none).allowed).toBe(false);
  });
  it("allows the histogram only, and no LUTs or A/B", () => {
    expect(evaluate("scope:histogram", FREE_LIMITS, none).allowed).toBe(true);
    for (const s of ["waveform", "parade", "vectorscope"])
      expect(evaluate(`scope:${s}`, FREE_LIMITS, none).allowed).toBe(false);
    for (const f of ["lut_import", "lut_export", "ab"] as const)
      expect(evaluate(f, FREE_LIMITS, none).allowed).toBe(false);
  });
});

describe("evaluate on the Pro tier", () => {
  it("allows everything however much was used", () => {
    const heavy = { usage: { imports: 9999, prompts: 9999 }, recipeCount: 9999 };
    for (const f of [
      "import",
      "prompt",
      "recipe_save",
      "lut_import",
      "lut_export",
      "ab",
      "console:Curves",
      "scope:waveform",
    ] as const)
      expect(evaluate(f, PRO_LIMITS, heavy).allowed).toBe(true);
  });
});

describe("providers", () => {
  it("is Pro for everyone while BETA_ALL_PRO is on", () => {
    expect(BETA_ALL_PRO).toBe(true);
    const p = new LocalBetaProvider();
    expect(p.tier()).toBe("pro");
    expect(p.beta).toBe(true);
  });
  it("previews the Free tier with ?beta=0 only", () => {
    expect(betaFromSearch("")).toBe(true);
    expect(betaFromSearch("?beta=0")).toBe(false);
    expect(betaFromSearch("?beta=1")).toBe(true);
  });
  it("is Free once the beta flag is off", () => {
    const p = new LocalBetaProvider(false);
    expect(p.tier()).toBe("free");
    expect(p.limits()).toBe(FREE_LIMITS);
    expect(p.beta).toBe(false);
  });
});

describe("gating through the reactive store", () => {
  beforeEach(() => {
    setClock(manualClock(Date.UTC(2026, 10, 18)));
    clearNotice();
  });

  it("keeps everything usable in the beta and says so honestly", () => {
    setEntitlementProvider(new LocalBetaProvider(true));
    expect(entitlements.gate("ab").allowed).toBe(true);
    expect(requireFeature("lut_export")).toBe(true);
    expect(currentNotice()).toBeNull();
    expect(proNote("lut_export")).toBe(BETA_NOTICE);
    expect(proNote("import")).toBeNull();
  });

  it("blocks Pro features on Free with a stated reason and counts prompts", () => {
    setEntitlementProvider(new LocalBetaProvider(false));
    expect(requireFeature("ab")).toBe(false);
    expect(currentNotice()).toBe("The A/B grade tool is a Pro feature.");
    expect(proNote("ab")).toBe("The A/B grade tool is a Pro feature.");
    for (let i = 0; i < 15; i++) entitlements.record("prompts");
    expect(entitlements.used("prompts")).toBe(15);
    expect(entitlements.gate("prompt").allowed).toBe(false);
  });
});
