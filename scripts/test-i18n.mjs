// Every Vietnamese UI sentence must have an English translation with the same {placeholders},
// every species an English name/fact, and the English table must not keep dead entries.
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { EN } from '../src/locales/en.ts';
import { SPECIES_EN } from '../src/locales/species-en.ts';
import { ZH } from '../src/locales/zh.ts';
import { SPECIES_ZH } from '../src/locales/species-zh.ts';

// Letters that only Vietnamese uses (plain á, é, í, ó, ú also appear in names like Yucatán).
const VIET = /[ăâđêôơưạảấầẩẫậắằẳẵặẹẻẽếềểễệỉịọỏốồổỗộớờởỡợụủứừửữựỳỷỹỵĩũ]/i;
const unescape = (s) => JSON.parse(`"${s}"`);
const srcFiles = readdirSync('src').filter((f) => f.endsWith('.ts')).map((f) => `src/${f}`);
const keys = new Set();

// t("…") calls with a literal first argument.
for (const file of srcFiles) {
  const text = readFileSync(file, 'utf8');
  for (const m of text.matchAll(/\bt\(\s*"((?:[^"\\]|\\.)*)"/g)) keys.add(unescape(m[1]));
}
// Lookup tables whose values are passed to t() later.
const grab = (file, startMarker, endMarker, re) => {
  const text = readFileSync(file, 'utf8');
  const a = text.indexOf(startMarker);
  assert.ok(a >= 0, `${startMarker} not found in ${file}`);
  const block = text.slice(a, text.indexOf(endMarker, a));
  for (const m of block.matchAll(re)) keys.add(unescape(m[1]));
};
grab('src/i18n.ts', 'const reasons', '};', /^\s+\w+: "((?:[^"\\]|\\.)*)",?$/gm);
grab('src/i18n.ts', 'export const attentionNoteSource', '};', /^\s+\w+: "((?:[^"\\]|\\.)*)",?$/gm);
grab('src/i18n.ts', 'export const stageSource', '};', /^\s+\w+: "((?:[^"\\]|\\.)*)",?$/gm);
grab('src/i18n.ts', 'export const categorySource', '};', /^\s+\w+: "((?:[^"\\]|\\.)*)",?$/gm);
grab('src/tank.ts', 'const SHELL_NAME', '\n', /: "((?:[^"\\]|\\.)*)"/g);
grab('src/shell-collection.ts', 'export const SHELL_CARDS', '] as const', /\b(?:name|fact|habitat): "((?:[^"\\]|\\.)*)"/g);
for (const colour of ['Trắng', 'Đỏ', 'Tím']) keys.add(colour); // t([...][card.sprite]) in shell-collection.ts
keys.add(JSON.parse(readFileSync('src-tauri/config/species.json', 'utf8')).disclaimer);

const placeholders = (s) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort().join(',');
const species = JSON.parse(readFileSync('src-tauri/config/species.json', 'utf8')).species;
for (const [name, TABLE, SP, foreign] of [['English', EN, SPECIES_EN, /^/], ['Chinese', ZH, SPECIES_ZH, /[\u4e00-\u9fff]/]]) {
  const missing = [...keys].filter((k) => !(k in TABLE));
  assert.deepEqual(missing, [], `${name} translation missing for:\n${missing.join('\n')}`);
  const dead = Object.keys(TABLE).filter((k) => !keys.has(k));
  assert.deepEqual(dead, [], `${name} table has entries no code uses:\n${dead.join('\n')}`);
  for (const k of keys) {
    assert.equal(placeholders(TABLE[k]), placeholders(k), `${name}: placeholders differ for: ${k}`);
    assert.ok(TABLE[k].trim().length > 0, `${name}: empty translation for: ${k}`);
    assert.ok(!VIET.test(TABLE[k]), `${name} text still contains Vietnamese: ${k}`);
  }
  for (const s of species) {
    const e = SP[s.id];
    assert.ok(e && e.name && e.fact, `${name} name/fact missing for species ${s.id}`);
    assert.ok(!VIET.test(e.name + e.fact), `Vietnamese left in ${name} text for ${s.id}`);
    assert.ok(foreign.test(e.name), `${name} species name looks untranslated for ${s.id}`);
  }
  assert.equal(Object.keys(SP).length, species.length, `${name} species table has species that no longer exist`);
}
console.log(`PASS: ${keys.size} UI sentences and ${species.length} species translated to English and Chinese, placeholders match, no dead entries`);
