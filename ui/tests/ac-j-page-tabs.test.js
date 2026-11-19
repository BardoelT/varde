// CSD-035: one tab per panel on the seven data pages. The tabs are markup and in-memory state; Rust sends no
// tab data, so these tests check which tabs and panels the screen shows and how the selection behaves.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { emptyReads, exampleReads } from './fixtures.js';

/** page id, page title, tab labels left to right, ids of the tabs left to right, and the tab a page opens on. */
const PAGES = [
  ['income', 'Income', ['Jobs', 'Yearly totals', 'Paychecks and statements', 'Insights'], ['jobs', 'totals', 'paychecks', 'insights'], 'jobs'],
  ['investments', 'Investments', ['Accounts', 'Yearly totals', 'Statements', 'Insights'], ['accounts', 'totals', 'statements', 'insights'], 'accounts'],
  ['bank', 'Bank accounts', ['Accounts', 'Yearly totals', 'Entries', 'Insights'], ['accounts', 'totals', 'entries', 'insights'], 'accounts'],
  ['houses', 'Houses', ['Houses', 'Yearly values', 'Valuations', 'Insights'], ['houses', 'totals', 'valuations', 'insights'], 'houses'],
  ['debts', 'Debts', ['Debts', 'Yearly totals', 'Statements', 'Insights'], ['debts', 'totals', 'statements', 'insights'], 'debts'],
  ['expenses', 'Expenses', ['Yearly totals', 'Expenses', 'Insights'], ['totals', 'expenses', 'insights'], 'totals'],
  [
    'transactions',
    'Transactions',
    ['Accounts', 'Categories', 'Rules', 'Yearly totals', 'Transactions', 'Insights'],
    ['accounts', 'categories', 'rules', 'totals', 'transactions', 'insights'],
    'transactions',
  ],
];

const tabsOf = (app, page) => [...app.$(`${page}-tab-insights`).parentElement.querySelectorAll('[role="tab"]')];
const selectedOf = (app, page) => tabsOf(app, page).filter((tab) => tab.getAttribute('aria-selected') === 'true').map((tab) => tab.id);
const panelsShown = (app, page) =>
  tabsOf(app, page).filter((tab) => app.isShown(app.$(tab.getAttribute('aria-controls')))).map((tab) => tab.id);

test('test_csd035_ac_1_1_the_tabs_of_a_page', async () => {
  const app = await openApp();
  for (const [page, , labels, slugs] of PAGES) {
    await app.click(app.item(page));
    assert.deepEqual(tabsOf(app, page).map((tab) => app.text(tab)), labels, `${page}: the tab labels`);
    assert.deepEqual(tabsOf(app, page).map((tab) => tab.id), slugs.map((slug) => `${page}-tab-${slug}`));
    assert.ok(!tabsOf(app, page).some((tab) => app.text(tab) === 'Overview'), 'no tab "Overview"');
    // The label of a tab is the title of its panel (REQ-7).
    for (const tab of tabsOf(app, page).slice(0, -1)) {
      const panel = app.$(tab.getAttribute('aria-controls'));
      assert.equal(app.text(panel.querySelector('.panel-title')), app.text(tab), `${tab.id}: label equals panel title`);
    }
  }
  app.close();
});

test('test_csd035_ac_1_2_one_panel_per_tab', async () => {
  const app = await openApp();
  await app.click(app.item('income'));
  await app.click(app.$('income-tab-totals'));
  assert.deepEqual(panelsShown(app, 'income'), ['income-tab-totals']);
  assert.equal(app.isShown(app.$('income-jobs')), false);
  assert.equal(app.isShown(app.$('income-paychecks')), false);
  app.close();
});

test('test_csd035_ac_1_3_pages_without_tabs_stay_as_they_are', async () => {
  const app = await openApp();
  for (const id of ['dashboard', 'net-worth', 'ratios', 'settings']) {
    assert.equal(app.$(id).querySelector('[role="tablist"]'), null, `${id} has no tabs`);
  }
  app.close();
});

test('test_csd035_ac_2_1_buttons_stay_in_their_panel', async () => {
  const app = await openApp();
  await app.click(app.item('income'));
  await app.click(app.$('income-tab-paychecks'));
  assert.equal(app.text(app.$('income-paychecks').querySelector('.panel-header .btn')), 'Add entry');
  assert.equal(app.isShown(app.$('paycheck-add')), true);
  assert.equal(app.isShown(app.$('job-add')), false);
  await app.click(app.$('income-tab-jobs'));
  assert.equal(app.text(app.$('income-jobs').querySelector('.panel-header .btn')), 'Add job');
  assert.equal(app.isShown(app.$('job-add')), true);
  app.close();
});

test('test_csd035_ac_2_3_transactions_panels', async () => {
  const app = await openApp();
  await app.click(app.item('transactions'));
  const header = (id) => app.$(id).querySelector('.panel-header');
  await app.click(app.$('transactions-tab-categories'));
  assert.ok(app.isShown(app.$('transaction-categorisation')), 'the categorized share is in the categories');
  assert.ok(header('transactions-categories').contains(app.$('transaction-categorisation')));
  await app.click(app.$('transactions-tab-transactions'));
  assert.ok(header('transactions-transactions').contains(app.$('transactions-bulk-toggle') ?? header('transactions-transactions').querySelector('button')));
  app.close();
});

test('test_csd035_ac_3_1_the_tab_shown_first', async () => {
  for (const [page, , , , first] of PAGES) {
    const app = await openApp();
    await app.click(app.item(page));
    assert.deepEqual(selectedOf(app, page), [`${page}-tab-${first}`], `${page} opens on ${first}`);
    assert.deepEqual(panelsShown(app, page), [`${page}-tab-${first}`], 'only its panel is shown');
    app.close();
  }
});

