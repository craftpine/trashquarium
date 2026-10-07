// Chị Cua's savings counter: helpers for the "Gửi tiết kiệm" tab. Savings are in-game CBCoin
// only; the rules (9% a day, simple interest, capped per book) live in balance.json.
import type { SavingsRules } from "./api";
import { formatCountdown } from "./egg-den";

/** Contact card shown at the counter, exactly as supplied by the advertiser. */
export const BANKER = {
  title: "Priority Relationship Manager",
  branch: "OCB - The Hallmark TP.HCM",
  phone: "0934.143.910",
  tiktok: "jennifererr",
};

/** Same formula as the Rust engine (Savings::interest): rounded down, capped per book. */
export function savingsInterest(rules: SavingsRules, principal: number, days: number): number {
  const raw = Math.floor(principal * rules.daily_rate * days);
  return Math.max(0, Math.min(raw, rules.max_interest));
}

/** Percent the whole term pays, e.g. 7 days at 9% → "63". */
export function termPercent(rules: SavingsRules, days: number): string {
  return String(Math.round(rules.daily_rate * days * 1000) / 10);
}

/** Time left as [days, "hh:mm:ss"]; days is 0 under one day. */
export function splitCountdown(seconds: number): [number, string] {
  const s = Math.max(0, Math.ceil(seconds));
  const days = Math.floor(s / 86_400);
  return [days, formatCountdown(s - days * 86_400)];
}

/** Share of the term already served, 0..1. */
export function progress(openedAt: number, maturesAt: number, now: number): number {
  if (maturesAt <= openedAt) return 1;
  return Math.min(1, Math.max(0, (now - openedAt) / (maturesAt - openedAt)));
}

/** The counter clerk: a little red crab with a green lanyard and a staff card. */
export const CRAB_SVG = `<svg viewBox="0 0 200 170" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
  <defs>
    <radialGradient id="crabShell" cx="45%" cy="35%" r="70%">
      <stop offset="0" stop-color="#ff9a6b"/><stop offset=".6" stop-color="#e2553a"/><stop offset="1" stop-color="#b63a28"/>
    </radialGradient>
  </defs>
  <ellipse cx="100" cy="158" rx="70" ry="8" fill="#0b2d33" opacity=".15"/>
  <g stroke="#b63a28" stroke-width="7" stroke-linecap="round" fill="none">
    <path d="M58 118 L30 132 L22 152"/><path d="M62 128 L40 146 L36 160"/><path d="M142 118 L170 132 L178 152"/><path d="M138 128 L160 146 L164 160"/>
  </g>
  <g fill="url(#crabShell)" stroke="#a5321f" stroke-width="2">
    <path d="M52 92 C30 80 22 58 30 44 C36 34 50 34 54 44 C48 46 44 54 50 62 C56 70 64 72 70 74 Z"/>
    <path d="M30 44 C20 34 24 20 36 18 C46 17 52 26 50 36 Z"/>
    <path d="M148 92 C170 80 178 58 170 44 C164 34 150 34 146 44 C152 46 156 54 150 62 C144 70 136 72 130 74 Z"/>
    <path d="M170 44 C180 34 176 20 164 18 C154 17 148 26 150 36 Z"/>
    <ellipse cx="100" cy="110" rx="58" ry="40"/>
  </g>
  <ellipse cx="86" cy="94" rx="18" ry="8" fill="#fff" opacity=".22"/>
  <g stroke="#a5321f" stroke-width="5" stroke-linecap="round"><path d="M84 76 L80 50"/><path d="M116 76 L120 50"/></g>
  <circle cx="80" cy="44" r="12" fill="#fff" stroke="#a5321f" stroke-width="2"/><circle cx="120" cy="44" r="12" fill="#fff" stroke="#a5321f" stroke-width="2"/>
  <circle cx="82" cy="46" r="6" fill="#17343a"/><circle cx="118" cy="46" r="6" fill="#17343a"/>
  <circle cx="84" cy="43" r="2" fill="#fff"/><circle cx="120" cy="43" r="2" fill="#fff"/>
  <path d="M72 30 Q80 26 88 30" stroke="#17343a" stroke-width="2.5" fill="none" stroke-linecap="round"/>
  <path d="M112 30 Q120 26 128 30" stroke="#17343a" stroke-width="2.5" fill="none" stroke-linecap="round"/>
  <circle cx="74" cy="112" r="6" fill="#ff8f8f" opacity=".55"/><circle cx="126" cy="112" r="6" fill="#ff8f8f" opacity=".55"/>
  <path d="M90 116 Q100 124 110 116" stroke="#17343a" stroke-width="3" fill="none" stroke-linecap="round"/>
  <path d="M78 82 L100 120 L122 82" stroke="#1f8a4c" stroke-width="5" fill="none" stroke-linejoin="round"/>
  <rect x="88" y="118" width="24" height="28" rx="3" fill="#fff" stroke="#1f8a4c" stroke-width="2"/>
  <rect x="92" y="122" width="16" height="9" rx="1.5" fill="#cfe9d9"/>
  <rect x="92" y="134" width="16" height="2.5" rx="1" fill="#1f8a4c"/><rect x="92" y="139" width="11" height="2.5" rx="1" fill="#9cc9ad"/>
</svg>`;
