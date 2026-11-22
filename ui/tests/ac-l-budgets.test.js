// Display ACs of CSD-037 (budgets): the tab, one month at a time, the rows with the bar, the dialog with its
// three scopes, the label of a single-month amount, Discrete mode, the Dutch texts, the empty state. The
// answers are small months made for one AC (what Rust returns for them is proven by `cargo test`).
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp, commandError } from './harness.js';
import { exampleReads } from './fixtures.js';

const row = (name, { budget = null, spent = '0.00', remaining = null, bar = null, over = false, single = false } = {}) => ({
  category: `cat-${name.toLowerCase()}`, name, budget, single_month: single, spent, remaining, bar_percent: bar, over,
});

/** A month as Rust sends it. */
function month(value, rows, { partial = false, earliest = '2026-08', notBudgeted = '30.00', total = { budget: '500.00', spent: '270.00', remaining: '230.00' } } = {}) {
  return { month: value, is_partial: partial, earliest_month: earliest, rows, not_budgeted: notBudgeted, total };
}

const OCTOBER = () =>
  month('2026-10', [
    row('Groceries', { budget: '400.00', spent: '150.00', remaining: '250.00', bar: '37.50' }),
    row('Leisure', { budget: '100.00', spent: '120.00', remaining: '-20.00', bar: '100.00', over: true, single: true }),
    row('Shopping', { spent: '45.00' }),
  ], { partial: true });

async function budgetsPage(handlers = {}, reads = exampleReads()) {
  const app = await openApp({ reads, handlers: { get_budgets: OCTOBER, ...handlers } });
  await app.click(app.item('transactions'));
  await app.click(app.$('transactions-tab-budgets'));
  return app;
}

const rowOf = (app, name) => [...app.$('budgets-rows').querySelectorAll('tr')].find((tr) => app.text(tr.querySelector('th')) === name);
const cells = (app, name) => [...rowOf(app, name).querySelectorAll('td')].map((cell) => app.text(cell));

test('test_csd037_ac_1_1_the_budgets_tab_is_the_last_tab_and_opens_the_panel', async () => {
  const app = await openApp({ handlers: { get_budgets: OCTOBER } });
  await app.click(app.item('transactions'));
  assert.deepEqual([...app.document.querySelectorAll('#transactions [role="tab"]')].map((tab) => app.text(tab)), ['Accounts', 'Categories', 'Rules', 'Yearly totals', 'Transactions', 'Insights', 'Budgets']);
  assert.equal(app.$('transactions-tab-transactions').getAttribute('aria-selected'), 'true');
  assert.equal(app.isShown(app.$('transactions-budgets')), false);
  await app.click(app.$('transactions-tab-budgets'));
  assert.ok(app.isShown(app.$('transactions-budgets')));
  assert.equal(app.text(app.$('transactions-budgets').querySelector('.panel-title')), 'Budgets');
  app.close();
});

test('test_csd037_ac_1_2_there_is_no_budget_page_in_the_left_pane', async () => {
  const app = await openApp();
  const texts = [...app.document.querySelectorAll('.nav-item')].map((item) => app.text(item));
  assert.ok(!texts.includes('Budget') && !texts.includes('Budgets'));
  app.close();
});

test('test_csd037_ac_2_1_the_tab_opens_on_the_current_month', async () => {
  const app = await budgetsPage();
  assert.equal(app.textOf('budgets-month'), '2026-10 MTD');
  assert.deepEqual(app.callsOf('get_budgets').at(-1).args, {});
  app.close();
});

