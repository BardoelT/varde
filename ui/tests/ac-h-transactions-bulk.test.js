// DOM tests of CSD-032 (bulk select). Rust computes every figure; the tests feed canned answers and check only what is displayed.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { SHOPPING, transaction, readsWith, transactionsPage, bulkRows, bulkHandlers } from './transactions-helpers.js';

test('test_csd032_ac_28_8_the_bulk_select_button_looks_pressed_while_bulk_select_is_on', async () => {
  const app = await transactionsPage(readsWith({ rows: bulkRows(), count: 3 }), bulkHandlers());
  const button = app.$('transaction-bulk-toggle');
  assert.equal(button.getAttribute('aria-pressed'), 'false');
  await app.click(button);
  assert.equal(button.getAttribute('aria-pressed'), 'true', 'pressed while on');
  await app.click(button);
  assert.equal(button.getAttribute('aria-pressed'), 'false', 'released when switched off');
  app.close();
  // jsdom has no layout: the pressed look is guarded as a rule of the stylesheet.
  const css = readFileSync(fileURLToPath(new URL('../styles/components.css', import.meta.url)), 'utf8');
  assert.match(css, /\.btn\[aria-pressed="true"\]\s*\{[^}]*background:\s*var\(--accent\)/);
});

test('test_csd032_ac_28_1_bulk_select_is_off_until_enabled', async () => {
  const app = await transactionsPage(readsWith({ rows: bulkRows(), count: 3 }), bulkHandlers());
  assert.equal(app.document.querySelectorAll('#transactions-rows input[type="checkbox"]').length, 0);
  assert.equal(app.isShown(app.$('transactions-bulk')), false);
  assert.equal(app.$('transactions-select-all'), null, 'no header checkbox while it is off');
  const dateHead = app.document.querySelector('#transactions-table thead th');
  assert.equal(app.text(dateHead).startsWith('Date'), true, 'the date column heads the table, so its padding matches the rows');
  await app.click(app.$('transaction-bulk-toggle'));
  assert.equal(app.document.querySelectorAll('#transactions-rows input[type="checkbox"]').length, 3);
  assert.equal(app.isShown(app.$('transactions-bulk')), true);
  assert.equal(app.isShown(app.$('transactions-select-all')), true);
  assert.equal(app.textOf('transactions-bulk-count'), '0 selected');
  assert.equal(app.$('transactions-bulk-set').disabled, true);
  await app.click(app.$('transaction-bulk-toggle'));
  assert.equal(app.document.querySelectorAll('#transactions-rows input[type="checkbox"]').length, 0);
  app.close();
});

test('test_csd032_ac_28_2_a_checkbox_selects_one_and_the_header_selects_all_in_the_filter', async () => {
  const app = await transactionsPage(readsWith({ rows: bulkRows(), count: 3 }), bulkHandlers());
  await app.click(app.$('transaction-bulk-toggle'));
  await app.click(app.document.querySelector('[aria-label="Select Example Shop"]'));
  assert.equal(app.textOf('transactions-bulk-count'), '1 selected');
  assert.equal(app.$('transactions-select-all').indeterminate, true);
  await app.click(app.$('transactions-select-all'));
  assert.equal(app.textOf('transactions-bulk-count'), '3 selected');
  assert.equal(app.$('transactions-select-all').checked, true);
  assert.equal(app.callsOf('list_transaction_ids').length, 1, 'Rust names every id of the filter');
  await app.click(app.$('transactions-select-all'));
  assert.equal(app.textOf('transactions-bulk-count'), '0 selected');
  app.close();
});

test('test_csd032_ac_28_3_select_all_covers_rows_that_are_not_loaded', async () => {
  const ids = Array.from({ length: 450 }, (_, n) => `t${n}`);
  const rows = ids.slice(0, 200).map((id, n) => transaction(id, '2026-10-01', '-1.00', `Example ${n}`));
  const app = await transactionsPage(readsWith({ rows, count: 450 }), {
    list_transactions: () => ({ rows, count: 450, total: '-450.00' }),
    list_transaction_ids: ids,
    set_transactions_category: (args) => args.transactionIds.length,
  });
  await app.click(app.$('transaction-bulk-toggle'));
  await app.click(app.$('transactions-select-all'));
  assert.equal(app.textOf('transactions-bulk-count'), '450 selected');
  assert.match(app.$('transactions-select-all').getAttribute('aria-label'), /450/);
  await app.choose(app.$('transactions-bulk-category'), SHOPPING);
  await app.click(app.$('transactions-bulk-set'));
  const call = app.callsOf('set_transactions_category').at(-1).args;
  assert.equal(call.transactionIds.length, 450);
  assert.equal(call.categoryId, SHOPPING);
  assert.equal(app.textOf('toast'), '450 changed');
  assert.equal(app.textOf('transactions-bulk-count'), '0 selected', 'the selection is cleared after the change');
  app.close();
});

test('test_csd032_ac_28_4_select_all_follows_the_filter', async () => {
  const app = await transactionsPage(readsWith({ rows: bulkRows(), count: 3 }), bulkHandlers());
  await app.click(app.$('transaction-bulk-toggle'));
  await app.click(app.$('transaction-manual-only'));
  await app.click(app.$('transactions-select-all'));
  assert.deepEqual(app.callsOf('list_transaction_ids').at(-1).args, { manualOnly: true });
  await app.click(app.$('transaction-manual-only'));
  assert.equal(app.textOf('transactions-bulk-count'), '0 selected', 'changing the filter clears the selection');
  app.close();
});

test('test_csd032_ac_28_5_deleting_the_selected_asks_first', async () => {
  const app = await transactionsPage(readsWith({ rows: bulkRows(), count: 3 }), bulkHandlers({ delete_transactions: 2 }));
  await app.click(app.$('transaction-bulk-toggle'));
  await app.click(app.document.querySelector('[aria-label="Select Example Shop"]'));
  await app.click(app.document.querySelector('[aria-label="Select Example Cafe"]'));
  await app.click(app.$('transactions-bulk-delete'));
  assert.equal(app.$('confirm-dialog').open, true);
  assert.match(app.textOf('confirm-message'), /2 selected transactions/);
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.callsOf('delete_transactions').length, 0);
  assert.equal(app.textOf('transactions-bulk-count'), '2 selected');
  await app.click(app.$('transactions-bulk-delete'));
  await app.click(app.$('confirm-ok'));
  assert.deepEqual(app.callsOf('delete_transactions').at(-1).args, { transactionIds: ['t1', 't2'] });
  assert.equal(app.textOf('transactions-bulk-count'), '0 selected');
  app.close();
});
