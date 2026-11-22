// Display ACs of CSD-032 REQ-35: the two donut charts of the Categories tab, with their dropdowns, legend,
// tooltip, highlight, empty note and Discrete mode. The answers are small canned shares (what Rust returns
// for them is proven by `cargo test`); the DOM tests check only what is drawn.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { exampleReads } from './fixtures.js';

const slice = (id, name, slot, amount, percent, start, end) => ({ id, name, colour_slot: slot, amount, percent_label: percent, start, end });
const GROCERIES = slice('cat-groceries', 'Groceries', 0, '15.00', '66.67%', '0.000000', '0.666667');
const OTHER = slice('uncategorized', 'Uncategorized', 7, '7.50', '33.33%', '0.666667', '1.000000');
const SHARES = { total: '22.50', slices: [GROCERIES, OTHER] };
const PERIODS = { years: [2026, 2025], months: ['2026-11', '2026-10'] };

/** The shares per period; any period not listed has no spending. */
function sharesFor(table) {
  return ({ period }) => table[period] ?? { total: '0.00', slices: [] };
}

async function categoriesTab({ periods = PERIODS, shares = { 2026: SHARES, '2026-11': SHARES }, reads = exampleReads() } = {}) {
  const app = await openApp({ reads, handlers: { transaction_spending_periods: periods, transaction_spending_shares: sharesFor(shares) } });
  await app.click(app.item('transactions'));
  await app.click(app.$('transactions-tab-categories'));
  return app;
}

const slices = (app, stem) => [...app.$(`${stem}-chart`).querySelectorAll('.donut-slice')];
const field = (app, stem) => app.$(`transaction-donut-${stem}-field`);
const picker = (app) => app.document.querySelector('.date-picker');
const enabled = (app) => [...picker(app).querySelectorAll('.dp-choice:not(:disabled)')].map((cell) => app.text(cell));
const legend = (app, stem) => [...app.$(`${stem}-legend`).querySelectorAll('li')].map((item) => app.text(item));

test('test_csd032_ac_35_3_only_the_periods_with_spending_can_be_chosen_and_the_others_are_grey', async () => {
  const app = await categoriesTab();
  assert.equal(field(app, 'year').value, '2026');
  assert.equal(field(app, 'month').value, 'November 2026');
  assert.equal(field(app, 'year').readOnly, true);
  await app.click(field(app, 'year'));
  assert.deepEqual(enabled(app), ['2025', '2026'], 'only the years with a debit');
  assert.equal(picker(app).querySelectorAll('.dp-choice:disabled').length, 8, 'the others are disabled');
  await app.click(picker(app).querySelector('[data-choice="2025"]'));
  assert.equal(field(app, 'year').value, '2025');
  assert.equal(field(app, 'month').value, 'November 2026', 'the month does not follow the year');
  assert.deepEqual(app.callsOf('transaction_spending_shares').at(-1).args, { period: '2025' });
  await app.click(field(app, 'month'));
  assert.equal(app.text(picker(app).querySelector('.dp-title')), '2026', 'the month picker opens on the latest month with a debit');
  assert.deepEqual(enabled(app), ['Oct', 'Nov'], 'only the months with a debit');
  app.close();
});

test('test_csd032_ac_35_4_the_month_names_follow_the_language_in_dutch', async () => {
  const reads = exampleReads();
  reads.get_preferences = { ...reads.get_preferences, language: 'nl' };
  const app = await categoriesTab({ reads });
  assert.equal(field(app, 'month').value, 'november 2026');
  app.close();
});

test('test_csd032_ac_35_4_the_month_field_shows_the_month_name_in_english', async () => {
  const app = await categoriesTab();
  assert.equal(field(app, 'month').value, 'November 2026');
  app.close();
});

test('test_csd032_ac_35_5_the_donut_draws_the_slices_as_received', async () => {
  const app = await categoriesTab();
  const drawn = slices(app, 'transaction-donut-year');
  assert.equal(drawn.length, 2);
  assert.ok(drawn[0].classList.contains('bar-slot-0') && drawn[1].classList.contains('bar-slot-7'), 'the colours of the slots');
  assert.match(drawn[0].getAttribute('d'), /^M100\.000 10\.000A/, 'the first slice starts at 12 o\'clock');
  assert.match(app.text(app.$('transaction-donut-year-chart').querySelector('.donut-centre')), /22\.50\s*Total spent/);
  assert.deepEqual(legend(app, 'transaction-donut-year'), ['Groceries', 'Uncategorized']);
  assert.match(drawn[0].getAttribute('aria-label'), /Groceries, 2026, 15\.00, 66\.67%/);
  app.close();
});

