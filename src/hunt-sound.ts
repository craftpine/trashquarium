// Optional synthesized hunt sounds, played from the dock window (the desktop
// tank never receives clicks, and a click is needed to unlock audio). Off by default.
import type { HuntView } from "./api";

const KEY = "tq.hunt-sound";
let enabled = false;
let audio: AudioContext | null = null;

try { enabled = localStorage.getItem(KEY) === "1"; } catch { /* storage may be unavailable */ }

export const soundEnabled = () => enabled;

export async function setSound(on: boolean) {
  enabled = on;
  try { localStorage.setItem(KEY, on ? "1" : "0"); } catch { /* ignore */ }
  if (on) await unlockSound();
}

/** Call from a user gesture so the browser allows playback. */
export async function unlockSound() {
  if (!enabled) return;
  try { audio ??= new AudioContext(); if (audio.state !== "running") await audio.resume(); }
  catch { enabled = false; }
}

function tone(frequency: number, duration: number) {
  if (!enabled || document.hidden || audio?.state !== "running") return;
  try {
    const osc = audio.createOscillator(), gain = audio.createGain();
    osc.type = "triangle"; osc.frequency.setValueAtTime(frequency, audio.currentTime);
    gain.gain.setValueAtTime(0.04, audio.currentTime);
    gain.gain.exponentialRampToValueAtTime(0.001, audio.currentTime + duration);
    osc.connect(gain); gain.connect(audio.destination);
    osc.start(); osc.stop(audio.currentTime + duration);
    osc.onended = () => { osc.disconnect(); gain.disconnect(); };
  } catch { /* sound never blocks gameplay */ }
}

/** Play the cues that happened between two views of the same hunt. */
export function soundCues(prev: HuntView | null, next: HuntView) {
  const a = prev?.session, b = next.session;
  if (!b || next.session?.paused) return;
  if (a && a.length < 0.12 && b.length >= 0.12 && b.phase === "extending") tone(180, 0.12); // claw enters the water
  if (b.caught_id && b.caught_id !== a?.caught_id) tone(620, 0.07); // claw closes on a shell
  const receipt = next.last_catch;
  if (receipt && prev && receipt.shell_id !== prev.last_catch?.shell_id) tone(receipt.pearl ? 960 : 760, 0.16); // paid
}
