import { controllerFamily, controllerName, heldActions, type GamepadLike } from "@/features/console/input/gamepad-map";
import { isTypingTarget, mapKey } from "@/features/console/input/key-map";
import {
  DIRECTION_ACTIONS,
  type ConsoleAction,
  type ConsoleActionHandler,
  type GamepadStatus,
  type InputSource,
} from "@/features/console/input/types";

export const FIRST_REPEAT_MS = 380;
export const REPEAT_MS = 110;
export const FAST_REPEAT_MS = 80;
/** After this long on a direction the repeat speeds up a little. */
export const FAST_REPEAT_AFTER_MS = 1000;

export interface InputEngineDeps {
  getGamepads: () => ArrayLike<GamepadLike | null>;
  now: () => number;
  requestFrame: (callback: () => void) => number;
  cancelFrame: (id: number) => void;
  /** Window-like: keydown, pointermove, pointerdown, gamepadconnected, gamepaddisconnected. */
  windowTarget: EventTarget;
  /** Document-like: visibilitychange. */
  documentTarget: EventTarget;
  isHidden: () => boolean;
}

export interface InputMount {
  setEnabled: (enabled: boolean) => void;
  dispose: () => void;
}

export interface RumbleOptions {
  strong?: number;
  weak?: number;
  durationMs?: number;
}

export interface InputEngine {
  /** Registers a receiver. Of all enabled receivers only the one that became enabled last gets actions. */
  register: (handler: ConsoleActionHandler, enabled: boolean) => InputMount;
  subscribeSource: (listener: () => void) => () => void;
  subscribeStatus: (listener: () => void) => () => void;
  getSource: () => InputSource;
  getStatus: () => GamepadStatus;
  rumble: (options?: RumbleOptions) => void;
  /** Runs one polling pass now (what each animation frame does). Exposed for tests. */
  tick: () => void;
}

interface Mount {
  handler: ConsoleActionHandler;
  enabled: boolean;
  /** Raised each time the receiver becomes enabled, so the latest one wins. */
  order: number;
}

interface HeldDirection {
  since: number;
  next: number;
}

const DISCONNECTED: GamepadStatus = { connected: false, name: null, family: "generic" };
const clamp01 = (value: number) => Math.min(1, Math.max(0, value));

