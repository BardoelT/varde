// Display ACs of CSD-036 (budgets): the tab, one month at a time, the rows with the bar, the dialog with its
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

test('test_csd036_ac_1_1_the_budgets_tab_is_the_last_tab_and_opens_the_panel', async () => {
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

test('test_csd036_ac_1_2_there_is_no_budget_page_in_the_left_pane', async () => {
  const app = await openApp();
  const texts = [...app.document.querySelectorAll('.nav-item')].map((item) => app.text(item));
  assert.ok(!texts.includes('Budget') && !texts.includes('Budgets'));
  app.close();
});

test('test_csd036_ac_2_1_the_tab_opens_on_the_current_month', async () => {
  const app = await budgetsPage();
  assert.equal(app.textOf('budgets-month'), '2026-10 MTD');
  assert.deepEqual(app.callsOf('get_budgets').at(-1).args, {});
  app.close();
});

test('test_csd036_ac_2_2_the_buttons_move_by_one_month_and_back_stops_at_the_earliest', async () => {
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

test('test_csd036_ac_2_2_the_year_changes_between_december_and_january', async () => {
  const app = await budgetsPage({
    get_budgets: ({ month: asked }) => (asked === undefined ? month('2026-12', [row('Groceries')], { partial: true }) : month(asked, [row('Groceries')], { earliest: '2026-01' })),
  });
  await app.click(app.$('budgets-next'));
  assert.equal(app.textOf('budgets-month'), '2027-01');
  await app.click(app.$('budgets-previous'));
  assert.equal(app.textOf('budgets-month'), '2026-12');
  app.close();
});

test('test_csd036_ac_2_3_nothing_to_go_back_to', async () => {
  const app = await budgetsPage({ get_budgets: () => month('2026-10', [row('Groceries')], { partial: true, earliest: '2026-10' }) });
  assert.equal(app.$('budgets-previous').disabled, true);
  app.close();
});

test('test_csd036_ac_2_4_the_month_stays_while_the_tab_is_reopened_and_resets_for_another_project', async () => {
  const app = await budgetsPage({ get_budgets: ({ month: asked }) => month(asked ?? '2026-10', [row('Groceries')], { partial: asked === undefined }) });
  await app.click(app.$('budgets-previous'));
  assert.equal(app.textOf('budgets-month'), '2026-09');
  await app.click(app.$('transactions-tab-insights'));
  await app.click(app.$('transactions-tab-budgets'));
  assert.equal(app.textOf('budgets-month'), '2026-09');
  assert.equal(app.callsOf('get_budgets').at(-1).args.month, '2026-09');
  app.close();
});

test('test_csd036_ac_3_1_a_month_shows_budget_spent_remaining_and_a_bar_per_category', async () => {
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

test('test_csd036_ac_8_1_a_single_month_amount_is_labelled', async () => {
  const app = await budgetsPage();
  assert.match(cells(app, 'Leisure')[0], /This month only/);
  assert.doesNotMatch(cells(app, 'Groceries')[0], /This month only/);
  app.close();
});

test('test_csd036_ac_6_1_the_dialog_asks_for_the_amount_and_a_scope_and_changes_nothing_until_confirmed', async () => {
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

test('test_csd036_ac_6_2_confirming_sends_the_amount_the_month_and_the_chosen_scope', async () => {
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

test('test_csd036_ac_6_3_the_default_scope_is_from_this_month_on', async () => {
  const app = await budgetsPage({ set_budget: () => OCTOBER() });
  await app.click(rowOf(app, 'Shopping').querySelector('button'));
  app.$('budget-form').elements.amount.value = '75.00';
  app.$('budget-form').requestSubmit();
  await app.settle();
  assert.equal(app.callsOf('set_budget').at(-1).args.scope, 'from');
  app.close();
});

test('test_csd036_ac_7_1_removing_asks_for_a_scope_without_an_amount', async () => {
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

test('test_csd036_ac_7_5_a_row_without_a_budget_has_no_remove_button', async () => {
  const app = await budgetsPage();
  assert.equal(rowOf(app, 'Shopping').querySelector('button[aria-label^="Remove budget"]'), null);
  assert.ok(rowOf(app, 'Groceries').querySelector('button[aria-label^="Remove budget"]'));
  app.close();
});

test('test_csd036_ac_9_1_a_rejected_amount_shows_its_message_in_the_dialog', async () => {
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

test('test_csd036_ac_14_1_amounts_are_hidden_in_discrete_mode_and_the_bars_stay', async () => {
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

test('test_csd036_ac_15_1_dutch', async () => {
  const reads = exampleReads();
  reads.get_preferences = { ...reads.get_preferences, language: 'nl' };
  const app = await budgetsPage({}, reads);
  assert.equal(app.text(app.$('transactions-tab-budgets')), 'Budgetten');
  assert.equal(cells(app, 'Shopping')[0], 'Budget instellen');
  assert.ok(rowOf(app, 'Niet begroot'));
  assert.equal(app.textOf('budgets-previous'), 'Vorige maand');
  app.close();
});

test('test_csd036_ac_16_1_no_categories_shows_a_note_and_no_table', async () => {
  const app = await budgetsPage({ get_budgets: () => month('2026-10', [], { partial: true }) });
  assert.ok(app.isShown(app.$('budgets-empty')));
  assert.equal(app.textOf('budgets-empty'), 'No categories yet. Add a category on the tab Categories to set a budget.');
  assert.equal(app.isShown(app.$('budgets-content')), false);
  app.close();
});

test('test_csd036_ac_16_2_the_tab_asks_rust_again_each_time_it_opens', async () => {
  const app = await budgetsPage();
  const before = app.callsOf('get_budgets').length;
  await app.click(app.$('transactions-tab-transactions'));
  await app.click(app.$('transactions-tab-budgets'));
  assert.equal(app.callsOf('get_budgets').length, before + 1);
  app.close();
});
