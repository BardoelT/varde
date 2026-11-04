// The dark design tokens are written twice in base.css, once for the system setting (@media) and
// once for an explicit choice ([data-theme="dark"]); CSS cannot share them. This guards the copies.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const css = readFileSync(fileURLToPath(new URL('../styles/base.css', import.meta.url)), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');

/** The declarations of the block that follows `selector`, as sorted `name: value` strings. */
function declarations(selector) {
  const start = css.indexOf(selector);
  assert.ok(start >= 0, `${selector} exists`);
  const body = css.slice(css.indexOf('{', start) + 1, css.indexOf('}', start));
  return body.split(';').map((part) => part.replace(/\s+/g, ' ').trim()).filter(Boolean).sort();
}

test('test_ui_the_two_dark_token_blocks_are_identical', () => {
  const system = declarations(':root:not([data-theme="light"])');
  const chosen = declarations(':root[data-theme="dark"]');
  assert.ok(system.length > 10);
  assert.deepEqual(system, chosen);
});
