// CSD-011 REQ-21, REQ-33 and REQ-34, CSD-026 REQ-16: the display ACs of the opening and the closing of a
// debt. Rust computes and orders every row, so the tests feed canned answers shaped like the ones Rust
// returns (see src-tauri/golden/ipc-reads.json) and check only what is displayed.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { exampleReads } from './fixtures.js';

const debt = (extra = {}) => ({
  id: 'id-1', name: 'Example Mortgage', period: 'Monthly', category: 'Other', colour_slot: 0,
  house_id: null, house_name: null, latest_balance: '150000.00', latest_period: '2026-09', statement_count: 1, repaid: false,
  opening_date: '2020-03-15', opening_amount: '200000.00', settlement: null, ...extra,
});
const statement = (extra = {}) => ({
  balance: '150000.00', change: null, debt_id: 'id-1', debt_name: 'Example Mortgage', locked: false, note: '',
  opening: false, outcome: null, period: '2026-09', settlement: false, ...extra,
});
const openingRow = (extra = {}) => statement({ balance: '200000.00', locked: true, opening: true, period: '2020-03-15', ...extra });

function reads(debts, statements) {
  const answer = exampleReads();
  Object.assign(answer, {
    list_debts: { debts, total: '150000.00' },
    list_debt_statements: statements,
    yearly_debt_totals: [{ year: 2026, closing_balance: '150000.00', repaid: null, statement_count: 1 }],
  });
  return answer;
}

async function openDebts(answer, handlers = {}) {
  const app = await openApp({ reads: answer, handlers });
  await app.click(app.item('debts'));
  return app;
}

const statementRows = (app) => [...app.document.querySelectorAll('#debt-statements-rows tr')];
const cells = (app, row) => [...row.children].map((cell) => app.text(cell));

test('test_csd011_ac_33_1_the_opening_row_is_the_oldest_row_of_its_debt', async () => {
  const app = await openDebts(reads([debt()], [statement(), statement({ period: '2026-08', balance: '150500.00' }), openingRow()]));
  const rows = statementRows(app);
  assert.deepEqual(rows.map((row) => app.text(row.querySelector('th'))), ['2026-09', '2026-08', '15/03/2020']);
  assert.deepEqual(cells(app, rows[2]), ['15/03/2020', 'Example Mortgage', '200,000.00', '—', 'Opening', '']);
  assert.equal(rows[2].querySelectorAll('button').length, 0, 'the Opening row has no Edit or Delete buttons');
  assert.equal(rows[0].querySelectorAll('button').length, 2, 'a statement keeps its buttons');
  app.close();
});

test('test_csd011_ac_33_3_a_debt_without_statements_still_shows_its_opening_row', async () => {
  const empty = debt({ latest_balance: null, latest_period: null, statement_count: 0 });
  const app = await openDebts(reads([empty], [openingRow()]));
  assert.equal(statementRows(app).length, 1);
  assert.equal(app.text(statementRows(app)[0].querySelector('.note-cell')), 'Opening');
  assert.equal(app.isShown(app.$('debt-statements-table')), true);
  assert.equal(app.textOf('debt-statements-empty'), 'No statements yet.');
  assert.equal(app.isShown(app.$('debt-statements-empty')), true, 'one row is still no statement');
  app.close();
});

test('test_csd011_ac_21_4_more_than_one_row_in_the_table', async () => {
  const empty = (id, name) => debt({ id, name, latest_balance: null, latest_period: null, statement_count: 0 });
  const rows = [openingRow(), openingRow({ debt_id: 'id-2', debt_name: 'Example Card' })];
  const app = await openDebts(reads([empty('id-1', 'Example Mortgage'), empty('id-2', 'Example Card')], rows));
  assert.equal(statementRows(app).length, 2);
  assert.equal(app.isShown(app.$('debt-statements-empty')), false, 'no "No statements yet."');
  app.close();
});