test('test_csd035_ac_3_2_another_project_resets_the_tabs', async () => {
  const app = await openApp({ handlers: { open_project: { done: true, status: { has_path: true, unsaved_changes: false } } } });
  await app.click(app.item('income'));
  await app.click(app.$('income-tab-insights'));
  await app.click(app.item('transactions'));
  await app.click(app.$('transactions-tab-rules'));
  await app.emit('menu-action', 'project-open');
  await app.settle();
  assert.deepEqual(selectedOf(app, 'income'), ['income-tab-jobs']);
  assert.deepEqual(selectedOf(app, 'transactions'), ['transactions-tab-transactions']);
  app.close();
});

test('test_csd035_ac_4_1_a_reload_keeps_the_tab', async () => {
  const status = { has_path: true, unsaved_changes: true, undo_label: null, redo_label: null };
  const app = await openApp({ handlers: { undo: { step: 'Add paycheck', status } } });
  await app.click(app.item('income'));
  await app.click(app.$('income-tab-paychecks'));
  const before = app.callsOf('list_entries').length;
  await app.emit('menu-action', 'project-undo');
  await app.settle();
  assert.ok(app.callsOf('list_entries').length > before, 'the page was loaded again');
  assert.deepEqual(selectedOf(app, 'income'), ['income-tab-paychecks']);
  assert.deepEqual(panelsShown(app, 'income'), ['income-tab-paychecks']);
  app.close();
});

test('test_csd035_ac_4_2_switching_pages_keeps_the_tab', async () => {
  const app = await openApp();
  await app.click(app.item('debts'));
  await app.click(app.$('debts-tab-statements'));
  await app.click(app.item('bank'));
  await app.click(app.item('debts'));
  assert.deepEqual(selectedOf(app, 'debts'), ['debts-tab-statements']);
  assert.ok(app.isShown(app.$('debts-statements')));
  app.close();
});

test('test_csd035_ac_5_1_no_tabs_while_empty_and_expenses_and_transactions_always_have_them', async () => {
  const app = await openApp({ reads: emptyReads() });
  await app.click(app.item('income'));
  assert.equal(app.isShown(app.$('income-empty')), true);
  assert.equal(app.isShown(app.$('income-tab-jobs')), false);
  for (const page of ['expenses', 'transactions']) {
    await app.click(app.item(page));
    assert.equal(app.isShown(app.$(`${page}-tab-insights`)), true, `${page} shows its tabs`);
  }
  assert.equal(tabsOf(app, 'expenses').length, 3);
  assert.equal(tabsOf(app, 'transactions').length, 6);
  app.close();
});

test('test_csd035_ac_5_2_the_first_item_shows_the_first_tab', async () => {
  // The page has been on Insights, then loses its jobs (an undo), then gets one again.
  const reads = exampleReads();
  const status = { has_path: true, unsaved_changes: true, undo_label: null, redo_label: null };
  const app = await openApp({ reads, handlers: { undo: { step: 'Add job', status } } });
  await app.click(app.item('income'));
  await app.click(app.$('income-tab-insights'));
  const jobs = app.bridge.table.list_jobs;
  app.bridge.table.list_jobs = [];
  await app.emit('menu-action', 'project-undo');
  await app.settle();
  assert.equal(app.isShown(app.$('income-empty')), true, 'the empty state is shown');
  assert.equal(app.isShown(app.$('income-tab-jobs')), false, 'with no tabs');
  app.bridge.table.list_jobs = jobs;
  app.bridge.table.redo = { step: 'Add job', status };
  await app.emit('menu-action', 'project-redo');
  await app.settle();
  assert.equal(app.isShown(app.$('income-tab-jobs')), true, 'the tabs are back');
  assert.deepEqual(selectedOf(app, 'income'), ['income-tab-jobs'], 'on the first tab');
  app.close();
});

test('test_csd035_ac_6_2_arrow_keys_wrap_around', async () => {
  const app = await openApp();
  await app.click(app.item('debts'));
  await app.click(app.$('debts-tab-insights'));
  const insights = app.$('debts-tab-insights');
  insights.focus();
  insights.dispatchEvent(new app.window.KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true, cancelable: true }));
  await app.settle();
  assert.equal(app.document.activeElement, app.$('debts-tab-debts'));
  assert.deepEqual(selectedOf(app, 'debts'), ['debts-tab-debts']);
  app.close();
});

test('test_csd035_ac_6_4_only_the_selected_tab_is_in_the_tab_order', async () => {
  const app = await openApp();
  for (const [page] of PAGES) {
    await app.click(app.item(page));
    const inOrder = tabsOf(app, page).filter((tab) => tab.tabIndex === 0);
    assert.deepEqual(inOrder.map((tab) => tab.id), selectedOf(app, page), `${page}: one tab in the tab order`);
  }
  app.close();
});

test('test_csd035_ac_7_1_dutch', async () => {
  const reads = exampleReads();
  reads.get_preferences = { theme: 'system', language: 'nl' };
  const app = await openApp({ reads });
  await app.click(app.item('income'));
  const labels = tabsOf(app, 'income').map((tab) => app.text(tab));
  assert.equal(labels[0], 'Banen');
  assert.equal(labels[1], 'Jaartotalen');
  assert.notEqual(labels[2], 'Paychecks and statements', 'translated');
  assert.notEqual(labels[3], 'Insights', 'translated');
  for (const tab of tabsOf(app, 'income').slice(0, -1)) {
    assert.equal(app.text(app.$(tab.getAttribute('aria-controls')).querySelector('.panel-title')), app.text(tab), 'the label is the Dutch panel title');
  }
  app.close();
});