test('test_csd032_ac_35_6_the_tooltip_and_the_highlight', async () => {
  const app = await categoriesTab();
  const [first, second] = slices(app, 'transaction-donut-year');
  first.dispatchEvent(new app.window.MouseEvent('mouseenter'));
  const tooltip = app.$('transaction-donut-year-chart').querySelector('.chart-tooltip');
  assert.ok(tooltip.classList.contains('is-open'));
  assert.equal(app.text(tooltip.querySelector('.tip-title')), 'Groceries', 'the title is the category');
  assert.deepEqual([...tooltip.querySelectorAll('dt, dd')].map((cell) => app.text(cell)), ['Amount', '15.00', 'Percentage', '66.67%'], 'the amount and the percentage');
  first.dispatchEvent(new app.window.MouseEvent('mouseover', { bubbles: true }));
  const items = [...app.$('transaction-donut-year-legend').querySelectorAll('li')];
  assert.ok(first.classList.contains('hl') && items[0].classList.contains('hl'), 'the slice and its legend item are highlighted');
  assert.ok(second.classList.contains('dim') && items[1].classList.contains('dim'));
  first.parentNode.dispatchEvent(new app.window.MouseEvent('mouseleave'));
  assert.ok(!first.classList.contains('hl'), 'leaving the ring clears it');
  items[1].dispatchEvent(new app.window.MouseEvent('mouseenter'));
  assert.ok(second.classList.contains('hl'), 'hovering the legend does the same');
  app.close();
});

test('test_csd032_ac_35_8_a_single_category_is_a_full_ring', async () => {
  const only = { total: '10.00', slices: [slice('cat-groceries', 'Groceries', 0, '10.00', '100.00%', '0.000000', '1.000000')] };
  const app = await categoriesTab({ shares: { 2026: only, '2026-11': only } });
  const [ring] = slices(app, 'transaction-donut-year');
  assert.equal((ring.getAttribute('d').match(/M/g) ?? []).length, 2, 'two halves make a ring');
  assert.match(ring.getAttribute('aria-label'), /100\.00%/);
  app.close();
});

test('test_csd032_ac_35_8_a_tiny_slice_keeps_its_legend_entry_and_tooltip', async () => {
  const tiny = { total: '10000.00', slices: [slice('cat-groceries', 'Groceries', 0, '9990.00', '99.90%', '0.000000', '0.999000'), slice('uncategorized', 'Uncategorized', 7, '10.00', '0.10%', '0.999000', '1.000000')] };
  const app = await categoriesTab({ shares: { 2026: tiny, '2026-11': tiny } });
  assert.deepEqual(legend(app, 'transaction-donut-year'), ['Groceries', 'Uncategorized']);
  slices(app, 'transaction-donut-year')[1].dispatchEvent(new app.window.MouseEvent('mouseenter'));
  assert.match(app.text(app.$('transaction-donut-year-chart').querySelector('.chart-tooltip')), /0\.10%/);
  app.close();
});

test('test_csd032_ac_35_9_no_spending_shows_the_note_and_no_donuts', async () => {
  const app = await categoriesTab({ periods: { years: [], months: [] } });
  assert.ok(app.isShown(app.$('transaction-spending-empty')));
  assert.equal(app.isShown(app.$('transaction-spending-donuts')), false);
  assert.equal(app.callsOf('transaction_spending_shares').length, 0);
  app.close();
});

test('test_csd032_ac_35_10_discrete_mode_hides_the_amounts_but_not_the_percentages', async () => {
  const reads = exampleReads();
  reads.get_preferences = { ...reads.get_preferences, discrete_mode: true };
  const app = await categoriesTab({ reads });
  const chart = app.$('transaction-donut-year-chart');
  assert.doesNotMatch(app.text(chart.querySelector('.donut-centre')), /22\.50/);
  const [first] = slices(app, 'transaction-donut-year');
  assert.doesNotMatch(first.getAttribute('aria-label'), /15\.00/);
  assert.match(first.getAttribute('aria-label'), /66\.67%/);
  first.dispatchEvent(new app.window.MouseEvent('mouseenter'));
  const tip = app.text(chart.querySelector('.chart-tooltip'));
  assert.doesNotMatch(tip, /15\.00/);
  assert.equal(chart.querySelectorAll('.chart-tooltip dt').length, 1, 'only the percentage row');
  assert.match(tip, /66\.67%/);
  app.close();
});

test('test_csd032_ac_35_11_the_donuts_ask_only_for_a_period_not_for_the_list_filters', async () => {
  const app = await categoriesTab();
  for (const call of app.callsOf('transaction_spending_shares')) assert.deepEqual(Object.keys(call.args), ['period']);
  assert.deepEqual(app.callsOf('transaction_spending_periods').at(-1).args ?? {}, {});
  app.close();
});

test('test_csd032_ac_35_12_a_change_of_the_transactions_redraws_the_donuts', async () => {
  const app = await categoriesTab();
  assert.equal(slices(app, 'transaction-donut-year').length, 2);
  const before = app.callsOf('transaction_spending_periods').length;
  app.bridge.table.transaction_spending_shares = sharesFor({ 2026: { total: '15.00', slices: [{ ...GROCERIES, percent_label: '100.00%', end: '1.000000' }] }, '2026-11': SHARES });
  app.bridge.table.rename_transaction_category = {};
  await app.click(app.$('transaction-categories').querySelector('button[aria-label^="Edit"]'));
  app.$('transaction-category-form').elements.name.value = 'Food';
  app.$('transaction-category-form').requestSubmit();
  await app.settle();
  assert.ok(app.callsOf('transaction_spending_periods').length > before, 'the donuts are asked again');
  assert.equal(slices(app, 'transaction-donut-year').length, 1);
  app.close();
});
