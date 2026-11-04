// Expense insights: the yearly stacked chart on the Expenses page (CSD-014), one
// segment per category. Its frame (axis, bars, tooltip, loading) is shared with the other
// charts in shared/yearly-chart.js; this file supplies what is specific to expenses: the
// segments and the legend of the eight categories.
import { formatAmount } from '../shared/format.js';
import { swatch, noSwatch, legendLabel, tooltipRow, topToBottom } from '../shared/chart.js';
import { createYearlyChart, yearLabel } from '../shared/yearly-chart.js';
import { fillLegend, initInsightsTabs, tooltipFrame, yearLabelLines } from '../shared/insights-page.js';
import { t } from '../shared/i18n.js';

/** The categories as Rust returned them, in the fixed order, by name. */
let categories = new Map();

/** The category as Rust names it (a fixed list), in the chosen language. */
function categoryName(category) {
  switch (category) {
    case 'Home': return t('Home');
    case 'Vehicle': return t('Vehicle');
    case 'Holiday': return t('Holiday');
    case 'Sport': return t('Sport');
    case 'Study': return t('Study');
    case 'Appliances': return t('Appliances');
    case 'Hobby': return t('Hobby');
    case 'Other': return t('Other');
    default: return category;
  }
}

const keyOf = (category) => `category-${category}`;
const slotOf = (category) => categories.get(category)?.colour_slot ?? 0;

function describe(row) {
  if (row.expense_count === 0) return t('{year}: no expenses', { year: yearLabel(row) });
  const parts = row.segments.map((segment) => `${categoryName(segment.category)} ${formatAmount(segment.amount)}`);
  return t('{year}: {parts}, total {total}', { year: yearLabel(row), parts: parts.join(', '), total: formatAmount(row.total) });
}

/** The rows read like the bar: the highest segment first. */
function tooltipContent(row) {
  return tooltipFrame({
    row,
    partialNote: t('Partial year: expenses so far'),
    emptyNote: t('No expenses'),
    isEmpty: row.expense_count === 0,
    rows: [
      ...topToBottom(row.segments, (segment) => keyOf(segment.category), row.pieces).flatMap((segment) =>
        tooltipRow(keyOf(segment.category), swatch(`slot-${slotOf(segment.category)}`), categoryName(segment.category), formatAmount(segment.amount)),
      ),
      ...(row.expense_count === 0 ? [] : tooltipRow(null, noSwatch(), t('Total'), formatAmount(row.total))),
    ],
  });
}

/** How a received piece looks: the colour slot and the name of its category, and its accessible name. */
function pieceStyle(row, piece) {
  const segment = row.segments.find((candidate) => keyOf(candidate.category) === piece.series);
  return {
    cssClass: `bar-slot-${slotOf(segment.category)}`,
    label: `${categoryName(segment.category)}, ${row.year}, ${formatAmount(segment.amount)}`,
  };
}

/** The legend: a swatch and the name of every category, also those without an expense (CSD-014 REQ-7). */
function buildLegend({ categories: list }) {
  categories = new Map(list.map((category) => [category.name, category]));
  fillLegend(
    'expenses-insights',
    list.map((category) => [keyOf(category.name), swatch(`slot-${category.colour_slot}`), legendLabel(categoryName(category.name))]),
  );
}

const chart = createYearlyChart({
  id: 'expenses-insights',
  command: 'get_expense_insights',
  get ariaLabel() {
    return t('Expenses per year');
  },
  onLoaded: buildLegend,
  pieceStyle,
  describe,
  tooltipContent,
  decorate: () => [],
  labelLines: yearLabelLines,
});

/** Reloads the chart only while the Insights tab is showing (after a project is opened). */
export const reloadExpensesInsightsIfOpen = chart.reloadIfOpen;

export function initExpensesInsights() {
  chart.init();
  initInsightsTabs('expenses', chart); // opening Insights recomputes it (CSD-014 REQ-13)
}