test('test_csd011_ac_34_1_the_form_shows_the_opening_and_closing_fields', async () => {
  const app = await openDebts(reads([debt()], [statement(), openingRow()]));
  await app.click(app.$('debt-add'));
  const form = app.$('debt-form');
  assert.equal(form.elements.opening_date.required, true);
  assert.equal(form.elements.opening_amount.required, true);
  for (const name of ['closing_date', 'closing_amount', 'closing_outcome']) {
    assert.ok(form.elements[name], name);
    assert.equal(form.elements[name].required, false, `${name} is optional`);
  }
  const labels = [...form.querySelectorAll('label.field')].map((label) => app.text(label).split(' ').slice(0, 2).join(' '));
  for (const label of ['Opening date', 'Opening amount', 'Closing date', 'Closing amount', 'Outcome (optional)']) {
    assert.ok(labels.includes(label), label);
  }
  app.close();
});

test('test_csd011_ac_34_2_a_linked_debt_has_editable_closing_fields', async () => {
  const linked = debt({ category: 'Mortgage', house_id: 'id-6', house_name: 'Example House' });
  const app = await openDebts(reads([linked], [statement(), openingRow()]));
  await app.click(app.document.querySelector('#debt-list [aria-label="Edit Example Mortgage"]'));
  const form = app.$('debt-form');
  for (const name of ['closing_date', 'closing_amount', 'closing_outcome', 'opening_date', 'opening_amount']) {
    assert.equal(form.elements[name].disabled, false, name);
  }
  assert.equal(app.isShown(app.$('debt-closing-hint')), false, 'no hint about a sale');
  app.close();
});

test('test_csd011_ac_34_4_a_closing_by_sale_shows_a_hint_and_can_be_edited', async () => {
  const closed = debt({
    category: 'Loan', house_id: 'id-6', house_name: 'Example House',
    settlement: { outcome: 'Cancelled', origin: 'Sale', date: '2026-06-10', amount: '3500.00' },
  });
  const handlers = {
    update_debt: ({ input }) => {
      assert.deepEqual(input.closing, { date: '11/06/2026', amount: '3500.00', outcome: 'Cancelled' });
      assert.equal(input.confirm_undo, undefined);
      return { id: 'id-1' };
    },
  };
  const app = await openDebts(reads([closed], [statement(), openingRow()]), handlers);
  await app.click(app.document.querySelector('#debt-list [aria-label="Edit Example Mortgage"]'));
  const form = app.$('debt-form');
  for (const name of ['closing_date', 'closing_amount', 'closing_outcome']) assert.equal(form.elements[name].disabled, false, name);
  assert.equal(form.elements.closing_date.value, '10/06/2026');
  assert.equal(form.elements.closing_amount.value, '3500.00');
  assert.equal(form.elements.closing_outcome.value, 'Cancelled');
  assert.equal(
    app.textOf('debt-closing-hint'),
    'Set by the sale of Example House. Changing it makes it a manual closing.',
  );
  form.elements.closing_date.value = '11/06/2026';
  form.requestSubmit();
  await app.settle();
  assert.equal(app.callsOf('update_debt').length, 1);
  assert.equal(app.$('debt-dialog').open, false, 'the closing is saved');
  app.close();
});

test('test_csd011_ac_34_4_a_manual_closing_on_a_linked_debt_shows_no_hint', async () => {
  const closed = debt({
    category: 'Loan', house_id: 'id-6', house_name: 'Example House',
    settlement: { outcome: 'Repaid', origin: 'Manual', date: '2026-05-15', amount: '3500.00' },
  });
  const app = await openDebts(reads([closed], [statement(), openingRow()]));
  await app.click(app.document.querySelector('#debt-list [aria-label="Edit Example Mortgage"]'));
  assert.equal(app.isShown(app.$('debt-closing-hint')), false);
  assert.equal(app.$('debt-form').elements.closing_date.value, '15/05/2026');
  app.close();
});

