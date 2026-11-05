import { createContext, useContext, useEffect, useRef, type MutableRefObject } from "react";

import type { ConsoleAction, InputSource } from "@/features/console/input";

export type ViewHandler = (action: ConsoleAction, source: InputSource) => void;

interface ViewEntry {
  handler: ViewHandler;
  enabled: boolean;
}

/** The registered views, in mount order. */
export type ViewRegistry = MutableRefObject<ViewEntry[]>;

const ViewInputContext = createContext<ViewRegistry>({ current: [] });

/** Lets the console root hand actions it does not handle itself to whichever view is on top. */
export const ViewInputProvider = ViewInputContext.Provider;

/**
 * A view registers its handler here. The most recently mounted enabled view receives the actions, so a game panel
 * sliding over the library takes the input, and gives it back the moment it is told to close (`enabled` false).
 */
export function useViewInput(handler: ViewHandler, enabled = true): void {
  const registry = useContext(ViewInputContext);
  const entry = useRef<ViewEntry>({ handler: () => undefined, enabled: false });
  // Refreshed after every render (never during one), so the registry always holds the latest closure.
  useEffect(() => {
    entry.current.handler = handler;
    entry.current.enabled = enabled;
  });
  useEffect(() => {
    const mine = entry.current;
    registry.current.push(mine);
    return () => {
      registry.current = registry.current.filter((candidate) => candidate !== mine);
    };
  }, [registry]);
}

/** Sends an action to the topmost enabled view; returns false when none took it. */
export function dispatchToViews(registry: ViewRegistry, action: ConsoleAction, source: InputSource): boolean {
  for (let index = registry.current.length - 1; index >= 0; index -= 1) {
    const entry = registry.current[index];
    if (entry?.enabled) {
      entry.handler(action, source);
      return true;
    }
  }
  return false;
}
