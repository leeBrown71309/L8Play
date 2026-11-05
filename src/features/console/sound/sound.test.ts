import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { describeSound, SOUND_NAMES } from "@/features/console/sound/sound-definitions";
import { createSoundEngine, MAX_VOICES } from "@/features/console/sound/sound-engine";
import {
  DEFAULT_SOUND_SETTINGS,
  loadSoundSettings,
  SOUND_STORAGE_KEYS,
  useSoundSettingsStore,
  type SoundSettings,
} from "@/features/console/sound/sound-settings";

/** An AudioParam stand-in that records nothing but accepts every call. */
function fakeParam() {
  return {
    value: 0,
    setValueAtTime: vi.fn(),
    linearRampToValueAtTime: vi.fn(),
    exponentialRampToValueAtTime: vi.fn(),
    setTargetAtTime: vi.fn(),
  };
}

function fakeNode(extra: Record<string, unknown> = {}) {
  return { connect: vi.fn(), disconnect: vi.fn(), start: vi.fn(), stop: vi.fn(), ...extra };
}

function fakeContext(state: "running" | "suspended" = "running") {
  const counts = { oscillators: 0, sources: 0 };
  const context = {
    state,
    currentTime: 0,
    sampleRate: 8000,
    destination: fakeNode(),
    resume: vi.fn(async () => {
      context.state = "running";
    }),
    createDynamicsCompressor: () => fakeNode(),
    createGain: () => fakeNode({ gain: fakeParam() }),
    createDelay: () => fakeNode({ delayTime: fakeParam() }),
    createBiquadFilter: () => fakeNode({ frequency: fakeParam(), Q: fakeParam(), type: "lowpass" }),
    createOscillator: () => {
      counts.oscillators += 1;
      return fakeNode({ frequency: fakeParam(), type: "sine" });
    },
    createBufferSource: () => {
      counts.sources += 1;
      return fakeNode({ buffer: null, loop: false });
    },
    createBuffer: (_channels: number, length: number) => ({ getChannelData: () => new Float32Array(length) }),
  };
  return { context, counts };
}

describe("sound definitions", () => {
  it("describes every sound with sane notes, envelopes and gains", () => {
    expect(SOUND_NAMES.sort()).toEqual(
      ["back", "boot", "close", "error", "launch", "move", "open", "select", "tab", "toggle"].sort(),
    );
    for (const name of SOUND_NAMES) {
      const sound = describeSound(name);
      expect(sound.tones.length + sound.noises.length).toBeGreaterThan(0);
      expect(sound.duration).toBeGreaterThan(0);
      expect(sound.duration).toBeLessThanOrEqual(1.5);
      expect(sound.echo).toBeGreaterThanOrEqual(0);
      expect(sound.echo).toBeLessThanOrEqual(1);
      for (const tone of sound.tones) {
        for (const frequency of [tone.from, tone.to ?? tone.from]) {
          expect(frequency).toBeGreaterThanOrEqual(80);
          expect(frequency).toBeLessThanOrEqual(2000);
        }
        expect(tone.peak).toBeGreaterThan(0);
        expect(tone.peak).toBeLessThanOrEqual(0.35);
        expect(tone.attack).toBeLessThan(tone.duration);
        expect(tone.start + tone.duration).toBeLessThanOrEqual(1.5);
      }
      for (const noise of sound.noises) {
        expect(noise.peak).toBeLessThanOrEqual(0.35);
        expect(noise.attack).toBeLessThan(noise.duration);
        for (const frequency of [noise.sweepFrom, noise.sweepTo]) {
          expect(frequency).toBeGreaterThanOrEqual(80);
          expect(frequency).toBeLessThanOrEqual(4000);
        }
      }
    }
  });

  it("makes the tab tick higher than the move tick", () => {
    expect(describeSound("tab").tones[0]?.from).toBeGreaterThan(describeSound("move").tones[0]?.from ?? 0);
  });

  it("sizes the launch whoosh near 900 ms and the boot chord near 1.2 s", () => {
    expect(describeSound("launch").duration).toBeCloseTo(0.9, 1);
    expect(describeSound("boot").duration).toBeCloseTo(1.2, 1);
  });
});

