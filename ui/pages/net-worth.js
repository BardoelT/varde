// Net worth: the yearly stacked chart on the Net worth page (CSD-017). Its frame
// (axis, bars, tooltip, loading) is shared with the other charts in
// shared/yearly-chart.js; this file supplies what is specific to it: the four
// series (houses, investments and cash above the zero line, debts below it) and
// the net worth marker.
import { showToast } from '../shared/dom.js';
import { errorText } from '../shared/ipc.js';
import { formatAmount } from '../shared/format.js';
import { t } from '../shared/i18n.js';
import { swatch, legendLabel, tooltipRow, topToBottom } from '../shared/chart.js';
import { createYearlyChart, isNegative, yearLabel } from '../shared/yearly-chart.js';
import { fillLegend, markerDecorator, tooltipFrame, yearLabelLines } from '../shared/insights-page.js';

/** The series from the bottom of the bar up: the Rust field, the legend key, the colour class and the label (English; shown through `t`). */
const SERIES = [
  ['houses', 'houses', 'nw-houses', 'Houses'],
  ['investments', 'investments', 'nw-investments', 'Investments'],
  ['cash', 'cash', 'nw-cash', 'Cash'],
  ['debts', 'debts', 'nw-debts', 'Debts'],
];

/** Whether Rust drew a piece of the series below the zero line: that is a debt (or an overdrawn bank's cash) with a figure. */
const hangs = (row, field) => row.pieces.some((piece) => piece.series === field && piece.hangs);

/** A debt is shown with a minus sign when it has a figure (CSD-017 REQ-10); the other amounts as they are. */
const shown = (row, field) => (field === 'debts' && hangs(row, 'debts') ? `-${formatAmount(row.debts)}` : formatAmount(row[field]));

/** The series in the order the bar shows them, top to bottom; debts, which hang below the zero line, come last. */
function rowsTopToBottom(row) {
  const [debts] = SERIES.filter(([field]) => field === 'debts');
  const others = SERIES.filter(([field]) => field !== 'debts');
  return [...topToBottom(others, ([field]) => field, row.pieces), debts];
}

function describe(row) {
  if (!row.has_records) return t('{year}: no records', { year: yearLabel(row) });
  const parts = SERIES.map(([field, , , label]) => `${t(label)} ${shown(row, field)}`);
  return t('{year}: {parts}, net worth {worth}', { year: yearLabel(row), parts: parts.join(', '), worth: formatAmount(row.net_worth) });
}

/**
 * The rows read like the bar, top to bottom: cash, investments, houses, then what hangs below
 * the zero line (debts, and cash of an overdrawn bank), and the net worth (CSD-017 REQ-10).
 */
function tooltipContent(row) {
  const rows = row.has_records
    ? [
        ...rowsTopToBottom(row).flatMap(([field, key, cssClass, label]) =>
          tooltipRow(key, swatch(cssClass), t(label), shown(row, field), field === 'cash' && isNegative(row.cash) ? 'negative' : ''),
        ),
        ...tooltipRow('marker', swatch('marker'), t('Net worth'), formatAmount(row.net_worth), isNegative(row.net_worth) ? 'negative' : ''),
      ]
    : [];
  return tooltipFrame({ row, partialNote: t('Partial year: figures so far'), emptyNote: t('No records'), isEmpty: !row.has_records, rows });
}

/** How a received piece looks: the colour class of its series, and its accessible name. */
function pieceStyle(row, piece) {
  const [field, , cssClass, label] = SERIES.find(([key]) => key === piece.series);
  return { cssClass: `bar-${cssClass}`, label: `${t(label)}, ${row.year}, ${shown(row, field)}` };
}

/** The legend: the four series and the net worth marker; hovering one highlights it in every year. */
function buildLegend() {
  fillLegend('net-worth-insights', [
    ...SERIES.map(([, key, cssClass, label]) => [key, swatch(cssClass), legendLabel(t(label))]),
    ['marker', swatch('marker'), legendLabel(t('Net worth'))],
  ]);
}

const chart = createYearlyChart({
  id: 'net-worth-insights',
  command: 'get_net_worth_insights',
  ariaLabel: 'Net worth per year', // shown through `t` by the chart
  onLoaded: buildLegend,
  pieceStyle,
  describe,
  tooltipContent,
  /** The net worth marker, as Rust delivers it. */
  decorate: markerDecorator,
  labelLines: yearLabelLines,
});


/** Reloads the chart only while the page is showing, so a hidden chart is not drawn at zero width. */
export const reloadNetWorthIfOpen = async () => {
  if (document.getElementById('net-worth').classList.contains('active')) await chart.reload();
};

export function initNetWorth() {
  chart.init();
  // Opening the page recomputes it, so it never shows stale figures (CSD-017 REQ-13).
  document.querySelector('[data-section="net-worth"]').addEventListener('click', () => chart.reload().catch((error) => showToast(errorText(error))));
}
