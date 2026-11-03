// Debt insights: the yearly stacked chart on the Debts page (CSD-012), one segment per debt.
// The page is built by createSeriesInsights (shared/insights-page.js); this file supplies what
// is specific to debts: the texts, the total debt marker and the repaid row.
import { t } from '../shared/i18n.js';
import { formatAmount } from '../shared/format.js';
import { noSwatch, tooltipRow } from '../shared/chart.js';
import { isNegative, yearLabel } from '../shared/yearly-chart.js';
import { createSeriesInsights } from '../shared/insights-page.js';

/** The repaid amount as shown: a dash when Rust has none. */
const deltaText = (row) => (row.repaid === null ? '–' : formatAmount(row.repaid));

function describe(row, nameOf) {
  if (row.statement_count === 0) return t('{year}: no statements', { year: yearLabel(row) });
  const parts = row.segments.map((segment) => `${nameOf(segment)} ${formatAmount(segment.balance)}`);
  const delta = row.repaid === null
    ? t('repaid none')
    : isNegative(row.repaid)
      ? t('repaid {amount} (negative)', { amount: formatAmount(row.repaid) })
      : t('repaid {amount}', { amount: formatAmount(row.repaid) });
  return t('{year}: {parts}, total debt {total}, {delta}', { year: yearLabel(row), parts: parts.join(', '), total: formatAmount(row.closing_balance), delta });
}

const insights = createSeriesInsights({
  prefix: 'debts',
  command: 'get_debt_insights',
  listKey: 'debts',
  keyPrefix: 'debt-',
  segmentId: 'debt_id',
  amount: 'balance',
  count: 'statement_count',
  closing: 'closing_balance',
  chartLabel: () => t('Debts per year'),
  markerLabel: () => t('Total debt'),
  partialNote: () => t('Partial year: statements so far'),
  emptyNote: () => t('No statements'),
  describe,
  extraRows: (row) =>
    tooltipRow(null, noSwatch(), t('Repaid'), deltaText(row), row.repaid !== null && isNegative(row.repaid) ? 'negative' : ''),
});

export const reloadDebtsInsightsIfOpen = insights.reloadIfOpen;
export const initDebtsInsights = insights.init;
