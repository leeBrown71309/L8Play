import { create } from "zustand";

export interface SoundSettings {
  enabled: boolean;
  /** 0–1. */
  volume: number;
  /** Quiet background pad while the console interface is open. */
  ambient: boolean;
}

export const SOUND_STORAGE_KEYS = {
  enabled: "l8.console.sound.enabled",
  volume: "l8.console.sound.volume",
  ambient: "l8.console.sound.ambient",
} as const;

export const DEFAULT_SOUND_SETTINGS: SoundSettings = { enabled: true, volume: 0.6, ambient: false };

const clampVolume = (value: number) =>
  Number.isFinite(value) ? Math.min(1, Math.max(0, value)) : DEFAULT_SOUND_SETTINGS.volume;

function readStored(key: string): string | null {
  try {
    return window.localStorage.getItem(key);
  } catch {
    // Private windows and blocked storage: the defaults apply and nothing is remembered.
    return null;
  }
}

function writeStored(key: string, value: string): void {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // Not remembering a preference must never break the interface.
  }
}

const readBoolean = (key: string, fallback: boolean): boolean => {
  const stored = readStored(key);
  return stored === null ? fallback : stored === "true";
};

export function loadSoundSettings(): SoundSettings {
  const volume = readStored(SOUND_STORAGE_KEYS.volume);
  return {
    enabled: readBoolean(SOUND_STORAGE_KEYS.enabled, DEFAULT_SOUND_SETTINGS.enabled),
    volume: volume === null ? DEFAULT_SOUND_SETTINGS.volume : clampVolume(Number(volume)),
    ambient: readBoolean(SOUND_STORAGE_KEYS.ambient, DEFAULT_SOUND_SETTINGS.ambient),
  };
}

interface SoundSettingsStore extends SoundSettings {
  setEnabled: (enabled: boolean) => void;
  setVolume: (volume: number) => void;
  setAmbient: (ambient: boolean) => void;
}

export const useSoundSettingsStore = create<SoundSettingsStore>((set) => ({
  ...loadSoundSettings(),
  setEnabled: (enabled) => {
    writeStored(SOUND_STORAGE_KEYS.enabled, String(enabled));
    set({ enabled });
  },
  setVolume: (volume) => {
    const clamped = clampVolume(volume);
    writeStored(SOUND_STORAGE_KEYS.volume, String(clamped));
    set({ volume: clamped });
  },
  setAmbient: (ambient) => {
    writeStored(SOUND_STORAGE_KEYS.ambient, String(ambient));
    set({ ambient });
  },
}));

/** Sound preferences (persisted) and their setters. */
export function useSoundSettings(): SoundSettingsStore {
  return useSoundSettingsStore();
}
