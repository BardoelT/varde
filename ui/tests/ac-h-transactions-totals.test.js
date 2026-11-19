// CSD-032 REQ-24: the Yearly totals of the Transactions page, one collapsible block per year with a grid of
// the net per category and month. Rust computes every figure; these tests feed canned answers and check what is shown.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { readsWith, transactionsPage, transaction, GROCERIES } from './transactions-helpers.js';

/** Twelve months, `null` except the given ones: `months({ 9: '2500.00' })`. */
const months = (given = {}) => Array.from({ length: 12 }, (_, index) => given[index] ?? null);

const TOTALS = [
  {
    year: 2026,
    income: '2500.00',
    spent: '23.97',
    net: '2476.03',
    transaction_count: 2,
    total: '2476.03',
    months: months({ 9: '2476.03' }),
    categories: [
      { id: 'cat-salary', name: 'Salary', months: months({ 9: '2500.00' }), total: '2500.00' },
      { id: GROCERIES, name: 'Groceries', months: months({ 9: '-23.97' }), total: '-23.97' },
    ],
  },
  {
    year: 2025,
    income: '0.00',
    spent: '150.00',
    net: '-150.00',
    transaction_count: 2,
    total: '-150.00',
    months: months({ 11: '-150.00', 5: '0.00' }),
    categories: [{ id: 'uncategorized', name: 'Uncategorized', months: months({ 11: '-150.00', 5: '0.00' }), total: '-150.00' }],
  },
];

async function totalsPage({ totals = TOTALS, preferences, handlers } = {}) {
  const reads = readsWith({ rows: [transaction('t1', '2026-10-30', '-23.97', 'Example Shop')] });
  reads.get_transactions_overview.totals = totals;
  if (preferences) reads.get_preferences = { theme: 'system', language: 'en', number_format: 'point_comma', discrete_mode: false, ...preferences };
  const app = await transactionsPage(reads, handlers);
  await app.click(app.$('transactions-tab-totals'));
  return app;
}

const blocks = (app) => [...app.$('transaction-totals-years').querySelectorAll('.year-block')];
const toggle = (block) => block.querySelector('button.year-toggle');
const grid = (block) => block.querySelector('.year-grid-wrap');
const gridRows = (app, block) => [...block.querySelectorAll('.year-grid tr')].map((row) => [...row.children].map((cell) => app.text(cell)));
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

test('test_csd032_ac_24_1_each_year_is_a_block_with_income_spent_and_net_in_the_header', async () => {
  const app = await totalsPage();
  assert.equal(blocks(app).length, 2);
  const headers = blocks(app).map((block) => app.text(toggle(block)));
  assert.deepEqual(headers, ['2026Income2,500.00Spent23.97Net2,476.03', '2025Income0.00Spent150.00Net-150.00'], 'newest year first');
  assert.equal(app.$('transaction-totals-years').querySelector('table.data-table:not(.year-grid)'), null, 'the old table is gone');
  assert.equal(app.isShown(app.$('transaction-totals-empty')), false);
  app.close();
});

test('test_csd032_ac_24_2_no_transactions_no_totals', async () => {
  const app = await totalsPage({ totals: [] });
  assert.equal(blocks(app).length, 0);
  assert.equal(app.textOf('transaction-totals-empty'), 'No totals yet. Totals appear once there is a transaction.');
  assert.ok(app.isShown(app.$('transaction-totals-empty')));
  app.close();
});

test('test_csd032_ac_24_3_a_filter_does_not_change_the_totals', async () => {
  const app = await totalsPage();
  const before = app.callsOf('get_transactions_overview').length;
  await app.click(app.$('transactions-tab-transactions'));
  await app.click(app.$('transaction-entity-filter').querySelector('button'));
  const box = [...app.document.querySelectorAll('.filter-check input')].find((input) => input.value === GROCERIES);
  box.checked = true;
  box.dispatchEvent(new app.window.Event('change', { bubbles: true }));
  await app.settle();
  assert.equal(app.callsOf('get_transactions_overview').length, before, 'the totals are not asked for again');
  assert.equal(blocks(app).length, 2);
  app.close();
});

test('test_csd032_ac_24_5_discrete_mode_hides_the_amounts_but_not_years_categories_months_or_dashes', async () => {
  const app = await totalsPage({ preferences: { discrete_mode: true } });
  const [newest] = blocks(app);
  assert.equal(app.text(toggle(newest)), '2026Income•••••Spent•••••Net•••••');
  const rows = gridRows(app, newest);
  assert.deepEqual(rows[1], ['Salary', '–', '–', '–', '–', '–', '–', '–', '–', '–', '•••••', '–', '–', '•••••']);
  assert.deepEqual(rows[0], ['Category', ...MONTHS, 'Total']);
  app.close();
});

test('test_csd032_ac_24_6_the_newest_year_is_open_and_a_header_collapses_and_expands_its_grid', async () => {
  const app = await totalsPage();
  const [newest, older] = blocks(app);
  assert.equal(toggle(newest).tagName, 'BUTTON');
  assert.equal(toggle(newest).getAttribute('aria-expanded'), 'true');
  assert.equal(toggle(older).getAttribute('aria-expanded'), 'false');
  assert.equal(app.isShown(grid(newest)), true);
  assert.equal(grid(older).hidden, true, 'a collapsed grid is hidden, so out of the tab order and the accessibility tree');
  assert.equal(toggle(older).getAttribute('aria-controls'), grid(older).id);
  await app.click(toggle(older));
  assert.equal(toggle(older).getAttribute('aria-expanded'), 'true');
  assert.equal(grid(older).hidden, false);
  await app.click(toggle(newest));
  assert.equal(toggle(newest).getAttribute('aria-expanded'), 'false');
  assert.equal(grid(newest).hidden, true);
  app.close();
});

