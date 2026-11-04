// Debts: debts with a monthly or yearly statement, each a remaining balance and a note (CSD-011).
import { invoke } from '../shared/ipc.js';
import { $, h, showError, setHint, submitOnce } from '../shared/dom.js';
import { formatAmount, noteSummary } from '../shared/format.js';
import { askSettlements } from '../shared/settle.js';
import { t } from '../shared/i18n.js';
import { openDialog, saveDialog, deleteWithConfirm } from '../shared/crud.js';
import { createRecordForm } from '../shared/record-form.js';
import { accountRow, latestAmount, numCell, recordActions, renderTable, rowHeader, showTotal, toggleEmpty } from '../shared/list-rows.js';

let debts = [];
let houses = [];
let onChanged = async () => {};

/** An amount that Rust may leave out (a change or a repaid amount of none): a dash then. */
function optionalAmount(value) {
  return value === null ? '—' : formatAmount(value);
}

// ---- Rendering ----

/** The settlement badge of a settled debt, else Repaid while the latest balance is 0.00 (CSD-011 REQ-26). */
function debtBadge(debt) {
  if (debt.settlement) return h('span', { class: 'badge' }, `${t(debt.settlement.outcome)} ${formatAmount(debt.settlement.amount)}`);
  return debt.repaid ? h('span', { class: 'badge' }, t('Repaid')) : '';
}

function renderDebts(overview) {
  debts = overview.debts;
  const hasDebts = debts.length > 0;
  toggleEmpty('debts-empty', 'debts-content', hasDebts);
  $('debt-list').replaceChildren(
    ...debts.map((debt) =>
      accountRow({
        name: debt.name,
        meta: [`${t(debt.category)} · ${t(debt.period)}${debt.house_name ? ` · ${debt.house_name}` : ''} `, debtBadge(debt)],
        amount: latestAmount(debt.latest_balance, debt.latest_period),
        edit: { label: t('Edit {name}', { name: debt.name }), onClick: () => openDebtDialog(debt) },
        remove: { label: t('Delete {name}', { name: debt.name }), onClick: () => deleteDebt(debt) },
      }),
    ),
  );
  showTotal('debt-total', 'debt-total-value', overview.total, hasDebts);
}

/** Yearly totals over all debts, as returned by Rust (CSD-011 REQ-19). */
function renderTotals(totals) {
  renderTable({ empty: 'debt-totals-empty', table: 'debt-totals-table', body: 'debt-totals-rows' }, totals, (total) =>
    h(
      'tr',
      {},
      rowHeader(String(total.year)),
      numCell(formatAmount(total.closing_balance)),
      numCell(optionalAmount(total.repaid)),
    ),
  );
}

function renderStatements(statements) {
  renderTable({ empty: 'debt-statements-empty', table: 'debt-statements-table', body: 'debt-statements-rows' }, statements, (s) =>
    h(
      'tr',
      {},
      rowHeader(s.period),
      h('td', { class: 'wrap' }, s.debt_name),
      numCell(formatAmount(s.balance)),
      numCell(optionalAmount(s.change)),
      h(
        'td',
        { class: 'note-cell' },
        noteSummary(s.note),
        s.settlement && s.outcome ? h('span', { class: 'badge' }, t(s.outcome)) : '',
      ),
      // A statement of a settled debt, from its settlement period on, cannot change (CSD-026 REQ-14).
      recordActions({
        locked: s.locked,
        edit: { label: t('Edit {name} {period}', { name: s.debt_name, period: s.period }), onClick: () => statementForm.open(s) },
        remove: { label: t('Delete {name} {period}', { name: s.debt_name, period: s.period }), onClick: () => deleteStatement(s) },
      }),
    ),
  );
}

/** Reloads everything from Rust. */
export async function reloadDebtsOverview() {
  const [overview, houseOverview] = await Promise.all([invoke('list_debts'), invoke('list_houses')]);
  houses = houseOverview.houses;
  renderDebts(overview);
  if (overview.debts.length === 0) return;
  const [totals, statements] = await Promise.all([invoke('yearly_debt_totals'), invoke('list_debt_statements')]);
  renderTotals(totals);
  renderStatements(statements);
}

async function changed() {
  await reloadDebtsOverview();
  await onChanged();
}

// ---- Debt dialog ----

let editingDebt = null;

/** The House dropdown: no house and all houses, a sold one marked (CSD-020 REQ-30). */
function fillHouseChoices(debt) {
  const form = $('debt-form');
  const select = form.elements.house_id;
  select.replaceChildren(
    h('option', { value: '' }, t('No house')),
    ...houses.map((house) => h('option', { value: house.id }, house.held ? house.name : t('{name} (sold)', { name: house.name }))),
  );
  select.value = debt?.house_id ?? '';
  // The link of a settled debt, or of a debt whose house is sold, cannot change (CSD-020 REQ-16, CSD-026 REQ-6).
  const locked = Boolean(debt && (debt.settlement || (debt.house_id && !houses.find((house) => house.id === debt.house_id)?.held)));
  select.disabled = locked;
  form.elements.category.disabled = locked && Boolean(debt.house_id);
  setHint('debt-house-hint', locked);
  applyCategory();
}