test('test_csd037_ac_2_2_the_buttons_move_by_one_month_and_back_stops_at_the_earliest', async () => {
  const app = await budgetsPage({
    get_budgets: ({ month: asked }) => (asked === undefined ? OCTOBER() : month(asked, [row('Groceries')], { earliest: '2026-08', partial: false })),
  });
  assert.equal(app.$('budgets-previous').disabled, false);
  await app.click(app.$('budgets-previous'));
  await app.click(app.$('budgets-previous'));
  assert.deepEqual(app.callsOf('get_budgets').map((call) => call.args.month), [undefined, '2026-09', '2026-08']);
  assert.equal(app.textOf('budgets-month'), '2026-08');
  assert.equal(app.$('budgets-previous').disabled, true);
  for (let step = 0; step < 4; step += 1) await app.click(app.$('budgets-next'));
  assert.equal(app.textOf('budgets-month'), '2026-12');
  assert.equal(app.$('budgets-next').disabled, false);
  await app.click(app.$('budgets-previous'));
  assert.equal(app.textOf('budgets-month'), '2026-11');
  app.close();
});

test('test_csd037_ac_2_2_the_year_changes_between_december_and_january', async () => {
  const app = await budgetsPage({
    get_budgets: ({ month: asked }) => (asked === undefined ? month('2026-12', [row('Groceries')], { partial: true }) : month(asked, [row('Groceries')], { earliest: '2026-01' })),
  });
  await app.click(app.$('budgets-next'));
  assert.equal(app.textOf('budgets-month'), '2027-01');
  await app.click(app.$('budgets-previous'));
  assert.equal(app.textOf('budgets-month'), '2026-12');
  app.close();
});

test('test_csd037_ac_2_3_nothing_to_go_back_to', async () => {
  const app = await budgetsPage({ get_budgets: () => month('2026-10', [row('Groceries')], { partial: true, earliest: '2026-10' }) });
  assert.equal(app.$('budgets-previous').disabled, true);
  app.close();
});

test('test_csd037_ac_2_4_the_month_stays_while_the_tab_is_reopened_and_resets_for_another_project', async () => {
  const app = await budgetsPage({ get_budgets: ({ month: asked }) => month(asked ?? '2026-10', [row('Groceries')], { partial: asked === undefined }) });
  await app.click(app.$('budgets-previous'));
  assert.equal(app.textOf('budgets-month'), '2026-09');
  await app.click(app.$('transactions-tab-insights'));
  await app.click(app.$('transactions-tab-budgets'));
  assert.equal(app.textOf('budgets-month'), '2026-09');
  assert.equal(app.callsOf('get_budgets').at(-1).args.month, '2026-09');
  app.close();
});

test('test_csd037_ac_3_1_a_month_shows_budget_spent_remaining_and_a_bar_per_category', async () => {
  const app = await budgetsPage();
  assert.deepEqual(cells(app, 'Groceries').slice(0, 3), ['400.00', '150.00', '250.00']);
  const leisure = cells(app, 'Leisure');
  assert.deepEqual([leisure[0].startsWith('100.00'), leisure[1], leisure[2]], [true, '120.00', '-20.00']);
  assert.ok(rowOf(app, 'Leisure').querySelector('.budget-bar.over'), 'over budget is marked');
  assert.ok(!rowOf(app, 'Groceries').querySelector('.budget-bar.over'));
  assert.equal(rowOf(app, 'Groceries').querySelector('.budget-bar-fill').style.width, '37.5%');
  const shopping = cells(app, 'Shopping');
  assert.deepEqual([shopping[0], shopping[1], shopping[2]], ['Set budget', '45.00', '']);
  assert.equal(rowOf(app, 'Shopping').querySelector('.budget-bar'), null, 'no budget, no bar');
  assert.equal(cells(app, 'Not budgeted')[1], '30.00');
  assert.deepEqual(cells(app, 'Total').slice(0, 3), ['500.00', '270.00', '230.00']);
  const names = [...app.$('budgets-rows').querySelectorAll('th')].map((th) => app.text(th));
  assert.deepEqual(names, ['Groceries', 'Leisure', 'Shopping', 'Not budgeted', 'Total']);
  app.close();
});

