import { useEffect } from "react";

import { useConsoleMode } from "@/features/console/console-mode-store";
import { toggleFullscreen } from "@/features/console/core/fullscreen";

/** Ctrl+Shift+Enter opens or closes console mode from anywhere; F11 toggles fullscreen while it is open. */
export function useConsoleShortcut(): void {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.ctrlKey && event.shiftKey && event.key === "Enter") {
        event.preventDefault();
        useConsoleMode.getState().toggle();
      } else if (event.key === "F11" && useConsoleMode.getState().active) {
        event.preventDefault();
        void toggleFullscreen();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);
}
