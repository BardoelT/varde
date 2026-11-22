// DOM tests of CSD-034 (the Accounts tab of the Transactions page): the list of accounts, adding, renaming and deleting an account,
// importing into an account and deleting its transactions. Rust checks and counts; the tests feed canned answers
// and check only what is displayed and what is sent.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { BANK, SAVINGS, transaction, readsWith, transactionsPage } from './transactions-helpers.js';

/** The Accounts tab with the canned accounts: "Example Bank" (3 transactions) and "Example Savings" (1). */
async function accountsPage(handlers = {}, readsOptions = { rows: [], count: 0 }) {
  const app = await transactionsPage(readsWith(readsOptions), handlers);
  await app.click(app.$('transactions-tab-accounts'));
  return app;
}

const accountRows = (app) => [...app.$('transaction-accounts').querySelectorAll('li')];
const button = (row, label) => row.querySelector(`[aria-label^="${label}"]`);

test('test_csd034_ac_1_3_accounts_is_the_first_tab_of_transactions_and_not_in_the_left_pane', async () => {
  const app = await openApp();
  const items = [...app.document.querySelectorAll('.nav-item')].map((item) => app.text(item));
  assert.ok(!items.includes('Accounts'), 'no item in the left pane');
  await app.click(app.item('transactions'));
  const tabs = [...app.document.querySelectorAll('#transactions [role="tab"]')].map((tab) => app.text(tab));
  assert.deepEqual(tabs, ['Accounts', 'Categories', 'Rules', 'Yearly totals', 'Transactions', 'Insights', 'Budgets']);
  assert.equal(app.$('transactions-tab-transactions').getAttribute('aria-selected'), 'true', 'the Transactions tab stays the one shown first');
  assert.equal(app.isShown(app.$('transactions-accounts')), false);
  await app.click(app.$('transactions-tab-accounts'));
  assert.ok(app.isShown(app.$('transactions-accounts')));
  assert.equal(app.isShown(app.$('transactions-transactions')), false);
  app.close();
});
test('test_csd034_ac_1_1_the_rows_show_the_name_the_count_and_the_three_buttons', async () => {
  const app = await accountsPage();
  const rows = accountRows(app);
  assert.deepEqual(rows.map((row) => app.text(row.querySelector('.job-name'))), ['Example Bank', 'Example Savings']);
  assert.deepEqual(rows.map((row) => app.text(row.querySelector('.rule-count'))), ['3 transactions', '1 transaction']);
  for (const row of rows) {
    assert.ok(button(row, 'Import transactions into'), 'import');
    assert.ok(button(row, 'Edit'), 'edit');
    assert.ok(button(row, 'Delete'), 'delete');
  }
  assert.equal(app.isShown(app.$('transaction-accounts-empty')), false);
  app.close();
});

test('test_csd034_ac_1_2_no_account_yet_says_how_to_start', async () => {
  const app = await accountsPage({}, { rows: [], count: 0, accounts: [] });
  assert.equal(accountRows(app).length, 0);
  assert.ok(app.isShown(app.$('transaction-accounts-empty')));
  assert.equal(app.textOf('transaction-accounts-empty'), 'No accounts yet. Add an account, then import the transactions of a bank into it.');
  app.close();
});

test('test_csd034_ac_2_1_adding_sends_the_name_and_a_rejection_shows_in_the_dialog_in_dutch', async () => {
  const app = await accountsPage({
    add_transaction_account: ({ name }) => {
      if (name === 'example bank') throw { code: 'TRANSACTION_ACCOUNT_NAME_TAKEN', message: 'An account with this name already exists.' };
      return { id: 'acc-new', name };
    },
  });
  await app.click(app.$('transaction-account-add'));
  const form = app.$('transaction-account-form');
  assert.equal(app.isShown(app.$('transaction-account-transactions')), false, 'a new account has nothing to import or delete yet');
  form.elements.name.value = 'example bank';
  form.requestSubmit();
  await app.settle();
  assert.match(app.textOf('transaction-account-error'), /already exists/);
  assert.ok(app.$('transaction-account-dialog').open, 'the dialog stays open');
  form.elements.name.value = 'Example New';
  form.requestSubmit();
  await app.settle();
  assert.deepEqual(app.callsOf('add_transaction_account').at(-1).args, { name: 'Example New' });
  assert.equal(app.$('transaction-account-dialog').open, false);
  app.close();
  const reads = readsWith({ rows: [], count: 0 });
  reads.get_preferences = { theme: 'system', language: 'nl', number_format: 'point_comma', discrete_mode: false };
  const dutch = await transactionsPage(reads, {
    add_transaction_account: () => {
      throw { code: 'TRANSACTION_ACCOUNT_NAME_TAKEN', message: 'An account with this name already exists.' };
    },
  });
  await dutch.click(dutch.$('transactions-tab-accounts'));
  await dutch.click(dutch.$('transaction-account-add'));
  dutch.$('transaction-account-form').requestSubmit();
  await dutch.settle();
  assert.equal(dutch.textOf('transaction-account-error'), 'Er bestaat al een rekening met deze naam.');
  assert.equal(dutch.textOf('transactions-tab-accounts'), 'Rekeningen');
  dutch.close();
});

