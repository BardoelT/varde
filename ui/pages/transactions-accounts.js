// The Accounts tab of Transactions (CSD-034): the accounts the transactions belong to, adding, renaming and deleting an account,
// importing a bank export into an account and deleting the transactions of an account. Rust checks the names,
// reads the files and counts; this file shows the accounts and sends what the user chose.
import { invoke } from '../shared/ipc.js';
import { $, h, iconButton, submitOnce } from '../shared/dom.js';
import { t, tn } from '../shared/i18n.js';
import { openDialog, saveDialog, deleteWithConfirm } from '../shared/crud.js';
import { state, changed, run, showResult } from './transactions-shared.js';

export function renderAccounts() {
  const { accounts } = state;
  $('transaction-accounts-empty').hidden = accounts.length > 0;
  $('transaction-accounts').replaceChildren(
    ...accounts.map((account) =>
      h(
        'li',
        { class: 'job-row' },
        h('span', { class: 'job-name' }, account.name),
        h('span', { class: 'job-meta rule-count' }, tn('{count} transaction', '{count} transactions', account.transaction_count)),
        h(
          'span',
          { class: 'job-actions' },
          iconButton('import', t('Import transactions into {name}', { name: account.name }), () => importInto(account)),
          iconButton('edit', t('Edit {name}', { name: account.name }), () => openAccountDialog(account)),
          iconButton('delete', t('Delete {name}', { name: account.name }), () => deleteAccount(account)),
        ),
      ),
    ),
  );
}

/** Asks Rust for a file, reads it into the account and shows how many were added (CSD-034 REQ-3). */
function importInto(account) {
  return run(async () => {
    const summary = await invoke('import_transactions', { accountId: account.id });
    if (summary === null) return;
    showResult(t('{added} added, {skipped} skipped as duplicates', summary));
    await changed();
  });
}

let editingAccount = null;

function openAccountDialog(account = null) {
  editingAccount = account;
  openDialog({
    dialog: 'transaction-account-dialog',
    form: 'transaction-account-form',
    error: 'transaction-account-error',
    title: 'transaction-account-dialog-title',
    titleText: account ? t('Edit account') : t('Add account'),
    focus: 'name',
    fill(form) {
      form.elements.name.value = account ? account.name : '';
      // The import and the deletion of the transactions belong to an account that exists.
      $('transaction-account-transactions').hidden = account === null;
      if (account) {
        $('transaction-account-count').textContent = tn('{count} transaction', '{count} transactions', account.transaction_count);
        $('transaction-account-delete-transactions').disabled = account.transaction_count === 0;
      }
    },
  });
}

function submitAccount() {
  const name = $('transaction-account-form').elements.name.value;
  return saveDialog({
    dialog: 'transaction-account-dialog',
    error: 'transaction-account-error',
    changed,
    save: () =>
      editingAccount
        ? invoke('rename_transaction_account', { accountId: editingAccount.id, name })
        : invoke('add_transaction_account', { name }),
  });
}

/** Removes every transaction of the account after a confirmation; the account stays (CSD-034 REQ-4). */
function deleteTransactionsOf(account) {
  return deleteWithConfirm({
    title: t('Delete the transactions of {name}?', { name: account.name }),
    message: tn(
      'The transaction of this account will be deleted. The account, the categories and the rules stay. You can undo this.',
      'The {count} transactions of this account will be deleted. The account, the categories and the rules stay. You can undo this.',
      account.transaction_count,
    ),
    confirmLabel: t('Delete transactions'),
    command: 'delete_account_transactions',
    args: { accountId: account.id },
    changed,
  });
}

/** Deletes the account with its transactions after a confirmation that says how many go (CSD-034 REQ-2). */
function deleteAccount(account) {
  return deleteWithConfirm({
    title: t('Delete {name}?', { name: account.name }),
    message: tn(
      'The account and its transaction will be deleted. The categories and the rules stay. You can undo this.',
      'The account and its {count} transactions will be deleted. The categories and the rules stay. You can undo this.',
      account.transaction_count,
    ),
    confirmLabel: t('Delete account'),
    command: 'delete_transaction_account',
    args: { accountId: account.id },
    changed,
  });
}

export function initTransactionAccounts() {
  $('transaction-account-add').addEventListener('click', () => openAccountDialog(null));
  submitOnce($('transaction-account-form'), submitAccount);
  // The dialog of an account holds the import and the deletion of its transactions too.
  $('transaction-account-import').addEventListener('click', () => {
    const account = editingAccount;
    $('transaction-account-dialog').close();
    return importInto(account);
  });
  $('transaction-account-delete-transactions').addEventListener('click', () => {
    const account = editingAccount;
    $('transaction-account-dialog').close();
    return deleteTransactionsOf(account);
  });
}
