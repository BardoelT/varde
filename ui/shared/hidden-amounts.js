// Marks every hidden amount the pages write while Discrete mode is on (CSD-030). The pages write
// the placeholder through `formatAmount`; this file gives each one its blur (a class, styled in
// CSS) and the words screen readers read instead of the bullets (REQ-8, OQ-2), in the text and in
// the `aria-label` and `title` attributes. Only the placeholder is wrapped, so a text that holds an
// amount and a percentage keeps the percentage readable (REQ-6).
import { HIDDEN, onDiscreteModeChange } from './discrete-mode.js';
import { t } from './i18n.js';

const SVG_NS = 'http://www.w3.org/2000/svg';
const CLASS = 'hidden-amount';
/** A placeholder with the sign that may stand before it. */
// The Unicode minus is built from its number: no source of the app contains it (CSD-018 AC-10.1).
const AMOUNT = new RegExp(`[-${String.fromCharCode(0x2212)}+]?${HIDDEN}`, 'g');
const ATTRIBUTES = ['aria-label', 'title'];

let observer = null;

const inside = (node) => Boolean(node.parentElement?.closest(`.${CLASS}`));

/** Replaces the placeholders of the text node by marked elements. */
function wrapText(node) {
  const text = node.data;
  if (!text.includes(HIDDEN) || inside(node)) return;
  const parent = node.parentNode;
  const isSvg = parent.namespaceURI === SVG_NS;
  const parts = [];
  let last = 0;
  for (const match of text.matchAll(AMOUNT)) {
    if (match.index > last) parts.push(document.createTextNode(text.slice(last, match.index)));
    const mark = isSvg ? document.createElementNS(SVG_NS, 'tspan') : document.createElement('span');
    mark.setAttribute('class', CLASS);
    if (!isSvg) {
      mark.setAttribute('role', 'img');
      mark.setAttribute('aria-label', t('hidden amount'));
    }
    mark.append(HIDDEN);
    parts.push(mark);
    last = match.index + match[0].length;
  }
  if (last < text.length) parts.push(document.createTextNode(text.slice(last)));
  node.replaceWith(...parts);
}

/** Replaces the placeholders in the attributes screen readers and tooltips read. */
function readableAttributes(element) {
  for (const name of ATTRIBUTES) {
    const value = element.getAttribute(name);
    if (value?.includes(HIDDEN)) element.setAttribute(name, value.replace(AMOUNT, t('hidden amount')));
  }
}

/** Marks everything below `node`, and `node` itself. */
function mark(node) {
  if (node.nodeType === Node.TEXT_NODE) {
    wrapText(node);
  } else if (node.nodeType === Node.ELEMENT_NODE && !node.classList.contains(CLASS)) {
    readableAttributes(node);
    for (const child of [...node.childNodes]) mark(child);
  }
}

function start() {
  if (observer) return;
  observer = new MutationObserver((records) => {
    for (const record of records) {
      if (record.type === 'attributes') readableAttributes(record.target);
      else if (record.type === 'characterData') mark(record.target);
      else for (const added of record.addedNodes) mark(added);
    }
  });
  observer.observe(document.body, {
    childList: true,
    subtree: true,
    characterData: true,
    attributes: true,
    attributeFilter: ATTRIBUTES,
  });
  mark(document.body);
}

function stop() {
  observer?.disconnect();
  observer = null;
}

onDiscreteModeChange((on) => (on ? start() : stop()));
