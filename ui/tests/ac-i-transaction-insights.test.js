// Display ACs of CSD-033 (transaction insights): the tab, the stacked bars per category, the legend, the
// choice between Yearly and Monthly, the labels, the tooltip, the empty state and the Dutch texts. The
// answers are small series made for one AC (what Rust returns for them is proven by `cargo test`).
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { exampleReads } from './fixtures.js';

const categories = [
  { id: 'cat-groceries', name: 'Groceries', colour_slot: 0 },
  { id: 'cat-shopping', name: 'Shopping', colour_slot: 1 },
  { id: 'uncategorized', name: 'Uncategorized', colour_slot: 7 },
];
const AXIS = { ticks: [['0.00', '0'], ['20.00', '20'], ['40.00', '40']].map(([value, label]) => ({ value, label })) };

/** A bar as Rust sends it: `segments` as `[category, amount]`, stacked bottom to top in this order. */
function bar(year, month, segments, { partial = false, income = 0 } = {}) {
  let low = 0;
  const pieces = segments.map(([category, amount], index) => {
    const piece = { series: `category-${category}`, low: low.toFixed(2), high: (low + Number(amount)).toFixed(2), hangs: false, rounding: index === segments.length - 1 ? 'top' : 'none' };
    low += Number(amount);
    return piece;
  });
  return {
    year, month, is_partial: partial, transaction_count: segments.length + (income > 0 ? 1 : 0), income: income.toFixed(2),
    marker: income > 0 ? { value: income.toFixed(2), label: String(income) } : null, pieces,
    segments: segments.map(([category, amount]) => ({ category, amount })),
    total: low.toFixed(2),
  };
}
const yearly = (years) => ({ granularity: 'yearly', categories, axis: AXIS, years });
const monthly = (years) => ({ granularity: 'monthly', categories, axis: AXIS, years });
const YEARS = () => yearly([bar(2025, null, [['cat-groceries', '20.00']]), bar(2026, null, [['cat-groceries', '15.00'], ['uncategorized', '7.50']], { partial: true, income: 30 })]);
const MONTHS = () => monthly([bar(2026, 9, [['cat-groceries', '5.00']]), bar(2026, 10, [['cat-groceries', '15.00'], ['cat-shopping', '2.50']], { partial: true })]);

async function insightsPage(handlers = {}, reads = exampleReads()) {
  const app = await openApp({ reads, handlers });
  await app.click(app.item('transactions'));
  await app.click(app.$('transactions-tab-insights'));
  return app;
}

const groups = (app) => [...app.$('transactions-insights-chart').querySelectorAll('.chart-year-group')];
const labels = (app) => groups(app).map((group) => app.text(group.querySelector('.chart-year')));
const legendNames = (app) => [...app.$('transactions-insights-legend').querySelectorAll('li')].map((item) => app.text(item));

test('test_csd033_ac_9_1_the_insights_tab_is_the_third_tab_and_opens_the_chart', async () => {
  const app = await openApp({ handlers: { get_transaction_insights: YEARS } });
  await app.click(app.item('transactions'));
  assert.deepEqual([...app.document.querySelectorAll('#transactions [role="tab"]')].map((tab) => app.text(tab)), ['Categories and rules', 'Overview', 'Insights']);
  assert.equal(app.isShown(app.$('transactions-insights')), false);
  await app.click(app.$('transactions-tab-insights'));
  assert.ok(app.isShown(app.$('transactions-insights')));
  assert.equal(groups(app).length, 2);
  assert.equal(app.$('transactions-tab-insights').getAttribute('aria-selected'), 'true');
  app.close();
});

test('test_csd033_ac_5_1_the_chart_opens_on_the_years_and_the_toggle_says_so', async () => {
  const app = await insightsPage({ get_transaction_insights: YEARS });
  assert.equal(app.$('transactions-insights-yearly').getAttribute('aria-pressed'), 'true');
  assert.equal(app.$('transactions-insights-monthly').getAttribute('aria-pressed'), 'false');
  assert.equal(app.textOf('transactions-insights-title'), 'Spending and income per year');
  assert.deepEqual(labels(app), ['2025', '2026 YTD']);
  assert.deepEqual(app.callsOf('get_transaction_insights').at(-1).args, { granularity: 'yearly' });
  app.close();
});

