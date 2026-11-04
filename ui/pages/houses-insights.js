// House insights: the yearly stacked chart on the Houses page (CSD-016), one segment per house.
// The page is built by createSeriesInsights (shared/insights-page.js); this file supplies what
// is specific to houses: the texts, the total value marker and the change row.
import { t } from '../shared/i18n.js';
import { formatAmount } from '../shared/format.js';
import { noSwatch, tooltipRow } from '../shared/chart.js';
import { isNegative, yearLabel } from '../shared/yearly-chart.js';
import { createSeriesInsights } from '../shared/insights-page.js';

/** The change amount as shown: a dash when Rust has none. */
const deltaText = (row) => (row.change === null ? '–' : formatAmount(row.change));

function describe(row, nameOf) {
  if (row.valuation_count === 0) return t('{year}: no valuations', { year: yearLabel(row) });
  const parts = row.segments.map((segment) => `${nameOf(segment)} ${formatAmount(segment.value)}`);
  const delta = row.change === null
    ? t('change none')
    : isNegative(row.change)
      ? t('change {amount} (negative)', { amount: formatAmount(row.change) })
      : t('change {amount}', { amount: formatAmount(row.change) });
  return t('{year}: {parts}, total value {total}, {delta}', { year: yearLabel(row), parts: parts.join(', '), total: formatAmount(row.closing_value), delta });
}

const insights = createSeriesInsights({
  prefix: 'houses',
  command: 'get_house_insights',
  listKey: 'houses',
  keyPrefix: 'house-',
  segmentId: 'house_id',
  amount: 'value',
  count: 'valuation_count',
  closing: 'closing_value',
  chartLabel: () => t('Houses per year'),
  markerLabel: () => t('Total value'),
  partialNote: () => t('Partial year: valuations so far'),
  emptyNote: () => t('No valuations'),
  describe,
  extraRows: (row) =>
    tooltipRow(null, noSwatch(), t('Change'), deltaText(row), row.change !== null && isNegative(row.change) ? 'negative' : ''),
});

export const reloadHousesInsightsIfOpen = insights.reloadIfOpen;
export const initHousesInsights = insights.init;