/** Keyboard, pointer and gamepad input for the console interface, behind one polling loop. */
export function createInputEngine(deps: InputEngineDeps): InputEngine {
  const mounts: Mount[] = [];
  const sourceListeners = new Set<() => void>();
  const statusListeners = new Set<() => void>();
  let order = 0;
  let attached = false;
  let frame: number | null = null;
  let source: InputSource = "keyboard";
  let status: GamepadStatus = DISCONNECTED;
  // Polling state: buttons held on the previous pass, directions with their repeat timers.
  let primed = false;
  let heldButtons = new Set<ConsoleAction>();
  const heldDirections = new Map<ConsoleAction, HeldDirection>();

  const activeMount = (): Mount | undefined =>
    mounts
      .filter((mount) => mount.enabled)
      .reduce<Mount | undefined>((best, m) => (!best || m.order > best.order ? m : best), undefined);

  const setSource = (next: InputSource) => {
    if (source === next) return;
    source = next;
    for (const listener of [...sourceListeners]) listener();
  };

  const setStatus = (next: GamepadStatus) => {
    if (status.connected === next.connected && status.name === next.name && status.family === next.family) return;
    status = next;
    for (const listener of [...statusListeners]) listener();
  };

  const connectedPads = (): GamepadLike[] =>
    Array.from(deps.getGamepads()).filter((pad): pad is GamepadLike => pad !== null && pad.connected);

  const statusOf = (pads: readonly GamepadLike[]): GamepadStatus => {
    const first = pads[0];
    return first
      ? { connected: true, name: controllerName(first.id), family: controllerFamily(first.id) }
      : DISCONNECTED;
  };

  const dispatch = (action: ConsoleAction, from: InputSource) => {
    setSource(from);
    const target = activeMount();
    if (!target) return;
    try {
      target.handler(action, from);
    } catch (error) {
      console.error("A console input handler failed.", error);
    }
  };

  const resetPolling = () => {
    primed = false;
    heldButtons = new Set();
    heldDirections.clear();
  };

  const tick = () => {
    const now = deps.now();
    const pads = connectedPads();
    setStatus(statusOf(pads));

    const down = new Set<ConsoleAction>();
    for (const pad of pads) for (const action of heldActions(pad)) down.add(action);

    // The first pass after (re)starting only records what is already held, so a button that was pressed
    // before the console took over never fires a phantom action.
    if (!primed) {
      primed = true;
      heldButtons = new Set([...down].filter((action) => !DIRECTION_ACTIONS.includes(action)));
      for (const action of DIRECTION_ACTIONS) {
        if (down.has(action)) heldDirections.set(action, { since: now, next: now + FIRST_REPEAT_MS });
      }
      return;
    }

    for (const action of DIRECTION_ACTIONS) {
      const held = heldDirections.get(action);
      if (!down.has(action)) {
        heldDirections.delete(action);
      } else if (!held) {
        heldDirections.set(action, { since: now, next: now + FIRST_REPEAT_MS });
        dispatch(action, "gamepad");
      } else if (now >= held.next) {
        held.next = now + (now - held.since >= FAST_REPEAT_AFTER_MS ? FAST_REPEAT_MS : REPEAT_MS);
        dispatch(action, "gamepad");
      }
    }

    const buttons = new Set([...down].filter((action) => !DIRECTION_ACTIONS.includes(action)));
    for (const action of buttons) if (!heldButtons.has(action)) dispatch(action, "gamepad");
    heldButtons = buttons;
  };

  const shouldPoll = () => mounts.some((mount) => mount.enabled) && !deps.isHidden();

  const syncLoop = () => {
    if (shouldPoll()) {
      if (frame === null) frame = deps.requestFrame(loop);
    } else if (frame !== null) {
      deps.cancelFrame(frame);
      frame = null;
      resetPolling();
    } else {
      resetPolling();
    }
  };

  function loop() {
    frame = null;
    tick();
    syncLoop();
  }

  const onKeyDown = (event: Event) => {
    const key = event as KeyboardEvent;
    const action = mapKey(key);
    if (!action) return;
    if (isTypingTarget(key.target) && key.key !== "Escape") return;
    setSource("keyboard");
    if (!activeMount()) return;
    key.preventDefault();
    dispatch(action, "keyboard");
  };
  const onPointer = () => setSource("pointer");
  const onGamepadConnected = (event: Event) => {
    const pad = (event as GamepadEvent).gamepad as GamepadLike | undefined;
    setStatus(pad ? statusOf([pad]) : statusOf(connectedPads()));
  };
  const onGamepadDisconnected = () => setStatus(statusOf(connectedPads()));
  const onVisibility = () => syncLoop();

  const attach = () => {
    if (attached) return;
    attached = true;
    deps.windowTarget.addEventListener("keydown", onKeyDown);
    deps.windowTarget.addEventListener("pointermove", onPointer);
    deps.windowTarget.addEventListener("pointerdown", onPointer);
    deps.windowTarget.addEventListener("gamepadconnected", onGamepadConnected);
    deps.windowTarget.addEventListener("gamepaddisconnected", onGamepadDisconnected);
    deps.documentTarget.addEventListener("visibilitychange", onVisibility);
    setStatus(statusOf(connectedPads()));
  };

  const detach = () => {
    if (!attached) return;
    attached = false;
    deps.windowTarget.removeEventListener("keydown", onKeyDown);
    deps.windowTarget.removeEventListener("pointermove", onPointer);
    deps.windowTarget.removeEventListener("pointerdown", onPointer);
    deps.windowTarget.removeEventListener("gamepadconnected", onGamepadConnected);
    deps.windowTarget.removeEventListener("gamepaddisconnected", onGamepadDisconnected);
    deps.documentTarget.removeEventListener("visibilitychange", onVisibility);
  };

  /** Listeners stay attached while anything (a receiver or a hook reading the source/status) needs them. */
  const syncListeners = () => {
    if (mounts.length + sourceListeners.size + statusListeners.size > 0) attach();
    else detach();
  };

  const subscribe = (listeners: Set<() => void>) => (listener: () => void) => {
    listeners.add(listener);
    syncListeners();
    return () => {
      listeners.delete(listener);
      syncListeners();
    };
  };

  return {
    register(handler, enabled) {
      const mount: Mount = { handler, enabled, order: enabled ? ++order : 0 };
      mounts.push(mount);
      syncListeners();
      syncLoop();
      return {
        setEnabled(next) {
          if (mount.enabled === next) return;
          mount.enabled = next;
          if (next) mount.order = ++order;
          syncLoop();
        },
        dispose() {
          const index = mounts.indexOf(mount);
          if (index === -1) return;
          mounts.splice(index, 1);
          syncListeners();
          syncLoop();
        },
      };
    },
    subscribeSource: subscribe(sourceListeners),
    subscribeStatus: subscribe(statusListeners),
    getSource: () => source,
    getStatus: () => status,
    rumble(options = {}) {
      const pad = connectedPads().find((candidate) => candidate.vibrationActuator);
      if (!pad?.vibrationActuator) return;
      try {
        void pad.vibrationActuator
          .playEffect("dual-rumble", {
            startDelay: 0,
            duration: Math.min(1000, Math.max(0, options.durationMs ?? 120)),
            strongMagnitude: clamp01(options.strong ?? 0.3),
            weakMagnitude: clamp01(options.weak ?? 0.5),
          })
          .catch(() => undefined);
      } catch {
        // Some browsers expose the actuator but refuse the effect: vibration is optional.
      }
    },
    tick,
  };
}
