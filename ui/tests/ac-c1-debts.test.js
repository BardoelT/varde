// CSD-011: the display ACs of the Debts overview, the statement form, the settle dialog and the
// currency confirmation. Rust computes every figure, so the tests feed canned answers shaped like
// the ones Rust returns (see src-tauri/golden/ipc-reads.json) and check only what is displayed.
// Amounts are shown with the digit grouping of CSD-022 ("150,000.00").
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { emptyReads, exampleReads } from './fixtures.js';

const MORTGAGE = {
  id: 'id-1', name: 'Example Mortgage', period: 'Monthly', category: 'Other', colour_slot: 0,
  house_id: null, house_name: null, latest_balance: '150000.00', latest_period: '2026-09', statement_count: 2, repaid: false,
};
const STUDENT = {
  id: 'id-2', name: 'Example Student Loan', period: 'Yearly', category: 'Other', colour_slot: 1,
  house_id: null, house_name: null, latest_balance: '8000.00', latest_period: '2025', statement_count: 1, repaid: false,
};
const statementRow = (debt, period, balance, change = null, note = '') => ({
  balance, change, debt_id: debt.id, debt_name: debt.name, locked: false, note, outcome: null, period, settlement: false,
});

/** Two debts with three statements, as the Debts overview receives them. */
function debtReads() {
  const reads = exampleReads();
  Object.assign(reads, {
    list_debts: { debts: [MORTGAGE, STUDENT], total: '158000.00' },
    list_debt_statements: [
      statementRow(MORTGAGE, '2026-09', '150000.00', '500.00'),
      statementRow(MORTGAGE, '2026-08', '150500.00'),
      statementRow(STUDENT, '2025', '8000.00'),
    ],
    yearly_debt_totals: [
      { year: 2026, closing_balance: '150000.00', repaid: '500.00', statement_count: 2 },
      { year: 2025, closing_balance: '8000.00', repaid: null, statement_count: 1 },
    ],
  });
  return reads;
}

async function openDebts(reads = debtReads(), handlers = {}) {
  const app = await openApp({ reads, handlers });
  await app.click(app.item('debts'));
  return app;
}

const button = (app, selector) => app.document.querySelector(selector);
const rowOf = (app, name) => [...app.document.querySelectorAll('#debt-list .job-row')].find((row) => app.text(row.querySelector('.job-name')) === name);
const statementRows = (app) => [...app.document.querySelectorAll('#debt-statements-rows tr')];
const headers = (app, tableId) => [...app.$(tableId).querySelectorAll('thead th')].map((th) => app.text(th));

// ---- REQ-6 and REQ-14: deleting a debt or a statement ----

test('test_csd011_ac_6_2_a_cancelled_deletion_of_a_debt_changes_nothing', async () => {
  const app = await openDebts();
  await app.click(button(app, '#debt-list [aria-label="Delete Example Mortgage"]'));
  assert.equal(app.$('confirm-dialog').open, true, 'the confirmation is asked');
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.$('confirm-dialog').open, false);
  assert.equal(app.callsOf('delete_debt').length, 0, 'nothing is deleted');
  assert.ok(rowOf(app, 'Example Mortgage'), 'the debt is still listed');
  assert.equal(statementRows(app).filter((row) => app.text(row).includes('Example Mortgage')).length, 2, 'and so are its statements');
  app.close();
});

test('test_csd011_ac_6_4_the_deletion_confirmation_states_no_count', async () => {
  const reads = debtReads();
  const empty = { ...STUDENT, id: 'id-3', name: 'Example Empty', latest_balance: null, latest_period: null, statement_count: 0 };
  reads.list_debts = { debts: [MORTGAGE, empty], total: '150000.00' };
  const app = await openDebts(reads);
  for (const name of ['Example Mortgage', 'Example Empty']) {
    await app.click(button(app, `#debt-list [aria-label="Delete ${name}"]`));
    assert.equal(app.textOf('confirm-message'), 'This debt and all its statements will be deleted.', name);
    assert.doesNotMatch(app.textOf('confirm-message'), /\d/, 'no number of statements');
    await app.click(app.$('confirm-cancel'));
  }
  app.close();
});

