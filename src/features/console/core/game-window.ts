import { create } from "zustand";

import { isTauri } from "@/lib/tauri/runtime";

/** How long to wait for the game to show up before giving the console back (launcher stuck, game crashed). */
export const GAME_START_TIMEOUT_MS = 90_000;

interface GameWindowState {
  /** The game the console launched and is waiting on; null when the console is in front. */
  gameId: number | null;
  /** True once the backend saw the game's process, so its exit means "bring the console back". */
  started: boolean;
  /** Called after a successful launch: the console steps aside for this game. */
  handOver: (gameId: number) => void;
  /** The backend saw the game start (`true`) or exit (`false`). Returns true when the console must come back. */
  onSession: (gameId: number, running: boolean) => boolean;
  /** Called when the console is back in front, for any reason. */
  reset: () => void;
}

export const useConsoleGameWindow = create<GameWindowState>((set, get) => ({
  gameId: null,
  started: false,
  handOver: (gameId) => set({ gameId, started: false }),
  onSession: (gameId, running) => {
    const state = get();
    if (state.gameId !== gameId) return false;
    if (running) {
      set({ started: true });
      return false;
    }
    return state.started;
  },
  reset: () => set({ gameId: null, started: false }),
}));

async function currentWindow() {
  if (!isTauri()) return null;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return getCurrentWindow();
}

/** Minimizes the window so a running game has the screen and all the GPU to itself. */
export async function minimizeForGame(): Promise<void> {
  try {
    await (await currentWindow())?.minimize();
  } catch (error) {
    console.warn("The window could not be minimized.", error);
  }
}

/** Brings the window back, focused, in the state it was minimized from (still fullscreen console mode). */
export async function restoreAfterGame(): Promise<void> {
  try {
    const window = await currentWindow();
    if (!window) return;
    await window.show();
    await window.unminimize();
    await window.setFocus();
  } catch (error) {
    console.warn("The window could not be restored.", error);
  }
}
