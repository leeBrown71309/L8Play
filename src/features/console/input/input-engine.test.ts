import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { actionLabel } from "@/features/console/input/action-labels";
import {
  controllerFamily,
  controllerName,
  stickDirection,
  type GamepadLike,
} from "@/features/console/input/gamepad-map";
import { createInputEngine, FIRST_REPEAT_MS, REPEAT_MS, type InputEngine } from "@/features/console/input/input-engine";
import { mapKey } from "@/features/console/input/key-map";
import type { ConsoleAction, InputSource } from "@/features/console/input/types";

const BUTTON_COUNT = 17;

function makePad(overrides: Partial<GamepadLike> = {}): GamepadLike {
  return {
    index: 0,
    id: "Xbox 360 Controller (XInput STANDARD GAMEPAD)",
    connected: true,
    buttons: Array.from({ length: BUTTON_COUNT }, () => ({ pressed: false })),
    axes: [0, 0, 0, 0],
    ...overrides,
  };
}

function press(pad: GamepadLike, index: number, pressed: boolean) {
  (pad.buttons as { pressed: boolean }[])[index] = { pressed };
}

interface Rig {
  engine: InputEngine;
  pads: (GamepadLike | null)[];
  clock: { now: number };
  win: EventTarget;
  doc: EventTarget;
  hidden: { value: boolean };
  frames: (() => void)[];
  /** Runs the next scheduled animation frame at the given time. */
  frame: (at?: number) => void;
}

function makeRig(): Rig {
  const pads: (GamepadLike | null)[] = [];
  const clock = { now: 0 };
  const win = new EventTarget();
  const doc = new EventTarget();
  const hidden = { value: false };
  const frames: (() => void)[] = [];
  const engine = createInputEngine({
    getGamepads: () => pads,
    now: () => clock.now,
    requestFrame: (callback) => frames.push(callback),
    cancelFrame: (id) => {
      frames[id - 1] = () => undefined;
    },
    windowTarget: win,
    documentTarget: doc,
    isHidden: () => hidden.value,
  });
  return {
    engine,
    pads,
    clock,
    win,
    doc,
    hidden,
    frames,
    frame: (at) => {
      if (at !== undefined) clock.now = at;
      frames.shift()?.();
    },
  };
}

function key(win: EventTarget, init: KeyboardEventInit, target?: HTMLElement) {
  const event = new KeyboardEvent("keydown", { cancelable: true, bubbles: true, ...init });
  if (target) Object.defineProperty(event, "target", { value: target });
  win.dispatchEvent(event);
  return event;
}

