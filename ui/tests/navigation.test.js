// CSD-024: the left pane, the first page, choosing a page, and the menu items that run in the window.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { emptyReads, exampleReads } from './fixtures.js';

const GROUPS = [
  ['Overview', ['Dashboard']],
  ['Finance', ['Income', 'Expenses']],
  ['Wealth', ['Bank accounts', 'Investments', 'Houses']],
  ['Liabilities', ['Debts']],
  ['Insights', ['Net worth', 'Ratios']],
];

/** The left pane as it reads top to bottom: `[heading, [items]]`. */
function readPane(app) {
  const groups = [];
  for (const node of app.document.querySelector('.sidebar-nav').children) {
    if (node.classList.contains('nav-section-label')) groups.push([app.text(node), []]);
    else groups.at(-1)[1].push(app.text(node));
  }
  return groups;
}

const currentItems = (app) => [...app.document.querySelectorAll('.nav-item')].filter((item) => item.classList.contains('active'));
const shownSections = (app) => app.sections().filter((section) => app.isShown(section)).map((section) => section.id);

test('test_csd024_ac_1_1_groups_and_pages_in_order', async () => {
  const app = await openApp();
  assert.deepEqual(readPane(app), GROUPS);
  app.close();
});

test('test_csd024_ac_1_2_pages_without_a_function_are_not_offered', async () => {
  const app = await openApp();
  const texts = [...app.document.querySelectorAll('.nav-item')].map((item) => app.text(item));
  for (const name of ['Accounts', 'Transactions', 'Budget']) assert.ok(!texts.includes(name), `${name} must not be offered`);
  assert.equal(app.document.querySelectorAll('.nav-item').length, 10, 'nine pages and Settings');
  app.close();
});

test('test_csd024_ac_1_3_no_example_data_in_a_new_project', async () => {
  const app = await openApp({ reads: emptyReads() });
  const rows = [...app.document.querySelectorAll('.section tbody tr, .job-row')].filter((row) => !row.hidden);
  assert.deepEqual(rows.map((row) => row.outerHTML.slice(0, 80)), [], 'no rows anywhere');
  assert.equal(app.document.querySelectorAll('.section ul:not(.legend) li').length, 0, 'no list items anywhere');
  app.close();
});

test('test_csd024_ac_2_1_the_app_opens_on_the_dashboard', async () => {
  const app = await openApp({ reads: emptyReads() });
  assert.deepEqual(shownSections(app), ['dashboard']);
  assert.deepEqual(currentItems(app).map((item) => item.dataset.section), ['dashboard']);
  assert.equal(app.item('dashboard').getAttribute('aria-current'), 'page');
  for (const item of app.document.querySelectorAll('.nav-item:not([data-section="dashboard"])')) {
    assert.notEqual(item.getAttribute('aria-current'), 'page');
  }
  app.close();
});

test('test_csd024_ac_2_2_a_remembered_project_does_not_change_the_first_page', async () => {
  const reads = exampleReads();
  reads.project_status = { has_path: true, unsaved_changes: false };
  const app = await openApp({ reads });
  assert.deepEqual(shownSections(app), ['dashboard']);
  assert.deepEqual(currentItems(app).map((item) => item.dataset.section), ['dashboard']);
  app.close();
});

test('test_csd024_ac_3_1_choosing_a_page_with_the_mouse', async () => {
  const app = await openApp();
  await app.click(app.item('debts'));
  assert.deepEqual(shownSections(app), ['debts']);
  assert.equal(app.isShown(app.$('dashboard')), false);
  assert.deepEqual(currentItems(app).map((item) => item.dataset.section), ['debts']);
  assert.equal(app.item('debts').getAttribute('aria-current'), 'page');
  assert.equal(app.item('dashboard').getAttribute('aria-current'), 'false');
  app.close();
});

test('test_csd024_ac_3_2_choosing_a_page_with_tab_and_enter', async () => {
  const app = await openApp();
  const item = app.item('income');
  // The items are native buttons in the tab order: Tab reaches them and Enter or Space activates them.
  assert.equal(item.tagName, 'BUTTON');
  assert.ok(item.tabIndex >= 0, 'reachable with Tab');
  await app.press(item, 'Enter');
  assert.equal(app.document.activeElement, item);
  assert.deepEqual(shownSections(app), ['income']);
  assert.deepEqual(currentItems(app).map((entry) => entry.dataset.section), ['income']);

  await app.press(app.item('houses'), ' ');
  assert.deepEqual(shownSections(app), ['houses']);
  app.close();
});