test('test_csd011_ac_34_5_clearing_any_closing_asks_for_confirmation', async () => {
  const closed = debt({
    category: 'Loan', house_id: 'id-6', house_name: 'Example House',
    settlement: { outcome: 'Cancelled', origin: 'Sale', date: '2026-06-10', amount: '3500.00' },
  });
  const handlers = {
    update_debt: ({ input }) => {
      assert.equal(input.confirm_undo, true);
      assert.deepEqual(input.closing, { date: '', amount: '', outcome: '' });
      return { id: 'id-1' };
    },
  };
  const app = await openDebts(reads([closed], [statement(), openingRow()]), handlers);
  await app.click(app.document.querySelector('#debt-list [aria-label="Edit Example Mortgage"]'));
  const form = app.$('debt-form');
  form.elements.closing_date.value = '';
  form.elements.closing_amount.value = '';
  form.elements.closing_outcome.value = '';
  form.requestSubmit();
  await app.settle();
  assert.equal(app.textOf('confirm-title'), 'Undo the closing?');
  assert.equal(app.callsOf('update_debt').length, 0, 'nothing is saved before the confirmation');
  await app.click(app.$('confirm-ok'));
  assert.equal(app.callsOf('update_debt').length, 1);
  app.close();
});

test('test_csd011_ac_34_3_an_unlinked_debt_can_be_closed_in_the_form', async () => {
  const handlers = {
    update_debt: ({ debtId, input }) => {
      assert.equal(debtId, 'id-1');
      assert.equal(input.opening_date, '15/03/2020');
      assert.equal(input.opening_amount, '200000.00');
      assert.deepEqual(input.closing, { date: '10/06/2026', amount: '0.00', outcome: 'Repaid' });
      assert.equal(input.confirm_undo, undefined);
      return { id: 'id-1' };
    },
  };
  const app = await openDebts(reads([debt()], [statement(), openingRow()]), handlers);
  await app.click(app.document.querySelector('#debt-list [aria-label="Edit Example Mortgage"]'));
  const form = app.$('debt-form');
  assert.equal(form.elements.closing_date.disabled, false, 'the closing fields are editable');
  form.elements.closing_date.value = '10/06/2026';
  form.elements.closing_amount.value = '0.00';
  form.elements.closing_outcome.value = 'Repaid';
  form.requestSubmit();
  await app.settle();
  assert.equal(app.callsOf('update_debt').length, 1);
  assert.equal(app.$('debt-dialog').open, false, 'the debt is saved');
  app.close();
});

test('test_csd026_ac_16_1_clearing_a_manual_closing_asks_for_confirmation', async () => {
  const closed = debt({ settlement: { outcome: 'Repaid', origin: 'Manual', date: '2026-06-10', amount: '800.00' } });
  const handlers = {
    update_debt: ({ input }) => {
      assert.equal(input.confirm_undo, true);
      assert.deepEqual(input.closing, { date: '', amount: '', outcome: '' });
      return { id: 'id-1' };
    },
  };
  const app = await openDebts(reads([closed], [statement(), openingRow()]), handlers);
  await app.click(app.document.querySelector('#debt-list [aria-label="Edit Example Mortgage"]'));
  const form = app.$('debt-form');
  form.elements.closing_date.value = '';
  form.elements.closing_amount.value = '';
  form.elements.closing_outcome.value = '';
  form.requestSubmit();
  await app.settle();
  assert.equal(app.textOf('confirm-title'), 'Undo the closing?');
  assert.equal(app.callsOf('update_debt').length, 0, 'nothing is saved before the confirmation');
  await app.click(app.$('confirm-ok'));
  assert.equal(app.callsOf('update_debt').length, 1);
  app.close();
});

test('test_csd026_ac_16_2_declining_the_confirmation_saves_nothing', async () => {
  const closed = debt({ settlement: { outcome: 'Repaid', origin: 'Manual', date: '2026-06-10', amount: '800.00' } });
  const app = await openDebts(reads([closed], [statement(), openingRow()]));
  await app.click(app.document.querySelector('#debt-list [aria-label="Edit Example Mortgage"]'));
  const form = app.$('debt-form');
  form.elements.closing_date.value = '';
  form.elements.closing_amount.value = '';
  form.elements.closing_outcome.value = '';
  form.requestSubmit();
  await app.settle();
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.callsOf('update_debt').length, 0);
  app.close();
});