describe("gamepad polling", () => {
  let rig: Rig;
  let seen: [ConsoleAction, InputSource][];

  beforeEach(() => {
    rig = makeRig();
    seen = [];
    rig.engine.register((action, source) => seen.push([action, source]), true);
  });

  it("fires a button once when pressed and not again while it is held", () => {
    const pad = makePad();
    rig.pads.push(pad);
    rig.frame(0);
    press(pad, 0, true);
    rig.frame(16);
    rig.frame(32);
    rig.frame(5000);
    expect(seen).toEqual([["confirm", "gamepad"]]);
    press(pad, 0, false);
    rig.frame(5016);
    press(pad, 0, true);
    rig.frame(5032);
    expect(seen).toHaveLength(2);
  });

  it("maps the standard buttons", () => {
    const pad = makePad();
    rig.pads.push(pad);
    rig.frame(0);
    const expected: [number, ConsoleAction][] = [
      [1, "back"],
      [2, "secondary"],
      [3, "tertiary"],
      [4, "prevTab"],
      [5, "nextTab"],
      [8, "search"],
      [9, "menu"],
    ];
    let time = 16;
    for (const [index, action] of expected) {
      press(pad, index, true);
      rig.frame(time);
      press(pad, index, false);
      rig.frame(time + 8);
      time += 16;
      expect(seen.at(-1)?.[0]).toBe(action);
    }
  });

  it("ignores a button that was already held when polling started", () => {
    const pad = makePad();
    press(pad, 0, true);
    rig.pads.push(pad);
    rig.frame(0);
    rig.frame(16);
    expect(seen).toEqual([]);
  });

  it("repeats a held direction after 380 ms and then every 110 ms, but never a button", () => {
    const pad = makePad();
    rig.pads.push(pad);
    rig.frame(0);
    press(pad, 15, true);
    press(pad, 0, true);
    rig.frame(100);
    expect(seen.map(([action]) => action).sort()).toEqual(["confirm", "right"]);
    rig.frame(100 + FIRST_REPEAT_MS - 1);
    expect(seen).toHaveLength(2);
    rig.frame(100 + FIRST_REPEAT_MS);
    expect(seen.filter(([action]) => action === "right")).toHaveLength(2);
    rig.frame(100 + FIRST_REPEAT_MS + REPEAT_MS - 1);
    expect(seen.filter(([action]) => action === "right")).toHaveLength(2);
    rig.frame(100 + FIRST_REPEAT_MS + REPEAT_MS);
    expect(seen.filter(([action]) => action === "right")).toHaveLength(3);
    expect(seen.filter(([action]) => action === "confirm")).toHaveLength(1);
  });

  it("repeats a little faster after a second", () => {
    const pad = makePad();
    rig.pads.push(pad);
    rig.frame(0);
    press(pad, 12, true);
    rig.frame(0);
    const steps: number[] = [];
    let time = 0;
    let count = seen.length;
    while (time < 2000) {
      time += 5;
      rig.frame(time);
      if (seen.length !== count) {
        steps.push(time);
        count = seen.length;
      }
    }
    const gaps = steps.slice(1).map((value, index) => value - (steps[index] ?? 0));
    expect(Math.min(...gaps)).toBeLessThan(REPEAT_MS);
    expect(Math.min(...gaps)).toBeGreaterThanOrEqual(75);
  });

  it("uses the left stick beyond the dead zone, dominant axis only", () => {
    const pad = makePad({ axes: [0.3, -0.3, 0, 0] });
    rig.pads.push(pad);
    rig.frame(0);
    rig.frame(16);
    expect(seen).toEqual([]);
    pad.axes = [0.9, -0.6, 0, 0];
    rig.frame(32);
    expect(seen).toEqual([["right", "gamepad"]]);
    pad.axes = [0.1, -0.95, 0, 0];
    rig.frame(48);
    expect(seen.map(([action]) => action)).toEqual(["right", "up"]);
  });

  it("merges several pads", () => {
    const first = makePad({ index: 0 });
    const second = makePad({ index: 1 });
    rig.pads.push(first, second);
    rig.frame(0);
    press(second, 1, true);
    rig.frame(16);
    press(first, 2, true);
    rig.frame(32);
    expect(seen.map(([action]) => action)).toEqual(["back", "secondary"]);
  });

  it("keeps running when a handler throws", () => {
    const errors = vi.spyOn(console, "error").mockImplementation(() => undefined);
    const pad = makePad();
    rig.pads.push(pad);
    const next = makeRig();
    const calls: ConsoleAction[] = [];
    let first = true;
    next.engine.register((action) => {
      calls.push(action);
      if (first) {
        first = false;
        throw new Error("boom");
      }
    }, true);
    const other = makePad();
    next.pads.push(other);
    next.frame(0);
    press(other, 0, true);
    next.frame(16);
    press(other, 0, false);
    next.frame(32);
    press(other, 1, true);
    next.frame(48);
    expect(calls).toEqual(["confirm", "back"]);
    expect(errors).toHaveBeenCalled();
    errors.mockRestore();
  });

  it("stops polling while the document is hidden and resumes when it is visible", () => {
    const pad = makePad();
    rig.pads.push(pad);
    expect(rig.frames).toHaveLength(1);
    rig.hidden.value = true;
    rig.frame(0);
    expect(rig.frames).toHaveLength(0);
    rig.doc.dispatchEvent(new Event("visibilitychange"));
    expect(rig.frames).toHaveLength(0);
    rig.hidden.value = false;
    rig.doc.dispatchEvent(new Event("visibilitychange"));
    expect(rig.frames).toHaveLength(1);
  });
});

