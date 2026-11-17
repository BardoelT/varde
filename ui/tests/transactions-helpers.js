// Helpers of the DOM tests of the Transactions page (CSD-032): canned categories, rules and rows, the page
// opened through the fake bridge, and the helpers of the suggest dialog and bulk select.
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { exampleReads } from './fixtures.js';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

export const GROCERIES = 'cat-groceries';

export const SHOPPING = 'cat-shopping';

export const CATEGORIES = [
  { id: GROCERIES, name: 'Groceries', transaction_count: 3 },
  { id: SHOPPING, name: 'Shopping', transaction_count: 1 },
];

export const RULES = [
  { id: 'rule-1', text: 'Example Market', category_id: GROCERIES, transaction_count: 3 },
  { id: 'rule-2', text: 'Example', category_id: SHOPPING, transaction_count: 1 },
];

export const transaction = (id, date, amount, name, category = null) => ({
  id,
  date,
  amount,
  counterparty_name: name,
  counterparty_iban: '',
  description: 'Example description',
  own_account: 'NL00EXAM0000000001',
  reference: `R-${id}`,
  payment_method: 'Betaalautomaat',
  category_id: category,
  manual: false,
});

/** The golden example project with this list, the count and total Rust would return for it. */
export function readsWith({ rows, count = rows.length, total = '0.00', categories = CATEGORIES, rules = RULES }) {
  const reads = exampleReads();
  reads.list_transactions = { rows, count, total };
  reads.list_transaction_categories = categories;
  reads.list_transaction_rules = rules;
  return reads;
}

export async function transactionsPage(reads, handlers) {
  const app = await openApp({ reads, handlers });
  await app.click(app.item('transactions'));
  return app;
}

export const rowCells = (app, row) => [...row.querySelectorAll('th, td')].map((cell) => app.text(cell));

export const tableRows = (app) => [...app.$('transactions-rows').querySelectorAll('tr')];

/** Lays the rule rows out as 40-pixel rows from the top, since jsdom has no layout. */
export function layOutRules(app) {
  [...app.$('transaction-rules').querySelectorAll('li')].forEach((row, index) => {
    row.getBoundingClientRect = () => ({ top: index * 40, height: 40, bottom: index * 40 + 40, left: 0, right: 100, width: 100 });
  });
}

export const pointer = (app, target, type, clientY = 0) =>
  target.dispatchEvent(new app.window.MouseEvent(type, { bubbles: true, cancelable: true, button: 0, clientY }));

export const HOLIDAY_RULE = { id: 'rule-3', text: '', category_id: SHOPPING, transaction_count: 4, from: '2026-07-01', to: '2026-07-15' };

export const bulkRows = () => [
  transaction('t1', '2026-10-30', '-1.00', 'Example Shop'),
  transaction('t2', '2026-10-29', '-2.00', 'Example Cafe'),
  transaction('t3', '2026-10-28', '-3.00', 'Example Bakery'),
];

export const bulkHandlers = (extra = {}) => ({
  list_transactions: (args) => ({ rows: bulkRows().slice(0, args?.limit ?? 200), count: 3, total: '-6.00' }),
  list_transaction_ids: ['t1', 't2', 't3'],
  ...extra,
});

export const suggestion = (text, count, categoryId = null) => ({ text, count, category_id: categoryId });

export const suggestHandlers = (extra = {}) => ({
  suggest_transaction_rules: (args) => ({
    suggestions: [suggestion('Albert Heijn*', 6, GROCERIES), suggestion('Example Cafe', 4)].filter((each) => each.count >= args.minCount),
    not_shown: 0,
  }),
  count_transaction_rule_matches: 3,
  ...extra,
});

export const openSuggest = async (handlers = suggestHandlers()) => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }), handlers);
  await app.click(app.$('transactions-tab-rules'));
  await app.click(app.$('transaction-suggest'));
  return app;
};

export const suggestRows = (app) => [...app.$('transaction-suggest-rows').querySelectorAll('tr')];

export const waitForRecount = async (app) => {
  await new Promise((resolve) => setTimeout(resolve, 320));
  await app.settle();
};

export const totalsRows = (app) => [...app.$('transaction-totals-rows').querySelectorAll('tr')].map((row) => rowCells(app, row));
