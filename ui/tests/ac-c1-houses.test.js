// CSD-019, CSD-020 and CSD-026: the display ACs of the house form, the debt form and the way the
// Houses and Debts overviews follow each other. Rust computes and validates everything, so the
// tests feed canned answers shaped like the golden ones (src-tauri/golden/ipc-reads.json), change
// them the way the command would, and check only what is displayed.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { exampleReads } from './fixtures.js';

const rowOf = (app, listId, name) => [...app.document.querySelectorAll(`#${listId} .job-row`)].find((row) => app.text(row.querySelector('.job-name')) === name);
const editHouse = async (app, name) => app.click(app.document.querySelector(`#house-list [aria-label="Edit ${name}"]`));

// ---- CSD-019: the house form ----

test('test_csd019_ac_12_1_the_add_house_form_has_the_purchase_and_the_optional_sale_fields', async () => {
  const app = await openApp();
  await app.click(app.item('houses'));
  await app.click(app.$('house-add'));
  const form = app.$('house-form');
  const labelOf = (name) => app.text(form.elements[name].closest('label'));
  assert.equal(app.textOf('house-dialog-title'), 'Add house');
  assert.match(labelOf('name'), /^House name/);
  assert.match(labelOf('period'), /^Valuation period/);
  assert.match(labelOf('purchase_date'), /^Purchase date/);
  assert.equal(form.elements.purchase_date.placeholder, 'dd/mm/yyyy', 'the purchase date is typed as dd/mm/yyyy');
  assert.match(labelOf('purchase_amount'), /^Purchase amount/);
  assert.match(labelOf('sell_date'), /^Sell date \(optional\)/);
  assert.equal(form.elements.sell_date.placeholder, 'dd/mm/yyyy');
  assert.match(labelOf('sell_amount'), /^Sell amount \(optional\)/);
  assert.equal(form.elements.sell_date.required, false, 'the sell fields are optional');
  assert.equal(form.elements.sell_amount.required, false);
  assert.equal(form.elements.purchase_date.required && form.elements.purchase_amount.required, true, 'the purchase is required');
  app.close();
});

test('test_csd019_ac_12_2_editing_a_sold_house_shows_the_stored_values', async () => {
  const app = await openApp();
  await app.click(app.item('houses'));
  await editHouse(app, 'Example House');
  const form = app.$('house-form');
  assert.equal(app.textOf('house-dialog-title'), 'Edit house');
  assert.equal(form.elements.sell_date.value, '10/06/2026', 'the sell date as dd/mm/yyyy');
  assert.equal(form.elements.sell_amount.value, '300000.00');
  assert.equal(form.elements.purchase_date.value, '15/03/2024');
  assert.equal(form.elements.purchase_amount.value, '250000.00');
  app.close();
});

// ---- CSD-026 REQ-12: what the Houses overview and the house form show of debts ----

test('test_csd026_ac_12_1_the_overview_rows_carry_no_debts', async () => {
  const app = await openApp();
  await app.click(app.item('houses'));
  const row = rowOf(app, 'house-list', 'Example House');
  assert.ok(row, 'the house is listed');
  assert.doesNotMatch(app.text(row), /Example Mortgage|Example Home Loan|Mortgage|Loan/, 'the row shows no debt');
  assert.equal(row.querySelectorAll('.badge').length, 0);
  app.close();
});

test('test_csd026_ac_12_3_the_add_house_form_has_no_list_of_debts', async () => {
  const app = await openApp();
  await app.click(app.item('houses'));
  await editHouse(app, 'Example House'); // a house with linked debts shows its table first
  assert.equal(app.isShown(app.$('house-debts')), true);
  await app.escape(app.$('house-dialog'));
  await app.click(app.$('house-add'));
  assert.equal(app.isShown(app.$('house-debts')), false, 'no table of debts');
  assert.equal(app.$('house-debt-rows').children.length, 0);
  app.close();
});

// ---- CSD-020: the debt form and the Houses overview ----

const heldHouse = (id, name) => ({
  colour_slot: 0, held: true, id, latest_period: null, latest_value: null, linked_debts: [], name, period: 'Monthly',
  purchase_amount: '250000.00', purchase_date: '2024-03-15', result: null, sell_amount: null, sell_date: null, valuation_count: 0,
});
const soldHouse = (id, name) => ({ ...heldHouse(id, name), held: false, sell_amount: '300000.00', sell_date: '2025-06-10' });
const mortgage = (extra = {}) => ({
  category: 'Other', colour_slot: 0, house_id: null, house_name: null, id: 'id-3', latest_balance: '150000.00', latest_period: '2026-09',
  name: 'Example Mortgage', period: 'Monthly', repaid: false, statement_count: 1, ...extra,
});

