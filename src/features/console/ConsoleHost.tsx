import { useLingui } from "@lingui/react/macro";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import { createPortal } from "react-dom";

import { ConsoleApp } from "@/features/console/ConsoleApp";
import { useConsoleMode } from "@/features/console/console-mode-store";
import { enterFullscreen, setFullscreen } from "@/features/console/core/fullscreen";
import {
  GAME_START_TIMEOUT_MS,
  restoreAfterGame,
  useConsoleGameWindow,
} from "@/features/console/core/game-window";
import { useConsoleNav } from "@/features/console/core/nav-store";
import { usePresence } from "@/features/console/core/use-presence";
import { consoleSound } from "@/features/console/sound";
import { cn } from "@/lib/cn";
import { queryKeys } from "@/lib/query-keys";
import { getSettings } from "@/lib/tauri/commands";
import { EVENTS, onBackendEvent } from "@/lib/tauri/events";

import "@/features/console/console.css";
import "@/features/console/console-library.css";
import "@/features/console/console-panels.css";

/** How long the exit animation runs before the layer is removed. */
const LEAVE_MS = 380;

/** While console mode is open the desktop interface is inert (no focus, no clicks, hidden from screen readers). */
function useDesktopInert(inert: boolean) {
  useEffect(() => {
    const root = document.getElementById("root");
    if (!root || !inert) return;
    root.setAttribute("inert", "");
    root.setAttribute("aria-hidden", "true");
    return () => {
      root.removeAttribute("inert");
      root.removeAttribute("aria-hidden");
    };
  }, [inert]);
}

/** Native fullscreen for the length of console mode; the window goes back to how it was when it closes. */
function useConsoleFullscreen(active: boolean) {
  const wasFullscreen = useRef(false);
  useEffect(() => {
    if (!active) return;
    let cancelled = false;
    void enterFullscreen().then((already) => {
      if (cancelled) void setFullscreen(already);
      else wasFullscreen.current = already;
    });
    return () => {
      cancelled = true;
      void setFullscreen(wasFullscreen.current);
    };
  }, [active]);
}

/**
 * The console steps aside while a game it launched is running (see `use-console-play`) and comes back
 * when that game exits. It also comes back if the game never appears, so the screen is never stranded.
 */
function useGameWindowHandover(active: boolean) {
  const gameId = useConsoleGameWindow((state) => state.gameId);

  useEffect(() => {
    if (!active) return undefined;
    const bringBack = () => {
      useConsoleGameWindow.getState().reset();
      consoleSound.startAmbient();
      void restoreAfterGame();
    };
    const subscription = onBackendEvent(EVENTS.gameSession, (event) => {
      if (useConsoleGameWindow.getState().onSession(event.gameId, event.running)) bringBack();
    });
    return () => void subscription.then((unlisten) => unlisten());
  }, [active]);

  useEffect(() => {
    if (gameId === null) return undefined;
    const timer = window.setTimeout(() => {
      const state = useConsoleGameWindow.getState();
      if (state.gameId !== gameId || state.started) return;
      state.reset();
      void restoreAfterGame();
      // Console mode may have been left meanwhile: the ambient sound belongs to it alone.
      if (useConsoleMode.getState().active) consoleSound.startAmbient();
    }, GAME_START_TIMEOUT_MS);
    return () => window.clearTimeout(timer);
  }, [gameId]);
}

/** Opens console mode once per run when "start in console mode" is on, after the saved settings are read. */
function useStartInConsole() {
  const { data } = useQuery({ queryKey: queryKeys.settings, queryFn: getSettings });
  const started = useRef(false);
  useEffect(() => {
    if (!data || started.current) return;
    started.current = true;
    // The first-run walkthrough must be finished before the console covers it.
    if (data.startInConsole && data.onboardingDone) useConsoleMode.getState().enter();
  }, [data]);
}

/**
 * Renders the console interface above the desktop. The desktop and the app's providers stay mounted
 * underneath, so backend events and queries keep running while a game is chosen with the pad.
 */
export function ConsoleHost() {
  const { t } = useLingui();
  const active = useConsoleMode((state) => state.active);
  const resetNav = useConsoleNav((state) => state.reset);
  const { mounted, closing } = usePresence(active, LEAVE_MS);

  useEffect(() => {
    if (!active) return undefined;
    resetNav();
    consoleSound.startAmbient();
    return () => consoleSound.stopAmbient();
  }, [active, resetNav]);

  useDesktopInert(mounted);
  useConsoleFullscreen(active);
  useGameWindowHandover(active);
  useStartInConsole();

  if (!mounted) return null;
  return createPortal(
    <div
      className={cn("cs-root", closing && "is-leaving")}
      role="dialog"
      aria-modal="true"
      aria-label={t`Console mode`}
    >
      <ConsoleApp />
    </div>,
    document.body,
  );
}