test('test_csd033_ac_5_2_choosing_monthly_asks_for_months_and_labels_them_yyyy_mm', async () => {
  const app = await insightsPage({ get_transaction_insights: ({ granularity }) => (granularity === 'monthly' ? MONTHS() : YEARS()) });
  await app.click(app.$('transactions-insights-monthly'));
  assert.deepEqual(app.callsOf('get_transaction_insights').at(-1).args, { granularity: 'monthly' });
  assert.deepEqual(labels(app), ['2026-09', '2026-10 MTD']);
  assert.equal(app.$('transactions-insights-monthly').getAttribute('aria-pressed'), 'true');
  assert.equal(app.$('transactions-insights-yearly').getAttribute('aria-pressed'), 'false');
  assert.equal(app.textOf('transactions-insights-title'), 'Spending and income per month');
  assert.equal(app.$('transactions-insights-chart').querySelector('svg').getAttribute('aria-label'), 'Spending and income per month');
  await app.click(app.$('transactions-insights-yearly'));
  assert.deepEqual(labels(app), ['2025', '2026 YTD']);
  assert.equal(app.textOf('transactions-insights-title'), 'Spending and income per year');
  app.close();
});

test('test_csd033_ac_5_3_choosing_the_current_view_again_asks_nothing', async () => {
  const app = await insightsPage({ get_transaction_insights: YEARS });
  const before = app.callsOf('get_transaction_insights').length;
  await app.click(app.$('transactions-insights-yearly'));
  assert.equal(app.callsOf('get_transaction_insights').length, before);
  app.close();
});

test('test_csd033_ac_5_4_the_choice_is_kept_while_the_tab_is_reopened', async () => {
  const app = await insightsPage({ get_transaction_insights: ({ granularity }) => (granularity === 'monthly' ? MONTHS() : YEARS()) });
  await app.click(app.$('transactions-insights-monthly'));
  await app.click(app.$('transactions-tab-overview'));
  await app.click(app.$('transactions-tab-insights'));
  assert.deepEqual(labels(app), ['2026-09', '2026-10 MTD']);
  app.close();
});

test('test_csd033_ac_4_1_a_bar_is_stacked_from_its_segments_as_received', async () => {
  const app = await insightsPage({ get_transaction_insights: YEARS });
  const last = groups(app)[1];
  const pieces = [...last.querySelectorAll('[data-legend^="category-"]')];
  assert.deepEqual(pieces.map((piece) => piece.getAttribute('data-legend')), ['category-cat-groceries', 'category-uncategorized']);
  assert.match(pieces[0].getAttribute('aria-label'), /^Groceries, 2026 YTD, 15\.00$/);
  assert.match(pieces[1].getAttribute('aria-label'), /^Uncategorized, 2026 YTD, 7\.50$/);
  assert.ok(pieces[0].classList.contains('bar-slot-0') && pieces[1].classList.contains('bar-slot-7'));
  app.close();
});

test('test_csd033_ac_7_1_the_legend_lists_every_category_with_uncategorized_last_and_then_income', async () => {
  const app = await insightsPage({ get_transaction_insights: YEARS });
  assert.deepEqual(legendNames(app), ['Groceries', 'Shopping', 'Uncategorized', 'Income']);
  app.close();
});

test('test_csd033_ac_8_1_the_tooltip_lists_the_segments_like_the_bar_top_to_bottom_and_the_total', async () => {
  const app = await insightsPage({ get_transaction_insights: YEARS });
  const group = groups(app)[1];
  group.dispatchEvent(new app.window.MouseEvent('mouseenter'));
  await app.settle();
  const tip = app.$('transactions-insights-chart').querySelector('.chart-tooltip');
  const text = app.text(tip);
  assert.match(text, /^2026 YTDPartial year: spending so far/);
  assert.ok(text.indexOf('Uncategorized') < text.indexOf('Groceries'), 'the top of the bar first');
  assert.match(text, /Total spent22\.50Income30\.00$/);
  app.close();
});

test('test_csd033_ac_8_2_a_month_tooltip_names_the_month_and_a_partial_month', async () => {
  const app = await insightsPage({ get_transaction_insights: () => MONTHS() });
  await app.click(app.$('transactions-insights-monthly'));
  groups(app)[1].dispatchEvent(new app.window.MouseEvent('mouseenter'));
  await app.settle();
  assert.match(app.text(app.$('transactions-insights-chart').querySelector('.chart-tooltip')), /^2026-10 MTDPartial month: spending so far/);
  app.close();
});

test('test_csd033_ac_12_1_without_spending_the_note_is_shown_instead_of_a_chart', async () => {
  const app = await insightsPage({ get_transaction_insights: () => yearly([]) });
  assert.ok(app.isShown(app.$('transactions-insights-empty')));
  assert.equal(app.isShown(app.$('transactions-insights-scroll')), false);
  assert.equal(app.textOf('transactions-insights-empty'), 'No transactions yet. The chart appears once there is a transaction.');
  app.close();
});

