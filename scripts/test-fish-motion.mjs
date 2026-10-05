import assert from 'node:assert/strict';
import { bodyWave, finStretch, nextBurst, swimStyle } from '../src/fish-motion.ts';
const peak = (style, u) => Math.max(...Array.from({ length: 64 }, (_, i) => Math.abs(bodyWave(style, u, i / 64 * Math.PI * 2))));
for (const style of ['fish', 'tuna', 'eel']) {
  assert.ok(peak(style, 0) > peak(style, 0.9) * 2, `${style}: the tail must move more than the head`);
  assert.ok(peak(style, 0) < 0.08, `${style}: body wave stays subtle`);
}
assert.ok(peak('eel', 0.6) > peak('tuna', 0.6) * 3, 'eels ripple the middle of the body, tunas keep it stiff');
assert.equal(swimStyle('mobula_birostris'), 'ray');
assert.equal(swimStyle('poecilia_reticulata'), 'fish');
const span = (style, u) => { const v = Array.from({ length: 64 }, (_, i) => finStretch(style, u, i / 10, i / 7)); return Math.max(...v) - Math.min(...v); };
assert.ok(span('fish', 0.05) > 0.1 && span('fish', 0.55) > 0.05, 'tail fin fans and side fins flutter');
assert.ok(span('ray', 0.5) > 0.3, 'rays flap their wings');
for (let u = 0; u <= 1; u += 0.1) for (const st of ['fish', 'tuna', 'eel', 'ray']) assert.ok(finStretch(st, u, 1, 2) > 0.7 && finStretch(st, u, 1, 2) < 1.3);
assert.equal(nextBurst(0.2, 0.1, 0.99, true), 1);
assert.ok(nextBurst(1, 0.5, 0.99, false) < 1 && nextBurst(0, 0.5, 0.99, false) === 0);
console.log('PASS: tail moves more than head, eel/tuna/ray styles, fins flutter, burst-and-glide');
