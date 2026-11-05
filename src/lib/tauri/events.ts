import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { AutoSnapshotEvent } from "@/lib/bindings/AutoSnapshotEvent";
import type { CloudChangedEvent } from "@/lib/bindings/CloudChangedEvent";
import type { SavesChangedEvent } from "@/lib/bindings/SavesChangedEvent";
import type { SessionEvent } from "@/lib/bindings/SessionEvent";
import { isTauri } from "@/lib/tauri/runtime";

/** Event names emitted by the backend (`src-tauri/src/events.rs`). */
export const EVENTS = {
  libraryChanged: "library://changed",
  gameSession: "game://session",
  savesChanged: "saves://changed",
  autoSnapshot: "snapshot://auto",
  cloudChanged: "cloud://changed",
  artworkFetched: "artwork://fetched",
} as const;

interface EventPayloads {
  [EVENTS.libraryChanged]: null;
  [EVENTS.gameSession]: SessionEvent;
  [EVENTS.savesChanged]: SavesChangedEvent;
  [EVENTS.autoSnapshot]: AutoSnapshotEvent;
  [EVENTS.cloudChanged]: CloudChangedEvent;
  /** Id of the game whose background artwork lookup just ended (found something or not). */
  [EVENTS.artworkFetched]: number;
}

/** Browser preview only: lets the mock backend emit events the way the real one does. */
const previewBus = new EventTarget();

export function emitPreviewEvent<E extends keyof EventPayloads>(event: E, payload: EventPayloads[E]): void {
  previewBus.dispatchEvent(new CustomEvent(event, { detail: payload }));
}

/** Typed `listen`; outside Tauri (browser preview, tests) it only hears `emitPreviewEvent`. */
export async function onBackendEvent<E extends keyof EventPayloads>(
  event: E,
  handler: (payload: EventPayloads[E]) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) {
    const listener = (message: Event) => handler((message as CustomEvent<EventPayloads[E]>).detail);
    previewBus.addEventListener(event, listener);
    return () => previewBus.removeEventListener(event, listener);
  }
  return listen<EventPayloads[E]>(event, (message) => handler(message.payload));
}
