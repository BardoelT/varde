// Language of the app's own texts, English or Dutch (CSD-022). A text is written in English
// where it is used, and that English text is also the key of its Dutch version in `locales/`.
// A text without a Dutch version shows in English (REQ-6). Project data is never translated.
import nl from '../locales/nl.js';

const DICTIONARIES = { nl };
const ATTRIBUTES = ['placeholder', 'aria-label', 'title', 'alt'];

let language = 'en';
const listeners = [];
/** The static texts of index.html, found once at start: `{ node, attribute, source, original, shown }`. */
const staticTexts = [];

/** The code of the language that is shown: `en` or `nl`. */
export const currentLanguage = () => language;

/** Fills `{name}` places in `text` with `params`. */
function fill(text, params) {
  if (!params) return text;
  return text.replace(/\{(\w+)\}/g, (place, name) => (name in params ? String(params[name]) : place));
}

/** The text of `key` in the chosen language, English when it has no translation (REQ-6). */
function lookup(key) {
  return DICTIONARIES[language]?.[key] ?? null;
}

/** Translates an English text, filling `{name}` places from `params`. */
export function t(text, params) {
  return fill(lookup(text) ?? text, params);
}

/** Singular or plural text by `count`; `{count}` is filled in. Both are English texts with Dutch versions. */
export function tn(singular, plural, count, params) {
  return t(count === 1 ? singular : plural, { count, ...params });
}

/**
 * The text of an error as the chosen language gives it: the Dutch text of its code when there is
 * one, otherwise the English message that Rust sent (CSD-022 REQ-7).
 */
export function errorMessage(error) {
  const translated = error && typeof error.code === 'string' ? lookup(`code:${error.code}`) : null;
  if (translated) return translated;
  if (error && typeof error.message === 'string' && error.message) return error.message;
  return t('Something went wrong. Please try again.');
}

/** The key of a static text: its words with the white space of the markup collapsed. */
const normalise = (text) => text.replace(/\s+/g, ' ').trim();

/** Notes every static text of the page once, before any script writes project data into it. */
export function collectStaticTexts(root = document.body) {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT | NodeFilter.SHOW_ELEMENT);
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    if (node.nodeType === Node.TEXT_NODE) {
      const parent = node.parentElement;
      const source = normalise(node.data);
      if (source && !['SCRIPT', 'STYLE'].includes(parent.tagName)) {
        staticTexts.push({ node, attribute: null, source, original: node.data, shown: node.data });
      }
    } else {
      for (const attribute of ATTRIBUTES) {
        const value = node.getAttribute(attribute);
        if (value && normalise(value)) staticTexts.push({ node, attribute, source: normalise(value), original: value, shown: value });
      }
    }
  }
}

/** Shows the static texts in the chosen language. A text a script has changed since is left alone. */
function showStaticTexts() {
  for (const entry of staticTexts) {
    const { node, attribute, source, original } = entry;
    if (!node.isConnected) continue;
    const current = attribute ? node.getAttribute(attribute) : node.data;
    if (current !== entry.shown) continue;
    const translated = lookup(source);
    // Keep the white space around the words as the markup had it.
    const next = translated === null ? original : original.replace(/^(\s*)[\s\S]*?(\s*)$/, (_all, lead, trail) => lead + translated + trail);
    if (attribute) node.setAttribute(attribute, next);
    else node.data = next;
    entry.shown = next;
  }
}

/** Calls `listener` after the language has changed, to draw the pages again. */
export function onLanguageChange(listener) {
  listeners.push(listener);
}

/** Switches to `code` (`en` or `nl`) and redraws the static texts; the listeners redraw the pages. */
export async function setLanguage(code) {
  language = code === 'nl' ? 'nl' : 'en';
  document.documentElement.lang = language;
  showStaticTexts();
  for (const listener of listeners) await listener(language);
}
