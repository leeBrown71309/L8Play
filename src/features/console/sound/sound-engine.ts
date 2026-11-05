import {
  describeSound,
  type ConsoleSoundName,
  type NoiseDef,
  type ToneDef,
} from "@/features/console/sound/sound-definitions";
import type { SoundSettings } from "@/features/console/sound/sound-settings";

export const MAX_VOICES = 8;
/** Extra seconds a voice stays counted after its last note, for the echo tail. */
const TAIL_SECONDS = 0.6;
const FLOOR = 0.0001;
const AMBIENT_LEVEL = 0.05;
const ECHO_DELAY_SECONDS = 0.17;
const ECHO_FEEDBACK = 0.35;

export interface SoundEngineDeps {
  /** Creates the shared AudioContext; null when the platform has none. */
  createContext: () => AudioContext | null;
  /** Wall clock in milliseconds (throttling and voice counting). */
  now: () => number;
  settings: () => SoundSettings;
  /** Called whenever the settings change. */
  subscribeSettings?: (listener: () => void) => void;
}

export interface SoundEngine {
  play: (name: ConsoleSoundName) => void;
  unlock: () => Promise<void>;
  startAmbient: () => void;
  stopAmbient: () => void;
}

interface Graph {
  ctx: AudioContext;
  master: GainNode;
  echoIn: GainNode;
  noise: AudioBuffer;
}

interface AmbientVoice {
  out: GainNode;
  stop: () => void;
}

function buildGraph(ctx: AudioContext): Graph {
  const compressor = ctx.createDynamicsCompressor();
  compressor.connect(ctx.destination);
  const master = ctx.createGain();
  master.connect(compressor);

  // One shared feedback echo: sounds send a share of themselves into it.
  const echoIn = ctx.createGain();
  const delay = ctx.createDelay(1);
  delay.delayTime.value = ECHO_DELAY_SECONDS;
  const feedback = ctx.createGain();
  feedback.gain.value = ECHO_FEEDBACK;
  const wet = ctx.createGain();
  wet.gain.value = 0.5;
  echoIn.connect(delay);
  delay.connect(feedback);
  feedback.connect(delay);
  delay.connect(wet);
  wet.connect(master);

  const length = Math.max(1, Math.floor(ctx.sampleRate));
  const noise = ctx.createBuffer(1, length, ctx.sampleRate);
  const samples = noise.getChannelData(0);
  for (let index = 0; index < length; index += 1) samples[index] = Math.random() * 2 - 1;
  return { ctx, master, echoIn, noise };
}