test('test_csd037_ac_8_1_a_single_month_amount_is_labelled', async () => {
  const app = await budgetsPage();
  assert.match(cells(app, 'Leisure')[0], /This month only/);
  assert.doesNotMatch(cells(app, 'Groceries')[0], /This month only/);
  app.close();
});

test('test_csd037_ac_6_1_the_dialog_asks_for_the_amount_and_a_scope_and_changes_nothing_until_confirmed', async () => {
  const app = await budgetsPage();
  await app.click(rowOf(app, 'Shopping').querySelector('button'));
  assert.ok(app.isShown(app.$('budget-dialog')));
  assert.equal(app.textOf('budget-dialog-title'), 'Set budget');
  const scopes = [...app.$('budget-form').querySelectorAll('input[name="scope"]')];
  assert.deepEqual(scopes.map((input) => [input.value, input.checked]), [['month', false], ['from', true], ['all', false]]);
  assert.deepEqual([...app.$('budget-form').querySelectorAll('.scope-option')].map((label) => app.text(label)), ['Only this month', 'From this month on', 'All months']);
  assert.match(app.textOf('budget-context'), /Shopping, 2026-10/);
  await app.click(app.$('budget-dialog').querySelector('[data-cancel]'));
  assert.equal(app.callsOf('set_budget').length, 0);
  app.close();
});

test('test_csd037_ac_6_2_confirming_sends_the_amount_the_month_and_the_chosen_scope', async () => {
  const app = await budgetsPage({ set_budget: () => OCTOBER() });
  await app.click(rowOf(app, 'Shopping').querySelector('button'));
  app.$('budget-form').elements.amount.value = '75.00';
  [...app.$('budget-form').querySelectorAll('input[name="scope"]')].find((input) => input.value === 'month').checked = true;
  app.$('budget-form').requestSubmit();
  await app.settle();
  assert.deepEqual(app.callsOf('set_budget').at(-1).args, { categoryId: 'cat-shopping', month: '2026-10', amount: '75.00', scope: 'month' });
  assert.equal(app.$('budget-dialog').open, false);
  app.close();
});

test('test_csd037_ac_6_3_the_default_scope_is_from_this_month_on', async () => {
  const app = await budgetsPage({ set_budget: () => OCTOBER() });
  await app.click(rowOf(app, 'Shopping').querySelector('button'));
  app.$('budget-form').elements.amount.value = '75.00';
  app.$('budget-form').requestSubmit();
  await app.settle();
  assert.equal(app.callsOf('set_budget').at(-1).args.scope, 'from');
  app.close();
});

test('test_csd037_ac_7_1_removing_asks_for_a_scope_without_an_amount', async () => {
  const app = await budgetsPage({ remove_budget: () => OCTOBER() });
  await app.click(rowOf(app, 'Groceries').querySelector('button[aria-label="Remove budget of Groceries"]'));
  assert.equal(app.textOf('budget-dialog-title'), 'Remove budget');
  assert.equal(app.isShown(app.$('budget-amount-field')), false);
  app.$('budget-form').requestSubmit();
  await app.settle();
  assert.deepEqual(app.callsOf('remove_budget').at(-1).args, { categoryId: 'cat-groceries', month: '2026-10', scope: 'from' });
  assert.equal(app.callsOf('set_budget').length, 0);
  app.close();
});

test('test_csd037_ac_7_5_a_row_without_a_budget_has_no_remove_button', async () => {
  const app = await budgetsPage();
  assert.equal(rowOf(app, 'Shopping').querySelector('button[aria-label^="Remove budget"]'), null);
  assert.ok(rowOf(app, 'Groceries').querySelector('button[aria-label^="Remove budget"]'));
  app.close();
});

