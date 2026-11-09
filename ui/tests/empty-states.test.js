// Empty states: the Insights notes (CSD-025 REQ-16 and the area CSDs) and the prompts of the overview pages.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { emptyReads, exampleReads } from './fixtures.js';
import { TAB_PAGES } from './areas.js';

/** Per area: the AC of its own CSD that names the Insights note, and the note. */
const INSIGHTS_NOTES = {
  income: ['csd006_ac_10_1', 'No income yet. The chart appears once there is a paycheck or an annual statement.'],
  investments: ['csd007_ac_12_1', 'No statements yet. The chart appears once there is a statement.'],
  bank: ['csd010_ac_14_1', 'No entries yet. The chart appears once there is an entry.'],
  houses: ['csd016_ac_14_1', 'No valuations yet. The chart appears once there is a valuation.'],
  debts: ['csd012_ac_14_1', 'No statements yet. The chart appears once there is a statement.'],
  expenses: ['csd014_ac_12_1', 'No expenses yet. The chart appears once there is an expense.'],
};

for (const { page, insights } of TAB_PAGES) {
  const [ac, note] = INSIGHTS_NOTES[page];

  test(`test_${ac}_the_note_replaces_the_chart_${page}`, async () => {
    // The page has records (an account, a debt, a job...) but the insights series is empty.
    const reads = exampleReads();
    reads[insights] = { ...reads[insights], years: [] };
    const app = await openApp({ reads });
    await app.click(app.item(page));
    await app.click(app.$(`${page}-tab-insights`));
    assert.equal(app.isShown(app.$(`${page}-insights-empty`)), true);
    assert.equal(app.textOf(`${page}-insights-empty`), note);
    assert.equal(app.isShown(app.$(`${page}-insights-scroll`)), false);
    assert.equal(app.$(`${page}-insights-chart`).children.length, 0, 'no chart is drawn');
    app.close();
  });
}

test('test_csd025_ac_16_1_nothing_to_draw', async () => {
  for (const { page, insights } of TAB_PAGES) {
    const reads = exampleReads();
    reads[insights] = { ...reads[insights], years: [] };
    const app = await openApp({ reads });
    await app.click(app.item(page));
    await app.click(app.$(`${page}-tab-insights`));
    assert.equal(app.isShown(app.$(`${page}-insights-empty`)), true, `${page}: the note of the area is shown`);
    assert.equal(app.isShown(app.$(`${page}-insights-scroll`)), false);
    assert.equal(app.$(`${page}-insights-chart`).querySelectorAll('svg').length, 0, `${page}: no chart is drawn`);
    app.close();
  }
});

test('test_csd025_ac_16_2_the_last_figure_is_deleted', async () => {
  for (const { page, insights } of TAB_PAGES) {
    const app = await openApp();
    await app.click(app.item(page));
    await app.click(app.$(`${page}-tab-insights`));
    assert.equal(app.isShown(app.$(`${page}-insights-empty`)), false);
    assert.ok(app.$(`${page}-insights-chart`).children.length > 0, 'the chart showed');

    // The only figure is deleted on the Overview tab, so Rust now answers with no years.
    await app.click(app.$(`${page}-tab-overview`));
    app.bridge.table[insights] = { ...app.bridge.table[insights], years: [] };
    await app.click(app.$(`${page}-tab-insights`));
    assert.equal(app.isShown(app.$(`${page}-insights-empty`)), true, 'the note is shown');
    assert.equal(app.$(`${page}-insights-chart`).children.length, 0, 'no chart is drawn');
    app.close();
  }
});

test('test_csd017_ac_12_1_empty_project_net_worth', async () => {
  const app = await openApp({ reads: emptyReads() });
  await app.click(app.item('net-worth'));
  assert.equal(app.textOf('net-worth-insights-empty'), 'No data yet. The chart appears once there is a house, investment, bank account entry or debt.');
  assert.equal(app.$('net-worth-insights-chart').children.length, 0, 'no chart is drawn');
  app.close();
});