test('test_csd024_ac_3_3_choosing_the_current_page_again', async () => {
  const app = await openApp();
  await app.click(app.item('income'));
  await app.click(app.item('income'));
  assert.deepEqual(shownSections(app), ['income']);
  assert.deepEqual(currentItems(app).map((item) => item.dataset.section), ['income']);
  app.close();
});

test('test_csd024_ac_4_1_settings_and_project_status_are_pinned_at_the_bottom', async () => {
  const app = await openApp();
  const { document } = app;
  const nav = document.querySelector('.sidebar-nav');
  const settings = document.querySelector('.sidebar-settings');
  const status = document.querySelector('.sidebar-project');
  assert.ok(!nav.contains(app.item('settings')), 'Settings is outside every group');
  assert.ok(settings.contains(app.item('settings')));
  assert.ok(nav.compareDocumentPosition(settings) & app.window.Node.DOCUMENT_POSITION_FOLLOWING, 'Settings below the groups');
  assert.ok(settings.compareDocumentPosition(status) & app.window.Node.DOCUMENT_POSITION_FOLLOWING, 'status below Settings');
  assert.ok(status.contains(app.$('project-state')));
  app.close();
});

test('test_csd024_ac_4_2_pinned_items_stay_in_place', async () => {
  const app = await openApp();
  const sidebar = app.document.querySelector('.sidebar');
  await app.click(app.item('net-worth'));
  assert.deepEqual(shownSections(app), ['net-worth']);
  assert.ok(sidebar.contains(app.item('settings')) && sidebar.contains(app.$('project-state')));
  assert.equal(app.item('settings').closest('.section'), null, 'Settings belongs to no page');
  assert.equal(app.isShown(app.item('settings')), true);
  assert.equal(app.isShown(app.$('project-state')), true);
  app.close();
});

test('test_csd024_ac_6_1_dutch_language_dutch_left_pane', async () => {
  const reads = exampleReads();
  reads.get_preferences = { theme: 'system', language: 'nl' };
  const app = await openApp({ reads });
  assert.deepEqual(readPane(app), [
    ['Overzicht', ['Dashboard']],
    ['Financiën', ['Inkomen', 'Uitgaven']],
    ['Vermogen', ['Bankrekeningen', 'Beleggingen', 'Woningen']],
    ['Verplichtingen', ['Schulden']],
    ['Inzichten', ['Vermogen', 'Kengetallen']],
  ]);
  assert.equal(app.text(app.item('settings')), 'Instellingen');
  app.close();
});

test('test_csd024_ac_7_1_settings_from_another_page', async () => {
  const app = await openApp();
  await app.click(app.item('income'));
  // The menu item "Settings…" (⌘,) arrives as the event menu-action with the item id.
  await app.emit('menu-action', 'open-settings');
  assert.deepEqual(shownSections(app), ['settings']);
  assert.deepEqual(currentItems(app).map((item) => item.dataset.section), ['settings']);
  app.close();
});

test('test_csd024_ac_7_2_a_menu_item_that_needs_confirmation_asks_first', async () => {
  const reads = exampleReads();
  reads.project_status = { has_path: true, unsaved_changes: true };
  const app = await openApp({ reads, handlers: { new_project: { has_path: false, unsaved_changes: false } } });
  const dialog = app.$('confirm-dialog');

  await app.emit('menu-action', 'project-new');
  assert.equal(dialog.open, true, 'a confirmation is asked');
  assert.equal(app.textOf('confirm-title'), 'Discard unsaved changes?');
  assert.equal(app.textOf('confirm-ok'), 'Discard changes');
  assert.equal(app.callsOf('new_project').length, 0, 'the project is not replaced yet');

  await app.click(app.$('confirm-cancel'));
  assert.equal(dialog.open, false);
  assert.equal(app.callsOf('new_project').length, 0, 'Cancel keeps the project');

  await app.emit('menu-action', 'project-new');
  await app.click(app.$('confirm-ok'));
  assert.equal(app.callsOf('new_project').length, 1, 'only after the user confirms');
  app.close();
});