test('test_csd037_ac_9_1_a_rejected_amount_shows_its_message_in_the_dialog', async () => {
  const app = await budgetsPage({
    set_budget: () => {
      throw commandError('BUDGET_AMOUNT_INVALID', 'Enter a budget of 0.00 or more.');
    },
  });
  await app.click(rowOf(app, 'Shopping').querySelector('button'));
  app.$('budget-form').elements.amount.value = '-5';
  app.$('budget-form').requestSubmit();
  await app.settle();
  assert.equal(app.$('budget-dialog').open, true);
  assert.equal(app.textOf('budget-error'), 'Enter a budget of 0.00 or more.');
  app.close();
});

test('test_csd037_ac_14_1_amounts_are_hidden_in_discrete_mode_and_the_bars_stay', async () => {
  const reads = exampleReads();
  reads.get_preferences = { ...reads.get_preferences, discrete_mode: true };
  const app = await budgetsPage({}, reads);
  const text = app.text(app.$('budgets-rows'));
  assert.ok(!/\d{2,}\.\d{2}/.test(text), `no amount in the table: ${text}`);
  assert.ok(rowOf(app, 'Groceries').querySelector('.budget-bar'), 'the bar is drawn');
  assert.ok(rowOf(app, 'Leisure').querySelector('.budget-bar.over'), 'the over-budget mark stays');
  const labels = [...app.$('budgets-rows').querySelectorAll('[aria-label]')].map((element) => element.getAttribute('aria-label')).join(' ');
  assert.ok(!/\d{2,}\.\d{2}(?![\d%])/.test(labels), `no amount in the accessible names: ${labels}`);
  app.close();
});

test('test_csd037_ac_15_1_dutch', async () => {
  const reads = exampleReads();
  reads.get_preferences = { ...reads.get_preferences, language: 'nl' };
  const app = await budgetsPage({}, reads);
  assert.equal(app.text(app.$('transactions-tab-budgets')), 'Budgetten');
  assert.equal(cells(app, 'Shopping')[0], 'Budget instellen');
  assert.ok(rowOf(app, 'Niet begroot'));
  assert.equal(app.textOf('budgets-previous'), 'Vorige maand');
  app.close();
});

test('test_csd037_ac_16_1_no_categories_shows_a_note_and_no_table', async () => {
  const app = await budgetsPage({ get_budgets: () => month('2026-10', [], { partial: true }) });
  assert.ok(app.isShown(app.$('budgets-empty')));
  assert.equal(app.textOf('budgets-empty'), 'No categories yet. Add a category on the tab Categories to set a budget.');
  assert.equal(app.isShown(app.$('budgets-content')), false);
  app.close();
});

test('test_csd037_ac_16_2_the_tab_asks_rust_again_each_time_it_opens', async () => {
  const app = await budgetsPage();
  const before = app.callsOf('get_budgets').length;
  await app.click(app.$('transactions-tab-transactions'));
  await app.click(app.$('transactions-tab-budgets'));
  assert.equal(app.callsOf('get_budgets').length, before + 1);
  app.close();
});

// ── The Year view (CSD-037 REQ-18 to REQ-22) ──

const none = () => ({ budget: null, single_month: false });
const has = (amount, single = false) => ({ budget: amount, single_month: single });
const yearRow = (name, months, total) => ({ category: `cat-${name.toLowerCase()}`, name, months, total });

/** A year as Rust sends it: Groceries 400.00 every month, Leisure 100.00 from Oct, Shopping none (AC-20.1). */
function year(value, { partial = false, earliest = 2024, single = -1, rows = null } = {}) {
  const groceries = Array.from({ length: 12 }, (_, index) => has('400.00', index === single));
  const leisure = Array.from({ length: 12 }, (_, index) => (index >= 9 ? has('100.00') : none()));
  const shopping = Array.from({ length: 12 }, none);
  return {
    year: value, is_partial: partial, earliest_year: earliest,
    figures: { budget: '5100.00', spent: '290.00', remaining: '4810.00' },
    rows: rows ?? [yearRow('Groceries', groceries, '4800.00'), yearRow('Leisure', leisure, '300.00'), yearRow('Shopping', shopping, null)],
    months: [...Array(9).fill('400.00'), ...Array(3).fill('500.00')],
    total: '5100.00',
  };
}