describe("sound engine", () => {
  const settings: SoundSettings = { enabled: true, volume: 0.6, ambient: false };
  let clock = 0;
  let current: SoundSettings;
  let notify: () => void;

  const make = (createContext: () => AudioContext | null) =>
    createSoundEngine({
      createContext,
      now: () => clock,
      settings: () => current,
      subscribeSettings: (listener) => {
        notify = listener;
      },
    });

  beforeEach(() => {
    clock = 1000;
    current = { ...settings };
    notify = () => undefined;
  });
  afterEach(() => vi.restoreAllMocks());

  it("does nothing, without throwing, before unlock or when there is no AudioContext", async () => {
    const { context, counts } = fakeContext();
    const engine = make(() => context as unknown as AudioContext);
    expect(() => engine.play("select")).not.toThrow();
    expect(counts.oscillators).toBe(0);

    const silent = make(() => null);
    await silent.unlock();
    expect(() => silent.play("select")).not.toThrow();
    expect(() => silent.startAmbient()).not.toThrow();
    expect(() => silent.stopAmbient()).not.toThrow();
  });

  it("resumes a suspended context on unlock, then plays", async () => {
    const { context, counts } = fakeContext("suspended");
    const engine = make(() => context as unknown as AudioContext);
    await engine.unlock();
    expect(context.resume).toHaveBeenCalledTimes(1);
    engine.play("select");
    expect(counts.oscillators).toBe(describeSound("select").tones.length);
  });

  it("survives a context that refuses to resume", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const { context } = fakeContext("suspended");
    context.resume.mockRejectedValue(new Error("blocked"));
    const engine = make(() => context as unknown as AudioContext);
    await expect(engine.unlock()).resolves.toBeUndefined();
    expect(() => engine.play("move")).not.toThrow();
    expect(warn).toHaveBeenCalled();
  });

  it("respects the enabled setting", async () => {
    const { context, counts } = fakeContext();
    const engine = make(() => context as unknown as AudioContext);
    await engine.unlock();
    current = { ...current, enabled: false };
    engine.play("select");
    expect(counts.oscillators).toBe(0);
  });

  it("throttles the move tick to one per 45 ms", async () => {
    const { context, counts } = fakeContext();
    const engine = make(() => context as unknown as AudioContext);
    await engine.unlock();
    engine.play("move");
    clock += 20;
    engine.play("move");
    expect(counts.oscillators).toBe(1);
    clock += 30;
    engine.play("move");
    expect(counts.oscillators).toBe(2);
  });

  it("caps the number of simultaneous voices", async () => {
    const { context, counts } = fakeContext();
    const engine = make(() => context as unknown as AudioContext);
    await engine.unlock();
    for (let index = 0; index < MAX_VOICES + 5; index += 1) engine.play("toggle");
    expect(counts.oscillators).toBe(MAX_VOICES);
    clock += 5000;
    engine.play("toggle");
    expect(counts.oscillators).toBe(MAX_VOICES + 1);
  });

  it("plays the noise of a whoosh through a buffer source", async () => {
    const { context, counts } = fakeContext();
    const engine = make(() => context as unknown as AudioContext);
    await engine.unlock();
    engine.play("launch");
    expect(counts.sources).toBe(1);
  });

  it("starts the ambient pad only when wanted, enabled and switched on, and fades it out", async () => {
    const { context, counts } = fakeContext();
    const engine = make(() => context as unknown as AudioContext);
    await engine.unlock();
    engine.startAmbient();
    expect(counts.oscillators).toBe(0);
    current = { ...current, ambient: true };
    notify();
    expect(counts.oscillators).toBe(4);
    notify();
    expect(counts.oscillators).toBe(4);
    engine.stopAmbient();
    current = { ...current, ambient: true };
    engine.startAmbient();
    expect(counts.oscillators).toBe(8);
    current = { ...current, enabled: false };
    notify();
    engine.startAmbient();
    expect(counts.oscillators).toBe(8);
  });
});

describe("sound settings", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    window.localStorage.clear();
  });

  it("starts from the defaults", () => {
    window.localStorage.clear();
    expect(loadSoundSettings()).toEqual(DEFAULT_SOUND_SETTINGS);
    expect(DEFAULT_SOUND_SETTINGS).toEqual({ enabled: true, volume: 0.6, ambient: false });
  });

  it("persists changes and reads them back", () => {
    const { setEnabled, setVolume, setAmbient } = useSoundSettingsStore.getState();
    setEnabled(false);
    setVolume(0.25);
    setAmbient(true);
    expect(window.localStorage.getItem(SOUND_STORAGE_KEYS.enabled)).toBe("false");
    expect(window.localStorage.getItem(SOUND_STORAGE_KEYS.volume)).toBe("0.25");
    expect(window.localStorage.getItem(SOUND_STORAGE_KEYS.ambient)).toBe("true");
    expect(loadSoundSettings()).toEqual({ enabled: false, volume: 0.25, ambient: true });
    setEnabled(true);
    setAmbient(false);
    setVolume(DEFAULT_SOUND_SETTINGS.volume);
  });

  it("keeps the volume between 0 and 1 and ignores garbage in storage", () => {
    useSoundSettingsStore.getState().setVolume(7);
    expect(useSoundSettingsStore.getState().volume).toBe(1);
    useSoundSettingsStore.getState().setVolume(-2);
    expect(useSoundSettingsStore.getState().volume).toBe(0);
    window.localStorage.setItem(SOUND_STORAGE_KEYS.volume, "loud");
    expect(loadSoundSettings().volume).toBe(DEFAULT_SOUND_SETTINGS.volume);
    useSoundSettingsStore.getState().setVolume(DEFAULT_SOUND_SETTINGS.volume);
  });

  it("works when storage throws (private mode)", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("denied");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("denied");
    });
    expect(loadSoundSettings()).toEqual(DEFAULT_SOUND_SETTINGS);
    expect(() => useSoundSettingsStore.getState().setVolume(0.4)).not.toThrow();
    expect(useSoundSettingsStore.getState().volume).toBe(0.4);
    useSoundSettingsStore.getState().setVolume(DEFAULT_SOUND_SETTINGS.volume);
  });
});