test('test_csd018_ac_11_1_empty_project_ratios', async () => {
  const app = await openApp({ reads: emptyReads() });
  await app.click(app.item('ratios'));
  assert.equal(app.isShown(app.$('ratios-empty')), true);
  assert.equal(app.textOf('ratios-empty'), 'No data yet. The table appears once there is income, a house, an investment, a bank account entry, a debt or an expense.');
  assert.equal(app.isShown(app.$('ratios-table')), false, 'no table is drawn');
  app.close();
});

test('test_csd004_ac_14_1_empty_project_dashboard', async () => {
  const app = await openApp({ reads: emptyReads() });
  const tiles = [...app.document.querySelectorAll('#dashboard-tiles .tile')];
  assert.equal(tiles.length, 7, 'all seven tiles are shown');
  for (const tile of tiles) assert.match(app.text(tile), /No data yet/);
  const income = app.document.querySelector('.tile[data-tile="income"]');
  await app.click(income);
  assert.equal(app.isShown(app.$('income')), true, 'an empty tile opens its page');
  assert.equal(app.item('income').classList.contains('active'), true);
  app.close();
});

/** Overview pages with no record at all: the prompt to add the first one, and no form to add a child record. */
const FIRST_PROMPTS = [
  ['test_csd001_ac_13_1_no_jobs', 'income', 'No jobs yet. Add your first job to start tracking your monthly paychecks.', 'Add your first job'],
  ['test_csd005_ac_18_1_no_investment_accounts', 'investments', null, null],
  ['test_csd009_ac_21_1_no_bank_accounts', 'bank', null, null],
  ['test_csd011_ac_21_1_no_debts', 'debts', null, null],
  ['test_csd015_ac_22_1_no_houses', 'houses', null, null],
];

for (const [title, page, text, button] of FIRST_PROMPTS) {
  test(title, async () => {
    const app = await openApp({ reads: emptyReads() });
    await app.click(app.item(page));
    const prompt = app.$(`${page}-empty`);
    assert.equal(app.isShown(prompt), true, 'the prompt is shown');
    assert.match(app.text(prompt.querySelector('button')), /^Add your first /);
    if (text) assert.equal(app.text(prompt.querySelector('p')), text);
    if (button) assert.equal(app.text(prompt.querySelector('button')), button);
    assert.equal(app.isShown(app.$(`${page}-content`)), false, 'no tabs, no statement or entry form');
    assert.equal(app.isShown(app.$(`${page}-tab-overview`)), false);
    app.close();
  });
}

test('test_csd013_ac_14_1_no_expenses', async () => {
  const app = await openApp({ reads: emptyReads() });
  await app.click(app.item('expenses'));
  assert.equal(app.textOf('expenses-empty'), 'No expenses yet. Add your first expense.');
  assert.equal(app.isShown(app.$('expenses-empty')), true);
  assert.equal(app.textOf('expense-totals-empty'), 'No totals yet. Totals appear once there is an expense.');
  assert.equal(app.isShown(app.$('expense-totals-empty')), true);
  assert.equal(app.isShown(app.$('expense-totals-table')), false);
  const add = [...app.document.querySelectorAll('#expenses button')].find((button) => app.text(button) === 'Add expense');
  assert.ok(add && app.isShown(add), 'the button "Add expense" is available');
  assert.equal(app.$('toast').hidden, true, 'no error is shown');
  app.close();
});

test('test_csd001_ac_13_2_jobs_without_entries_show_a_note_and_no_error', async () => {
  const reads = exampleReads();
  Object.assign(reads, { yearly_totals: [], list_entries: [] });
  const app = await openApp({ reads });
  await app.click(app.item('income'));
  assert.equal(app.isShown(app.$('totals-empty')), true);
  assert.equal(app.isShown(app.$('paycheck-empty')), true);
  assert.equal(app.isShown(app.$('totals-table')), false);
  assert.equal(app.$('toast').hidden, true);
  app.close();
});