const yearHandlers = (extra = {}) => ({
  get_budget_year: ({ year: asked } = {}) => year(asked === undefined ? 2026 : Number(asked), { partial: asked === undefined || asked === '2026' }),
  ...extra,
});

async function yearPage(handlers = {}, reads = exampleReads()) {
  const app = await budgetsPage({ ...yearHandlers(), ...handlers }, reads);
  await app.click(app.$('budgets-view-year'));
  return app;
}

const gridRow = (app, name) => [...app.$('budgets-year-grid').querySelectorAll('tbody tr')].find((tr) => app.text(tr.querySelector('th')) === name);
const gridCells = (app, name) => [...gridRow(app, name).querySelectorAll('td')].map((cell) => app.text(cell));

test('test_csd037_ac_18_1_the_tab_opens_on_the_month_view', async () => {
  const app = await budgetsPage();
  assert.equal(app.$('budgets-view-month').getAttribute('aria-pressed'), 'true');
  assert.equal(app.$('budgets-view-year').getAttribute('aria-pressed'), 'false');
  assert.deepEqual([...app.$('budgets-view-year').parentElement.children].map((button) => app.text(button)), ['Year', 'Month'], 'the order of the choice, as Yearly | Monthly on the Insights tab');
  assert.equal(app.$('budgets-view-year').closest('.panel-header').classList.contains('budgets-header'), true, 'with the space of the Insights header');
  assert.equal(app.textOf('budgets-month'), '2026-10 MTD');
  assert.equal(app.callsOf('get_budget_year').length, 0);
  assert.equal(app.isShown(app.$('budgets-year')), false);
  app.close();
});

test('test_csd037_ac_18_2_year_opens_on_the_year_of_the_month_shown', async () => {
  const app = await budgetsPage({ ...yearHandlers(), get_budgets: ({ month: asked } = {}) => month(asked ?? '2026-10', [row('Groceries')], { partial: asked === undefined, earliest: '2024-01' }) });
  await app.click(app.$('budgets-previous'));
  for (let step = 0; step < 6; step += 1) await app.click(app.$('budgets-previous'));
  assert.equal(app.textOf('budgets-month'), '2026-03');
  await app.click(app.$('budgets-view-year'));
  assert.equal(app.$('budgets-view-year').getAttribute('aria-pressed'), 'true');
  assert.equal(app.textOf('budgets-month'), '2026 YTD');
  assert.deepEqual(app.callsOf('get_budget_year').at(-1).args, { year: '2026' });
  app.close();
});

test('test_csd037_ac_18_3_month_returns_to_the_month_it_was_on', async () => {
  const app = await budgetsPage({ ...yearHandlers(), get_budgets: ({ month: asked } = {}) => month(asked ?? '2026-10', [row('Groceries')], { partial: asked === undefined, earliest: '2024-01' }) });
  for (let step = 0; step < 7; step += 1) await app.click(app.$('budgets-previous'));
  assert.equal(app.textOf('budgets-month'), '2026-03');
  await app.click(app.$('budgets-view-year'));
  await app.click(app.$('budgets-previous'));
  await app.click(app.$('budgets-previous'));
  assert.equal(app.textOf('budgets-month'), '2024');
  await app.click(app.$('budgets-view-month'));
  assert.equal(app.textOf('budgets-month'), '2026-03');
  assert.equal(app.callsOf('get_budgets').at(-1).args.month, '2026-03');
  app.close();
});

test('test_csd037_ac_18_4_the_choice_is_kept_while_the_tab_is_reopened', async () => {
  const app = await yearPage();
  await app.click(app.$('transactions-tab-insights'));
  await app.click(app.$('transactions-tab-budgets'));
  assert.equal(app.$('budgets-view-year').getAttribute('aria-pressed'), 'true');
  assert.ok(app.isShown(app.$('budgets-year')));
  assert.equal(app.callsOf('get_budget_year').length, 2, 'asked again when the tab opened');
  app.close();
});

