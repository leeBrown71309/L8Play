import type { ConsoleAction, ControllerFamily, InputSource } from "@/features/console/input/types";

const KEYBOARD: Record<ConsoleAction, string> = {
  up: "↑",
  down: "↓",
  left: "←",
  right: "→",
  confirm: "Enter",
  back: "Esc",
  menu: "F1",
  secondary: "X",
  tertiary: "Y",
  prevTab: "Q",
  nextTab: "E",
  search: "/",
};

const XBOX: Record<ConsoleAction, string> = {
  up: "↑",
  down: "↓",
  left: "←",
  right: "→",
  confirm: "A",
  back: "B",
  menu: "Menu",
  secondary: "X",
  tertiary: "Y",
  prevTab: "LB",
  nextTab: "RB",
  search: "View",
};

const PLAYSTATION: Record<ConsoleAction, string> = {
  ...XBOX,
  confirm: "Cross",
  back: "Circle",
  menu: "Options",
  secondary: "Square",
  tertiary: "Triangle",
  prevTab: "L1",
  nextTab: "R1",
  search: "Share",
};

/** Label of an action on the device the player is using, for the hint bar ("A", "Cross", "Enter"…). */
export function actionLabel(action: ConsoleAction, source: InputSource, family: ControllerFamily): string {
  if (source !== "gamepad") return KEYBOARD[action];
  return family === "playstation" ? PLAYSTATION[action] : XBOX[action];
}