test('test_csd011_ac_14_2_a_cancelled_deletion_of_a_statement_changes_nothing', async () => {
  const app = await openDebts();
  await app.click(button(app, '#debt-statements-rows [aria-label="Delete Example Mortgage 2026-09"]'));
  assert.equal(app.$('confirm-dialog').open, true);
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.callsOf('delete_debt_statement').length, 0);
  const periods = statementRows(app).map((row) => app.text(row.querySelector('th')));
  assert.deepEqual(periods, ['2026-09', '2026-08', '2025'], 'both statements of the debt still exist');
  app.close();
});

// ---- REQ-12 and REQ-16: the statement form ----

test('test_csd011_ac_12_7_the_edit_form_offers_the_period_and_the_debt_cannot_change', async () => {
  const app = await openDebts();
  const form = app.$('debt-statement-form');
  for (const [label, period, field, value, debt] of [
    ['Edit Example Mortgage 2026-09', '2026-09', 'Month', '2026-09', 'id-1'],
    ['Edit Example Student Loan 2025', '2025', 'Year', '2025', 'id-2'],
  ]) {
    await app.click(button(app, `#debt-statements-rows [aria-label="${label}"]`));
    assert.equal(app.$('debt-statement-dialog').open, true);
    assert.equal(app.textOf('debt-statement-period-label'), field, `the form asks for the ${field.toLowerCase()}`);
    assert.equal(form.elements.period.value, value, period);
    assert.equal(form.elements.debt_id.value, debt, 'the debt is shown');
    assert.equal(form.elements.debt_id.disabled, true, 'and cannot be changed');
    await app.escape(app.$('debt-statement-dialog'));
  }
  app.close();
});

test('test_csd011_ac_16_1_a_monthly_debt_asks_for_a_month', async () => {
  const app = await openDebts();
  await app.click(app.$('debt-statement-add'));
  const form = app.$('debt-statement-form');
  assert.equal(form.elements.debt_id.value, 'id-1', 'the monthly debt is chosen first (alphabetical)');
  assert.equal(app.textOf('debt-statement-period-label'), 'Month');
  assert.equal(form.elements.period.placeholder, 'yyyy-mm');
  assert.equal(app.textOf('debt-statement-copy'), 'Copy previous month');
  app.close();
});

test('test_csd011_ac_16_2_a_yearly_debt_asks_for_a_year', async () => {
  const app = await openDebts();
  await app.click(app.$('debt-statement-add'));
  await app.choose(app.$('debt-statement-form').elements.debt_id, 'id-2');
  assert.equal(app.textOf('debt-statement-period-label'), 'Year');
  assert.equal(app.$('debt-statement-form').elements.period.maxLength, 4);
  assert.equal(app.textOf('debt-statement-copy'), 'Copy previous year');
  app.close();
});

test('test_csd011_ac_16_3_switching_debt_switches_the_form', async () => {
  const app = await openDebts();
  await app.click(app.$('debt-statement-add'));
  const select = app.$('debt-statement-form').elements.debt_id;
  await app.choose(select, 'id-2');
  assert.equal(app.textOf('debt-statement-period-label'), 'Year');
  await app.choose(select, 'id-1');
  assert.equal(app.textOf('debt-statement-period-label'), 'Month', 'a month again');
  assert.equal(app.$('debt-statement-form').elements.period.maxLength, 7);
  app.close();
});

// ---- REQ-20: the sections ----

test('test_csd011_ac_20_1_the_sections_appear_in_order_and_debts_is_in_the_left_pane', async () => {
  const app = await openDebts();
  const titles = [...app.$('debts-overview').querySelectorAll('.panel-title')].map((node) => app.text(node));
  assert.deepEqual(titles, ['Debts', 'Yearly totals', 'Statements']);
  assert.ok(app.item('debts'), 'Debts is an item in the left pane');
  assert.match(app.text(app.item('debts')), /^Debts$/);
  app.close();
});