test('test_csd037_ac_18_5_choosing_the_shown_option_again_asks_nothing', async () => {
  const app = await budgetsPage();
  const before = app.callsOf('get_budgets').length;
  await app.click(app.$('budgets-view-month'));
  assert.equal(app.callsOf('get_budgets').length, before);
  const yearApp = await yearPage();
  await yearApp.click(yearApp.$('budgets-view-year'));
  assert.equal(yearApp.callsOf('get_budget_year').length, 1);
  app.close();
  yearApp.close();
});

test('test_csd037_ac_19_1_the_year_view_opens_on_the_current_year', async () => {
  const app = await yearPage();
  assert.equal(app.textOf('budgets-month'), '2026 YTD');
  assert.equal(app.textOf('budgets-previous'), 'Previous year');
  assert.equal(app.textOf('budgets-next'), 'Next year');
  app.close();
});

test('test_csd037_ac_19_2_the_buttons_move_by_one_year_and_back_stops_at_the_earliest', async () => {
  const app = await yearPage();
  assert.equal(app.$('budgets-previous').disabled, false);
  await app.click(app.$('budgets-previous'));
  await app.click(app.$('budgets-previous'));
  assert.deepEqual(app.callsOf('get_budget_year').map((call) => call.args.year), ['2026', '2025', '2024']);
  assert.equal(app.textOf('budgets-month'), '2024');
  assert.equal(app.$('budgets-previous').disabled, true);
  for (let step = 0; step < 4; step += 1) await app.click(app.$('budgets-next'));
  assert.equal(app.textOf('budgets-month'), '2028');
  assert.equal(app.$('budgets-next').disabled, false);
  app.close();
});

test('test_csd037_ac_19_3_nothing_to_go_back_to', async () => {
  const app = await yearPage({ get_budget_year: () => year(2026, { partial: true, earliest: 2026 }) });
  assert.equal(app.$('budgets-previous').disabled, true);
  app.close();
});

test('test_csd037_ac_19_4_the_buttons_are_the_same_buttons_in_the_same_place', async () => {
  const app = await budgetsPage(yearHandlers());
  const [previous, next, month] = [app.$('budgets-previous'), app.$('budgets-next'), app.$('budgets-month')];
  await app.click(app.$('budgets-view-year'));
  assert.equal(app.$('budgets-previous'), previous);
  assert.equal(app.$('budgets-next'), next);
  assert.equal(app.$('budgets-month'), month);
  assert.ok(previous.parentElement.classList.contains('month-nav'));
  assert.equal(app.textOf('budgets-previous'), 'Previous year');
  await app.click(app.$('budgets-view-month'));
  assert.equal(app.textOf('budgets-previous'), 'Previous month');
  app.close();
});

test('test_csd037_ac_20_1_a_year_with_budgets', async () => {
  const app = await yearPage();
  assert.deepEqual([...app.$('budgets-year-grid').querySelectorAll('thead th')].map((th) => app.text(th)), ['Category', 'Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec', 'Total']);
  assert.deepEqual(gridCells(app, 'Groceries'), [...Array(12).fill('400.00'), '4,800.00']);
  assert.deepEqual(gridCells(app, 'Leisure'), [...Array(9).fill('–'), '100.00', '100.00', '100.00', '300.00']);
  assert.deepEqual(gridCells(app, 'Total'), [...Array(9).fill('400.00'), ...Array(3).fill('500.00'), '5,100.00']);
  assert.deepEqual([...app.$('budgets-year-figures').querySelectorAll('.year-figure')].map((figure) => app.text(figure)), ['Budget5,100.00', 'Spent290.00', 'Remaining4,810.00']);
  assert.equal(app.isShown(app.$('budgets-content')), false, 'the month table is hidden');
  app.close();
});

