// Helpers of the AC tests of the investment and bank insights charts: canned yearly series as Rust
// returns them (pieces, markers, labels) and readers of the drawn SVG. The chart code draws what it
// is given, so each test hands it the series of its own scenario.
import assert from 'node:assert/strict';
import { setTimeout as sleep } from 'node:timers/promises';

export { sleep };

/** A piece as Rust delivers it. */
export const piece = (series, low, high, hangs = false, rounding = 'none') => ({ series, low, high, hangs, rounding });

const cents = (value) => Number(value).toFixed(2);

/** The waterfall of `[series, amount]` parts, the way Rust stacks them: a negative part hangs from the running total. */
export function waterfall(parts) {
  let running = 0;
  const pieces = [];
  for (const [series, text] of parts) {
    const amount = Number(text);
    if (amount === 0) continue;
    if (amount > 0) pieces.push(piece(series, cents(running), cents(running + amount)));
    else pieces.push(piece(series, cents(running + amount), cents(running), true));
    running += amount;
  }
  return pieces;
}

/** The short label of a marker, such as "12.50k". */
const shortLabel = (value) => `${(Number(value) / 1000).toFixed(2)}k`;

/**
 * One year of the investment insights answer. `opening`, `deposits` and `growth` are amounts; the
 * pieces, the closing balance and the marker follow from them as in Rust. A year without statements
 * has none of them.
 */
export function investmentYear({ year, opening = '0.00', deposits = '0.00', growth = '0.00', label = '–', partial = false, count = 1 }) {
  const closing = cents(Number(opening) + Number(deposits) + Number(growth));
  const empty = count === 0;
  const growthKey = Number(growth) < 0 ? 'growth-down' : 'growth-up';
  return {
    year,
    opening_balance: empty ? '0.00' : opening,
    net_deposits: empty ? '0.00' : deposits,
    capital_growth: empty ? '0.00' : growth,
    closing_balance: empty ? '0.00' : closing,
    growth_label: label,
    growth_percent: null,
    is_partial: partial,
    statement_count: count,
    pieces: empty ? [] : waterfall([['opening', opening], ['deposits', deposits], [growthKey, growth]]),
    marker: empty ? null : { label: shortLabel(closing), value: closing },
  };
}

export const axisOf = (...values) => ({ ticks: values.map((value) => ({ value: cents(value), label: value === 0 ? '0' : `${value / 1000}k` })) });

// ---- reading the drawn chart ----

export const groups = (chartNode) => [...chartNode.querySelectorAll('.chart-year-group')];
export const groupOf = (chartNode, year) => groups(chartNode).find((group) => group.querySelector('.chart-year')?.textContent.startsWith(String(year)));
export const shapes = (group) => [...group.querySelectorAll('rect[data-legend], path[data-legend]')];
export const num = (node, name) => Number(node.getAttribute(name));
export const tooltipOf = (chartNode) => chartNode.querySelector('.chart-tooltip');
/** The top and the bottom of a piece, as y positions. */
export const span = (shape) => ({ top: num(shape, 'y') - 0.5, bottom: num(shape, 'y') + num(shape, 'height') + 0.5 });
export const near = (actual, expected, message) => assert.ok(Math.abs(actual - expected) < 0.01, `${message}: ${actual} is not ${expected}`);

/** The y position of a value, read from the gridlines: the first and the last tick are the ends of the axis. */
export function yScale(chartNode, axis) {
  const lines = [...chartNode.querySelectorAll('.chart-grid, .chart-zero')].map((line) => num(line, 'y1'));
  const first = Number(axis.ticks[0].value);
  const last = Number(axis.ticks.at(-1).value);
  return (value) => lines[0] + ((Number(value) - first) / (last - first)) * (lines.at(-1) - lines[0]);
}

export async function hover(app, group) {
  group.dispatchEvent(new app.window.MouseEvent('mouseenter'));
  await app.settle();
}

export const over = (app, node) => node.dispatchEvent(new app.window.MouseEvent('mouseover', { bubbles: true }));

/** The rows of the open tooltip as `{ label, value, swatch }`, top to bottom. */
export function tooltipRows(app, chartNode) {
  return [...tooltipOf(chartNode).querySelectorAll('dl > dt')].map((dt) => ({
    label: app.text(dt.querySelector('.legend-label')),
    value: app.text(dt.nextElementSibling),
    swatch: [...(dt.querySelector('.swatch')?.classList ?? [])].filter((name) => name !== 'swatch').join(' ') || null,
  }));
}

/** Lets the tooltip's closing timer run out (the tooltip closes 80 ms after the pointer or focus leaves). */
export const afterTooltipCloses = () => sleep(150);