test('test_csd011_ac_20_2_debt_rows_carry_their_period_latest_balance_and_actions', async () => {
  const app = await openDebts();
  const mortgage = rowOf(app, 'Example Mortgage');
  const student = rowOf(app, 'Example Student Loan');
  assert.match(app.text(mortgage.querySelector('.job-meta')), /Monthly/);
  assert.equal(app.text(mortgage.querySelector('.account-amount')), '150,000.002026-09');
  assert.match(app.text(student.querySelector('.job-meta')), /Yearly/);
  assert.equal(app.text(student.querySelector('.account-amount')), '8,000.002025');
  for (const row of [mortgage, student]) {
    assert.doesNotMatch(app.text(row), /statement/i, 'no number of statements');
    assert.doesNotMatch(app.text(row.querySelector('.job-meta')), /\d/);
  }
  const edit = student.querySelector('.job-actions button[aria-label="Edit Example Student Loan"]');
  const remove = student.querySelector('.job-actions button[aria-label="Delete Example Student Loan"]');
  assert.ok(edit && remove, 'Edit and Delete icon buttons, each named for screen readers');
  assert.equal(edit.title, 'Edit Example Student Loan', 'and in a tooltip');
  assert.equal(remove.title, 'Delete Example Student Loan');
  assert.equal(student.querySelector('.job-actions').lastElementChild, remove, 'at the right edge');
  assert.equal(app.isShown(app.$('debt-total')), true);
  assert.equal(app.text(app.$('debt-total').querySelector('.job-name')), 'Total debt');
  assert.equal(app.textOf('debt-total-value'), '158,000.00');
  app.close();
});

test('test_csd011_ac_20_3_the_statements_table_shows_period_debt_balance_change_and_note', async () => {
  const app = await openDebts();
  assert.deepEqual(headers(app, 'debt-statements-table').slice(0, 5), ['Period', 'Debt', 'Balance', 'Change', 'Note']);
  const [september, august] = statementRows(app);
  const cells = (row) => [...row.children].map((cell) => app.text(cell));
  assert.deepEqual(cells(september).slice(0, 4), ['2026-09', 'Example Mortgage', '150,000.00', '500.00']);
  assert.deepEqual(cells(august).slice(0, 4), ['2026-08', 'Example Mortgage', '150,500.00', '—'], 'a dash as change');
  app.close();
});

test('test_csd011_ac_20_4_a_long_note_is_cut_to_one_line_and_editing_shows_all_of_it', async () => {
  const reads = debtReads();
  reads.list_debt_statements[0].note = 'line one\nline two\nline three';
  const app = await openDebts(reads);
  const note = statementRows(app)[0].querySelector('.note-cell');
  assert.equal(app.text(note), 'line one …', 'one line of it');
  await app.click(button(app, '#debt-statements-rows [aria-label="Edit Example Mortgage 2026-09"]'));
  assert.equal(app.$('debt-statement-form').elements.note.value, 'line one\nline two\nline three');
  app.close();
});

test('test_csd011_ac_20_5_clicking_a_debt_row_changes_nothing', async () => {
  const app = await openDebts();
  const before = [app.$('debt-total').innerHTML, app.$('debt-totals-rows').innerHTML, app.$('debt-statements-rows').innerHTML];
  const calls = app.calls.length;
  for (const name of ['Example Mortgage', 'Example Student Loan']) {
    await app.click(rowOf(app, name));
    await app.click(rowOf(app, name).querySelector('.job-name'));
  }
  assert.deepEqual([app.$('debt-total').innerHTML, app.$('debt-totals-rows').innerHTML, app.$('debt-statements-rows').innerHTML], before);
  assert.equal(app.calls.length, calls, 'nothing is requested');
  assert.equal(app.$('debt-dialog').open, false);
  app.close();
});

test('test_csd011_ac_20_6_notes_are_shown_as_plain_text', async () => {
  const reads = debtReads();
  reads.list_debt_statements[0].note = '<b>bold</b>';
  const app = await openDebts(reads);
  const note = statementRows(app)[0].querySelector('.note-cell');
  assert.equal(note.textContent, '<b>bold</b>');
  assert.equal(note.querySelector('b'), null, 'no formatting');
  app.close();
});