test('test_csd034_ac_2_1_editing_renames_the_account', async () => {
  const app = await accountsPage({ rename_transaction_account: ({ name }) => ({ id: BANK, name }) });
  await app.click(button(accountRows(app)[0], 'Edit'));
  const form = app.$('transaction-account-form');
  assert.equal(form.elements.name.value, 'Example Bank');
  form.elements.name.value = 'Example Current';
  form.requestSubmit();
  await app.settle();
  assert.deepEqual(app.callsOf('rename_transaction_account').at(-1).args, { accountId: BANK, name: 'Example Current' });
  app.close();
});

test('test_csd034_ac_2_4_deleting_an_account_names_how_many_transactions_go_with_it', async () => {
  const app = await accountsPage({ delete_transaction_account: 3 });
  await app.click(button(accountRows(app)[0], 'Delete'));
  assert.equal(app.$('confirm-dialog').open, true);
  assert.match(app.text(app.$('confirm-dialog')), /The account and its 3 transactions will be deleted/);
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.callsOf('delete_transaction_account').length, 0, 'cancelling changes nothing');
  await app.click(button(accountRows(app)[1], 'Delete'));
  assert.match(app.text(app.$('confirm-dialog')), /The account and its transaction will be deleted/, 'one transaction, singular');
  await app.click(app.$('confirm-cancel'));
  await app.click(button(accountRows(app)[0], 'Delete'));
  await app.click(app.$('confirm-ok'));
  assert.deepEqual(app.callsOf('delete_transaction_account').at(-1).args, { accountId: BANK });
  app.close();
});

test('test_csd034_ac_3_1_importing_goes_into_the_account_of_the_row_and_shows_the_result', async () => {
  const app = await accountsPage({ import_transactions: { added: 3, skipped: 2 } });
  await app.click(button(accountRows(app)[1], 'Import transactions into'));
  assert.deepEqual(app.callsOf('import_transactions').at(-1).args, { accountId: SAVINGS });
  assert.equal(app.textOf('toast'), '3 added, 2 skipped as duplicates');
  assert.ok(app.$('toast').classList.contains('toast-notice'), 'yellow, not the red of an error');
  app.close();
});

test('test_csd034_ac_3_2_the_dialog_of_an_account_has_the_import_and_the_deletion_of_its_transactions', async () => {
  const app = await accountsPage({ import_transactions: { added: 1, skipped: 0 }, delete_account_transactions: 3 });
  await app.click(button(accountRows(app)[0], 'Edit'));
  assert.ok(app.isShown(app.$('transaction-account-transactions')));
  assert.equal(app.textOf('transaction-account-count'), '3 transactions');
  await app.click(app.$('transaction-account-import'));
  assert.deepEqual(app.callsOf('import_transactions').at(-1).args, { accountId: BANK });
  assert.equal(app.$('transaction-account-dialog').open, false, 'the dialog closes for the file dialog');
  assert.equal(app.textOf('toast'), '1 added, 0 skipped as duplicates');
  await app.click(button(accountRows(app)[0], 'Edit'));
  await app.click(app.$('transaction-account-delete-transactions'));
  assert.match(app.text(app.$('confirm-dialog')), /The 3 transactions of this account will be deleted/);
  await app.click(app.$('confirm-ok'));
  assert.deepEqual(app.callsOf('delete_account_transactions').at(-1).args, { accountId: BANK });
  app.close();
});

test('test_csd034_ac_3_3_cancelling_the_file_dialog_shows_nothing_and_a_rejected_file_shows_its_error', async () => {
  const cancelled = await accountsPage({ import_transactions: null });
  await cancelled.click(button(accountRows(cancelled)[0], 'Import transactions into'));
  assert.equal(cancelled.$('toast').hidden, true);
  cancelled.close();
  const rejected = await accountsPage({
    import_transactions: () => {
      throw { code: 'TRANSACTION_IMPORT_DATE', message: 'Line 4: the date is not a real date as dd-mm-yyyy (Knab) or yyyymmdd (ABN AMRO).' };
    },
  });
  await rejected.click(button(accountRows(rejected)[0], 'Import transactions into'));
  assert.match(rejected.textOf('toast'), /Line 4/);
  assert.equal(rejected.$('toast').classList.contains('toast-notice'), false, 'an error stays red');
  rejected.close();
});

test('test_csd034_ac_4_1_an_account_without_transactions_has_nothing_to_delete', async () => {
  const empty = [{ id: BANK, name: 'Example Bank', transaction_count: 0 }];
  const app = await accountsPage({}, { rows: [], count: 0, accounts: empty });
  await app.click(button(accountRows(app)[0], 'Edit'));
  assert.equal(app.$('transaction-account-delete-transactions').disabled, true);
  app.close();
});

test('test_csd034_ac_9_1_a_change_reloads_the_accounts_with_the_overview', async () => {
  const rows = [transaction('t1', '2026-10-30', '-1.00', 'Example Shop')];
  const app = await accountsPage({ delete_transaction: null }, { rows, count: 1 });
  const before = app.callsOf('get_transactions_overview').length;
  await app.click(app.item('transactions'));
  await app.click(app.document.querySelector('[aria-label="Delete Example Shop"]'));
  await app.click(app.$('confirm-ok'));
  assert.ok(app.callsOf('get_transactions_overview').length > before, 'the accounts come with the overview');
  app.close();
});
