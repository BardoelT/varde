// Display ACs of CSD-013 (expenses): the left-pane item, the page layout, the rows, the footer,
// the forms and the currency confirmation. Rust computes every figure; the fake bridge answers
// with the golden example project or a small list made for one AC.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { exampleReads } from './fixtures.js';

const expense = (id, name, amount, date, category = 'Other', note = '') => ({ id, name, amount, date, category, note });

/** The example project with this list of expenses and the yearly totals Rust would return for it. */
function readsWith(expenses, total) {
  const reads = exampleReads();
  reads.list_expenses = { expenses, total };
  return reads;
}

async function expensesPage(reads, handlers) {
  const app = await openApp({ reads, handlers });
  await app.click(app.item('expenses'));
  return app;
}

const rowCells = (app, row) => [...row.querySelectorAll('th, td')].map((cell) => app.text(cell));
const tableRows = (app) => [...app.$('expenses-rows').querySelectorAll('tr')];
const CURRENCY = /\b(EUR|USD|GBP)\b|[$€£]/;

test('test_csd013_ac_1_3_the_category_defaults_to_other_in_the_form', async () => {
  const app = await expensesPage();
  await app.click(app.$('expense-add'));
  assert.equal(app.$('expense-form').elements.category.value, 'Other');
  assert.equal(app.text(app.$('expense-form').elements.category.selectedOptions[0]), 'Other');
  app.close();
});

test('test_csd013_ac_8_2_cancelled_deletion_changes_nothing', async () => {
  const app = await expensesPage();
  const before = tableRows(app).map((row) => rowCells(app, row));
  assert.equal(before.length, 5);
  await app.click(app.document.querySelector('[aria-label="Delete Example Bike"]'));
  assert.equal(app.$('confirm-dialog').open, true);
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.callsOf('delete_expense').length, 0, 'nothing is deleted');
  assert.deepEqual(tableRows(app).map((row) => rowCells(app, row)), before, 'both expenses still exist');
  assert.ok(before.some((cells) => cells.includes('Example Bike')));
  app.close();
});

test('test_csd013_ac_13_1_expenses_is_an_item_in_the_left_pane_with_a_receipt_icon', async () => {
  const app = await openApp();
  const item = app.item('expenses');
  assert.equal(app.text(item), 'Expenses');
  assert.equal(item.closest('.sidebar-nav') !== null, true, 'an item in the left pane');
  const icon = item.querySelector('svg');
  assert.ok(icon, 'it has an icon');
  assert.equal(icon.getAttribute('aria-hidden'), 'true');
  assert.match(icon.querySelector('path').getAttribute('d'), /^M4 2v20l2-1 2 1/, 'the receipt outline with a torn edge');
  app.close();
});

test('test_csd013_ac_13_2_the_sections_appear_in_order', async () => {
  const app = await expensesPage();
  assert.equal(app.$('expenses-tab-overview').getAttribute('aria-selected'), 'true', 'the Overview tab is selected');
  assert.equal(app.isShown(app.$('expenses-overview')), true);
  const titles = [...app.$('expenses-overview').querySelectorAll('.panel-title')].map((node) => app.text(node));
  assert.deepEqual(titles, ['Yearly totals', 'Expenses']);
  app.close();
});

test('test_csd013_ac_13_3_expense_rows', async () => {
  const reads = readsWith([expense('id-1', 'Example Roof Repair', '4500.00', '2026-09-15', 'Home', 'Replaced tiles')], '4500.00');
  const app = await expensesPage(reads);
  const head = [...app.$('expenses-table').querySelectorAll('thead th')].map((cell) => app.text(cell));
  assert.deepEqual(head.slice(0, 5), ['Date', 'Name', 'Category', 'Amount', 'Note']);
  const [row] = tableRows(app);
  assert.deepEqual(rowCells(app, row).slice(0, 5), ['15/09/2026', 'Example Roof Repair', 'Home', '4,500.00', 'Replaced tiles']);
  const actions = row.lastElementChild;
  assert.equal(actions.className, 'row-actions', 'at its right edge');
  const [edit, remove] = actions.querySelectorAll('button');
  assert.equal(edit.getAttribute('aria-label'), 'Edit Example Roof Repair');
  assert.equal(edit.title, 'Edit Example Roof Repair', 'and in a tooltip');
  assert.equal(remove.getAttribute('aria-label'), 'Delete Example Roof Repair');
  assert.ok(remove.classList.contains('danger'));
  app.close();
});

test('test_csd013_ac_13_4_the_footer_shows_the_total', async () => {
  const reads = readsWith([expense('id-2', 'Example Sofa', '80.00', '2026-09-12'), expense('id-1', 'Example Bike', '900.00', '2025-09-10', 'Hobby')], '980.00');
  const app = await expensesPage(reads);
  assert.equal(tableRows(app).length, 2, 'the list shows both expenses');
  const footer = app.$('expenses-table').querySelector('tfoot tr');
  assert.equal(app.text(footer.querySelector('th')), 'Total');
  assert.equal(app.textOf('expenses-total'), '980.00');
  app.close();
});

test('test_csd013_ac_13_5_the_header_of_the_expenses_section_holds_only_add_expense', async () => {
  const app = await expensesPage();
  const header = app.$('expense-add').closest('.panel-header');
  assert.equal(app.text(header.querySelector('.panel-title')), 'Expenses');
  assert.deepEqual([...header.querySelectorAll('button, select, input')].map((node) => app.text(node)), ['Add expense'], 'only the button, no year or month filter');
  assert.equal(app.$('expenses').querySelector('.section-header').querySelectorAll('button').length, 0, 'the title row holds no button');
  app.close();
});

