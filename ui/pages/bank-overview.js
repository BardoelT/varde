// Bank accounts: accounts with a monthly or yearly entry, each a closing balance and a note (CSD-009).
import { invoke, errorText } from '../shared/ipc.js';
import { $, h, showError, showToast, setHint, submitOnce } from '../shared/dom.js';
import { formatAmount, noteSummary } from '../shared/format.js';
import { t } from '../shared/i18n.js';
import { createStatementFilter, renderFilteredTable } from '../shared/statement-filter.js';
import { openDialog, saveDialog, deleteWithConfirm } from '../shared/crud.js';
import { createRecordForm } from '../shared/record-form.js';
import { accountRow, latestAmount, numCell, recordActions, renderTable, rowHeader, showTotal, toggleEmpty } from '../shared/list-rows.js';

let accounts = [];
let onChanged = async () => {};

// ---- Rendering ----

function renderAccounts(overview) {
  accounts = overview.accounts;
  const hasAccounts = accounts.length > 0;
  toggleEmpty('bank-empty', 'bank-content', hasAccounts);
  $('bank-account-list').replaceChildren(
    ...accounts.map((account) =>
      accountRow({
        name: account.name,
        meta: account.period,
        amount: latestAmount(account.latest_balance, account.latest_period),
        edit: { label: t('Edit {name}', { name: account.name }), onClick: () => openAccountDialog(account) },
        remove: { label: t('Delete {name}', { name: account.name }), onClick: () => deleteAccount(account) },
      }),
    ),
  );
  showTotal('bank-account-total', 'bank-account-total-value', overview.total, hasAccounts);
}

/** Yearly totals over all accounts, as returned by Rust (CSD-009 REQ-19). */
function renderTotals(totals) {
  renderTable({ empty: 'bank-totals-empty', table: 'bank-totals-table', body: 'bank-totals-rows' }, totals, (total) =>
    h('tr', {}, rowHeader(String(total.year)), numCell(formatAmount(total.closing_balance))),
  );
}

const entryFilter = createStatementFilter({
  periodHost: 'bank-entry-period-filter',
  entityHost: 'bank-entry-entity-filter',
  entityTitle: 'Account',
  entities: () => accounts.filter((account) => account.entry_count > 0).map((account) => ({ id: account.id, label: account.name })),
  onChange: () => reloadEntries().catch((error) => showToast(errorText(error))),
});

/** Loads the entries through the filter and shows them (CSD-028). */
async function reloadEntries() {
  showEntries(await entryFilter.load('list_bank_entries', (rows) => rows.length));
}

function showEntries({ result, hasData }) {
  entryFilter.show(hasData);
  renderEntries(result, hasData);
}

function renderEntries(entries, hasData) {
  renderFilteredTable({ empty: 'bank-entries-empty', nomatch: 'bank-entries-nomatch', table: 'bank-entries-table', body: 'bank-entries-rows' }, { rows: entries, hasData }, (e) =>
    h(
      'tr',
      {},
      rowHeader(e.period),
      h('td', { class: 'wrap' }, e.account_name),
      numCell(formatAmount(e.balance)),
      h('td', { class: 'note-cell' }, noteSummary(e.note)),
      recordActions({
        edit: { label: t('Edit {account} {period}', { account: e.account_name, period: e.period }), onClick: () => entryForm.open(e) },
        remove: { label: t('Delete {account} {period}', { account: e.account_name, period: e.period }), onClick: () => deleteEntry(e) },
      }),
    ),
  );
}

/** Reloads everything from Rust. */
export async function reloadBankOverview() {
  const overview = await invoke('list_bank_accounts');
  renderAccounts(overview);
  if (overview.accounts.length === 0) {
    entryFilter.show(false);
    return;
  }
  const [totals, loaded] = await Promise.all([invoke('yearly_bank_totals'), entryFilter.load('list_bank_entries', (rows) => rows.length)]);
  renderTotals(totals);
  showEntries(loaded);
}

async function changed() {
  await reloadBankOverview();
  await onChanged();
}

// ---- Account dialog ----

let editingAccount = null;

