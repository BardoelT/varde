// The Budgets tab of Transactions (CSD-036): one month at a time, a row per category with its budget, what
// was spent, what remains and a bar, and the dialog that sets or removes a budget for this month only, from
// this month on, or for all months. Rust holds every budget and computes every figure, including the bar and
// which month is the earliest; this file only lays out what Rust returned.
import { invoke } from '../shared/ipc.js';
import { $, h, iconButton, submitOnce, showToast } from '../shared/dom.js';
import { errorText } from '../shared/ipc.js';
import { formatAmount, amountForInput } from '../shared/format.js';
import { t } from '../shared/i18n.js';
import { openDialog, saveDialog } from '../shared/crud.js';
import { isNegative } from '../shared/yearly-chart.js';

/** The month shown (`YYYY-MM`), or `null` until Rust has named the current month for this project. */
let shownMonth = null;
/** What Rust returned for the month shown. */
let view = null;
let onChanged = async () => {};

/** `YYYY-MM` moved by `step` months: only the way to the neighbouring month, no figure. */
function shiftMonth(month, step) {
  const [year, number] = month.split('-').map(Number);
  const index = year * 12 + (number - 1) + step;
  return `${Math.floor(index / 12)}-${String((index % 12) + 1).padStart(2, '0')}`;
}

/** "2026-10" or, for the current month, "2026-10 MTD" (CSD-036 REQ-2). */
const monthLabel = (data) => (data.is_partial ? t('{month} MTD', { month: data.month }) : data.month);

const amountCell = (amount, { negative = false } = {}) => h('td', { class: `num${negative ? ' negative' : ''}` }, formatAmount(amount));

/** The bar of a row: as full as Rust says, marked when the category is over its budget. */
function barOf(row) {
  if (row.bar_percent === null) return h('td', { class: 'budget-bar-cell' });
  const label = row.over ? t('Over budget') : t('{percent} of the budget spent', { percent: `${row.bar_percent}%` });
  const fill = h('span', { class: 'budget-bar-fill' });
  fill.style.width = `${row.bar_percent}%`;
  return h('td', { class: 'budget-bar-cell' }, h('div', { class: `budget-bar${row.over ? ' over' : ''}`, role: 'img', 'aria-label': label }, fill));
}

function budgetCell(row) {
  if (row.budget === null) {
    return h('td', { class: 'num' }, h('button', { type: 'button', class: 'btn', onclick: () => openBudgetDialog(row, 'set') }, t('Set budget')));
  }
  return h('td', { class: 'num' }, formatAmount(row.budget), row.single_month ? h('span', { class: 'single-month-label' }, t('This month only')) : null);
}

function actionsCell(row) {
  if (row.budget === null) return h('td', { class: 'row-actions' });
  return h(
    'td',
    { class: 'row-actions' },
    iconButton('edit', t('Edit budget of {name}', { name: row.name }), () => openBudgetDialog(row, 'set')),
    iconButton('delete', t('Remove budget of {name}', { name: row.name }), () => openBudgetDialog(row, 'remove')),
  );
}

function rowOf(row) {
  return h(
    'tr',
    {},
    h('th', { scope: 'row' }, row.name),
    budgetCell(row),
    amountCell(row.spent),
    row.remaining === null ? h('td', { class: 'num' }) : amountCell(row.remaining, { negative: isNegative(row.remaining) }),
    barOf(row),
    actionsCell(row),
  );
}

function render(data) {
  view = data;
  shownMonth = data.month;
  $('budgets-month').textContent = monthLabel(data);
  $('budgets-previous').disabled = data.month <= data.earliest_month;
  const hasCategories = data.rows.length > 0;
  $('budgets-empty').hidden = hasCategories;
  $('budgets-content').hidden = !hasCategories;
  const notBudgeted = h('tr', {}, h('th', { scope: 'row' }, t('Not budgeted')), h('td', { class: 'num' }), amountCell(data.not_budgeted), h('td', { class: 'num' }), h('td'), h('td'));
  const total = h(
    'tr',
    { class: 'budget-total-row' },
    h('th', { scope: 'row' }, t('Total')),
    amountCell(data.total.budget),
    amountCell(data.total.spent),
    amountCell(data.total.remaining, { negative: isNegative(data.total.remaining) }),
    h('td'),
    h('td'),
  );
  $('budgets-rows').replaceChildren(...data.rows.map(rowOf), notBudgeted, total);
}

/** Asks Rust for `month` (the current month when `null`) and draws the answer. */
async function load(month) {
  render(await invoke('get_budgets', month === null ? {} : { month }));
}

/** Draws the month again; used when the tab opens and after a project, the language or a format changed. */
export const reloadBudgets = () => load(shownMonth);

/** Reloads only while the tab is showing (a project was opened, the language changed). */
export async function reloadBudgetsIfOpen() {
  if (!$('transactions-budgets').hidden) await reloadBudgets();
}

/** A new project starts on the current month again (CSD-036 REQ-2). */
export function resetBudgets() {
  shownMonth = null;
  view = null;
}

let editing = null;

function openBudgetDialog(row, mode) {
  editing = { row, mode };
  const setting = mode === 'set';
  openDialog({
    dialog: 'budget-dialog',
    form: 'budget-form',
    error: 'budget-error',
    title: 'budget-dialog-title',
    titleText: setting ? t('Set budget') : t('Remove budget'),
    focus: setting ? 'amount' : 'save',
    fill(form) {
      $('budget-amount-field').hidden = !setting;
      $('budget-context').textContent = t('{name}, {month}', { name: row.name, month: view.month });
      $('budget-submit').textContent = setting ? t('Save budget') : t('Remove budget');
      if (setting && row.budget !== null) form.elements.amount.value = amountForInput(row.budget);
    },
  });
}

function submitBudget() {
  const form = $('budget-form');
  const { row, mode } = editing;
  const scope = form.elements.scope.value;
  const month = view.month;
  return saveDialog({
    dialog: 'budget-dialog',
    error: 'budget-error',
    changed: onChanged,
    save: async () => {
      render(
        mode === 'set'
          ? await invoke('set_budget', { categoryId: row.category, month, amount: form.elements.amount.value, scope })
          : await invoke('remove_budget', { categoryId: row.category, month, scope }),
      );
    },
  });
}

async function move(step) {
  try {
    await load(shiftMonth(view.month, step));
  } catch (error) {
    showToast(errorText(error));
  }
}

/** `onChanged()` refreshes what shows budgets data elsewhere: the unsaved dot and the currency confirmation. */
export function initTransactionsBudgets(options) {
  onChanged = options.onChanged;
  $('budgets-previous').addEventListener('click', () => move(-1));
  $('budgets-next').addEventListener('click', () => move(1));
  submitOnce($('budget-form'), submitBudget);
}
