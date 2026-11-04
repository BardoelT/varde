// What the Insights pages share: the legend, the tooltip frame, the tabs, the year label and the
// marker of the yearly charts, and `createSeriesInsights`, the whole page of an area built from
// named accounts with a record per period (bank accounts, debts, houses). The frame of the chart
// itself (axis, bars, tooltip, loading) is in yearly-chart.js; a page only says how its series look.
import { $, h } from './dom.js';
import { formatAmount } from './format.js';
import { swatch, legendLabel, nameLabel, legendItem, tooltipRow, topToBottom } from './chart.js';
import { createYearlyChart, levelMarker, isNegative, yearLabel } from './yearly-chart.js';
import { initTabs } from './tabs.js';

/** The label under a bar: the year (`labelLines` of the chart). */
export const yearLabelLines = (row) => [{ text: yearLabel(row), className: 'chart-year' }];

/** The marker line of a year as Rust delivers it, or nothing without one (`decorate` of the chart). */
export const markerDecorator = (row, geometry) => (row.marker ? levelMarker(row.marker, geometry) : []);

/**
 * Fills the legend of the chart `id` (`bank-insights`) from `[key, swatchNode, labelNode]` items.
 * Hovering or focusing an item highlights its series in the legend and in the chart.
 */
export function fillLegend(id, items) {
  const legendList = $(`${id}-legend`);
  const scopes = () => [legendList, $(`${id}-chart`)];
  legendList.replaceChildren(...items.map(([key, swatchNode, label]) => legendItem(key, swatchNode, label, scopes)));
}

/**
 * The tooltip of a year: its title, a note while the year is partial, a note when it has no
 * records, and the list of `rows`. The notes are texts already in the chosen language.
 */
export function tooltipFrame({ row, partialNote, emptyNote, isEmpty, rows }) {
  return [
    h('div', { class: 'tip-title' }, yearLabel(row)),
    row.is_partial ? h('div', { class: 'tip-note' }, partialNote) : null,
    isEmpty ? h('div', { class: 'tip-note' }, emptyNote) : null,
    h('dl', {}, ...rows),
  ].filter(Boolean);
}

/**
 * The Overview and Insights tabs of the page `prefix` (`bank`). Opening Insights recomputes the
 * chart, so it never shows stale figures.
 */
export function initInsightsTabs(prefix, chart) {
  initTabs([
    { tab: `${prefix}-tab-overview`, panel: `${prefix}-overview` },
    { tab: `${prefix}-tab-insights`, panel: `${prefix}-insights`, onOpen: chart.reload },
  ]);
}

/**
 * Builds the Insights page of a series area: one stacked segment per account, a legend of the
 * accounts and a marker for the closing figure. Options:
 * - `prefix` (`bank`) names the page ids; `command` is the IPC command; `listKey` the field of its
 *   answer that lists the accounts (`banks`); `keyPrefix` (`bank-`) starts every legend key.
 * - `segmentId`, `amount`, `count` and `closing` are the fields of a segment (its account id and
 *   amount), of a year (how many records it has) and of a year (its closing figure).
 * - `chartLabel()`, `markerLabel()`, `partialNote()` and `emptyNote()` give the texts, in the
 *   chosen language, each time they are needed.
 * - `describe(row, nameOf)` is the screen-reader text of a year; `extraRows(row)` (optional) adds
 *   tooltip rows after the closing figure; `flagNegative` marks negative amounts in the tooltip.
 * Returns `{ init, reloadIfOpen }`.
 */
export function createSeriesInsights(config) {
  const { prefix, command, listKey, keyPrefix, segmentId, amount, count, closing } = config;
  const { chartLabel, markerLabel, partialNote, emptyNote, describe, extraRows = () => [], flagNegative = false } = config;
  const id = `${prefix}-insights`;
  /** The accounts of the series by id, as Rust returned them (creation order). */
  let items = new Map();

  const keyOf = (accountId) => `${keyPrefix}${accountId}`;
  const nameOf = (segment) => items.get(segment[segmentId])?.name ?? '';
  const slotOf = (accountId) => items.get(accountId)?.colour_slot ?? 0;
  const negativeClass = (value) => (flagNegative && isNegative(value) ? 'negative' : '');

  /** The rows read like the bar: the highest segment first, those that hang below the zero line last. */
  function tooltipContent(row) {
    const isEmpty = row[count] === 0;
    const rows = isEmpty
      ? []
      : [
          ...topToBottom(row.segments, (segment) => keyOf(segment[segmentId]), row.pieces).flatMap((segment) =>
            tooltipRow(
              keyOf(segment[segmentId]),
              swatch(`slot-${slotOf(segment[segmentId])}`),
              nameOf(segment),
              formatAmount(segment[amount]),
              negativeClass(segment[amount]),
            ),
          ),
          ...tooltipRow('marker', swatch('marker'), markerLabel(), formatAmount(row[closing]), negativeClass(row[closing])),
          ...extraRows(row),
        ];
    return tooltipFrame({ row, partialNote: partialNote(), emptyNote: emptyNote(), isEmpty, rows });
  }

  /** How a received piece looks: the colour slot and the name of its account, and its accessible name. */
  function pieceStyle(row, piece) {
    const segment = row.segments.find((candidate) => keyOf(candidate[segmentId]) === piece.series);
    return {
      cssClass: `bar-slot-${slotOf(segment[segmentId])}`,
      label: `${nameOf(segment)}, ${row.year}, ${formatAmount(segment[amount])}`,
    };
  }

  /** The legend: a swatch and the name of every account in the series, and the closing marker. */
  function buildLegend(answer) {
    const list = answer[listKey];
    items = new Map(list.map((item) => [item.id, item]));
    fillLegend(id, [
      ...list.map((item) => [keyOf(item.id), swatch(`slot-${item.colour_slot}`), nameLabel(item.name)]),
      ['marker', swatch('marker'), legendLabel(markerLabel())],
    ]);
  }

  const chart = createYearlyChart({
    id,
    command,
    // A getter, so the name follows the language when the chart is drawn again.
    get ariaLabel() {
      return chartLabel();
    },
    onLoaded: buildLegend,
    pieceStyle,
    describe: (row) => describe(row, nameOf),
    tooltipContent,
    decorate: markerDecorator,
    labelLines: yearLabelLines,
  });

  return {
    /** Reloads the chart only while the Insights tab is showing (after a project is opened). */
    reloadIfOpen: chart.reloadIfOpen,
    init() {
      chart.init();
      initInsightsTabs(prefix, chart);
    },
  };
}
