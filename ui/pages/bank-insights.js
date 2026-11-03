// Bank account insights: the yearly stacked chart on the Bank accounts page
// (CSD-010), one segment per bank. Its frame (axis, bars, tooltip, loading) is
// shared with the other charts in shared/yearly-chart.js; this file supplies
// what is specific to banks: the segments, the legend of banks and the
// closing-balance marker.
import { h } from '../shared/dom.js';
import { t } from '../shared/i18n.js';
import { formatAmount } from '../shared/format.js';
import { swatch, legendLabel, nameLabel, legendItem, tooltipRow, topToBottom } from '../shared/chart.js';
import { createYearlyChart, levelMarker, isNegative, yearLabel } from '../shared/yearly-chart.js';
import { initTabs } from '../shared/tabs.js';

/** The banks of the series by id, as Rust returned them (creation order). */
let banks = new Map();

const keyOf = (accountId) => `bank-${accountId}`;
const nameOf = (segment) => banks.get(segment.account_id)?.name ?? '';
const slotOf = (accountId) => banks.get(accountId)?.colour_slot ?? 0;

function describe(row) {
  if (row.entry_count === 0) return t('{year}: no entries', { year: yearLabel(row) });
  const parts = row.segments.map(
    (segment) => {
      const amount = formatAmount(segment.balance);
      return `${nameOf(segment)} ${isNegative(segment.balance) ? t('{amount} (negative)', { amount }) : amount}`;
    },
  );
  return t('{year}: {parts}, closing balance {closing}', {
    year: yearLabel(row),
    parts: parts.join(', '),
    closing: formatAmount(row.closing_balance),
  });
}

/** The rows read like the bar: the highest segment first, those that hang below the zero line last. */
function tooltipContent(row) {
  const rows =
    row.entry_count === 0
      ? []
      : [
          ...topToBottom(row.segments, (segment) => keyOf(segment.account_id), row.pieces).flatMap((segment) =>
            tooltipRow(
              keyOf(segment.account_id),
              swatch(`slot-${slotOf(segment.account_id)}`),
              nameOf(segment),
              formatAmount(segment.balance),
              isNegative(segment.balance) ? 'negative' : '',
            ),
          ),
          ...tooltipRow('marker', swatch('marker'), t('Closing balance'), formatAmount(row.closing_balance), isNegative(row.closing_balance) ? 'negative' : ''),
        ];
  return [
    h('div', { class: 'tip-title' }, yearLabel(row)),
    row.is_partial ? h('div', { class: 'tip-note' }, t('Partial year: entries so far')) : null,
    row.entry_count === 0 ? h('div', { class: 'tip-note' }, t('No entries')) : null,
    h('dl', {}, ...rows),
  ].filter(Boolean);
}
/** How a received piece looks: the colour slot and the name of its bank, and its accessible name. */
function pieceStyle(row, piece) {
  const segment = row.segments.find((candidate) => keyOf(candidate.account_id) === piece.series);
  return {
    cssClass: `bar-slot-${slotOf(segment.account_id)}`,
    label: `${nameOf(segment)}, ${row.year}, ${formatAmount(segment.balance)}`,
  };
}

/** The legend: a swatch and the name of every bank in the series, and the closing-balance marker (CSD-010 REQ-9). */
function buildLegend({ banks: list }) {
  banks = new Map(list.map((bank) => [bank.id, bank]));
  const legendList = document.getElementById('bank-insights-legend');
  const scopes = () => [legendList, document.getElementById('bank-insights-chart')];
  legendList.replaceChildren(
    ...list.map((bank) => legendItem(keyOf(bank.id), swatch(`slot-${bank.colour_slot}`), nameLabel(bank.name), scopes)),
    legendItem('marker', swatch('marker'), legendLabel(t('Closing balance')), scopes),
  );
}

const chart = createYearlyChart({
  id: 'bank-insights',
  command: 'get_bank_insights',
  // A getter, so the name follows the language when the chart is drawn again.
  get ariaLabel() {
    return t('Bank accounts per year');
  },
  onLoaded: buildLegend,
  pieceStyle,
  describe,
  tooltipContent,
  /** The closing-balance marker, as Rust delivers it. */
  decorate: (row, geometry) => (row.marker ? levelMarker(row.marker, geometry) : []),
  labelLines: (row) => [{ text: yearLabel(row), className: 'chart-year' }],
});

/** Reloads the chart only while the Insights tab is showing (after a project is opened). */
export const reloadBankInsightsIfOpen = chart.reloadIfOpen;

export function initBankInsights() {
  chart.init();
  initTabs([
    { tab: 'bank-tab-overview', panel: 'bank-overview' },
    // Opening Insights recomputes it, so it never shows stale figures (CSD-010 REQ-15).
    { tab: 'bank-tab-insights', panel: 'bank-insights', onOpen: chart.reload },
  ]);
}