test('test_csd020_ac_15_7_the_houses_overview_follows_the_debt_form', async () => {
  const reads = exampleReads();
  reads.list_houses = { houses: [heldHouse('id-6', 'Example House')], total: null };
  reads.list_debts = { debts: [mortgage()], total: '150000.00' };
  const handlers = {
    update_debt: ({ debtId, input }) => {
      assert.equal(debtId, 'id-3');
      assert.equal(input.category, 'Mortgage');
      assert.equal(input.house_id, 'id-6');
      // Rust now links the debt: the next answers carry the link.
      app.bridge.table.list_debts = { debts: [mortgage({ category: 'Mortgage', house_id: 'id-6', house_name: 'Example House' })], total: '150000.00' };
      app.bridge.table.list_houses = {
        houses: [{ ...heldHouse('id-6', 'Example House'), linked_debts: [{ category: 'Mortgage', debt_id: 'id-3', debt_name: 'Example Mortgage', latest_balance: '150000.00', settlement: null }] }],
        total: null,
      };
      return { id: 'id-3' };
    },
  };
  const app = await openApp({ reads, handlers });
  await app.click(app.item('houses'));
  await editHouse(app, 'Example House');
  assert.equal(app.isShown(app.$('house-debts')), false, 'the Edit house form lists no linked debts');
  await app.escape(app.$('house-dialog'));

  await app.click(app.item('debts'));
  const houseCalls = app.callsOf('list_houses').length;
  await app.click(app.document.querySelector('#debt-list [aria-label="Edit Example Mortgage"]'));
  const form = app.$('debt-form');
  await app.choose(form.elements.category, 'Mortgage');
  form.elements.house_id.value = 'id-6';
  form.requestSubmit();
  await app.settle();
  assert.equal(app.$('debt-dialog').open, false, 'the debt is saved');
  assert.ok(app.callsOf('list_houses').length > houseCalls, 'the Houses overview is loaded again by the change');

  await app.click(app.item('houses'));
  await editHouse(app, 'Example House');
  assert.equal(app.isShown(app.$('house-debts')), true);
  const rows = [...app.$('house-debt-rows').querySelectorAll('tr')].map((row) => [...row.children].map((cell) => app.text(cell)));
  assert.deepEqual(rows, [['Example Mortgage', 'Mortgage', '150,000.00', '—']], 'the Edit house form lists the mortgage');
  app.close();
});

test('test_csd020_ac_30_2_the_house_dropdown_follows_the_category', async () => {
  const reads = exampleReads();
  reads.list_houses = { houses: [heldHouse('id-6', 'Example House'), soldHouse('id-7', 'Example Guesthouse')], total: null };
  const app = await openApp({ reads });
  await app.click(app.item('debts'));
  await app.click(app.$('debt-add'));
  const form = app.$('debt-form');
  assert.equal(form.elements.category.value, 'Other', 'the category is Other');
  assert.equal(app.isShown(app.$('debt-house-field')), false, 'and there is no House dropdown');

  await app.choose(form.elements.category, 'Mortgage');
  assert.equal(app.isShown(app.$('debt-house-field')), true, 'a House dropdown appears');
  assert.deepEqual([...form.elements.house_id.options].map((option) => option.textContent), ['No house', 'Example House', 'Example Guesthouse (sold)']);

  for (const category of ['Scholarship', 'Other']) {
    await app.choose(form.elements.category, 'Mortgage');
    await app.choose(form.elements.category, category);
    assert.equal(app.isShown(app.$('debt-house-field')), false, `the dropdown disappears for ${category}`);
  }
  app.close();
});

// ---- CSD-026 REQ-4: undoing a sale ----

test('test_csd026_ac_4_3_the_debts_overview_follows_the_undo', async () => {
  const reads = exampleReads();
  const unsettled = (debt) => {
    const { settlement, ...rest } = debt;
    return { ...rest, latest_balance: '20000.00', latest_period: '2026-04', repaid: false, statement_count: 1 };
  };
  const handlers = {
    update_house: ({ houseId, input, links }) => {
      assert.equal(houseId, 'id-6');
      assert.equal(input.sell_date, '', 'the sale is emptied');
      assert.equal(links.confirm_undo, true, 'the undo is confirmed');
      // Rust removes the settlement statements and the settlements; the sale is gone.
      const table = app.bridge.table;
      table.list_debts = {
        debts: table.list_debts.debts.map((debt) => (debt.settlement ? unsettled(debt) : debt)),
        total: table.list_debts.total,
      };
      table.list_debt_statements = table.list_debt_statements.filter((row) => !row.settlement);
      table.list_houses = {
        ...table.list_houses,
        houses: table.list_houses.houses.map((house) => (house.id === 'id-6'
          ? { ...house, held: true, sell_date: null, sell_amount: null, result: null, linked_debts: house.linked_debts.map((debt) => ({ ...debt, settlement: null })) }
          : house)),
      };
      return { id: 'id-6' };
    },
  };
  const app = await openApp({ reads, handlers });
  await app.click(app.item('debts'));
  const loanBadges = () => [...rowOf(app, 'debt-list', 'Example Home Loan').querySelectorAll('.badge')].map((node) => app.text(node));
  const settlementRows = () => [...app.document.querySelectorAll('#debt-statements-rows tr')].filter((row) => app.text(row).includes('Cancelled on sale'));
  assert.deepEqual(loanBadges(), ['Cancelled 20,000.00'], 'the loan shows the badge Cancelled');
  assert.equal(settlementRows().length, 1, 'and its settlement statement');

  await app.click(app.item('houses'));
  await editHouse(app, 'Example House');
  const form = app.$('house-form');
  form.elements.sell_date.value = '';
  form.elements.sell_amount.value = '';
  form.requestSubmit();
  await app.settle();
  assert.equal(app.$('confirm-dialog').open, true, 'the undo is confirmed');
  const debtCalls = app.callsOf('list_debts').length;
  await app.click(app.$('confirm-ok'));
  assert.equal(app.$('house-dialog').open, false);
  assert.equal(app.callsOf('update_house').length, 1);
  assert.ok(app.callsOf('list_debts').length > debtCalls, 'the Debts overview was loaded again by the undo');

  // The Debts page is only shown again: it was refreshed by the undo, not by opening it.
  const loaded = app.callsOf('list_debts').length;
  await app.click(app.item('debts'));
  assert.equal(app.callsOf('list_debts').length, loaded, 'the page was not reopened');
  assert.deepEqual(loanBadges(), [], 'the loan has no badge');
  assert.equal(settlementRows().length, 0, 'and no settlement statement');
  app.close();
});
