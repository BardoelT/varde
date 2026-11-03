// Debt insights: the yearly stacked chart on the Debts page (CSD-012), one
// segment per debt. Its frame (axis, bars, tooltip, loading) is shared with the
// other charts in shared/yearly-chart.js; this file supplies what is specific
// to debts: the segments, the legend of debts and the total debt marker.
import { h } from '../shared/dom.js';
import { formatAmount } from '../shared/format.js';
import { swatch, noSwatch, legendLabel, nameLabel, legendItem, tooltipRow, topToBottom } from '../shared/chart.js';
import { createYearlyChart, levelMarker, isNegative, yearLabel } from '../shared/yearly-chart.js';
import { initTabs } from '../shared/tabs.js';
import { t } from '../shared/i18n.js';

/** The debts of the series by id, as Rust returned them (creation order). */
let items = new Map();

const keyOf = (id) => `debt-${id}`;
const nameOf = (segment) => items.get(segment.debt_id)?.name ?? '';
const slotOf = (id) => items.get(id)?.colour_slot ?? 0;
/** The repaid amount as shown: a dash when Rust has none. */
const deltaText = (row) => (row.repaid === null ? '–' : formatAmount(row.repaid));

function describe(row) {
  if (row.statement_count === 0) return t('{year}: no statements', { year: yearLabel(row) });
  const parts = row.segments.map((segment) => `${nameOf(segment)} ${formatAmount(segment.balance)}`);
  const delta = row.repaid === null
    ? t('repaid none')
    : isNegative(row.repaid)
      ? t('repaid {amount} (negative)', { amount: formatAmount(row.repaid) })
      : t('repaid {amount}', { amount: formatAmount(row.repaid) });
  return t('{year}: {parts}, total debt {total}, {delta}', { year: yearLabel(row), parts: parts.join(', '), total: formatAmount(row.closing_balance), delta });
}

/** The rows read like the bar: the highest segment first. */
function tooltipContent(row) {
  const rows =
    row.statement_count === 0
      ? []
      : [
          ...topToBottom(row.segments, (segment) => keyOf(segment.debt_id), row.pieces).flatMap((segment) =>
            tooltipRow(keyOf(segment.debt_id), swatch(`slot-${slotOf(segment.debt_id)}`), nameOf(segment), formatAmount(segment.balance)),
          ),
          ...tooltipRow('marker', swatch('marker'), t('Total debt'), formatAmount(row.closing_balance)),
          ...tooltipRow(null, noSwatch(), t('Repaid'), deltaText(row), row.repaid !== null && isNegative(row.repaid) ? 'negative' : ''),
        ];
  return [
    h('div', { class: 'tip-title' }, yearLabel(row)),
    row.is_partial ? h('div', { class: 'tip-note' }, t('Partial year: statements so far')) : null,
    row.statement_count === 0 ? h('div', { class: 'tip-note' }, t('No statements')) : null,
    h('dl', {}, ...rows),
  ].filter(Boolean);
}

/** How a received piece looks: the colour slot and the name of its debt, and its accessible name. */
function pieceStyle(row, piece) {
  const segment = row.segments.find((candidate) => keyOf(candidate.debt_id) === piece.series);
  return {
    cssClass: `bar-slot-${slotOf(segment.debt_id)}`,
    label: `${nameOf(segment)}, ${row.year}, ${formatAmount(segment.balance)}`,
  };
}

/** The legend: a swatch and the name of every debt in the series, and the total debt marker (CSD-012 REQ-9). */
function buildLegend({ debts: list }) {
  items = new Map(list.map((item) => [item.id, item]));
  const legendList = document.getElementById('debts-insights-legend');
  const scopes = () => [legendList, document.getElementById('debts-insights-chart')];
  legendList.replaceChildren(
    ...list.map((item) => legendItem(keyOf(item.id), swatch(`slot-${item.colour_slot}`), nameLabel(item.name), scopes)),
    legendItem('marker', swatch('marker'), legendLabel(t('Total debt')), scopes),
  );
}

const chart = createYearlyChart({
  id: 'debts-insights',
  command: 'get_debt_insights',
  get ariaLabel() {
    return t('Debts per year');
  },
  onLoaded: buildLegend,
  pieceStyle,
  describe,
  tooltipContent,
  /** The total debt marker, as Rust delivers it. */
  decorate: (row, geometry) => (row.marker ? levelMarker(row.marker, geometry) : []),
  labelLines: (row) => [{ text: yearLabel(row), className: 'chart-year' }],
});

/** Reloads the chart only while the Insights tab is showing (after a project is opened). */
export const reloadDebtsInsightsIfOpen = chart.reloadIfOpen;

export function initDebtsInsights() {
  chart.init();
  initTabs([
    { tab: 'debts-tab-overview', panel: 'debts-overview' },
    // Opening Insights recomputes it, so it never shows stale figures (CSD-012 REQ-15).
    { tab: 'debts-tab-insights', panel: 'debts-insights', onOpen: chart.reload },
  ]);
}
