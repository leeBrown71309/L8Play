import type { ConsoleAction, ControllerFamily } from "@/features/console/input/types";

/** Minimal view of a Gamepad, so tests can fake pads. */
export interface GamepadLike {
  index: number;
  id: string;
  connected: boolean;
  buttons: ReadonlyArray<{ pressed: boolean }>;
  axes: ReadonlyArray<number>;
  vibrationActuator?: {
    playEffect: (type: string, params: Record<string, number>) => Promise<unknown>;
  } | null;
}

/** Standard-mapping button index → action. */
export const BUTTON_ACTIONS: Readonly<Record<number, ConsoleAction>> = {
  0: "confirm",
  1: "back",
  2: "secondary",
  3: "tertiary",
  4: "prevTab",
  5: "nextTab",
  8: "search",
  9: "menu",
  12: "up",
  13: "down",
  14: "left",
  15: "right",
};

export const STICK_DEAD_ZONE = 0.55;

/** One direction from the left stick: the dominant axis only, never a diagonal. */
export function stickDirection(x: number, y: number): ConsoleAction | null {
  if (Math.max(Math.abs(x), Math.abs(y)) < STICK_DEAD_ZONE) return null;
  if (Math.abs(x) > Math.abs(y)) return x < 0 ? "left" : "right";
  return y < 0 ? "up" : "down";
}

/** Every action a pad is holding down right now (buttons, d-pad and left stick). */
export function heldActions(pad: GamepadLike): Set<ConsoleAction> {
  const held = new Set<ConsoleAction>();
  for (const [index, action] of Object.entries(BUTTON_ACTIONS)) {
    if (pad.buttons[Number(index)]?.pressed) held.add(action);
  }
  const direction = stickDirection(pad.axes[0] ?? 0, pad.axes[1] ?? 0);
  if (direction) held.add(direction);
  return held;
}

export function controllerFamily(id: string): ControllerFamily {
  const text = id.toLowerCase();
  if (text.includes("xbox") || text.includes("045e")) return "xbox";
  if (
    text.includes("054c") ||
    text.includes("dualshock") ||
    text.includes("dualsense") ||
    text.includes("playstation") ||
    text.includes("wireless controller")
  ) {
    return "playstation";
  }
  return "generic";
}

/** "Xbox 360 Controller (XInput STANDARD GAMEPAD)" → "Xbox 360 Controller". */
export function controllerName(id: string): string {
  const name = id.replace(/\s*\(.*\)\s*$/, "").trim();
  return name === "" ? id : name;
}
