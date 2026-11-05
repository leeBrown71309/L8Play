import { createSoundEngine, type SoundEngine } from "@/features/console/sound/sound-engine";
import { useSoundSettingsStore } from "@/features/console/sound/sound-settings";

export { describeSound } from "@/features/console/sound/sound-definitions";
export type { ConsoleSoundName } from "@/features/console/sound/sound-definitions";
export { useSoundSettings } from "@/features/console/sound/sound-settings";
export type { SoundSettings } from "@/features/console/sound/sound-settings";

type WebkitWindow = typeof globalThis & { webkitAudioContext?: typeof AudioContext };

/** The app-wide sound player. Call `unlock()` from a user gesture (entering console mode) before playing. */
export const consoleSound: SoundEngine = createSoundEngine({
  createContext: () => {
    const scope = globalThis as WebkitWindow;
    const Context = scope.AudioContext ?? scope.webkitAudioContext;
    return Context ? new Context() : null;
  },
  now: () => performance.now(),
  settings: () => {
    const { enabled, volume, ambient } = useSoundSettingsStore.getState();
    return { enabled, volume, ambient };
  },
  subscribeSettings: (listener) => {
    useSoundSettingsStore.subscribe(listener);
  },
});
