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

/** The counter clerk: a shark banker in a white shirt, green tie and staff lanyard. */
export const SHARK_IMG = "/art/shark-banker.png";
