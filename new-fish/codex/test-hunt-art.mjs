import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
globalThis.Image = class { complete = false; naturalWidth = 0; src = ''; };
const { anchoredRect, BOAT_HATCH, CLAW_GRAB, CLAW_RING, REST_LENGTH, huntArt, ready } = await import('../src/hunt-art.ts');
for (const [anchor, w, h] of [[BOAT_HATCH, 120, 90], [CLAW_GRAB, 28, 38.5]]) {
  for (const [x, y] of [[320, 80], [280, 400], [960, 120]]) {
    const rect = anchoredRect(x, y, w, h, anchor);
    assert.ok(Math.abs(rect.x + rect.width * anchor.x - x) < 1e-9);
    assert.ok(Math.abs(rect.y + rect.height * anchor.y - y) < 1e-9);
  }
}
assert.ok(CLAW_RING.y < CLAW_GRAB.y);
for (const side of [320, 440, 600, 1080]) {
  const height = Math.min(38.5, side * REST_LENGTH / (CLAW_GRAB.y - CLAW_RING.y));
  const rect = anchoredRect(0, side * REST_LENGTH, height * 256 / 352, height, CLAW_GRAB);
  assert.ok(rect.y + rect.height * CLAW_RING.y >= -1e-9, 'ring above hatch at rest');
}
assert.equal(ready(huntArt.boat), false);
const expected = [['boat.png', 720, 540], ['claw.png', 256, 352], ...[0, 1, 2].map(i => [`shell_${i}.png`, 512, 512])];
for (const [file, w, h] of expected) {
  const path = fileURLToPath(new URL(`../public/art/hunt/${file}`, import.meta.url));
  const bytes = readFileSync(path);
  assert.equal(bytes.subarray(0, 8).toString('hex'), '89504e470d0a1a0a');
  assert.equal(bytes.readUInt32BE(16), w, file);
  assert.equal(bytes.readUInt32BE(20), h, file);
  assert.equal(bytes[25], 6, `${file}: expected RGBA PNG`);
}
console.log('PASS: 5 RGBA PNG sizes; boat/claw anchor geometry; loading fallback');