test('test_csd013_ac_13_6_the_yearly_totals_section', async () => {
  const app = await expensesPage();
  const head = [...app.$('expense-totals-table').querySelectorAll('thead th')].map((cell) => app.text(cell));
  assert.deepEqual(head, ['Year', 'Total', 'Largest'], 'and no column with a number of expenses');
  const panel = app.$('expense-totals-table').closest('.panel');
  assert.equal(panel.querySelector('.panel-header').querySelectorAll('button').length, 0, 'no button in its header');
  assert.equal(app.$('expense-totals-rows').querySelectorAll('button').length, 0, 'no Edit or Delete in its rows');
  const first = [...app.$('expense-totals-rows').querySelector('tr').children].map((cell) => app.text(cell));
  assert.deepEqual(first, ['2026', '920.00', '800.00']);
  app.close();
});

test('test_csd013_ac_13_7_a_long_note_is_cut_to_one_line', async () => {
  const note = 'First line\nSecond line\nThird line';
  const reads = readsWith([expense('id-1', 'Example Roof Repair', '4500.00', '2026-09-15', 'Home', note)], '4500.00');
  const app = await expensesPage(reads);
  assert.equal(app.text(tableRows(app)[0].querySelector('.note-cell')), 'First line …', 'one line of it');
  await app.click(app.document.querySelector('[aria-label="Edit Example Roof Repair"]'));
  assert.equal(app.$('expense-form').elements.note.value, note, 'editing shows the whole note');
  app.close();
});

test('test_csd013_ac_13_8_notes_and_names_are_shown_as_plain_text', async () => {
  const reads = readsWith([expense('id-1', '<i>Bike</i>', '10.00', '2026-09-15', 'Other', '<b>bold</b>')], '10.00');
  const app = await expensesPage(reads);
  const [row] = tableRows(app);
  assert.equal(app.text(row.children[1]), '<i>Bike</i>');
  assert.equal(app.text(row.querySelector('.note-cell')), '<b>bold</b>');
  assert.equal(row.querySelectorAll('i, b').length, 0, 'no formatting');
  app.close();
});

test('test_csd013_ac_13_9_the_insights_tab_shows_the_chart', async () => {
  const app = await expensesPage();
  await app.click(app.$('expenses-tab-insights'));
  assert.equal(app.$('expenses-tab-insights').getAttribute('aria-selected'), 'true');
  assert.equal(app.isShown(app.$('expenses-overview')), false, 'the Overview sections are hidden');
  assert.equal(app.isShown(app.$('expenses-insights')), true);
  assert.equal(app.isShown(app.$('expenses-insights-scroll')), true);
  assert.ok(app.$('expenses-insights-chart').querySelector('svg'), 'the chart of CSD-014 is drawn');
  app.close();
});

test('test_csd013_ac_15_1_no_currency_in_the_sections', async () => {
  const app = await expensesPage();
  assert.equal(app.text(app.$('expenses-overview').querySelectorAll('.panel-title')[0]), 'Yearly totals');
  assert.equal(app.text(app.$('expenses-overview').querySelectorAll('.panel-title')[1]), 'Expenses');
  for (const id of ['expense-totals-table', 'expenses-table', 'expense-dialog']) {
    assert.doesNotMatch(app.textOf(id), CURRENCY, `${id} shows no currency`);
  }
  assert.doesNotMatch(app.textOf('expenses-overview'), CURRENCY);
  app.close();
});

// ---- the currency confirmation of CSD-003 REQ-5, seen from the expenses ----

/** The Settings page of a project whose only entry is the expense of 4500.00 (or no entry at all). */
async function settingsPage(entryCount) {
  const reads = readsWith(entryCount ? [expense('id-1', 'Example Roof Repair', '4500.00', '2026-09-15', 'Home')] : [], entryCount ? '4500.00' : '0.00');
  reads.get_settings = { currency: 'EUR', entry_count: entryCount };
  const handlers = { set_currency: ({ currency }) => ({ currency, entry_count: entryCount }) };
  const app = await openApp({ reads, handlers });
  await app.click(app.item('settings'));
  return app;
}

test('test_csd013_ac_17_1_confirmation_with_only_an_expense', async () => {
  const app = await settingsPage(1);
  await app.choose(app.$('setting-currency'), 'USD');
  assert.equal(app.$('confirm-dialog').open, true, 'the app asks for confirmation before changing it');
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.callsOf('set_currency').length, 0);
  assert.equal(app.$('setting-currency').value, 'EUR', 'the currency stays EUR');
  await app.click(app.item('expenses'));
  assert.equal(app.text(tableRows(app)[0].querySelector('td.num')), '4,500.00', 'and the amount 4500.00');
  app.close();
});

test('test_csd013_ac_17_2_no_expenses_needs_no_confirmation_for_them', async () => {
  const app = await settingsPage(0);
  await app.choose(app.$('setting-currency'), 'USD');
  assert.equal(app.$('confirm-dialog').open, false, 'no confirmation');
  assert.deepEqual(app.callsOf('set_currency').map((call) => call.args), [{ currency: 'USD' }]);
  assert.equal(app.$('setting-currency').value, 'USD');
  app.close();
});
