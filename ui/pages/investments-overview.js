// Investments: accounts with a monthly or yearly statement, each a balance, net deposits and a note (CSD-005).
import { invoke } from '../shared/ipc.js';
import { $, h, showError, setHint, submitOnce } from '../shared/dom.js';
import { formatAmount, noteSummary } from '../shared/format.js';
import { t } from '../shared/i18n.js';
import { openDialog, saveDialog, deleteWithConfirm } from '../shared/crud.js';
import { createRecordForm } from '../shared/record-form.js';
import { accountRow, latestAmount, numCell, recordActions, renderTable, rowHeader, showTotal, toggleEmpty } from '../shared/list-rows.js';

let accounts = [];
let statementRows = [];
let onChanged = async () => {};

// ---- Rendering ----

function renderAccounts(overview) {
  accounts = overview.accounts;
  const hasAccounts = accounts.length > 0;
  toggleEmpty('investments-empty', 'investments-content', hasAccounts);
  $('account-add').hidden = !hasAccounts;
  $('account-list').replaceChildren(
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
  showTotal('account-total', 'account-total-value', overview.total, hasAccounts);
}

/** Yearly totals over all accounts, as returned by Rust (CSD-005 REQ-26). */
function renderTotals(totals) {
  renderTable({ empty: 'investment-totals-empty', table: 'investment-totals-table', body: 'investment-totals-rows' }, totals, (total) =>
    h(
      'tr',
      {},
      rowHeader(String(total.year)),
      numCell(formatAmount(total.closing_balance)),
      numCell(formatAmount(total.net_deposits)),
      numCell(formatAmount(total.capital_growth)),
    ),
  );
}

function renderStatements(statements) {
  statementRows = statements;
  renderTable({ empty: 'statements-empty', table: 'statements-table', body: 'statements-rows' }, statements, (s) =>
    h(
      'tr',
      {},
      rowHeader(s.period),
      h('td', { class: 'wrap' }, s.account_name),
      numCell(formatAmount(s.balance)),
      numCell(formatAmount(s.net_deposits)),
      numCell(formatAmount(s.capital_growth)),
      h('td', { class: 'note-cell' }, noteSummary(s.note)),
      recordActions({
        edit: { label: t('Edit {account} {period}', { account: s.account_name, period: s.period }), onClick: () => statementForm.open(s) },
        remove: { label: t('Delete {account} {period}', { account: s.account_name, period: s.period }), onClick: () => deleteStatement(s) },
      }),
    ),
  );
}

/** Reloads everything from Rust. */
export async function reloadInvestmentsOverview() {
  const overview = await invoke('list_investment_accounts');
  renderAccounts(overview);
  if (overview.accounts.length === 0) {
    statementRows = [];
    return;
  }
  const [totals, statements] = await Promise.all([
    invoke('yearly_investment_totals'),
    invoke('list_investment_statements'),
  ]);
  renderTotals(totals);
  renderStatements(statements);
}

async function changed() {
  await reloadInvestmentsOverview();
  await onChanged();
}

// ---- Account dialog ----

let editingAccount = null;

function openAccountDialog(account) {
  editingAccount = account;
  openDialog({
    dialog: 'account-dialog',
    form: 'account-form',
    error: 'account-error',
    title: 'account-dialog-title',
    titleText: account ? t('Edit account') : t('Add account'),
    focus: 'name',
    fill(form) {
      // The statement period is locked once the account has statements (CSD-005 REQ-4).
      const locked = Boolean(account && account.statement_count > 0);
      form.elements.period.disabled = locked;
      setHint('account-period-hint', locked);
      if (account) {
        form.elements.name.value = account.name;
        form.elements.period.value = account.period;
      }
    },
  });
}

function submitAccount() {
  const form = $('account-form');
  const input = { name: form.elements.name.value };
  if (!form.elements.period.disabled) {
    input.period = form.elements.period.value;
  }
  return saveDialog({
    dialog: 'account-dialog',
    error: 'account-error',
    changed,
    save: () =>
      editingAccount
        ? invoke('update_investment_account', { accountId: editingAccount.id, input })
        : invoke('create_investment_account', { input }),
  });
}

function deleteAccount(account) {
  return deleteWithConfirm({
    title: t('Delete {name}?', { name: account.name }),
    message: t('This account and all its statements will be deleted.'),
    confirmLabel: t('Delete account'),
    command: 'delete_investment_account',
    args: { accountId: account.id },
    changed,
  });
}

// ---- Statement dialog ----

/** Which of balance and growth the user typed in last; the other one is calculated. */
let typedField = 'balance';
let calculationRun = 0;

const statementForm = createRecordForm({
  form: 'statement-form',
  dialog: 'statement-dialog',
  title: 'statement-dialog-title',
  error: 'statement-error',
  periodLabel: 'statement-period-label',
  copy: 'statement-copy',
  parentField: 'account_id',
  parents: () => accounts,
  valueFields: ['balance', 'net_deposits', 'note'],
  focusField: 'balance',
  titleText: (statement) => (statement ? t('Edit statement {period}', { period: statement.period }) : t('Add statement')),
  beforeFill() {
    typedField = 'balance';
    calculationRun += 1; // drop any calculation still under way
    markCalculated(null);
  },
  afterFill: () => recalculate(),
});

// ---- Balance, net deposits and capital growth (CSD-005 REQ-24, REQ-25) ----

function periodLooksComplete(period) {
  return (statementForm.isYearly() ? /^\d{4}$/ : /^\d{4}-\d{2}$/).test(period);
}

/**
 * Shows the note while the account has no statement before the chosen period:
 * it is taken to start at 0.00 (CSD-005 REQ-27). The note is only a hint;
 * Rust calculates every figure.
 */
function updateEarlierNote() {
  const form = $('statement-form');
  const period = form.elements.period.value.trim();
  const accountId = form.elements.account_id.value;
  // The statement being edited never counts as an earlier one (CSD-005 REQ-24).
  const others = statementRows.filter(
    (s) => s.account_id === accountId && s.period !== period && s.period !== statementForm.editing?.period,
  );
  const hasEarlier = periodLooksComplete(period)
    ? others.some((s) => s.period < period)
    : others.length > 0;
  setHint('statement-growth-hint', !hasEarlier);
}

function markCalculated(field) {
  $('balance-calculated').hidden = field !== 'balance';
  $('growth-calculated').hidden = field !== 'growth';
}

/** Asks Rust for the balance or growth that follows from the other one and the net deposits. */
async function recalculate() {
  const form = $('statement-form');
  const run = ++calculationRun;
  updateEarlierNote();
  const source = typedField;
  const target = source === 'balance' ? 'growth' : 'balance';
  const value = form.elements[source].value;
  $('statement-error').hidden = true;
  if (!periodLooksComplete(form.elements.period.value.trim()) || value.trim() === '') {
    form.elements[target].value = '';
    markCalculated(null);
    return;
  }
  try {
    const result = await invoke('calculate_investment_statement', {
      accountId: form.elements.account_id.value,
      period: form.elements.period.value.trim(),
      netDeposits: form.elements.net_deposits.value,
      input: source,
      value,
      ignorePeriod: statementForm.editing?.period,
    });
    if (run !== calculationRun) return; // a newer calculation is under way
    if (target === 'growth') {
      form.elements.growth.value = result.capital_growth;
    } else {
      form.elements.balance.value = result.balance;
    }
    markCalculated(target);
  } catch (error) {
    if (run !== calculationRun) return;
    form.elements[target].value = '';
    markCalculated(null);
    showError('statement-error', error);
  }
}

/** The balance or the growth was typed in: it becomes the input and the other is calculated. */
function typedIn(field) {
  typedField = field;
  recalculate();
}

function submitStatement() {
  const form = $('statement-form');
  const args = {
    accountId: form.elements.account_id.value,
    period: form.elements.period.value,
    balance: form.elements.balance.value,
    netDeposits: form.elements.net_deposits.value,
    note: form.elements.note.value,
  };
  const editing = statementForm.editing;
  return saveDialog({
    dialog: 'statement-dialog',
    error: 'statement-error',
    changed,
    save: () =>
      editing
        ? // The statement is found by its current period; the field holds the new one (CSD-005 REQ-28).
          invoke('update_investment_statement', { ...args, period: editing.period, newPeriod: args.period })
        : invoke('add_investment_statement', args),
  });
}

/** Fills the balance from the latest statement of the account chosen in the form (CSD-005 REQ-13). */
async function copyPrevious() {
  const form = $('statement-form');
  try {
    const prefill = await invoke('copy_previous_investment_statement', {
      accountId: form.elements.account_id.value,
    });
    form.elements.balance.value = prefill.balance ?? '';
    form.elements.net_deposits.value = prefill.net_deposits;
    form.elements.note.value = '';
    typedIn('balance');
  } catch (error) {
    showError('statement-error', error);
  }
}

function deleteStatement(statement) {
  return deleteWithConfirm({
    title: t('Delete the statement of {period}?', { period: statement.period }),
    message: t('The statement of {period} of {account} and its note will be deleted.', { period: statement.period, account: statement.account_name }),
    confirmLabel: t('Delete statement'),
    command: 'delete_investment_statement',
    args: { accountId: statement.account_id, period: statement.period },
    changed,
  });
}

export function initInvestmentsOverview({ onChanged: callback }) {
  onChanged = callback;
  $('account-add').addEventListener('click', () => openAccountDialog(null));
  $('account-add-first').addEventListener('click', () => openAccountDialog(null));
  submitOnce($('account-form'), submitAccount);
  $('statement-add').addEventListener('click', () => statementForm.open());
  $('statement-copy').addEventListener('click', copyPrevious);
  submitOnce($('statement-form'), submitStatement);
  const form = $('statement-form');
  form.elements.account_id.addEventListener('change', () => {
    statementForm.apply({ resetPeriod: true });
    recalculate();
  });
  form.elements.period.addEventListener('input', recalculate);
  form.elements.balance.addEventListener('input', () => typedIn('balance'));
  form.elements.growth.addEventListener('input', () => typedIn('growth'));
  form.elements.net_deposits.addEventListener('input', recalculate);
}
