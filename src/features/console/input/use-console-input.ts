import { useEffect, useRef, useSyncExternalStore } from "react";

import type { GamepadLike } from "@/features/console/input/gamepad-map";
import {
  createInputEngine,
  type InputEngine,
  type InputMount,
  type RumbleOptions,
} from "@/features/console/input/input-engine";
import type { ConsoleActionHandler, GamepadStatus, InputSource } from "@/features/console/input/types";

let sharedEngine: InputEngine | null = null;

/** The app-wide engine, created on first use so importing this module never touches the DOM. */
export function getInputEngine(): InputEngine {
  sharedEngine ??= createInputEngine({
    getGamepads: () =>
      typeof navigator !== "undefined" && "getGamepads" in navigator
        ? (navigator.getGamepads() as unknown as (GamepadLike | null)[])
        : [],
    now: () => performance.now(),
    requestFrame: (callback) => window.requestAnimationFrame(callback),
    cancelFrame: (id) => window.cancelAnimationFrame(id),
    windowTarget: window,
    documentTarget: document,
    isHidden: () => document.hidden,
  });
  return sharedEngine;
}

/**
 * Receives console actions while `enabled`. When several receivers are enabled only the one that became
 * enabled last gets them, so a dialog opened over a page captures the input and hands it back on close.
 */
export function useConsoleInput(handler: ConsoleActionHandler, options?: { enabled?: boolean }): void {
  const enabled = options?.enabled ?? true;
  const handlerRef = useRef(handler);
  const initialEnabled = useRef(enabled);
  const mountRef = useRef<InputMount | null>(null);

  useEffect(() => {
    handlerRef.current = handler;
  });

  useEffect(() => {
    const mount = getInputEngine().register(
      (action, source) => handlerRef.current(action, source),
      initialEnabled.current,
    );
    mountRef.current = mount;
    return () => {
      mount.dispose();
      mountRef.current = null;
    };
  }, []);

  useEffect(() => {
    mountRef.current?.setEnabled(enabled);
  }, [enabled]);
}

/** The device used last: drives which button glyphs the hint bar shows. */
export function useInputSource(): InputSource {
  const engine = getInputEngine();
  return useSyncExternalStore(engine.subscribeSource, engine.getSource, () => "keyboard");
}

const SERVER_STATUS: GamepadStatus = { connected: false, name: null, family: "generic" };

export function useGamepadStatus(): GamepadStatus {
  const engine = getInputEngine();
  return useSyncExternalStore(engine.subscribeStatus, engine.getStatus, () => SERVER_STATUS);
}

/** Short vibration on the connected pad; silent when there is none. */
export function rumble(options?: RumbleOptions): void {
  getInputEngine().rumble(options);
}
