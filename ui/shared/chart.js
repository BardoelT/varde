// Small helpers shared by the SVG charts (CSD-006 and CSD-007). They draw what Rust
// returned (CSD-025): positions, shapes, the tooltip and the legend highlight. Figures
// and labels come from Rust and are not computed here.

import { h } from './dom.js';

const SVG_NS = 'http://www.w3.org/2000/svg';

export function svg(tag, attrs = {}, ...children) {
  const node = document.createElementNS(SVG_NS, tag);
  for (const [key, value] of Object.entries(attrs)) node.setAttribute(key, value);
  node.append(...children);
  return node;
}

/**
 * The one place a received decimal string becomes a number: for a position on the chart and
 * nothing else (CSD-025 REQ-5). Nothing shown is ever built from its result.
 */
export const toPosition = (text) => Number.parseFloat(text);

const clamp = (value, low, high) => Math.min(Math.max(value, low), Math.max(high, low));

const BAR_GAP = 1;
const BAR_RADIUS = 3;

/**
 * A bar segment from `yTop` down to `yBottom`, inset by half the 1 px gap so
 * stacked segments are separated by a gap and not by an outline. The topmost
 * segment of a bar has rounded top corners (CSD-006 REQ-16, CSD-007 REQ-18);
 * `rounded` is `true` or 'top' for that, and 'bottom' for the lowest segment
 * that hangs below the zero line (CSD-010 REQ-13).
 */
export function segmentShape(attrs, x, width, yTop, yBottom, { rounded = false } = {}) {
  const y = yTop + BAR_GAP / 2;
  const height = Math.max(yBottom - yTop - BAR_GAP, 0.5);
  if (!rounded) return svg('rect', { ...attrs, x, y, width, height });
  const r = Math.min(BAR_RADIUS, width / 2, height);
  const bottom = y + height;
  const d =
    rounded === 'bottom'
      ? `M${x},${y} H${x + width} V${bottom - r} Q${x + width},${bottom} ${x + width - r},${bottom} H${x + r} Q${x},${bottom} ${x},${bottom - r} Z`
      : `M${x},${bottom} V${y + r} Q${x},${y} ${x + r},${y} H${x + width - r} Q${x + width},${y} ${x + width},${y + r} V${bottom} Z`;
  return svg('path', { ...attrs, d });
}

/** A legend swatch for the series `key`, or a spacer that keeps rows without one aligned. */
export const swatch = (key) => h('span', { class: `swatch ${key}` });
export const noSwatch = () => h('span', { class: 'swatch-spacer' });

/**
 * A tooltip row, as the `dt` and `dd` that go in the tooltip's list. `legend` ties it to the legend
 * item of the same key, so it is highlighted with it; `null` for a row that has no legend item.
 */
export function tooltipRow(legend, swatchNode, label, value, cssClass = '') {
  return [
    h('dt', { 'data-legend': legend }, swatchNode, legendLabel(label)),
    h('dd', { 'data-legend': legend, class: cssClass }, value),
  ];
}

/**
 * The items in the order their bars show them from top to bottom: the ones that rise from the
 * zero line, highest first, then the ones that hang below it, nearest first. A tooltip lists
 * its rows in this order, so it reads like the bar. Whether an item hangs is what Rust says of
 * its piece (`hangs`, matched by the legend key `seriesOf(item)`); an item without a piece
 * (a value of 0.00) rises. No amount is compared.
 */
export function topToBottom(items, seriesOf, pieces) {
  const hanging = new Set(pieces.filter((piece) => piece.hangs).map((piece) => piece.series));
  const hangs = (item) => hanging.has(seriesOf(item));
  return [...items.filter((item) => !hangs(item)).reverse(), ...items.filter(hangs)];
}

/** A legend label whose heavier weight takes no extra width, so highlighting never moves its neighbours. */
export const legendLabel = (text) => h('span', { class: 'legend-label', 'data-text': text }, text);

