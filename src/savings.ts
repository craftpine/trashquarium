// Cá Mập's savings counter: helpers for the "Gửi tiết kiệm" tab. Savings are in-game CBCoin
// only; the rules (9% a day, simple interest, capped per book) live in balance.json.
import type { SavingsRules } from "./api";
import { formatCountdown } from "./egg-den";

/** Contact card shown at the counter, exactly as supplied by the advertiser. */
export const BANKER = {
  title: "Priority Relationship Manager",
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

/** The counter clerk: a shark-headed banker in a white office shirt with a green lanyard and a staff card. */
export const SHARK_SVG = `<svg viewBox="0 0 214 210" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
  <defs>
    <linearGradient id="sharkSkin" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#8db3c7"/><stop offset="1" stop-color="#557a8f"/>
    </linearGradient>
    <linearGradient id="shirt" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#ffffff"/><stop offset="1" stop-color="#e6eef0"/>
    </linearGradient>
  </defs>
  <ellipse cx="100" cy="203" rx="72" ry="6" fill="#0b2d33" opacity=".14"/>
  <!-- body: white office shirt -->
  <path d="M34 206 C34 170 46 150 74 143 L126 143 C154 150 166 170 166 206 Z" fill="url(#shirt)" stroke="#b9c8cc" stroke-width="2"/>
  <path d="M100 150 L100 206" stroke="#c9d5d8" stroke-width="1.5"/>
  <circle cx="100" cy="170" r="2" fill="#c9d5d8"/><circle cx="100" cy="188" r="2" fill="#c9d5d8"/>
  <!-- waving arm -->
  <path d="M146 162 C168 156 186 138 189 116" stroke="url(#shirt)" stroke-width="20" stroke-linecap="round" fill="none"/>
  <path d="M146 162 C168 156 186 138 189 116" stroke="#b9c8cc" stroke-width="22" stroke-linecap="round" fill="none" opacity=".35"/>
  <path d="M146 162 C168 156 186 138 189 116" stroke="#fbfdfd" stroke-width="18" stroke-linecap="round" fill="none"/>
  <g fill="#f3c8a4" stroke="#d9a07a" stroke-width="1.5">
    <rect x="180" y="86" width="5" height="14" rx="2.5"/><rect x="185.5" y="83" width="5" height="16" rx="2.5"/><rect x="191" y="84" width="5" height="15" rx="2.5"/><rect x="196" y="89" width="4.5" height="13" rx="2.2"/>
    <ellipse cx="189" cy="104" rx="10.5" ry="11"/>
    <path d="M180 104 C174 100 172 94 175 92 C178 92 181 96 183 99" />
  </g>
  <!-- neck -->
  <path d="M80 128 L120 128 L118 146 L82 146 Z" fill="#6f93a6"/>
  <!-- collar -->
  <path d="M76 141 L100 157 L90 162 Z" fill="#fff" stroke="#b9c8cc" stroke-width="1.5"/>
  <path d="M124 141 L100 157 L110 162 Z" fill="#fff" stroke="#b9c8cc" stroke-width="1.5"/>
  <!-- dorsal fin -->
  <path d="M86 46 C92 30 104 14 118 8 C114 22 116 36 124 48 Z" fill="#5f8498" stroke="#456a7d" stroke-width="2" stroke-linejoin="round"/>
  <!-- side fins (like ears) -->
  <path d="M46 92 C30 96 22 108 22 116 C34 112 44 106 52 102 Z" fill="#6f93a6" stroke="#456a7d" stroke-width="2" stroke-linejoin="round"/>
  <path d="M154 92 C170 96 178 108 178 116 C166 112 156 106 148 102 Z" fill="#6f93a6" stroke="#456a7d" stroke-width="2" stroke-linejoin="round"/>
  <!-- head -->
  <path d="M100 38 C138 38 158 66 156 96 C154 122 132 136 100 136 C68 136 46 122 44 96 C42 66 62 38 100 38 Z" fill="url(#sharkSkin)" stroke="#456a7d" stroke-width="2.5"/>
  <path d="M58 104 C66 126 84 133 100 133 C116 133 134 126 142 104 C128 112 114 114 100 114 C86 114 72 112 58 104 Z" fill="#eef4f6"/>
  <ellipse cx="84" cy="54" rx="16" ry="7" fill="#fff" opacity=".22" transform="rotate(-18 84 54)"/>
  <!-- gills -->
  <g stroke="#456a7d" stroke-width="2" stroke-linecap="round" fill="none" opacity=".7">
    <path d="M52 84 Q49 92 52 100"/><path d="M58 82 Q55 91 58 100"/>
    <path d="M148 84 Q151 92 148 100"/><path d="M142 82 Q145 91 142 100"/>
  </g>
  <!-- eyes -->
  <ellipse cx="77" cy="80" rx="10" ry="11" fill="#fff"/><ellipse cx="123" cy="80" rx="10" ry="11" fill="#fff"/>
  <circle cx="79" cy="82" r="6.5" fill="#17343a"/><circle cx="121" cy="82" r="6.5" fill="#17343a"/>
  <circle cx="81" cy="79" r="2.2" fill="#fff"/><circle cx="123" cy="79" r="2.2" fill="#fff"/>
  <path d="M66 66 Q76 61 86 65" stroke="#2c4a57" stroke-width="3" fill="none" stroke-linecap="round"/>
  <path d="M114 65 Q124 61 134 66" stroke="#2c4a57" stroke-width="3" fill="none" stroke-linecap="round"/>
  <circle cx="64" cy="98" r="6" fill="#ff9aa2" opacity=".5"/><circle cx="136" cy="98" r="6" fill="#ff9aa2" opacity=".5"/>
  <!-- grin with little teeth -->
  <path d="M74 100 Q100 124 126 100 Q100 110 74 100 Z" fill="#4a2635" stroke="#2c1a22" stroke-width="2" stroke-linejoin="round"/>
  <path d="M77 101.5 L81 107 L85 103.5 L89 109 L93 104.6 L97 110 L100 105 L103 110 L107 104.6 L111 109 L115 103.5 L119 107 L123 101.5 Q100 109 77 101.5 Z" fill="#fff"/>
  <!-- lanyard + staff card -->
  <path d="M84 146 L100 176 L116 146" stroke="#1f8a4c" stroke-width="5" fill="none" stroke-linejoin="round"/>
  <rect x="88" y="172" width="24" height="30" rx="3" fill="#fff" stroke="#1f8a4c" stroke-width="2"/>
  <rect x="92" y="176" width="16" height="10" rx="1.5" fill="#cfe9d9"/>
  <rect x="92" y="190" width="16" height="2.5" rx="1" fill="#1f8a4c"/><rect x="92" y="195" width="11" height="2.5" rx="1" fill="#9cc9ad"/>
</svg>`;