test('test_csd011_ac_20_7_the_add_buttons_sit_in_the_section_headers', async () => {
  const app = await openDebts();
  const panels = [...app.$('debts-overview').querySelectorAll('.panel')];
  const header = (panel) => panel.querySelector('.panel-header');
  assert.equal(app.text(header(panels[0]).querySelector('button')), 'Add debt');
  assert.equal(header(panels[0]).lastElementChild, app.$('debt-add'), 'at the right edge of the Debts header');
  assert.equal(app.text(header(panels[2]).querySelector('button')), 'Add statement');
  assert.equal(header(panels[2]).lastElementChild, app.$('debt-statement-add'), 'and of the Statements header');
  assert.equal(app.$('debts').querySelector('.section-header').querySelectorAll('button').length, 0, 'the title row holds no button');
  app.close();
});

test('test_csd011_ac_20_8_the_yearly_totals_section', async () => {
  const app = await openDebts();
  assert.deepEqual(headers(app, 'debt-totals-table'), ['Year', 'Closing balance', 'Repaid']);
  const [latest, first] = [...app.$('debt-totals-rows').querySelectorAll('tr')].map((row) => [...row.children].map((cell) => app.text(cell)));
  assert.deepEqual(latest, ['2026', '150,000.00', '500.00']);
  assert.deepEqual(first, ['2025', '8,000.00', '—'], 'repaid none shows a dash');
  const panel = app.$('debt-totals-table').closest('.panel');
  assert.equal(panel.querySelector('.panel-header').querySelectorAll('button').length, 0, 'its header holds no button');
  assert.equal(app.$('debt-totals-rows').querySelectorAll('button').length, 0, 'and its rows no Edit or Delete');
  app.close();

  const none = await openApp({ reads: emptyReads() });
  await none.click(none.item('debts'));
  assert.equal(none.isShown(none.$('debt-totals-table')), false, 'not shown while no debts exist');
  assert.equal(none.isShown(none.$('debts-content')), false);
  none.close();
});

// ---- REQ-21 and REQ-22: empty states, no currency ----

test('test_csd011_ac_21_2_a_debt_without_statements', async () => {
  const reads = debtReads();
  const empty = { ...MORTGAGE, latest_balance: null, latest_period: null, statement_count: 0 };
  // The Statements table still holds the Opening row of the debt (CSD-011 REQ-33), one row: the note shows (REQ-21).
  const opening = { ...statementRow(empty, '2020-03-15', '200000.00'), locked: true, opening: true };
  Object.assign(reads, { list_debts: { debts: [empty], total: null }, list_debt_statements: [opening], yearly_debt_totals: [] });
  const app = await openDebts(reads);
  const row = rowOf(app, 'Example Mortgage');
  assert.equal(app.text(row.querySelector('.account-amount')), '—', 'a dash as latest balance');
  assert.match(app.text(row.querySelector('.job-meta')), /Monthly/, 'its statement period');
  assert.doesNotMatch(app.text(row), /No statements yet/);
  assert.equal(app.isShown(app.$('debt-statements-empty')), true);
  assert.equal(app.textOf('debt-statements-empty'), 'No statements yet.');
  assert.equal(app.$('toast').hidden, true, 'no error is shown');
  app.close();
});

test('test_csd011_ac_21_3_no_statements_in_any_debt_means_no_totals_table', async () => {
  const reads = debtReads();
  const empty = { ...MORTGAGE, latest_balance: null, latest_period: null, statement_count: 0 };
  Object.assign(reads, { list_debts: { debts: [empty], total: null }, list_debt_statements: [], yearly_debt_totals: [] });
  const app = await openDebts(reads);
  assert.equal(app.isShown(app.$('debt-totals-empty')), true);
  assert.equal(app.textOf('debt-totals-empty'), 'No totals yet. Totals appear once there is a statement.');
  assert.equal(app.isShown(app.$('debt-totals-table')), false, 'no table');
  assert.equal(app.$('toast').hidden, true, 'no error is shown');
  app.close();
});

test('test_csd011_ac_22_1_no_currency_in_the_sections', async () => {
  const app = await openDebts();
  const titles = [...app.$('debts-overview').querySelectorAll('.panel-title')].map((node) => app.text(node));
  assert.deepEqual(titles, ['Debts', 'Yearly totals', 'Statements'], 'titled without a currency');
  const shown = [app.$('debt-list'), app.$('debt-total'), app.$('debt-totals-table'), app.$('debt-statements-table')].map((node) => app.text(node)).join(' ');
  assert.doesNotMatch(shown, /EUR|€|USD|\$|GBP|£/, 'no currency in the rows or the tables');
  app.close();
});