describe("receivers", () => {
  it("only starts the loop while an enabled receiver exists", () => {
    const rig = makeRig();
    const mount = rig.engine.register(() => undefined, false);
    expect(rig.frames).toHaveLength(0);
    mount.setEnabled(true);
    expect(rig.frames).toHaveLength(1);
    mount.setEnabled(false);
    expect(rig.frames[0]).toBeDefined();
    rig.frame(0);
    expect(rig.frames).toHaveLength(0);
    mount.dispose();
  });

  it("gives the actions to the receiver that became enabled last, and back when it leaves", () => {
    const rig = makeRig();
    const page: ConsoleAction[] = [];
    const dialog: ConsoleAction[] = [];
    rig.engine.register((action) => page.push(action), true);
    const dialogMount = rig.engine.register((action) => dialog.push(action), true);
    key(rig.win, { key: "ArrowDown" });
    dialogMount.setEnabled(false);
    key(rig.win, { key: "ArrowUp" });
    dialogMount.setEnabled(true);
    key(rig.win, { key: "Enter" });
    dialogMount.dispose();
    key(rig.win, { key: "Escape" });
    expect(dialog).toEqual(["down", "confirm"]);
    expect(page).toEqual(["up", "back"]);
  });
});

describe("keyboard", () => {
  let rig: Rig;
  let seen: [ConsoleAction, InputSource][];

  beforeEach(() => {
    rig = makeRig();
    seen = [];
    rig.engine.register((action, source) => seen.push([action, source]), true);
  });

  it("maps the documented keys and consumes them", () => {
    const cases: [KeyboardEventInit, ConsoleAction][] = [
      [{ key: "ArrowUp" }, "up"],
      [{ key: "w" }, "up"],
      [{ key: "S" }, "down"],
      [{ key: "ArrowLeft" }, "left"],
      [{ key: "d" }, "right"],
      [{ key: "Enter" }, "confirm"],
      [{ key: " " }, "confirm"],
      [{ key: "Escape" }, "back"],
      [{ key: "Backspace" }, "back"],
      [{ key: "Tab" }, "nextTab"],
      [{ key: "Tab", shiftKey: true }, "prevTab"],
      [{ key: "PageDown" }, "nextTab"],
      [{ key: "PageUp" }, "prevTab"],
      [{ key: "e" }, "nextTab"],
      [{ key: "q" }, "prevTab"],
      [{ key: "F1" }, "menu"],
      [{ key: "m" }, "menu"],
      [{ key: "x" }, "secondary"],
      [{ key: "y" }, "tertiary"],
      [{ key: "/" }, "search"],
      [{ key: "k", ctrlKey: true }, "search"],
    ];
    for (const [init, action] of cases) {
      const event = key(rig.win, init);
      expect(seen.at(-1)).toEqual([action, "keyboard"]);
      expect(event.defaultPrevented).toBe(true);
    }
  });

  it("ignores unrelated keys and modified letters", () => {
    const event = key(rig.win, { key: "z" });
    key(rig.win, { key: "w", ctrlKey: true });
    key(rig.win, { key: "ArrowDown", altKey: true });
    expect(seen).toEqual([]);
    expect(event.defaultPrevented).toBe(false);
  });

  it("ignores typing in text fields except Escape", () => {
    const input = document.createElement("input");
    key(rig.win, { key: "w" }, input);
    key(rig.win, { key: "Enter" }, input);
    key(rig.win, { key: "Backspace" }, input);
    expect(seen).toEqual([]);
    key(rig.win, { key: "Escape" }, input);
    expect(seen).toEqual([["back", "keyboard"]]);
    const area = document.createElement("textarea");
    key(rig.win, { key: "e" }, area);
    expect(seen).toHaveLength(1);
  });

  it("lets keys through untouched when no receiver is enabled", () => {
    const quiet = makeRig();
    const mount = quiet.engine.register(() => undefined, true);
    mount.setEnabled(false);
    const event = key(quiet.win, { key: "Enter" });
    expect(event.defaultPrevented).toBe(false);
  });

  it("tracks the last input source", () => {
    expect(rig.engine.getSource()).toBe("keyboard");
    rig.win.dispatchEvent(new Event("pointermove"));
    expect(rig.engine.getSource()).toBe("pointer");
    const pad = makePad();
    rig.pads.push(pad);
    rig.frame(0);
    press(pad, 0, true);
    rig.frame(16);
    expect(rig.engine.getSource()).toBe("gamepad");
    key(rig.win, { key: "Enter" });
    expect(rig.engine.getSource()).toBe("keyboard");
    rig.win.dispatchEvent(new Event("pointerdown"));
    expect(rig.engine.getSource()).toBe("pointer");
  });
});

