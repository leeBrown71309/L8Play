import { describe, expect, it } from "vitest";

import { hintsFor, type HintContext } from "@/features/console/core/hint-model";

const CONTEXTS: HintContext[] = ["library", "library-filters", "game", "list", "settings", "dialog", "letters"];

describe("hint bar model", () => {
  it("always offers a way to confirm and a way back", () => {
    for (const context of CONTEXTS) {
      const hints = hintsFor(context);
      expect(
        hints.some((hint) => hint.actions.includes("confirm")),
        context,
      ).toBe(true);
      expect(
        hints.some((hint) => hint.actions.includes("back")),
        context,
      ).toBe(true);
    }
  });

  it("calls the back button 'exit' at the library root and 'back' elsewhere", () => {
    expect(hintsFor("library").find((hint) => hint.actions.includes("back"))?.intent).toBe("exit");
    expect(hintsFor("game").find((hint) => hint.actions.includes("back"))?.intent).toBe("back");
  });

  it("shows the tab pair only where tabs can change", () => {
    expect(hintsFor("library").some((hint) => hint.intent === "tabs")).toBe(true);
    expect(hintsFor("dialog").some((hint) => hint.intent === "tabs")).toBe(false);
    expect(hintsFor("game").some((hint) => hint.intent === "tabs")).toBe(false);
  });

  it("settings explains left and right and the letter picker explains clearing", () => {
    expect(hintsFor("settings")[0]).toEqual({ actions: ["left", "right"], intent: "adjust" });
    expect(hintsFor("letters").some((hint) => hint.intent === "clear")).toBe(true);
  });
});