/**
 * The one tooltip of a chart. It fades in and slides 4 px, and glides to the
 * next year when the user moves along the bars (CSD-006 REQ-14, CSD-007 REQ-16).
 * It sits beside the bar, to its right, or to its left when there is no room,
 * and inside the chart; without a bar it is centred over the slot.
 */
export function createTooltip(inner) {
  const element = h('div', { class: 'chart-tooltip', role: 'presentation' });
  let closeTimer = null;

  function place(bar, slot) {
    const box = inner.getBoundingClientRect();
    const width = element.offsetWidth;
    const height = element.offsetHeight;
    if (!bar) {
      const slotBox = slot.getBoundingClientRect();
      const centre = slotBox.left - box.left + slotBox.width / 2;
      element.style.left = `${clamp(centre - width / 2, 0, box.width - width)}px`;
      element.style.top = '0px';
      element.style.setProperty('--slide-x', '0px');
      element.style.setProperty('--slide-y', '4px');
      return;
    }
    const edge = bar.getBoundingClientRect();
    const gap = 8;
    let left = edge.right - box.left + gap;
    let slide = '-4px';
    if (left + width > box.width) {
      left = edge.left - box.left - gap - width;
      slide = '4px';
    }
    element.style.left = `${clamp(left, 0, box.width - width)}px`;
    element.style.top = `${clamp(edge.top - box.top, 0, box.height - height)}px`;
    element.style.setProperty('--slide-x', slide);
    element.style.setProperty('--slide-y', '0px');
  }

  return {
    element,
    show(content, bar, slot) {
      clearTimeout(closeTimer);
      const wasOpen = element.classList.contains('is-open');
      // A tooltip that was closed appears in place; only an open one glides.
      if (!wasOpen) element.classList.add('no-glide');
      element.replaceChildren(...content);
      place(bar, slot);
      if (!wasOpen) {
        void element.offsetWidth;
        element.classList.remove('no-glide');
      }
      element.classList.add('is-open');
    },
    /** Closes after a moment, so moving to the next year glides instead of closing and reopening. */
    hide() {
      clearTimeout(closeTimer);
      closeTimer = setTimeout(() => element.classList.remove('is-open'), 80);
    },
  };
}

/**
 * Highlights the legend item `key`, or nothing for `null`: in the panel legend,
 * the tooltip and the bar. Items are found by `data-legend` (CSD-006 REQ-13,
 * CSD-007 REQ-15).
 */
export function setHighlight(key, scopes) {
  for (const scope of scopes) {
    for (const item of scope.querySelectorAll('[data-legend]')) {
      item.classList.toggle('hl', key !== null && item.dataset.legend === key);
      item.classList.toggle('dim', key !== null && item.dataset.legend !== key);
    }
  }
}

/**
 * A legend item for the series `key`. Hovering or focusing it highlights that series in every
 * year, in the panel legend and in the chart (`scopes()` gives the elements to search).
 */
export function legendItem(key, swatchNode, labelNode, scopes) {
  const item = h('li', { 'data-legend': key, tabindex: '0' }, swatchNode, labelNode);
  for (const on of ['mouseenter', 'focus']) item.addEventListener(on, () => setHighlight(key, scopes()));
  for (const off of ['mouseleave', 'blur']) item.addEventListener(off, () => setHighlight(null, scopes()));
  return item;
}

/** A legend label for a name that may be long: it is cut with an ellipsis, the full name stays in the text. */
export const nameLabel = (name) =>
  h('span', { class: 'legend-label legend-name', 'data-text': name }, h('span', { class: 'legend-name-text' }, name));

/** Highlights the legend item of the segment under the mouse; nothing elsewhere or on leaving. */
export function bindHighlight(group, scopes) {
  group.addEventListener('mouseover', (event) => {
    const target = event.target.closest?.('[data-legend]');
    setHighlight(target && group.contains(target) ? target.dataset.legend : null, scopes);
  });
  group.addEventListener('mouseleave', () => setHighlight(null, scopes));
  group.addEventListener('focus', () => setHighlight(null, scopes));
}