test('test_csd033_ac_13_1_opening_the_tab_again_asks_rust_again', async () => {
  const app = await insightsPage({ get_transaction_insights: YEARS });
  const before = app.callsOf('get_transaction_insights').length;
  await app.click(app.$('transactions-tab-overview'));
  await app.click(app.$('transactions-tab-insights'));
  assert.equal(app.callsOf('get_transaction_insights').length, before + 1);
  app.close();
});

test('test_csd033_ac_14_1_dutch_titles_and_names', async () => {
  const reads = exampleReads();
  reads.get_preferences = { theme: 'system', language: 'nl', number_format: 'point_comma', discrete_mode: false };
  const app = await insightsPage({ get_transaction_insights: YEARS }, reads);
  assert.equal(app.textOf('transactions-insights-title'), 'Uitgaven en inkomsten per jaar');
  assert.equal(app.textOf('transactions-insights-yearly'), 'Jaarlijks');
  assert.equal(app.textOf('transactions-insights-monthly'), 'Maandelijks');
  assert.deepEqual(legendNames(app), ['Groceries', 'Shopping', 'Zonder categorie', 'Inkomen']);
  app.close();
});

test('test_csd033_ac_15_1_discrete_mode_hides_the_amounts_in_the_tooltip_and_labels', async () => {
  const reads = exampleReads();
  reads.get_preferences = { theme: 'system', language: 'en', number_format: 'point_comma', discrete_mode: true };
  const app = await insightsPage({ get_transaction_insights: YEARS }, reads);
  const piece = groups(app)[1].querySelector('[data-legend^="category-"]');
  assert.doesNotMatch(piece.getAttribute('aria-label'), /\d\.\d\d/);
  app.close();
});

test('test_csd033_ac_4_3_income_is_a_marker_across_the_bar_with_its_label', async () => {
  const app = await insightsPage({ get_transaction_insights: YEARS });
  const [first, last] = groups(app);
  assert.equal(first.querySelector('.chart-marker'), null, 'no income, no marker');
  assert.ok(last.querySelector('.chart-marker'), 'a marker line');
  assert.equal(app.text(last.querySelector('.chart-marker-label')), '30');
  app.close();
});

test('test_csd033_ac_4_4_a_bar_with_income_only_has_a_marker_and_no_segments', async () => {
  const onlyIncome = yearly([bar(2026, null, [], { income: 9 })]);
  const app = await insightsPage({ get_transaction_insights: () => onlyIncome });
  const [group] = groups(app);
  assert.equal(group.querySelectorAll('[data-legend^="category-"]').length, 0);
  assert.ok(group.querySelector('.chart-marker'));
  group.dispatchEvent(new app.window.MouseEvent('mouseenter'));
  await app.settle();
  const text = app.text(app.$('transactions-insights-chart').querySelector('.chart-tooltip'));
  assert.match(text, /No spending/);
  assert.match(text, /Income9\.00$/);
  app.close();
});

test('test_csd033_ac_7_2_the_income_legend_item_has_the_marker_swatch', async () => {
  const app = await insightsPage({ get_transaction_insights: YEARS });
  const items = [...app.$('transactions-insights-legend').querySelectorAll('li')];
  assert.equal(items.at(-1).getAttribute('data-legend'), 'marker');
  app.close();
});

test('test_csd033_ac_4_3_the_accessible_name_of_a_bar_names_the_income', async () => {
  const app = await insightsPage({ get_transaction_insights: YEARS });
  assert.match(groups(app)[1].getAttribute('aria-label'), /income 30\.00$/);
  app.close();
});

test('test_csd033_ac_8_3_a_tooltip_that_does_not_fit_counts_the_segments_it_leaves_out', async () => {
  // jsdom has no layout, so the room is the least the page allows: three rows.
  const many = ['cat-groceries', 'cat-shopping', 'uncategorized'];
  const crowded = yearly([bar(2026, null, [...many, ...many.map((id) => id)].map((id, index) => [index < 3 ? id : id, '1.00']), { partial: true, income: 9 })]);
  crowded.years[0].segments = Array.from({ length: 6 }, (_, i) => ({ category: i % 3 === 0 ? 'cat-groceries' : i % 3 === 1 ? 'cat-shopping' : 'uncategorized', amount: '1.00' }));
  const app = await insightsPage({ get_transaction_insights: () => crowded });
  groups(app)[0].dispatchEvent(new app.window.MouseEvent('mouseenter'));
  await app.settle();
  const text = app.text(app.$('transactions-insights-chart').querySelector('.chart-tooltip'));
  assert.match(text, /and 4 more categories/, text);
  assert.match(text, /Total spent/);
  assert.match(text, /Income9\.00$/);
  app.close();
});
