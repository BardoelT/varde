// Bank account insights: the yearly stacked chart on the Bank accounts page (CSD-010), one
// segment per bank. The page is built by createSeriesInsights (shared/insights-page.js); this
// file supplies what is specific to banks: the texts and the closing-balance marker.
import { t } from '../shared/i18n.js';
import { formatAmount } from '../shared/format.js';
import { isNegative, yearLabel } from '../shared/yearly-chart.js';
import { createSeriesInsights } from '../shared/insights-page.js';

function describe(row, nameOf) {
  if (row.entry_count === 0) return t('{year}: no entries', { year: yearLabel(row) });
  const parts = row.segments.map((segment) => {
    const amount = formatAmount(segment.balance);
    return `${nameOf(segment)} ${isNegative(segment.balance) ? t('{amount} (negative)', { amount }) : amount}`;
  });
  return t('{year}: {parts}, closing balance {closing}', {
    year: yearLabel(row),
    parts: parts.join(', '),
    closing: formatAmount(row.closing_balance),
  });
}

const insights = createSeriesInsights({
  prefix: 'bank',
  command: 'get_bank_insights',
  listKey: 'banks',
  keyPrefix: 'bank-',
  segmentId: 'account_id',
  amount: 'balance',
  count: 'entry_count',
  closing: 'closing_balance',
  chartLabel: () => t('Bank accounts per year'),
  markerLabel: () => t('Closing balance'),
  partialNote: () => t('Partial year: entries so far'),
  emptyNote: () => t('No entries'),
  flagNegative: true, // a balance can be negative (CSD-010 REQ-13)
  describe,
});

export const reloadBankInsightsIfOpen = insights.reloadIfOpen;
export const initBankInsights = insights.init;
