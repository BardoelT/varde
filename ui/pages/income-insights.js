// Income insights: the yearly stacked chart on the Income page (CSD-006). Its
// frame (axis, bars, tooltip, loading) is shared with the other charts in
// shared/yearly-chart.js; this file supplies what is specific to income.
import { h } from '../shared/dom.js';
import { formatAmount } from '../shared/format.js';
import { svg, swatch, noSwatch, legendLabel, tooltipRow } from '../shared/chart.js';
import { createYearlyChart, isNegative, yearLabel } from '../shared/yearly-chart.js';
import { fillLegend, initInsightsTabs, tooltipFrame, yearLabelLines } from '../shared/insights-page.js';
import { t } from '../shared/i18n.js';

/** The stacked series, bottom to top: the Rust field, the legend key (the series of its pieces) and the label (in the chosen language). */
const segments = () => [
  ['gross_salary', 'salary', t('Gross salary')],
  ['holiday_pay', 'holiday', t('Holiday pay')],
  ['bonus', 'bonus', t('Bonus')],
];

/** How a received piece looks: its series is the legend key, which names the colour class. */
const pieceStyle = (row, piece) => ({ cssClass: `bar-${piece.series}` });

function describe(row) {
  const growth =
    row.growth_percent === null ? t('no growth figure') : t('growth {percent}', { percent: row.growth_label });
  return t('{year}: gross salary {salary}, holiday pay {holiday}, bonus {bonus}, total gross {total}, {growth}', {
    year: yearLabel(row),
    salary: formatAmount(row.gross_salary),
    holiday: formatAmount(row.holiday_pay),
    bonus: formatAmount(row.bonus),
    total: formatAmount(row.total_gross),
    growth,
  });
}

/** The rows read like the bar: the top piece first. A year without entries shows the note and no rows. */
function tooltipContent(row) {
  const negative = isNegative(row.growth_label);
  const isEmpty = row.entry_count === 0;
  const rows = isEmpty
    ? []
    : [
        ...segments().reverse().flatMap(([field, key, label]) => tooltipRow(key, swatch(key), label, formatAmount(row[field]))),
        ...tooltipRow(null, noSwatch(), t('Total gross'), formatAmount(row.total_gross)),
        ...tooltipRow(null, noSwatch(), t('Growth'), row.growth_label, negative ? 'negative' : ''),
      ];
  return tooltipFrame({ row, partialNote: t('Partial year: entries so far'), emptyNote: t('No entries'), isEmpty, rows });
}

/** The legend: one item per stacked series; hovering one highlights it in every year. */
function buildLegend() {
  fillLegend('income-insights', segments().map(([, key, label]) => [key, swatch(key), legendLabel(label)]));
}

const chart = createYearlyChart({
  id: 'income-insights',
  command: 'get_income_insights',
  get ariaLabel() {
    return t('Income per year');
  },
  onLoaded: buildLegend,
  pieceStyle,
  describe,
  tooltipContent,
  /** The growth percentage above the bar. */
  decorate(row, { centre, barTop }) {
    const negative = isNegative(row.growth_label);
    return [svg('text', { class: `chart-growth${negative ? ' negative' : ''}`, x: centre, y: barTop - 6 }, row.growth_label)];
  },
  labelLines: yearLabelLines,
});

/** Reloads the chart only while the Insights tab is showing (after a project is opened). */
export const reloadIncomeInsightsIfOpen = chart.reloadIfOpen;

export function initIncomeInsights() {
  chart.init();
  initInsightsTabs('income', chart); // opening Insights recomputes it (CSD-006 REQ-11)
}
