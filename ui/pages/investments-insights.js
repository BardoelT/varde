// Investment insights: the yearly stacked chart on the Investments page
// (CSD-007). Its frame (axis, bars, tooltip, loading) is shared with the other
// charts in shared/yearly-chart.js; this file supplies what is specific to
// investments: the colour of each piece, the tooltip rows and the growth
// label, all as Rust returns them.
import { h } from '../shared/dom.js';
import { t } from '../shared/i18n.js';
import { formatAmount, formatLabel } from '../shared/format.js';
import { swatch, noSwatch, legendLabel, tooltipRow } from '../shared/chart.js';
import { createYearlyChart, isNegative, yearLabel } from '../shared/yearly-chart.js';
import { fillLegend, initInsightsTabs, markerDecorator } from '../shared/insights-page.js';

/** The legend items in the order of the bar from the bottom: key and label. A loss and a gain are two items. */
const legendItems = () => [
  ['opening', t('Opening balance')],
  ['deposits', t('Net deposits')],
  ['growth-up', t('Capital growth (gain)')],
  ['growth-down', t('Capital growth (loss)')],
];

function describe(row) {
  if (row.statement_count === 0) return t('{year}: no statements', { year: yearLabel(row) });
  const amount = formatAmount(row.capital_growth);
  const growth = isNegative(row.capital_growth) ? t('{amount} (negative)', { amount }) : amount;
  return t('{year}: opening balance {opening}, net deposits {deposits}, capital growth {growth}, growth {percent}, closing balance {closing}', {
    year: yearLabel(row),
    opening: formatAmount(row.opening_balance),
    deposits: formatAmount(row.net_deposits),
    growth,
    percent: formatLabel(row.growth_label),
    closing: formatAmount(row.closing_balance),
  });
}

/** The rows read like the bar, top to bottom: capital growth, net deposits, opening balance. */
function tooltipContent(row) {
  const growthKey = isNegative(row.capital_growth) ? 'growth-down' : 'growth-up';
  const rows =
    row.statement_count === 0
      ? []
      : [
          ...tooltipRow(growthKey, swatch(growthKey), t('Capital growth'), formatAmount(row.capital_growth), isNegative(row.capital_growth) ? 'negative' : ''),
          ...tooltipRow('deposits', swatch('deposits'), t('Net deposits'), formatAmount(row.net_deposits)),
          ...tooltipRow('opening', swatch('opening'), t('Opening balance'), formatAmount(row.opening_balance)),
          ...tooltipRow('marker', swatch('marker'), t('Closing balance'), formatAmount(row.closing_balance)),
          ...tooltipRow(null, noSwatch(), t('Growth'), formatLabel(row.growth_label), isNegative(row.growth_label) ? 'negative' : ''),
        ];
  return [
    h('div', { class: 'tip-title' }, yearLabel(row)),
    row.statement_count === 0 ? h('div', { class: 'tip-note' }, t('No statements')) : null,
    h('dl', {}, ...rows),
  ].filter(Boolean);
}

/** How a received piece looks; a net-deposits piece that hangs is a withdrawal. */
const pieceStyle = (row, piece) => ({
  cssClass: `bar-${piece.series}${piece.series === 'deposits' && piece.hangs ? ' withdrawal' : ''}`,
});

/** The legend: one item per kind of piece, and the closing-balance marker; hovering one highlights it in every year. */
function buildLegend() {
  fillLegend('investments-insights', [
    ...legendItems().map(([key, label]) => [key, swatch(key), legendLabel(label)]),
    ['marker', swatch('marker'), legendLabel(t('Closing balance'))],
  ]);
}

const chart = createYearlyChart({
  id: 'investments-insights',
  command: 'get_investment_insights',
  // A getter, so the name follows the language when the chart is drawn again.
  get ariaLabel() {
    return t('Investments per year');
  },
  onLoaded: buildLegend,
  pieceStyle,
  describe,
  tooltipContent,
  /** The closing-balance marker, as Rust delivers it. */
  decorate: markerDecorator,
  /** The year, and the growth percentage on a second line. */
  labelLines: (row) => [
    { text: yearLabel(row), className: 'chart-year' },
    {
      text: formatLabel(row.growth_label),
      className: `chart-percent${isNegative(row.growth_label) ? ' negative' : ''}`,
    },
  ],
});

/** Reloads the chart only while the Insights tab is showing (after a project is opened). */
export const reloadInvestmentsInsightsIfOpen = chart.reloadIfOpen;

export function initInvestmentsInsights() {
  chart.init();
  initInsightsTabs('investments', chart); // opening Insights recomputes it (CSD-007 REQ-13)
}
