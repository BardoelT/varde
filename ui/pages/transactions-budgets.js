// The Budgets tab of Transactions (CSD-037): the Month view (one month at a time, a row per category with its
// budget, what was spent, what remains and a bar) and the Year view (one year at a time, a grid of the budget
// of every category in each month), and the dialog that sets or removes a budget for this month only, from
// this month on, or for all months. Rust holds every budget and computes every figure, including the bar and
// which month or year is the earliest; this file only lays out what Rust returned.
import { invoke } from '../shared/ipc.js';
import { $, h, iconButton, submitOnce, showToast } from '../shared/dom.js';
import { errorText } from '../shared/ipc.js';
import { formatAmount, amountForInput } from '../shared/format.js';
import { t } from '../shared/i18n.js';
import { openDialog, saveDialog } from '../shared/crud.js';
import { isNegative } from '../shared/yearly-chart.js';
import { attachDatePicker } from '../shared/date-picker.js';

/** `month` or `year`: the view the tab shows. It opens on the Month view and does not remember the choice (CSD-037 REQ-18). */
let mode = 'month';
/** The month shown (`YYYY-MM`), or `null` until Rust has named the current month for this project. */
let shownMonth = null;
/** The year shown (`YYYY`), or `null` until the Year view is first chosen. */
let shownYear = null;
/** What Rust returned for the month shown. */
let view = null;
/** What Rust returned for the year shown. */
let yearView = null;
let onChanged = async () => {};

/** `YYYY-MM` moved by `step` months: only the way to the neighbouring month, no figure. */
function shiftMonth(month, step) {
  const [year, number] = month.split('-').map(Number);
  const index = year * 12 + (number - 1) + step;
  return `${Math.floor(index / 12)}-${String((index % 12) + 1).padStart(2, '0')}`;
}

/** "2026-10" or, for the current month, "2026-10 MTD" (CSD-037 REQ-2). */
const monthLabel = (data) => (data.is_partial ? t('{month} MTD', { month: data.month }) : data.month);

/** The short month names, January first, in the chosen language. */
const monthNames = () => [t('Jan'), t('Feb'), t('Mar'), t('Apr'), t('May'), t('Jun'), t('Jul'), t('Aug'), t('Sep'), t('Oct'), t('Nov'), t('Dec')];

const amountCell = (amount, { negative = false } = {}) => h('td', { class: `num${negative ? ' negative' : ''}` }, formatAmount(amount));

/** The bar of a row: as full as Rust says, marked when the category is over its budget. */
function barOf(row) {
  if (row.bar_percent === null) return h('td', { class: 'budget-bar-cell' });
  const label = row.over ? t('Over budget') : t('{percent} of the budget spent', { percent: `${row.bar_percent}%` });
  const fill = h('span', { class: 'budget-bar-fill' });
  fill.style.width = `${row.bar_percent}%`;
  return h('td', { class: 'budget-bar-cell' }, h('div', { class: `budget-bar${row.over ? ' over' : ''}`, role: 'img', 'aria-label': label }, fill));
}

/** A row of the Month view as the dialog's target: the category, its budget and the month shown. */
const targetOf = (row) => ({ category: row.category, name: row.name, month: view.month, budget: row.budget });

function budgetCell(row) {
  if (row.budget === null) {
    return h('td', { class: 'num' }, h('button', { type: 'button', class: 'btn', onclick: () => openBudgetDialog(targetOf(row), 'set') }, t('Set budget')));
  }
  return h('td', { class: 'num' }, formatAmount(row.budget), row.single_month ? h('span', { class: 'single-month-label' }, t('This month only')) : null);
}