// ---- REQ-24: the currency confirmation ----

test('test_csd011_ac_24_1_a_debt_statement_needs_a_confirmation_before_the_currency_changes', async () => {
  const reads = debtReads();
  reads.get_settings = { currency: 'EUR', entry_count: 1 }; // what Rust counts: one debt statement and nothing else
  const app = await openApp({ reads });
  await app.click(app.item('settings'));
  const select = app.$('setting-currency');
  await app.choose(select, 'USD');
  assert.equal(app.$('confirm-dialog').open, true, 'the app asks for confirmation');
  assert.equal(app.textOf('confirm-title'), 'Change the currency to USD?');
  await app.click(app.$('confirm-cancel'));
  assert.equal(select.value, 'EUR', 'cancelling leaves the currency EUR');
  assert.equal(app.callsOf('set_currency').length, 0);
  app.close();
});

test('test_csd011_ac_24_2_a_debt_without_statements_needs_no_confirmation', async () => {
  const reads = debtReads();
  reads.get_settings = { currency: 'EUR', entry_count: 0 };
  const app = await openApp({ reads, handlers: { set_currency: ({ currency }) => ({ currency, entry_count: 0 }) } });
  await app.click(app.item('settings'));
  await app.choose(app.$('setting-currency'), 'USD');
  assert.equal(app.$('confirm-dialog').open, false, 'no confirmation');
  assert.deepEqual(app.callsOf('set_currency').map((call) => call.args), [{ currency: 'USD' }], 'the currency changes to USD');
  app.close();
});

// ---- REQ-26: the settlement badge ----

test('test_csd011_ac_26_4_a_settled_debt_shows_its_settlement_badge', async () => {
  const reads = debtReads();
  const loan = {
    ...MORTGAGE, id: 'id-3', name: 'Example Loan', category: 'Loan', house_id: 'id-9', house_name: 'Example House',
    latest_balance: '0.00', latest_period: '2026-06', repaid: true,
    settlement: { amount: '10000.00', date: '2026-06-10', outcome: 'Cancelled' },
  };
  reads.list_debts = { debts: [loan], total: '0.00' };
  const app = await openDebts(reads);
  const badges = [...rowOf(app, 'Example Loan').querySelectorAll('.badge')].map((node) => app.text(node));
  assert.deepEqual(badges, ['Cancelled 10,000.00'], 'the settlement badge and not the mark "Repaid"');
  app.close();
});

// ---- REQ-27 and REQ-28: the settle dialog ----

const OPEN_DEBTS = [
  { debt_id: 'debt-1', debt_name: 'Example Mortgage', amount: '190000.00' },
  { debt_id: 'debt-2', debt_name: 'Example Loan', amount: '10000.00' },
];

async function openSettle(app, openDebts = OPEN_DEBTS) {
  const { askSettlements } = await app.load('shared/settle.js');
  return { answer: askSettlements(openDebts) };
}

const radio = (app, debtId, outcome) => app.document.querySelector(`#house-settle-rows input[name="settle-${debtId}"][value="${outcome}"]`);

async function pick(app, debtId, outcome) {
  const input = radio(app, debtId, outcome);
  input.checked = true;
  input.dispatchEvent(new app.window.Event('change', { bubbles: true }));
  await app.settle();
}

test('test_csd011_ac_27_1_one_row_per_open_debt_and_no_choice_preselected', async () => {
  const app = await openApp();
  const { answer: answer } = await openSettle(app);
  assert.equal(app.$('house-settle-dialog').open, true);
  const rows = [...app.$('house-settle-rows').querySelectorAll('.settle-row')];
  assert.deepEqual(rows.map((row) => app.text(row.firstElementChild)), ['Example Mortgage 190,000.00', 'Example Loan 10,000.00']);
  for (const row of rows) {
    assert.deepEqual([...row.querySelectorAll('label')].map((label) => app.text(label)), ['Repaid', 'Cancelled']);
    assert.equal(row.querySelectorAll('input:checked').length, 0, 'none is selected');
  }
  assert.equal(app.$('house-settle-save').disabled, true);
  assert.equal(app.text(app.$('house-settle-save')), 'Save sale');
  await app.click(app.$('house-settle-back'));
  await answer;
  app.close();
});

