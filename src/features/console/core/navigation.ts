/** Console-mode navigation: the open section, a game panel or dialog on top of it, and each list's focus. */

export const SECTIONS = ["library", "saves", "cloud", "settings"] as const;
export type ConsoleSection = (typeof SECTIONS)[number];

export type ModalKind = "quick-menu" | "letters" | "game-snapshots" | "snapshot-detail" | "confirm-restore";

export interface ConsoleModal {
  kind: ModalKind;
  /** Snapshot id for the snapshot dialogs. */
  snapshotId?: string | undefined;
}

export interface NavState {
  section: ConsoleSection;
  /** The game whose panel is open, or null. */
  gameId: number | null;
  /** Dialog stack: the last entry is on top and owns the input. */
  modals: readonly ConsoleModal[];
  /** Focused index per list ("zone"); a missing zone is 0. */
  focus: Readonly<Record<string, number>>;
}

export type NavAction =
  | { type: "switchSection"; delta: 1 | -1 }
  | { type: "gotoSection"; section: ConsoleSection }
  | { type: "openGame"; gameId: number }
  | { type: "closeGame" }
  | { type: "pushModal"; modal: ConsoleModal }
  | { type: "popModal" }
  | { type: "setFocus"; zone: string; index: number; count: number }
  | { type: "moveFocus"; zone: string; delta: number; count: number };

export const INITIAL_NAV: NavState = { section: "library", gameId: null, modals: [], focus: {} };

export function clampIndex(index: number, count: number): number {
  return count <= 0 ? 0 : Math.min(count - 1, Math.max(0, index));
}

function withFocus(state: NavState, zone: string, index: number): NavState {
  return (state.focus[zone] ?? 0) === index ? state : { ...state, focus: { ...state.focus, [zone]: index } };
}

/** Pure reducer; returns the same object when nothing changes so subscribers stay quiet. */
export function navReducer(state: NavState, action: NavAction): NavState {
  switch (action.type) {
    case "switchSection": {
      if (state.modals.length > 0) return state;
      const next = SECTIONS[clampIndex(SECTIONS.indexOf(state.section) + action.delta, SECTIONS.length)];
      return next === undefined || (next === state.section && state.gameId === null)
        ? state
        : { ...state, section: next, gameId: null };
    }
    case "gotoSection":
      return action.section === state.section && state.gameId === null && state.modals.length === 0
        ? state
        : { ...state, section: action.section, gameId: null, modals: [] };
    case "openGame":
      return state.gameId === action.gameId ? state : { ...state, gameId: action.gameId };
    case "closeGame":
      return state.gameId === null ? state : { ...state, gameId: null };
    case "pushModal":
      return { ...state, modals: [...state.modals, action.modal] };
    case "popModal":
      return state.modals.length === 0 ? state : { ...state, modals: state.modals.slice(0, -1) };
    case "setFocus":
      return withFocus(state, action.zone, clampIndex(action.index, action.count));
    case "moveFocus": {
      const current = clampIndex(state.focus[action.zone] ?? 0, action.count);
      return withFocus(state, action.zone, clampIndex(current + action.delta, action.count));
    }
  }
}

/** What is on top of the screen right now: a dialog, a game panel, or the section itself. */
export function topLayer(state: NavState): "modal" | "game" | ConsoleSection {
  if (state.modals.length > 0) return "modal";
  return state.gameId !== null ? "game" : state.section;
}