/** Synthesised interface sounds (Web Audio): no audio files, nothing to license. */
export function createSoundEngine(deps: SoundEngineDeps): SoundEngine {
  let ctx: AudioContext | null = null;
  let graph: Graph | null = null;
  let ambient: AmbientVoice | null = null;
  let ambientWanted = false;
  const lastPlayed = new Map<ConsoleSoundName, number>();
  let busyUntil: number[] = [];

  const running = (): Graph | null => (graph && graph.ctx.state === "running" ? graph : null);

  const applyVolume = () => {
    if (!graph) return;
    graph.master.gain.setTargetAtTime(deps.settings().volume, graph.ctx.currentTime, 0.02);
  };

  const shapeEnvelope = (gain: GainNode, at: number, peak: number, attack: number, duration: number) => {
    gain.gain.setValueAtTime(FLOOR, at);
    gain.gain.linearRampToValueAtTime(peak, at + attack);
    gain.gain.exponentialRampToValueAtTime(FLOOR, at + duration);
  };

  const sendToEcho = (g: Graph, source: AudioNode, mix: number) => {
    if (mix <= 0) return;
    const send = g.ctx.createGain();
    send.gain.value = mix;
    source.connect(send);
    send.connect(g.echoIn);
  };

  const playTone = (g: Graph, tone: ToneDef, at: number, echo: number) => {
    const oscillator = g.ctx.createOscillator();
    oscillator.type = tone.wave;
    oscillator.frequency.setValueAtTime(tone.from, at);
    if (tone.to !== undefined) oscillator.frequency.exponentialRampToValueAtTime(tone.to, at + tone.duration);
    const gain = g.ctx.createGain();
    shapeEnvelope(gain, at, tone.peak, tone.attack, tone.duration);
    if (tone.lowpass !== undefined) {
      const filter = g.ctx.createBiquadFilter();
      filter.type = "lowpass";
      filter.frequency.value = tone.lowpass;
      oscillator.connect(filter);
      filter.connect(gain);
    } else {
      oscillator.connect(gain);
    }
    gain.connect(g.master);
    sendToEcho(g, gain, echo);
    oscillator.start(at);
    oscillator.stop(at + tone.duration + 0.03);
  };

  const playNoise = (g: Graph, noise: NoiseDef, at: number, echo: number) => {
    const source = g.ctx.createBufferSource();
    source.buffer = g.noise;
    source.loop = true;
    const filter = g.ctx.createBiquadFilter();
    filter.type = "bandpass";
    filter.Q.value = 1.2;
    filter.frequency.setValueAtTime(noise.sweepFrom, at);
    filter.frequency.exponentialRampToValueAtTime(noise.sweepTo, at + noise.duration);
    const gain = g.ctx.createGain();
    shapeEnvelope(gain, at, noise.peak, noise.attack, noise.duration);
    source.connect(filter);
    filter.connect(gain);
    gain.connect(g.master);
    sendToEcho(g, gain, echo);
    source.start(at);
    source.stop(at + noise.duration + 0.03);
  };

  const startAmbientVoice = (g: Graph): AmbientVoice => {
    const { ctx: context } = g;
    const out = context.createGain();
    out.gain.value = FLOOR;
    out.connect(g.master);
    const filter = context.createBiquadFilter();
    filter.type = "lowpass";
    filter.frequency.value = 700;
    filter.connect(out);

    const oscillators: OscillatorNode[] = [];
    const voice = (wave: OscillatorType, frequency: number, level: number) => {
      const oscillator = context.createOscillator();
      oscillator.type = wave;
      oscillator.frequency.value = frequency;
      const gain = context.createGain();
      gain.gain.value = level;
      oscillator.connect(gain);
      gain.connect(filter);
      oscillator.start();
      oscillators.push(oscillator);
    };
    // Two slightly detuned voices and a quiet fifth above: a slowly beating pad.
    voice("sine", 110, 0.6);
    voice("sine", 110.9, 0.6);
    voice("triangle", 164.8, 0.3);

    const lfo = context.createOscillator();
    lfo.frequency.value = 0.06;
    const lfoDepth = context.createGain();
    lfoDepth.gain.value = 250;
    lfo.connect(lfoDepth);
    lfoDepth.connect(filter.frequency);
    lfo.start();
    oscillators.push(lfo);

    out.gain.setTargetAtTime(AMBIENT_LEVEL, context.currentTime, 0.5);
    return {
      out,
      stop: () => {
        const at = context.currentTime;
        out.gain.setTargetAtTime(0, at, 0.5);
        for (const oscillator of oscillators) oscillator.stop(at + 2.5);
      },
    };
  };

  const syncAmbient = () => {
    const settings = deps.settings();
    const g = running();
    const wanted = ambientWanted && settings.enabled && settings.ambient && g !== null;
    if (wanted && !ambient && g) {
      ambient = startAmbientVoice(g);
    } else if (!wanted && ambient) {
      ambient.stop();
      ambient = null;
    }
  };

  deps.subscribeSettings?.(() => {
    try {
      applyVolume();
      syncAmbient();
    } catch (error) {
      console.warn("Console sound settings could not be applied.", error);
    }
  });

  return {
    play(name) {
      try {
        const g = running();
        if (!g || !deps.settings().enabled) return;
        const definition = describeSound(name);
        const now = deps.now();
        const last = lastPlayed.get(name);
        if (definition.throttleMs > 0 && last !== undefined && now - last < definition.throttleMs) return;
        busyUntil = busyUntil.filter((end) => end > now);
        if (busyUntil.length >= MAX_VOICES) return;
        lastPlayed.set(name, now);
        busyUntil.push(now + (definition.duration + TAIL_SECONDS) * 1000);

        const at = g.ctx.currentTime + 0.005;
        for (const tone of definition.tones) playTone(g, tone, at + tone.start, definition.echo);
        for (const noise of definition.noises) playNoise(g, noise, at + noise.start, definition.echo);
      } catch (error) {
        // A sound that fails must never take the interface down with it.
        console.warn("A console sound could not be played.", error);
      }
    },
    async unlock() {
      try {
        ctx ??= deps.createContext();
        if (!ctx) return;
        graph ??= buildGraph(ctx);
        if (ctx.state === "suspended") await ctx.resume();
        applyVolume();
        syncAmbient();
      } catch (error) {
        console.warn("Console sound could not be unlocked.", error);
      }
    },
    startAmbient() {
      ambientWanted = true;
      try {
        syncAmbient();
      } catch (error) {
        console.warn("Console ambient sound could not start.", error);
      }
    },
    stopAmbient() {
      ambientWanted = false;
      try {
        syncAmbient();
      } catch (error) {
        console.warn("Console ambient sound could not stop.", error);
      }
    },
  };
}