test('test_csd011_ac_27_2_save_sale_needs_a_choice_on_every_row', async () => {
  const app = await openApp();
  const { answer: answer } = await openSettle(app);
  await pick(app, 'debt-1', 'Repaid');
  assert.equal(app.$('house-settle-save').disabled, true, 'still disabled');
  await pick(app, 'debt-2', 'Cancelled');
  assert.equal(app.$('house-settle-save').disabled, false, 'enabled');
  await app.click(app.$('house-settle-back'));
  await answer;
  app.close();
});

test('test_csd011_ac_27_3_the_choices_are_named_per_debt', async () => {
  const app = await openApp();
  const { answer: answer } = await openSettle(app, [OPEN_DEBTS[0]]);
  const group = app.$('house-settle-rows').querySelector('[role="radiogroup"]');
  assert.equal(group.getAttribute('aria-label'), 'Outcome of Example Mortgage');
  const radios = [...group.querySelectorAll('input[type="radio"]')];
  assert.equal(radios.length, 2, 'two radio buttons');
  assert.deepEqual(radios.map((input) => input.value), ['Repaid', 'Cancelled']);
  assert.equal(new Set(radios.map((input) => input.name)).size, 1, 'one group');
  await app.click(app.$('house-settle-back'));
  await answer;
  app.close();
});

test('test_csd011_ac_27_4_changing_a_choice', async () => {
  const app = await openApp();
  const { answer: answer } = await openSettle(app, [OPEN_DEBTS[0]]);
  await pick(app, 'debt-1', 'Repaid');
  await pick(app, 'debt-1', 'Cancelled');
  assert.equal(radio(app, 'debt-1', 'Cancelled').checked, true);
  assert.equal(radio(app, 'debt-1', 'Repaid').checked, false, 'only Cancelled is selected');
  assert.equal(app.$('house-settle-save').disabled, false, 'Save sale stays enabled');
  await app.click(app.$('house-settle-back'));
  await answer;
  app.close();
});

test('test_csd011_ac_28_1_save_sale_returns_the_choices', async () => {
  const app = await openApp();
  const { answer: answer } = await openSettle(app);
  await pick(app, 'debt-1', 'Repaid');
  await pick(app, 'debt-2', 'Cancelled');
  await app.click(app.$('house-settle-save'));
  assert.equal(app.$('house-settle-dialog').open, false, 'the dialog closes');
  assert.deepEqual(await answer, [
    { debt_id: 'debt-1', outcome: 'Repaid' },
    { debt_id: 'debt-2', outcome: 'Cancelled' },
  ]);
  app.close();
});

test('test_csd011_ac_28_2_back_and_escape_save_nothing', async () => {
  for (const action of ['Back', 'Escape']) {
    const app = await openApp();
    const { answer: answer } = await openSettle(app);
    await pick(app, 'debt-1', 'Repaid');
    await pick(app, 'debt-2', 'Cancelled');
    if (action === 'Back') await app.click(app.$('house-settle-back'));
    else await app.escape(app.$('house-settle-dialog'));
    assert.equal(app.$('house-settle-dialog').open, false, `${action}: the dialog closes`);
    assert.equal(await answer, null, `${action}: no choice is returned`);
    assert.equal(app.callsOf('update_house').length + app.callsOf('update_debt').length, 0, `${action}: nothing is saved`);
    app.close();
  }
});

test('test_csd011_ac_28_3_a_reopened_dialog_starts_empty', async () => {
  const app = await openApp();
  const { answer: first } = await openSettle(app);
  await pick(app, 'debt-1', 'Repaid');
  await app.click(app.$('house-settle-back'));
  await first;
  const { answer: second } = await openSettle(app);
  assert.equal(app.$('house-settle-dialog').open, true);
  assert.equal(app.$('house-settle-rows').querySelectorAll('input:checked').length, 0, 'no choice is selected');
  assert.equal(app.$('house-settle-save').disabled, true, 'Save sale is disabled');
  await app.click(app.$('house-settle-back'));
  await second;
  app.close();
});
