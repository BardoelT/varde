// Transactions insights: what was spent per category, stacked, with what came in as a marker, per calendar
// year or per month (CSD-033). Its frame (axis, bars, tooltip, loading) is shared with the other charts in
// shared/yearly-chart.js; this file supplies what is specific to it: the segments, the legend of the
// categories, the labels of a year or a month, and the choice between Yearly and Monthly.
import { $, h } from '../shared/dom.js';
import { formatAmount } from '../shared/format.js';
import { swatch, noSwatch, legendLabel, tooltipRow, topToBottom } from '../shared/chart.js';
import { createYearlyChart, yearLabel } from '../shared/yearly-chart.js';
import { fillLegend, markerDecorator } from '../shared/insights-page.js';
import { initTabs } from '../shared/tabs.js';
import { t, tn } from '../shared/i18n.js';
import { errorText } from '../shared/ipc.js';
import { showToast } from '../shared/dom.js';

/** `yearly` or `monthly`; the page opens on the years and does not remember the choice. */
let granularity = 'yearly';

/** The categories as Rust returned them, by id. */
let categories = new Map();

const keyOf = (id) => `category-${id}`;
const slotOf = (id) => categories.get(id)?.colour_slot ?? 0;

/** The name of a category: the built-in one is translated, the others are the user's own text. */
const categoryName = (id) => (id === 'uncategorized' ? t('Uncategorized') : (categories.get(id)?.name ?? id));

/** "2026" or "2026 YTD" for a year, "2026-10" or "2026-10 MTD" for a month (months are written YYYY-MM). */
function periodLabel(row) {
  if (row.month === null) return yearLabel(row);
  const month = `${row.year}-${String(row.month).padStart(2, '0')}`;
  return row.is_partial ? t('{month} MTD', { month }) : month;
}

function describe(row) {
  if (row.transaction_count === 0) return t('{period}: no transactions', { period: periodLabel(row) });
  const parts = row.segments.map((segment) => `${categoryName(segment.category)} ${formatAmount(segment.amount)}`);
  const text = t('{period}: {parts}, total {total}', { period: periodLabel(row), parts: parts.join(', '), total: formatAmount(row.total) });
  return row.marker ? `${text}, ${t('income {income}', { income: formatAmount(row.income) })}` : text;
}

/** The height of one row of the tooltip and of what is not a row (title, note, total, income, padding). */
const TIP_ROW = 24;
const TIP_FRAME = 150;
/** The fewest category rows a tooltip shows, however low the chart is. */
const MIN_TIP_ROWS = 3;

/**
 * How many category rows the tooltip may show: as many as fit in the height of the chart. A taller
 * tooltip would stick out below the chart, give its scroll box a scrollbar and shift the whole page
 * while the pointer moves over the bars (CSD-033 REQ-8).
 */
function tipRows() {
  const room = Math.floor(($('transactions-insights-chart').clientHeight - TIP_FRAME) / TIP_ROW);
  return Math.max(MIN_TIP_ROWS, room);
}

/** The tooltip of a bar: the rows read like the bar, from the top; rows that do not fit are counted. */
function tooltipContent(row) {
  const ordered = topToBottom(row.segments, (segment) => keyOf(segment.category), row.pieces);
  const limit = tipRows();
  const shown = ordered.length > limit ? ordered.slice(0, limit - 1) : ordered;
  const hidden = ordered.length - shown.length;
  const partial = row.month === null ? t('Partial year: spending so far') : t('Partial month: spending so far');
  return [
    h('div', { class: 'tip-title' }, periodLabel(row)),
    row.is_partial ? h('div', { class: 'tip-note' }, partial) : null,
    row.segments.length === 0 ? h('div', { class: 'tip-note' }, row.transaction_count === 0 ? t('No transactions') : t('No spending')) : null,
    h(
      'dl',
      {},
      ...shown.flatMap((segment) =>
        tooltipRow(keyOf(segment.category), swatch(`slot-${slotOf(segment.category)}`), categoryName(segment.category), formatAmount(segment.amount)),
      ),
      ...(hidden > 0 ? tooltipRow(null, noSwatch(), tn('and {count} more category', 'and {count} more categories', hidden), '') : []),
      ...(row.segments.length === 0 ? [] : tooltipRow(null, noSwatch(), t('Total spent'), formatAmount(row.total))),
      ...(row.marker ? tooltipRow('marker', swatch('marker'), t('Income'), formatAmount(row.income)) : []),
    ),
  ].filter(Boolean);
}

/** How a received piece looks: the colour slot and the name of its category, and its accessible name. */
function pieceStyle(row, piece) {
  const segment = row.segments.find((candidate) => keyOf(candidate.category) === piece.series);
  return {
    cssClass: `bar-slot-${slotOf(segment.category)}`,
    label: `${categoryName(segment.category)}, ${periodLabel(row)}, ${formatAmount(segment.amount)}`,
  };
}

/** The legend: a swatch and the name of every category, also those without spending (CSD-033 REQ-7). */
function buildLegend({ categories: list }) {
  categories = new Map(list.map((category) => [category.id, category]));
  fillLegend(
    'transactions-insights',
    [
      ...list.map((category) => [keyOf(category.id), swatch(`slot-${category.colour_slot}`), legendLabel(categoryName(category.id))]),
      ['marker', swatch('marker'), legendLabel(t('Income'))],
    ],
  );
}

const chart = createYearlyChart({
  id: 'transactions-insights',
  // Read each time the chart loads, so the choice of the toggle decides what Rust is asked.
  get command() {
    return 'get_transaction_insights';
  },
  get args() {
    return { granularity };
  },
  get ariaLabel() {
    return granularity === 'yearly' ? t('Spending and income per year') : t('Spending and income per month');
  },
  onLoaded: buildLegend,
  pieceStyle,
  describe,
  tooltipContent,
  decorate: markerDecorator,
  labelLines: (row) => [{ text: periodLabel(row), className: 'chart-year' }],
});

/** Shows the title and the pressed state of the toggle for the current choice. */
function syncToggle() {
  $('transactions-insights-title').textContent = granularity === 'yearly' ? t('Spending and income per year') : t('Spending and income per month');
  for (const [id, value] of [['transactions-insights-yearly', 'yearly'], ['transactions-insights-monthly', 'monthly']]) {
    $(id).setAttribute('aria-pressed', String(granularity === value));
  }
}

async function choose(value) {
  if (granularity === value) return;
  granularity = value;
  syncToggle();
  try {
    await chart.reload();
  } catch (error) {
    showToast(errorText(error));
  }
}

/** Reloads the chart only while the Insights tab is showing (a project was opened, the language changed). */
export const reloadTransactionsInsightsIfOpen = async () => {
  syncToggle();
  await chart.reloadIfOpen();
};

export function initTransactionsInsights() {
  chart.init();
  // Categories and rules sit left of the Overview, which opens first; the Insights tab recomputes the chart
  // each time it opens (CSD-033 REQ-13).
  initTabs([
    { tab: 'transactions-tab-categories', panel: 'transactions-categories' },
    { tab: 'transactions-tab-overview', panel: 'transactions-overview' },
    { tab: 'transactions-tab-insights', panel: 'transactions-insights', onOpen: chart.reload },
  ]);
  $('transactions-insights-yearly').addEventListener('click', () => choose('yearly'));
  $('transactions-insights-monthly').addEventListener('click', () => choose('monthly'));
  syncToggle();
}