test('test_csd032_ac_24_7_the_grid_holds_the_net_per_category_and_month_with_a_total_row_and_column', async () => {
  const app = await totalsPage();
  const [newest] = blocks(app);
  assert.deepEqual(gridRows(app, newest), [
    ['Category', ...MONTHS, 'Total'],
    ['Salary', '–', '–', '–', '–', '–', '–', '–', '–', '–', '2,500.00', '–', '–', '2,500.00'],
    ['Groceries', '–', '–', '–', '–', '–', '–', '–', '–', '–', '-23.97', '–', '–', '-23.97'],
    ['Total', '–', '–', '–', '–', '–', '–', '–', '–', '–', '2,476.03', '–', '–', '2,476.03'],
  ]);
  const last = gridRows(app, newest).at(-1).at(-1);
  assert.match(app.text(toggle(newest)), new RegExp(`Net${last.replace(/[.,]/g, '\\$&')}$`), 'the Total cell equals the net of the header');
  app.close();
});

test('test_csd032_ac_24_8_and_24_9_the_rows_are_the_ones_rust_sent_with_uncategorized_by_name', async () => {
  const app = await totalsPage();
  const rows = gridRows(app, blocks(app)[1]);
  assert.deepEqual(rows.map((row) => row[0]), ['Category', 'Uncategorized', 'Total']);
  app.close();
});

test('test_csd032_ac_24_10_a_dash_for_no_transactions_and_0_00_for_a_month_that_cancels_out', async () => {
  const app = await totalsPage();
  const older = blocks(app)[1];
  const cells = [...older.querySelectorAll('.year-grid tbody tr:first-child td')];
  assert.equal(app.text(cells[5]), '0.00', 'June cancels out');
  assert.equal(cells[5].classList.contains('empty'), false);
  assert.equal(app.text(cells[4]), '–', 'May has no transaction');
  assert.ok(cells[4].classList.contains('empty'), 'the dash is dimmed');
  assert.ok(cells[11].classList.contains('negative'), 'a negative amount has the negative style');
  app.close();
});

test('test_csd032_ac_24_11_the_grid_always_has_the_twelve_months', async () => {
  const app = await totalsPage();
  for (const block of blocks(app)) {
    const head = [...block.querySelectorAll('.year-grid thead th')].map((cell) => app.text(cell));
    assert.deepEqual(head, ['Category', ...MONTHS, 'Total']);
  }
  app.close();
});

test('test_csd032_ac_24_12_the_choice_of_the_user_stays_after_a_change_and_a_new_newest_year_opens', async () => {
  const reads = readsWith({ rows: [transaction('t1', '2026-10-30', '-23.97', 'Example Shop')] });
  const overview = reads.get_transactions_overview;
  let imported = false;
  // After the change Rust also returns a year 2027 that the user never touched.
  reads.get_transactions_overview = () => ({ ...overview, totals: imported ? [{ ...TOTALS[0], year: 2027 }, ...TOTALS] : TOTALS });
  const app = await transactionsPage(reads, { delete_transaction: () => { imported = true; return null; } });
  await app.click(app.$('transactions-tab-totals'));
  await app.click(toggle(blocks(app)[0]));
  await app.click(toggle(blocks(app)[1]));
  await app.click(app.$('transactions-tab-transactions'));
  await app.click(app.document.querySelector('[aria-label="Delete Example Shop"]'));
  await app.click(app.$('confirm-ok'));
  await app.click(app.$('transactions-tab-totals'));
  const [year2027, year2026, year2025] = blocks(app);
  assert.equal(toggle(year2027).getAttribute('aria-expanded'), 'true', 'a year never touched follows the default: the newest is open');
  assert.equal(toggle(year2026).getAttribute('aria-expanded'), 'false', 'collapsed by the user');
  assert.equal(toggle(year2025).getAttribute('aria-expanded'), 'true', 'opened by the user');
  app.close();
});

test('test_csd032_ac_24_13_the_grid_scrolls_inside_its_wrapper_and_keeps_the_category_column', async () => {
  const app = await totalsPage();
  const [newest] = blocks(app);
  assert.ok(grid(newest).classList.contains('table-wrap'), 'the wrapper scrolls horizontally');
  assert.ok(newest.querySelector('.year-grid').classList.contains('year-grid'));
  app.close();
});

test('test_csd032_ac_24_14_the_month_names_are_dutch_when_the_language_is_dutch', async () => {
  const app = await totalsPage({ preferences: { language: 'nl' } });
  const head = [...blocks(app)[0].querySelectorAll('.year-grid thead th')].map((cell) => app.text(cell));
  assert.deepEqual(head, ['Categorie', 'Jan', 'Feb', 'Mrt', 'Apr', 'Mei', 'Jun', 'Jul', 'Aug', 'Sep', 'Okt', 'Nov', 'Dec', 'Totaal']);
  app.close();
});

test('test_csd032_ac_24_15_the_figures_sit_at_the_right_and_the_categorized_share_is_not_on_the_tab', async () => {
  const app = await totalsPage();
  assert.equal(app.$('transactions-totals').contains(app.$('transaction-categorisation')), false);
  // The first figure takes the free room (CSS `margin-left: auto`), so the three sit at the right of the row.
  const css = readFileSync(fileURLToPath(new URL('../styles/components.css', import.meta.url)), 'utf8');
  assert.match(css, /\.year-title \+ \.year-figure \{ margin-left: auto; \}/);
  const figures = [...toggle(blocks(app)[0]).children].map((node) => node.className.split(' ')[0]);
  assert.deepEqual(figures, ['year-chevron', 'year-title', 'year-figure', 'year-figure', 'year-figure']);
  app.close();
});
