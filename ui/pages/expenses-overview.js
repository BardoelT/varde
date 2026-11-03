// Expenses: dated expenses with a category and a note, and yearly totals (CSD-013).
import { invoke } from '../shared/ipc.js';
import { $, h, showError, submitOnce } from '../shared/dom.js';
import { formatAmount, formatDate, noteSummary } from '../shared/format.js';
import { t } from '../shared/i18n.js';
import { openDialog, saveDialog, deleteWithConfirm } from '../shared/crud.js';
import { numCell, recordActions, renderTable, rowHeader } from '../shared/list-rows.js';

let onChanged = async () => {};

/** The category as Rust names it (a fixed list), in the chosen language. */
function categoryName(category) {
  switch (category) {
    case 'Home': return t('Home');
    case 'Vehicle': return t('Vehicle');
    case 'Holiday': return t('Holiday');
    case 'Sport': return t('Sport');
    case 'Study': return t('Study');
    case 'Appliances': return t('Appliances');
    case 'Hobby': return t('Hobby');
    case 'Other': return t('Other');
    default: return category;
  }
}

// ---- Rendering ----

function renderTotals(totals) {
  renderTable({ empty: 'expense-totals-empty', table: 'expense-totals-table', body: 'expense-totals-rows' }, totals, (total) =>
    h('tr', {}, rowHeader(String(total.year)), numCell(formatAmount(total.total)), numCell(formatAmount(total.largest))),
  );
}

function renderExpenses(list) {
  if (list.expenses.length === 0) $('expenses-empty').textContent = t('No expenses yet. Add your first expense.');
  $('expenses-total').textContent = formatAmount(list.total);
  renderTable({ empty: 'expenses-empty', table: 'expenses-table', body: 'expenses-rows' }, list.expenses, (expense) =>
    h(
      'tr',
      {},
      rowHeader(formatDate(expense.date)),
      h('td', { class: 'wrap' }, expense.name),
      h('td', {}, categoryName(expense.category)),
      numCell(formatAmount(expense.amount)),
      h('td', { class: 'note-cell' }, noteSummary(expense.note)),
      recordActions({
        edit: { label: t('Edit {name}', { name: expense.name }), onClick: () => openExpenseDialog(expense) },
        remove: { label: t('Delete {name}', { name: expense.name }), onClick: () => deleteExpense(expense) },
      }),
    ),
  );
}

/** Reloads everything from Rust. */
export async function reloadExpensesOverview() {
  renderExpenses(await invoke('list_expenses'));
  renderTotals(await invoke('yearly_expense_totals'));
}

async function changed() {
  await reloadExpensesOverview();
  await onChanged();
}

// ---- Expense dialog ----

let editingExpense = null;

function openExpenseDialog(expense = null) {
  editingExpense = expense;
  openDialog({
    dialog: 'expense-dialog',
    form: 'expense-form',
    error: 'expense-error',
    title: 'expense-dialog-title',
    titleText: expense ? t('Edit expense') : t('Add expense'),
    focus: 'name',
    fill(form) {
      $('expense-copy').hidden = Boolean(expense);
      if (expense) {
        form.elements.name.value = expense.name;
        form.elements.amount.value = expense.amount;
        form.elements.date.value = formatDate(expense.date);
        form.elements.category.value = expense.category;
        form.elements.note.value = expense.note;
      }
    },
  });
}

function submitExpense() {
  const form = $('expense-form');
  const input = {
    name: form.elements.name.value,
    amount: form.elements.amount.value,
    date: form.elements.date.value,
    category: form.elements.category.value,
    note: form.elements.note.value,
  };
  return saveDialog({
    dialog: 'expense-dialog',
    error: 'expense-error',
    changed,
    save: () =>
      editingExpense
        ? invoke('update_expense', { expenseId: editingExpense.id, input })
        : invoke('add_expense', { input }),
  });
}

/** Fills the name, amount and category from the previous expense; date and note stay empty (CSD-013 REQ-9). */
async function copyPrevious() {
  const form = $('expense-form');
  try {
    const prefill = await invoke('copy_previous_expense');
    if (prefill === null) return;
    form.elements.name.value = prefill.name;
    form.elements.amount.value = prefill.amount;
    form.elements.category.value = prefill.category;
    form.elements.date.value = '';
    form.elements.note.value = '';
  } catch (error) {
    showError('expense-error', error);
  }
}

function deleteExpense(expense) {
  return deleteWithConfirm({
    title: t('Delete {name}?', { name: expense.name }),
    message: t('The expense of {date} and its note will be deleted.', { date: formatDate(expense.date) }),
    confirmLabel: t('Delete expense'),
    command: 'delete_expense',
    args: { expenseId: expense.id },
    changed,
  });
}

export function initExpensesOverview({ onChanged: callback }) {
  onChanged = callback;
  $('expense-add').addEventListener('click', () => openExpenseDialog(null));
  submitOnce($('expense-form'), submitExpense);
  $('expense-copy').addEventListener('click', copyPrevious);
}
