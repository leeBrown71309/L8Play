/** What the console interface understands, whatever device produced it. */
export type ConsoleAction =
  | "up"
  | "down"
  | "left"
  | "right"
  | "confirm"
  | "back"
  | "menu"
  | "secondary"
  | "tertiary"
  | "prevTab"
  | "nextTab"
  | "search";

export type InputSource = "keyboard" | "gamepad" | "pointer";

export type ControllerFamily = "xbox" | "playstation" | "generic";

export interface GamepadStatus {
  connected: boolean;
  name: string | null;
  family: ControllerFamily;
}

export type ConsoleActionHandler = (action: ConsoleAction, source: InputSource) => void;

export const DIRECTION_ACTIONS: readonly ConsoleAction[] = ["up", "down", "left", "right"];