test('test_csd037_ac_20_3_a_single_month_cell_is_marked', async () => {
  const app = await yearPage({ get_budget_year: () => year(2026, { partial: true, single: 9 }) });
  const marked = [...gridRow(app, 'Groceries').querySelectorAll('td.single-month-cell')];
  assert.equal(marked.length, 1);
  assert.equal(marked[0].getAttribute('title'), 'This month only');
  assert.match(marked[0].querySelector('button').getAttribute('aria-label'), /Oct.*This month only/);
  app.close();
});

test('test_csd037_ac_20_4_nothing_but_budgets_in_the_grid', async () => {
  const app = await yearPage();
  assert.equal(app.$('budgets-year-grid').querySelector('.budget-bar'), null);
  const names = [...app.$('budgets-year-grid').querySelectorAll('tbody th')].map((th) => app.text(th));
  assert.deepEqual(names, ['Groceries', 'Leisure', 'Shopping', 'Total']);
  app.close();
});

test('test_csd037_ac_20_6_a_year_without_a_budget_is_all_dashes', async () => {
  const empty = Array.from({ length: 12 }, none);
  const app = await yearPage({ get_budget_year: () => ({ ...year(2026, { partial: true, rows: [yearRow('Groceries', empty, null)] }), figures: { budget: '0.00', spent: '0.00', remaining: '0.00' }, months: Array(12).fill(null), total: null }) });
  assert.deepEqual(gridCells(app, 'Groceries'), Array(13).fill('–'));
  assert.deepEqual(gridCells(app, 'Total'), Array(13).fill('–'));
  assert.equal(app.text(app.$('budgets-year-figures').querySelector('.year-figure')), 'Budget0.00');
  app.close();
});

test('test_csd037_ac_21_1_an_empty_cell_opens_the_dialog_for_its_month', async () => {
  const app = await yearPage();
  await app.click(gridRow(app, 'Shopping').querySelectorAll('button.cell-button')[2]);
  assert.ok(app.isShown(app.$('budget-dialog')));
  assert.equal(app.textOf('budget-dialog-title'), 'Set budget');
  assert.match(app.textOf('budget-context'), /Shopping, 2026-03/);
  const scopes = [...app.$('budget-form').querySelectorAll('input[name="scope"]')];
  assert.deepEqual(scopes.map((input) => [input.value, input.checked]), [['month', false], ['from', true], ['all', false]]);
  assert.equal(app.isShown(app.$('budget-remove')), false, 'nothing to remove');
  app.close();
});

test('test_csd037_ac_21_2_saving_applies_the_scope_to_the_month_of_the_cell_and_shows_the_year_again', async () => {
  const app = await yearPage({ set_budget: () => OCTOBER() });
  await app.click(gridRow(app, 'Shopping').querySelectorAll('button.cell-button')[2]);
  app.$('budget-form').elements.amount.value = '75.00';
  app.$('budget-form').requestSubmit();
  await app.settle();
  assert.deepEqual(app.callsOf('set_budget').at(-1).args, { categoryId: 'cat-shopping', month: '2026-03', amount: '75.00', scope: 'from' });
  assert.equal(app.$('budget-dialog').open, false);
  assert.equal(app.callsOf('get_budget_year').length, 2, 'the year was asked for again');
  assert.equal(app.isShown(app.$('budgets-year')), true);
  app.close();
});