function actionsCell(row) {
  if (row.budget === null) return h('td', { class: 'row-actions' });
  return h(
    'td',
    { class: 'row-actions' },
    iconButton('edit', t('Edit budget of {name}', { name: row.name }), () => openBudgetDialog(targetOf(row), 'set')),
    iconButton('delete', t('Remove budget of {name}', { name: row.name }), () => openBudgetDialog(targetOf(row), 'remove')),
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

/** Shows the toggle, the buttons and which of the two views is visible; the views fill their own parts. */
function showMode() {
  const year = mode === 'year';
  $('budgets-view-month').setAttribute('aria-pressed', String(!year));
  $('budgets-view-year').setAttribute('aria-pressed', String(year));
  $('budgets-previous').textContent = year ? t('Previous year') : t('Previous month');
  $('budgets-next').textContent = year ? t('Next year') : t('Next month');
}

function render(data) {
  view = data;
  shownMonth = data.month;
  if (mode !== 'month') return;
  showMode();
  $('budgets-year').hidden = true;
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

/** "2026" or, for the current year, "2026 YTD" (CSD-037 REQ-19). */
const yearLabel = (data) => (data.is_partial ? t('{year} YTD', { year: data.year }) : String(data.year));

/** One amount of the grid: the budget in a button that opens the dialog for its month, or a dash. */
function cellOf(row, cell, index, year) {
  const month = `${year}-${String(index + 1).padStart(2, '0')}`;
  const names = monthNames();
  const shown = cell.budget === null ? '–' : formatAmount(cell.budget);
  const label = `${row.name}, ${names[index]}, ${cell.budget === null ? t('no budget') : formatAmount(cell.budget)}${cell.single_month ? `, ${t('This month only')}` : ''}`;
  const button = h('button', { type: 'button', class: 'cell-button', 'aria-label': label, onclick: () => openBudgetDialog({ category: row.category, name: row.name, month, budget: cell.budget }, 'set', { canRemove: cell.budget !== null }) }, shown);
  return h('td', { class: `num${cell.budget === null ? ' empty' : ''}${cell.single_month ? ' single-month-cell' : ''}`, title: cell.single_month ? t('This month only') : null }, button);
}

/** A total cell: a dash when there is no budget behind it. */
const totalCell = (amount, extra = '') => h('td', { class: `num ${amount === null ? 'empty ' : ''}${extra}`.trim() }, amount === null ? '–' : formatAmount(amount));

/** A labelled figure above the grid, styled like the figures of a year on the Yearly totals tab. */
const figure = (label, amount) => h('span', { class: `year-figure${isNegative(amount) ? ' negative' : ''}` }, h('span', { class: 'year-figure-label' }, label), h('span', { class: 'year-figure-value' }, formatAmount(amount)));

function renderYear(data) {
  yearView = data;
  shownYear = String(data.year);
  if (mode !== 'year') return;
  showMode();
  $('budgets-content').hidden = true;
  $('budgets-month').textContent = yearLabel(data);
  $('budgets-previous').disabled = data.year <= data.earliest_year;
  const hasCategories = data.rows.length > 0;
  $('budgets-empty').hidden = hasCategories;
  $('budgets-year').hidden = !hasCategories;
  $('budgets-year-figures').replaceChildren(figure(t('Budget'), data.figures.budget), figure(t('Spent'), data.figures.spent), figure(t('Remaining'), data.figures.remaining));
  const head = h('tr', {}, h('th', { scope: 'col' }, t('Category')), monthNames().map((name) => h('th', { scope: 'col', class: 'num' }, name)), h('th', { scope: 'col', class: 'num' }, t('Total')));
  const rows = data.rows.map((row) => h('tr', {}, h('th', { scope: 'row' }, row.name), row.months.map((cell, index) => cellOf(row, cell, index, data.year)), totalCell(row.total, 'total-cell')));
  const totals = h('tr', { class: 'total-row' }, h('th', { scope: 'row' }, t('Total')), data.months.map((amount) => totalCell(amount)), totalCell(data.total, 'total-cell'));
  $('budgets-year-grid').replaceChildren(h('table', { class: 'data-table year-grid' }, h('thead', {}, head), h('tbody', {}, rows, totals)));
}

/** Asks Rust for `month` (the current month when `null`) and draws the answer. */
async function load(month) {
  render(await invoke('get_budgets', month === null ? {} : { month }));
}

/** Asks Rust for `year` (the current year when `null`) and draws the answer. */
async function loadYear(year) {
  renderYear(await invoke('get_budget_year', year === null ? {} : { year }));
}

/** Draws the view that is shown again; used when the tab opens and after a project, the language or a format changed. */
export const reloadBudgets = () => (mode === 'year' ? loadYear(shownYear) : load(shownMonth));

/** Reloads only while the tab is showing (a project was opened, the language changed). */
export async function reloadBudgetsIfOpen() {
  if (!$('transactions-budgets').hidden) await reloadBudgets();
}

/** A new project starts on the current month again (CSD-037 REQ-2). */
export function resetBudgets() {
  mode = 'month';
  shownMonth = null;
  shownYear = null;
  view = null;
  yearView = null;
}

/** What the dialog is for: `target` is `{ category, name, month, budget }` and `kind` is `set` or `remove`. */
let editing = null;

/** Shows the dialog as the form that sets a budget or the one that removes it (CSD-037 REQ-6, REQ-7, REQ-21). */
function showKind(kind, canRemove) {
  const setting = kind === 'set';
  editing.kind = kind;
  $('budget-amount-field').hidden = !setting;
  $('budget-remove').hidden = !(setting && canRemove);
  $('budget-dialog-title').textContent = setting ? t('Set budget') : t('Remove budget');
  $('budget-submit').textContent = setting ? t('Save budget') : t('Remove budget');
}

function openBudgetDialog(target, kind, { canRemove = false } = {}) {
  editing = { target, kind };
  openDialog({
    dialog: 'budget-dialog',
    form: 'budget-form',
    error: 'budget-error',
    title: 'budget-dialog-title',
    titleText: kind === 'set' ? t('Set budget') : t('Remove budget'),
    focus: kind === 'set' ? 'amount' : 'save',
    fill(form) {
      showKind(kind, canRemove);
      $('budget-context').textContent = t('{name}, {month}', { name: target.name, month: target.month });
      if (kind === 'set' && target.budget !== null) form.elements.amount.value = amountForInput(target.budget);
    },
  });
}

function submitBudget() {
  const form = $('budget-form');
  const { target, kind } = editing;
  const scope = form.elements.scope.value;
  const args = { categoryId: target.category, month: target.month, scope };
  return saveDialog({
    dialog: 'budget-dialog',
    error: 'budget-error',
    changed: onChanged,
    save: async () => {
      const answer = kind === 'set' ? await invoke('set_budget', { ...args, amount: form.elements.amount.value }) : await invoke('remove_budget', args);
      // The answer is the month of the change: the Month view shows it, the Year view asks for its year again.
      if (mode === 'year') await loadYear(shownYear);
      else render(answer);
    },
  });
}

/** The Month view and the Year view share the buttons: they move the month or the year shown. */
async function move(step) {
  try {
    if (mode === 'year') await loadYear(String(Number(yearView.year) + step));
    else await load(shiftMonth(view.month, step));
  } catch (error) {
    showToast(errorText(error));
  }
}

/** Chooses the Month or the Year view: Year opens on the year of the month shown, Month on the month it was on (REQ-18). */
async function choose(next) {
  if (mode === next) return;
  mode = next;
  showMode();
  try {
    if (next === 'year') await loadYear(view.month.slice(0, 4));
    else await load(shownMonth);
  } catch (error) {
    showToast(errorText(error));
  }
}

/** `onChanged()` refreshes what shows budgets data elsewhere: the unsaved dot and the currency confirmation. */
export function initTransactionsBudgets(options) {
  onChanged = options.onChanged;
  // The label of the month or year opens the date picker: periods before the earliest one are grey, none after it
  // (CSD-037 REQ-2, REQ-19; CSD-036 REQ-12).
  const label = $('budgets-month');
  label.setAttribute('role', 'button');
  label.tabIndex = 0;
  attachDatePicker(label, {
    kind: () => (mode === 'year' ? 'year' : 'month'),
    offers: () => (mode === 'year' ? { from: String(yearView?.earliest_year ?? shownYear) } : { from: view?.earliest_month ?? shownMonth }),
    current: () => (mode === 'year' ? String(yearView?.year ?? shownYear ?? '') : (view?.month ?? shownMonth ?? '')),
    async onPick(text) {
      try {
        if (mode === 'year') await loadYear(text);
        else await load(text);
      } catch (error) {
        showToast(errorText(error));
      }
    },
  });
  $('budgets-previous').addEventListener('click', () => move(-1));
  $('budgets-next').addEventListener('click', () => move(1));
  $('budgets-view-month').addEventListener('click', () => choose('month'));
  $('budgets-view-year').addEventListener('click', () => choose('year'));
  $('budget-remove').addEventListener('click', () => showKind('remove', false));
  submitOnce($('budget-form'), submitBudget);
}
