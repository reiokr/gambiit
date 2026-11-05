let audio: AudioContext | null = null;

function context(): AudioContext | null {
  if (typeof window === "undefined") return null;
  if (!audio) {
    const Ctor =
      window.AudioContext ??
      (window as unknown as { webkitAudioContext?: typeof AudioContext })
        .webkitAudioContext;
    if (!Ctor) return null;
    audio = new Ctor();
  }
  return audio;
}

type SoundKind = "move" | "capture" | "check" | "end";

export function playSound(kind: SoundKind, enabled: boolean): void {
  if (!enabled) return;
  const ctx = context();
  if (!ctx) return;
  if (ctx.state === "suspended") void ctx.resume();

  const now = ctx.currentTime;
  const settings: Record<
    SoundKind,
    { frequency: number; duration: number; type: OscillatorType; gain: number }
  > = {
    move: { frequency: 190, duration: 0.06, type: "triangle", gain: 0.035 },
    capture: { frequency: 120, duration: 0.11, type: "sawtooth", gain: 0.045 },
    check: { frequency: 520, duration: 0.16, type: "sine", gain: 0.05 },
    end: { frequency: 330, duration: 0.5, type: "sine", gain: 0.05 },
  };
  const tone = settings[kind];

  const oscillator = ctx.createOscillator();
  const gain = ctx.createGain();
  oscillator.type = tone.type;
  oscillator.frequency.setValueAtTime(tone.frequency, now);
  gain.gain.setValueAtTime(0.0001, now);
  gain.gain.exponentialRampToValueAtTime(tone.gain, now + 0.01);
  gain.gain.exponentialRampToValueAtTime(0.0001, now + tone.duration);
  oscillator.connect(gain).connect(ctx.destination);
  oscillator.start(now);
  oscillator.stop(now + tone.duration + 0.02);

  if (kind === "check" || kind === "end") {
    const second = ctx.createOscillator();
    const secondGain = ctx.createGain();
    second.type = "sine";
    second.frequency.setValueAtTime(tone.frequency * 1.5, now + 0.06);
    secondGain.gain.setValueAtTime(0.0001, now + 0.06);
    secondGain.gain.exponentialRampToValueAtTime(tone.gain * 0.7, now + 0.08);
    secondGain.gain.exponentialRampToValueAtTime(
      0.0001,
      now + tone.duration + 0.08,
    );
    second.connect(secondGain).connect(ctx.destination);
    second.start(now + 0.06);
    second.stop(now + tone.duration + 0.1);
  }
}
