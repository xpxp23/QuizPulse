let sharedCtx: AudioContext | null = null;

function getAudioContext(): AudioContext | null {
  try {
    if (!sharedCtx) {
      const Ctor =
        window.AudioContext ||
        (window as unknown as { webkitAudioContext?: typeof AudioContext })
          .webkitAudioContext;
      if (!Ctor) return null;
      sharedCtx = new Ctor();
    }
    if (sharedCtx.state === "suspended") {
      sharedCtx.resume().catch(() => {});
    }
    return sharedCtx;
  } catch {
    return null;
  }
}

function playTone(
  freq: number,
  startOffset: number,
  duration: number,
  type: OscillatorType = "sine",
  gainPeak = 0.06
) {
  const ctx = getAudioContext();
  if (!ctx) return;

  const now = ctx.currentTime + startOffset;
  const osc = ctx.createOscillator();
  const gain = ctx.createGain();
  osc.type = type;
  osc.frequency.setValueAtTime(freq, now);

  gain.gain.setValueAtTime(0.0001, now);
  gain.gain.exponentialRampToValueAtTime(gainPeak, now + 0.015);
  gain.gain.exponentialRampToValueAtTime(0.0001, now + duration);

  osc.connect(gain);
  gain.connect(ctx.destination);
  osc.start(now);
  osc.stop(now + duration + 0.02);
}

export function playFeedbackSound(
  kind: "correct" | "wrong" | "clear",
  enabled: boolean
) {
  if (!enabled) return;
  if (kind === "correct") {
    playTone(587.33, 0, 0.14, "sine", 0.055);
    playTone(880.0, 0.07, 0.18, "sine", 0.055);
  } else if (kind === "clear") {
    playTone(523.25, 0, 0.12, "sine", 0.055);
    playTone(659.25, 0.07, 0.12, "sine", 0.055);
    playTone(783.99, 0.14, 0.22, "sine", 0.06);
  } else {
    playTone(250, 0, 0.11, "triangle", 0.045);
    playTone(210, 0.08, 0.15, "triangle", 0.04);
  }
}