function openAccountDialog(account) {
  editingAccount = account;
  openDialog({
    dialog: 'bank-account-dialog',
    form: 'bank-account-form',
    error: 'bank-account-error',
    title: 'bank-account-dialog-title',
    titleText: account ? t('Edit account') : t('Add account'),
    focus: 'name',
    fill(form) {
      // The entry period is locked once the account has entries (CSD-009 REQ-5).
      const locked = Boolean(account && account.entry_count > 0);
      form.elements.period.disabled = locked;
      setHint('bank-account-period-hint', locked);
      if (account) {
        form.elements.name.value = account.name;
        form.elements.period.value = account.period;
      }
    },
  });
}

function submitAccount() {
  const form = $('bank-account-form');
  const input = { name: form.elements.name.value };
  if (!form.elements.period.disabled) {
    input.period = form.elements.period.value;
  }
  return saveDialog({
    dialog: 'bank-account-dialog',
    error: 'bank-account-error',
    changed,
    save: () =>
      editingAccount
        ? invoke('update_bank_account', { accountId: editingAccount.id, input })
        : invoke('create_bank_account', { input }),
  });
}

function deleteAccount(account) {
  return deleteWithConfirm({
    title: t('Delete {name}?', { name: account.name }),
    message: t('This account and all its entries will be deleted.'),
    confirmLabel: t('Delete account'),
    command: 'delete_bank_account',
    args: { accountId: account.id },
    changed,
  });
}

// ---- Entry dialog ----

const entryForm = createRecordForm({
  form: 'bank-entry-form',
  dialog: 'bank-entry-dialog',
  title: 'bank-entry-dialog-title',
  error: 'bank-entry-error',
  periodLabel: 'bank-entry-period-label',
  copy: 'bank-entry-copy',
  parentField: 'account_id',
  parents: () => accounts,
  valueFields: ['balance', 'note'],
  focusField: 'balance',
  titleText: (entry) => (entry ? t('Edit entry {period}', { period: entry.period }) : t('Add entry')),
});

function submitEntry() {
  const form = $('bank-entry-form');
  const args = {
    accountId: form.elements.account_id.value,
    period: form.elements.period.value,
    balance: form.elements.balance.value,
    note: form.elements.note.value,
  };
  const editing = entryForm.editing;
  return saveDialog({
    dialog: 'bank-entry-dialog',
    error: 'bank-entry-error',
    changed,
    save: () =>
      editing
        ? // The entry is found by its current period; the field holds the new one (CSD-009 REQ-12).
          invoke('update_bank_entry', { ...args, period: editing.period, newPeriod: args.period })
        : invoke('add_bank_entry', args),
  });
}

/** Fills the balance from the latest entry of the account chosen in the form (CSD-009 REQ-15). */
async function copyPrevious() {
  const form = $('bank-entry-form');
  try {
    const prefill = await invoke('copy_previous_bank_entry', { accountId: form.elements.account_id.value });
    form.elements.balance.value = prefill.balance ?? '';
    form.elements.note.value = '';
  } catch (error) {
    showError('bank-entry-error', error);
  }
}

function deleteEntry(entry) {
  return deleteWithConfirm({
    title: t('Delete the entry of {period}?', { period: entry.period }),
    message: t('The entry of {period} of {account} and its note will be deleted.', { period: entry.period, account: entry.account_name }),
    confirmLabel: t('Delete entry'),
    command: 'delete_bank_entry',
    args: { accountId: entry.account_id, period: entry.period },
    changed,
  });
}

export function initBankOverview({ onChanged: callback }) {
  onChanged = callback;
  $('bank-account-add').addEventListener('click', () => openAccountDialog(null));
  $('bank-account-add-first').addEventListener('click', () => openAccountDialog(null));
  submitOnce($('bank-account-form'), submitAccount);
  $('bank-entry-add').addEventListener('click', () => entryForm.open());
  $('bank-entry-copy').addEventListener('click', copyPrevious);
  submitOnce($('bank-entry-form'), submitEntry);
  $('bank-entry-form').elements.account_id.addEventListener('change', () => entryForm.apply({ resetPeriod: true }));
}
