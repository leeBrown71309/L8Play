import { describe, expect, it } from "vitest";

import { INITIAL_NAV, navReducer, topLayer, type NavState } from "@/features/console/core/navigation";

const at = (patch: Partial<NavState>): NavState => ({ ...INITIAL_NAV, ...patch });

describe("console navigation reducer", () => {
  it("switches sections without wrapping and closes the game panel", () => {
    const opened = navReducer(INITIAL_NAV, { type: "openGame", gameId: 7 });
    const next = navReducer(opened, { type: "switchSection", delta: 1 });
    expect(next.section).toBe("saves");
    expect(next.gameId).toBeNull();
    expect(navReducer(INITIAL_NAV, { type: "switchSection", delta: -1 })).toBe(INITIAL_NAV);
    expect(navReducer(at({ section: "settings" }), { type: "switchSection", delta: 1 }).section).toBe("settings");
  });

  it("ignores tab switches while a dialog is open", () => {
    const state = at({ modals: [{ kind: "quick-menu" }] });
    expect(navReducer(state, { type: "switchSection", delta: 1 })).toBe(state);
  });

  it("stacks dialogs and pops the top one", () => {
    let state = navReducer(INITIAL_NAV, { type: "pushModal", modal: { kind: "game-snapshots" } });
    state = navReducer(state, { type: "pushModal", modal: { kind: "confirm-restore", snapshotId: "a" } });
    expect(topLayer(state)).toBe("modal");
    expect(state.modals.map((modal) => modal.kind)).toEqual(["game-snapshots", "confirm-restore"]);
    state = navReducer(state, { type: "popModal" });
    expect(state.modals).toHaveLength(1);
    expect(navReducer(INITIAL_NAV, { type: "popModal" })).toBe(INITIAL_NAV);
  });

  it("clamps focus inside a zone and keeps zones independent", () => {
    let state = navReducer(INITIAL_NAV, { type: "moveFocus", zone: "list", delta: 5, count: 3 });
    expect(state.focus["list"]).toBe(2);
    state = navReducer(state, { type: "moveFocus", zone: "list", delta: -9, count: 3 });
    expect(state.focus["list"]).toBe(0);
    state = navReducer(state, { type: "setFocus", zone: "other", index: 4, count: 10 });
    expect(state.focus).toEqual({ list: 0, other: 4 });
    expect(navReducer(state, { type: "setFocus", zone: "other", index: 4, count: 10 })).toBe(state);
  });

  it("handles an empty list and reports the top layer", () => {
    expect(navReducer(INITIAL_NAV, { type: "moveFocus", zone: "z", delta: 1, count: 0 })).toBe(INITIAL_NAV);
    expect(topLayer(at({ gameId: 3 }))).toBe("game");
    expect(topLayer(at({ section: "cloud" }))).toBe("cloud");
  });

  it("goto closes the game panel and any dialog", () => {
    const state = at({ gameId: 2, modals: [{ kind: "quick-menu" }] });
    const next = navReducer(state, { type: "gotoSection", section: "settings" });
    expect(next).toMatchObject({ section: "settings", gameId: null, modals: [] });
  });
});