test('test_csd037_ac_21_3_a_cell_with_a_budget_offers_to_remove_it', async () => {
  const app = await yearPage({ remove_budget: () => OCTOBER() });
  await app.click(gridRow(app, 'Groceries').querySelectorAll('button.cell-button')[5]);
  assert.equal(app.$('budget-form').elements.amount.value, '400.00');
  assert.ok(app.isShown(app.$('budget-remove')));
  await app.click(app.$('budget-remove'));
  assert.equal(app.textOf('budget-dialog-title'), 'Remove budget');
  assert.equal(app.isShown(app.$('budget-amount-field')), false);
  [...app.$('budget-form').querySelectorAll('input[name="scope"]')].find((input) => input.value === 'month').checked = true;
  app.$('budget-form').requestSubmit();
  await app.settle();
  assert.deepEqual(app.callsOf('remove_budget').at(-1).args, { categoryId: 'cat-groceries', month: '2026-06', scope: 'month' });
  assert.equal(app.callsOf('set_budget').length, 0);
  app.close();
});

test('test_csd037_ac_21_4_a_rejected_amount_stays_in_the_dialog', async () => {
  const app = await yearPage({
    set_budget: () => {
      throw commandError('BUDGET_AMOUNT_INVALID', 'Enter a budget of 0.00 or more.');
    },
  });
  await app.click(gridRow(app, 'Shopping').querySelectorAll('button.cell-button')[0]);
  app.$('budget-form').elements.amount.value = '-5';
  app.$('budget-form').requestSubmit();
  await app.settle();
  assert.equal(app.$('budget-dialog').open, true);
  assert.equal(app.textOf('budget-error'), 'Enter a budget of 0.00 or more.');
  app.close();
});

test('test_csd037_ac_21_the_month_view_never_offers_the_remove_button_in_the_dialog', async () => {
  const app = await budgetsPage();
  await app.click(rowOf(app, 'Groceries').querySelector('button[aria-label^="Edit budget"]'));
  assert.equal(app.isShown(app.$('budget-remove')), false);
  app.close();
});

test('test_csd037_ac_22_1_and_22_2_rust_is_asked_for_the_year_shown', async () => {
  const app = await yearPage();
  await app.click(app.$('budgets-previous'));
  assert.deepEqual(app.callsOf('get_budget_year').map((call) => call.args), [{ year: '2026' }, { year: '2025' }]);
  await app.click(app.$('transactions-tab-transactions'));
  await app.click(app.$('transactions-tab-budgets'));
  assert.deepEqual(app.callsOf('get_budget_year').at(-1).args, { year: '2025' }, 'fresh figures for the year shown');
  app.close();
});

test('test_csd037_ac_14_2_the_year_view_hides_its_amounts_in_discrete_mode', async () => {
  const reads = exampleReads();
  reads.get_preferences = { ...reads.get_preferences, discrete_mode: true };
  const app = await yearPage({}, reads);
  const text = app.text(app.$('transactions-budgets'));
  assert.ok(!/\d{2,}\.\d{2}/.test(text), `no amount in the page text: ${text}`);
  const labels = [...app.$('transactions-budgets').querySelectorAll('[aria-label]')].map((element) => element.getAttribute('aria-label')).join(' ');
  assert.ok(!/\d{2,}\.\d{2}(?![\d%])/.test(labels), `no amount in the accessible names: ${labels}`);
  app.close();
});

test('test_csd037_ac_15_1_the_toggle_and_the_year_buttons_in_dutch', async () => {
  const reads = exampleReads();
  reads.get_preferences = { ...reads.get_preferences, language: 'nl' };
  const app = await yearPage({}, reads);
  assert.equal(app.textOf('budgets-view-month'), 'Maand');
  assert.equal(app.textOf('budgets-view-year'), 'Jaar');
  assert.equal(app.textOf('budgets-previous'), 'Vorig jaar');
  assert.equal(app.textOf('budgets-next'), 'Volgend jaar');
  app.close();
});

test('test_csd037_ac_16_3_no_categories_shows_the_note_and_no_grid_in_the_year_view', async () => {
  const app = await yearPage({ get_budget_year: () => ({ ...year(2026, { partial: true, rows: [] }), months: Array(12).fill(null), total: null }) });
  assert.ok(app.isShown(app.$('budgets-empty')));
  assert.equal(app.isShown(app.$('budgets-year')), false);
  app.close();
});