/** The House dropdown shows only for a Mortgage or a Loan (CSD-020 REQ-30). */
function applyCategory() {
  const form = $('debt-form');
  const linkable = ['Mortgage', 'Loan'].includes(form.elements.category.value);
  $('debt-house-field').hidden = !linkable;
  if (!linkable) form.elements.house_id.value = '';
}

function openDebtDialog(debt) {
  editingDebt = debt;
  openDialog({
    dialog: 'debt-dialog',
    form: 'debt-form',
    error: 'debt-error',
    title: 'debt-dialog-title',
    titleText: debt ? t('Edit debt') : t('Add debt'),
    focus: 'name',
    fill(form) {
      // The statement period is locked once the debt has statements (CSD-011 REQ-5).
      const locked = Boolean(debt && debt.statement_count > 0);
      form.elements.period.disabled = locked;
      setHint('debt-period-hint', locked);
      if (debt) {
        form.elements.name.value = debt.name;
        form.elements.period.value = debt.period;
        form.elements.category.value = debt.category;
      }
      fillHouseChoices(debt);
    },
  });
}

function submitDebt() {
  const form = $('debt-form');
  const category = form.elements.category.value;
  const input = {
    name: form.elements.name.value,
    category,
    house_id: ['Mortgage', 'Loan'].includes(category) ? form.elements.house_id.value : '',
  };
  if (!form.elements.period.disabled) {
    input.period = form.elements.period.value;
  }
  return saveDialog({
    dialog: 'debt-dialog',
    error: 'debt-error',
    changed,
    async save() {
      // Linking to a house that is already sold settles the debt: Repaid or Cancelled (CSD-020 REQ-17).
      const chosen = houses.find((house) => house.id === input.house_id);
      if (editingDebt && chosen && !chosen.held && editingDebt.house_id !== chosen.id) {
        const open = await invoke('debt_open_balance', { debtId: editingDebt.id, houseId: chosen.id });
        if (open) {
          const choices = await askSettlements([open]);
          if (choices === null) return false;
          input.settlement = choices[0].outcome;
        }
      }
      return editingDebt
        ? invoke('update_debt', { debtId: editingDebt.id, input })
        : invoke('create_debt', { input });
    },
  });
}

function deleteDebt(debt) {
  return deleteWithConfirm({
    title: t('Delete {name}?', { name: debt.name }),
    message: t('This debt and all its statements will be deleted.'),
    confirmLabel: t('Delete debt'),
    command: 'delete_debt',
    args: { debtId: debt.id },
    changed,
  });
}

// ---- Statement dialog ----

const statementForm = createRecordForm({
  form: 'debt-statement-form',
  dialog: 'debt-statement-dialog',
  title: 'debt-statement-dialog-title',
  error: 'debt-statement-error',
  periodLabel: 'debt-statement-period-label',
  copy: 'debt-statement-copy',
  parentField: 'debt_id',
  parents: () => debts,
  valueFields: ['balance', 'note'],
  focusField: 'balance',
  titleText: (statement) => (statement ? t('Edit statement {period}', { period: statement.period }) : t('Add statement')),
});

function submitStatement() {
  const form = $('debt-statement-form');
  const args = {
    debtId: form.elements.debt_id.value,
    period: form.elements.period.value,
    balance: form.elements.balance.value,
    note: form.elements.note.value,
  };
  const editing = statementForm.editing;
  return saveDialog({
    dialog: 'debt-statement-dialog',
    error: 'debt-statement-error',
    changed,
    save: () =>
      editing
        ? // The statement is found by its current period; the field holds the new one (CSD-011 REQ-12).
          invoke('update_debt_statement', { ...args, period: editing.period, newPeriod: args.period })
        : invoke('add_debt_statement', args),
  });
}

/** Fills the balance from the latest statement of the debt chosen in the form (CSD-011 REQ-15). */
async function copyPrevious() {
  const form = $('debt-statement-form');
  try {
    const prefill = await invoke('copy_previous_debt_statement', { debtId: form.elements.debt_id.value });
    form.elements.balance.value = prefill.balance ?? '';
    form.elements.note.value = '';
  } catch (error) {
    showError('debt-statement-error', error);
  }
}

function deleteStatement(statement) {
  return deleteWithConfirm({
    title: t('Delete the statement of {period}?', { period: statement.period }),
    message: t('The statement of {period} of {name} and its note will be deleted.', { period: statement.period, name: statement.debt_name }),
    confirmLabel: t('Delete statement'),
    command: 'delete_debt_statement',
    args: { debtId: statement.debt_id, period: statement.period },
    changed,
  });
}

export function initDebtsOverview({ onChanged: callback }) {
  onChanged = callback;
  $('debt-add').addEventListener('click', () => openDebtDialog(null));
  $('debt-add-first').addEventListener('click', () => openDebtDialog(null));
  submitOnce($('debt-form'), submitDebt);
  $('debt-form').elements.category.addEventListener('change', applyCategory);
  $('debt-statement-add').addEventListener('click', () => statementForm.open());
  $('debt-statement-copy').addEventListener('click', copyPrevious);
  submitOnce($('debt-statement-form'), submitStatement);
  $('debt-statement-form').elements.debt_id.addEventListener('change', () => statementForm.apply({ resetPeriod: true }));
}
