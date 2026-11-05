export type ConsoleSoundName =
  "move" | "select" | "back" | "open" | "close" | "launch" | "error" | "toggle" | "tab" | "boot";

export interface ToneDef {
  wave: OscillatorType;
  /** Start frequency in Hz; the tone glides to `to` over its length when given. */
  from: number;
  to?: number;
  /** Seconds after the sound starts. */
  start: number;
  duration: number;
  /** Peak gain (before the master volume). */
  peak: number;
  attack: number;
  /** Optional low-pass cutoff in Hz that softens the tone. */
  lowpass?: number;
}

export interface NoiseDef {
  start: number;
  duration: number;
  peak: number;
  attack: number;
  /** Band-pass centre sweep in Hz. */
  sweepFrom: number;
  sweepTo: number;
}

export interface SoundDef {
  name: ConsoleSoundName;
  tones: readonly ToneDef[];
  noises: readonly NoiseDef[];
  /** 0–1 share of the sound sent through the shared echo. */
  echo: number;
  /** Minimum gap between two plays of this sound, in milliseconds (0 = none). */
  throttleMs: number;
  /** Seconds until the last note ends (the echo tail is extra). */
  duration: number;
}

type Definition = Omit<SoundDef, "name" | "duration">;

const DEFINITIONS: Record<ConsoleSoundName, Definition> = {
  move: {
    tones: [{ wave: "triangle", from: 520, to: 620, start: 0, duration: 0.06, peak: 0.05, attack: 0.012 }],
    noises: [],
    echo: 0,
    throttleMs: 45,
  },
  tab: {
    tones: [{ wave: "triangle", from: 640, to: 780, start: 0, duration: 0.07, peak: 0.06, attack: 0.012 }],
    noises: [],
    echo: 0,
    throttleMs: 45,
  },
  select: {
    tones: [
      { wave: "sine", from: 660, start: 0, duration: 0.14, peak: 0.16, attack: 0.008 },
      { wave: "sine", from: 990, start: 0.07, duration: 0.22, peak: 0.16, attack: 0.008 },
    ],
    noises: [],
    echo: 0.25,
    throttleMs: 0,
  },
  back: {
    tones: [
      { wave: "sine", from: 600, start: 0, duration: 0.12, peak: 0.14, attack: 0.008 },
      { wave: "sine", from: 400, start: 0.07, duration: 0.2, peak: 0.14, attack: 0.008 },
    ],
    noises: [],
    echo: 0.15,
    throttleMs: 0,
  },
  open: {
    tones: [{ wave: "sine", from: 320, to: 520, start: 0, duration: 0.22, peak: 0.08, attack: 0.04 }],
    noises: [{ start: 0, duration: 0.22, peak: 0.07, attack: 0.05, sweepFrom: 300, sweepTo: 1800 }],
    echo: 0.15,
    throttleMs: 0,
  },
  close: {
    tones: [{ wave: "sine", from: 520, to: 320, start: 0, duration: 0.22, peak: 0.08, attack: 0.02 }],
    noises: [{ start: 0, duration: 0.22, peak: 0.07, attack: 0.03, sweepFrom: 1800, sweepTo: 300 }],
    echo: 0.1,
    throttleMs: 0,
  },
  launch: {
    tones: [
      { wave: "sine", from: 261.6, start: 0, duration: 0.9, peak: 0.12, attack: 0.25 },
      { wave: "sine", from: 329.6, start: 0.05, duration: 0.85, peak: 0.12, attack: 0.25 },
      { wave: "sine", from: 392, start: 0.1, duration: 0.8, peak: 0.12, attack: 0.25 },
      { wave: "triangle", from: 523.3, start: 0.2, duration: 0.7, peak: 0.1, attack: 0.2 },
    ],
    noises: [{ start: 0, duration: 0.9, peak: 0.1, attack: 0.3, sweepFrom: 200, sweepTo: 3200 }],
    echo: 0.3,
    throttleMs: 0,
  },
  error: {
    tones: [
      { wave: "sawtooth", from: 140, start: 0, duration: 0.12, peak: 0.12, attack: 0.008, lowpass: 500 },
      { wave: "sawtooth", from: 130, start: 0.16, duration: 0.14, peak: 0.12, attack: 0.008, lowpass: 450 },
    ],
    noises: [],
    echo: 0,
    throttleMs: 0,
  },
  toggle: {
    tones: [{ wave: "triangle", from: 880, to: 600, start: 0, duration: 0.035, peak: 0.09, attack: 0.004 }],
    noises: [],
    echo: 0,
    throttleMs: 0,
  },
  boot: {
    tones: [
      { wave: "sine", from: 130.8, start: 0, duration: 1.2, peak: 0.1, attack: 0.25 },
      { wave: "sine", from: 196, start: 0.05, duration: 1.15, peak: 0.08, attack: 0.25 },
      { wave: "sine", from: 261.6, start: 0.1, duration: 1.1, peak: 0.08, attack: 0.25 },
      { wave: "sine", from: 329.6, start: 0.18, duration: 1, peak: 0.07, attack: 0.25 },
      { wave: "triangle", from: 392, start: 0.26, duration: 0.9, peak: 0.06, attack: 0.25 },
    ],
    noises: [],
    echo: 0.4,
    throttleMs: 0,
  },
};

export const SOUND_NAMES = Object.keys(DEFINITIONS) as ConsoleSoundName[];

/** The notes and envelope of a sound, as plain data. */
export function describeSound(name: ConsoleSoundName): SoundDef {
  const definition = DEFINITIONS[name];
  const ends = [...definition.tones, ...definition.noises].map((part) => part.start + part.duration);
  return { name, ...definition, duration: Math.max(...ends) };
}
