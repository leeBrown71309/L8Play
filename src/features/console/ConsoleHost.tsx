import { useLingui } from "@lingui/react/macro";
import { useEffect, useRef } from "react";
import { createPortal } from "react-dom";

import { ConsoleApp } from "@/features/console/ConsoleApp";
import { useConsoleMode } from "@/features/console/console-mode-store";
import { enterFullscreen, setFullscreen } from "@/features/console/core/fullscreen";
import { useConsoleNav } from "@/features/console/core/nav-store";
import { usePresence } from "@/features/console/core/use-presence";
import { consoleSound } from "@/features/console/sound";
import { cn } from "@/lib/cn";

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
