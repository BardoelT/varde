// The CSP of the app has no 'unsafe-inline' for styles (CSD-021 CON-3, CLAUDE.md Tauri security).
// jsdom does not enforce a CSP, so these tests guard the sources instead: no `style` attribute and
// no `<style>` block in the page, and no script code that writes a style attribute as text.
// Setting `element.style.x` through the CSSOM stays allowed under such a CSP.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { JSDOM } from 'jsdom';

const uiDir = fileURLToPath(new URL('..', import.meta.url));
const confPath = fileURLToPath(new URL('../../src-tauri/tauri.conf.json', import.meta.url));

/** Every `.js` file of the app (not of the tests). */
function appScripts(dir = uiDir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    if (entry.isDirectory()) return entry.name === 'tests' ? [] : appScripts(`${dir}${entry.name}/`);
    return entry.name.endsWith('.js') ? [`${dir}${entry.name}`] : [];
  });
}

test('test_ui_csp_index_html_has_no_style_attribute_or_style_block', () => {
  const dom = new JSDOM(readFileSync(`${uiDir}index.html`, 'utf8'));
  assert.deepEqual([...dom.window.document.querySelectorAll('[style]')].map((el) => el.outerHTML.slice(0, 60)), []);
  assert.equal(dom.window.document.querySelectorAll('style').length, 0);
});

test('test_ui_csp_scripts_write_no_style_attribute_or_style_element', () => {
  const offenders = appScripts().filter((file) => {
    const source = readFileSync(file, 'utf8');
    return /style\s*=\s*["'\\]|setAttribute\(\s*['"]style['"]|cssText|createElement\(\s*['"]style['"]|<style/.test(source);
  });
  assert.deepEqual(offenders, []);
});

test('test_ui_csp_tauri_conf_style_src_has_no_unsafe_inline', () => {
  const csp = JSON.parse(readFileSync(confPath, 'utf8')).app.security.csp;
  const styleSrc = csp.split(';').map((part) => part.trim()).find((part) => part.startsWith('style-src'));
  assert.equal(styleSrc, "style-src 'self'");
});