describe("gamepad status and rumble", () => {
  it("follows connect and disconnect events and detects the controller family", () => {
    const rig = makeRig();
    const listener = vi.fn();
    rig.engine.subscribeStatus(listener);
    expect(rig.engine.getStatus()).toEqual({ connected: false, name: null, family: "generic" });
    const pad = makePad({ id: "Wireless Controller (STANDARD GAMEPAD Vendor: 054c Product: 09cc)" });
    rig.pads.push(pad);
    const connected = new Event("gamepadconnected");
    Object.defineProperty(connected, "gamepad", { value: pad });
    rig.win.dispatchEvent(connected);
    expect(rig.engine.getStatus()).toEqual({ connected: true, name: "Wireless Controller", family: "playstation" });
    expect(listener).toHaveBeenCalledTimes(1);
    pad.connected = false;
    rig.win.dispatchEvent(new Event("gamepaddisconnected"));
    expect(rig.engine.getStatus().connected).toBe(false);
    expect(listener).toHaveBeenCalledTimes(2);
  });

  it("detects a pad that was connected before the window listened", () => {
    const rig = makeRig();
    rig.pads.push(makePad());
    rig.engine.subscribeStatus(() => undefined);
    expect(rig.engine.getStatus()).toMatchObject({ connected: true, family: "xbox" });
  });

  it("rumbles through the first pad that has an actuator and never throws without one", () => {
    const rig = makeRig();
    rig.engine.subscribeStatus(() => undefined);
    expect(() => rig.engine.rumble()).not.toThrow();
    const playEffect = vi.fn().mockResolvedValue(undefined);
    rig.pads.push(makePad({ vibrationActuator: { playEffect } }));
    rig.engine.rumble({ strong: 3, weak: -1, durationMs: 50 });
    expect(playEffect).toHaveBeenCalledWith("dual-rumble", {
      startDelay: 0,
      duration: 50,
      strongMagnitude: 1,
      weakMagnitude: 0,
    });
    playEffect.mockImplementation(() => {
      throw new Error("refused");
    });
    expect(() => rig.engine.rumble()).not.toThrow();
  });
});

describe("pure helpers", () => {
  afterEach(() => vi.restoreAllMocks());

  it("labels actions for each device", () => {
    expect(actionLabel("confirm", "gamepad", "xbox")).toBe("A");
    expect(actionLabel("confirm", "gamepad", "playstation")).toBe("Cross");
    expect(actionLabel("back", "gamepad", "playstation")).toBe("Circle");
    expect(actionLabel("confirm", "gamepad", "generic")).toBe("A");
    expect(actionLabel("confirm", "keyboard", "xbox")).toBe("Enter");
    expect(actionLabel("back", "pointer", "xbox")).toBe("Esc");
    expect(actionLabel("nextTab", "gamepad", "xbox")).toBe("RB");
    expect(actionLabel("nextTab", "gamepad", "playstation")).toBe("R1");
  });

  it("classifies controllers and cleans their names", () => {
    expect(controllerFamily("Xbox One Controller")).toBe("xbox");
    expect(controllerFamily("045e-02ea-Controller")).toBe("xbox");
    expect(controllerFamily("DualSense Wireless Controller")).toBe("playstation");
    expect(controllerFamily("Generic USB Pad")).toBe("generic");
    expect(controllerName("Xbox 360 Controller (XInput STANDARD GAMEPAD)")).toBe("Xbox 360 Controller");
    expect(controllerName("(odd)")).toBe("(odd)");
  });

  it("turns stick positions into one direction", () => {
    expect(stickDirection(0.5, 0.5)).toBeNull();
    expect(stickDirection(-0.8, 0.2)).toBe("left");
    expect(stickDirection(0.2, 0.8)).toBe("down");
    expect(stickDirection(0.7, 0.7)).toBe("down");
  });

  it("maps keys without touching the DOM", () => {
    const base = { shiftKey: false, ctrlKey: false, altKey: false, metaKey: false };
    expect(mapKey({ ...base, key: "ArrowRight" })).toBe("right");
    expect(mapKey({ ...base, key: "Meta" })).toBeNull();
  });
});
