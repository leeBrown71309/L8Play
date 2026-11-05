import type { ConsoleAction } from "@/features/console/input/types";

export interface KeyLike {
  key: string;
  shiftKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  metaKey: boolean;
}

const KEY_ACTIONS: Record<string, ConsoleAction> = {
  arrowup: "up",
  w: "up",
  arrowdown: "down",
  s: "down",
  arrowleft: "left",
  a: "left",
  arrowright: "right",
  d: "right",
  enter: "confirm",
  " ": "confirm",
  escape: "back",
  backspace: "back",
  pagedown: "nextTab",
  e: "nextTab",
  pageup: "prevTab",
  q: "prevTab",
  f1: "menu",
  contextmenu: "menu",
  m: "menu",
  x: "secondary",
  y: "tertiary",
  "/": "search",
};

/** The console action a key press stands for, or null when the key means nothing here. */
export function mapKey(event: KeyLike): ConsoleAction | null {
  const key = event.key.toLowerCase();
  if (event.ctrlKey && key === "k") return "search";
  if (event.ctrlKey || event.altKey || event.metaKey) return null;
  if (key === "tab") return event.shiftKey ? "prevTab" : "nextTab";
  return KEY_ACTIONS[key] ?? null;
}

/** True when the key press lands in a text field, where typing must never move the console focus. */
export function isTypingTarget(target: EventTarget | null): boolean {
  const element = target as HTMLElement | null;
  if (!element || typeof element.tagName !== "string") return false;
  const tag = element.tagName.toLowerCase();
  return tag === "input" || tag === "textarea" || tag === "select" || element.isContentEditable === true;
}
