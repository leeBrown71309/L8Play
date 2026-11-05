import type { ConsoleAction } from "@/features/console/input";

/** What a button does on the current screen; the hint bar turns it into a translated word. */
export type HintIntent =
  "select" | "open" | "play" | "back" | "exit" | "tabs" | "menu" | "search" | "adjust" | "clear" | "filters";

export interface Hint {
  /** Actions drawn as glyphs (two for a pair such as the tab buttons). */
  actions: readonly ConsoleAction[];
  intent: HintIntent;
}

/** Where the player is; decides which hints the bottom bar shows. */
export type HintContext = "library" | "library-filters" | "game" | "list" | "settings" | "dialog" | "letters";

const TABS: Hint = { actions: ["prevTab", "nextTab"], intent: "tabs" };
const MENU: Hint = { actions: ["menu"], intent: "menu" };

/** Hints for a screen, most important first. */
export function hintsFor(context: HintContext): Hint[] {
  switch (context) {
    case "library":
      return [
        { actions: ["confirm"], intent: "open" },
        { actions: ["secondary"], intent: "play" },
        { actions: ["search"], intent: "search" },
        TABS,
        { actions: ["back"], intent: "exit" },
        MENU,
      ];
    case "library-filters":
      return [{ actions: ["confirm"], intent: "select" }, TABS, { actions: ["back"], intent: "back" }, MENU];
    case "game":
      return [{ actions: ["confirm"], intent: "select" }, { actions: ["back"], intent: "back" }, MENU];
    case "settings":
      return [
        { actions: ["left", "right"], intent: "adjust" },
        { actions: ["confirm"], intent: "select" },
        TABS,
        { actions: ["back"], intent: "back" },
        MENU,
      ];
    case "dialog":
      return [
        { actions: ["confirm"], intent: "select" },
        { actions: ["back"], intent: "back" },
      ];
    case "letters":
      return [
        { actions: ["confirm"], intent: "select" },
        { actions: ["secondary"], intent: "clear" },
        { actions: ["back"], intent: "back" },
      ];
    default:
      return [{ actions: ["confirm"], intent: "select" }, TABS, { actions: ["back"], intent: "back" }, MENU];
  }
}
