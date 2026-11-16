// Transactions: bank transactions imported from a Knab export (the Overview tab), and their categories and
// the rules that categorise them (the Categories and Rules tabs, CSD-032). Rust parses, categorises, filters, orders, cuts and totals; this file only
// shows what Rust returns and sends what the user chose.
import { invoke, errorText } from '../shared/ipc.js';
import { $, h, iconButton, submitOnce, showToast, confirmAction } from '../shared/dom.js';
import { formatAmount, formatDate, formatLabel } from '../shared/format.js';
import { t, tn } from '../shared/i18n.js';
import { initTransactionSuggest } from './transactions-suggest.js';
import { createStatementFilter, renderFilteredTable } from '../shared/statement-filter.js';
import { makeSortable } from '../shared/sortable.js';
import { openDialog, saveDialog, deleteWithConfirm } from '../shared/crud.js';
import { numCell, renderTable, rowHeader } from '../shared/list-rows.js';

/** How many rows one step of "Load more" adds, and the rows shown first (CSD-032 REQ-15). */
const PAGE_ROWS = 200;
/** The name the frontend and Rust give the built-in category (CSD-032 section 5). */
const UNCATEGORIZED = 'uncategorized';

let onChanged = async () => {};
let categories = [];
let rules = [];
let limit = PAGE_ROWS;

/** Whether the list shows only the transactions whose category was set by hand (CSD-032 REQ-26). */
const manualOnly = () => $('transaction-manual-only').checked;

/** Back to the whole list, when another project is opened. */
export function resetTransactionsManualFilter() {
  $('transaction-manual-only').checked = false;
  endBulk();
}

/** Bulk select is switched on by the user; then every row has a checkbox (CSD-032 REQ-28). */
let bulk = false;
/** The ids of the selected transactions: always some of those the filters show. */
const selected = new Set();
/** How many transactions the filters show, as Rust counted them, for the checkbox of the header. */
let shownCount = 0;

/** The arguments that say which transactions the filters show, as `list_transactions` takes them. */
function filterArgs() {
  const filter = transactionsFilter.payload();
  const args = { ...(filter ? { filter } : {}), ...(manualOnly() ? { manualOnly: true } : {}) };
  return Object.keys(args).length > 0 ? args : undefined;
}

let selectHeadCell = null;

/**
 * The header cell with the checkbox that selects all. It is in the header only while bulk select is on: a
 * hidden cell would still be the first child, and the date column would lose its left padding.
 */
function selectHead() {
  if (!selectHeadCell) {
    const box = h('input', { type: 'checkbox', id: 'transactions-select-all' });
    box.addEventListener('change', selectAll);
    selectHeadCell = h('th', { scope: 'col', class: 'select-col', id: 'transactions-select-head' }, box);
  }
  return selectHeadCell;
}

/** Shows how many are selected, and the state of the checkbox of the header. */
function syncBulk() {
  $('transactions-bulk').hidden = !bulk;
  if (bulk) {
    if (!selectHead().isConnected) $('transactions-table').tHead.rows[0].prepend(selectHead());
  } else {
    selectHead().remove();
  }
  $('transaction-bulk-toggle').setAttribute('aria-pressed', String(bulk));
  $('transactions-bulk-count').textContent = t('{count} selected', { count: selected.size });
  const all = selectHead().firstElementChild;
  all.checked = shownCount > 0 && selected.size === shownCount;
  all.indeterminate = selected.size > 0 && selected.size < shownCount;
  all.setAttribute('aria-label', t('Select all {count} in the filter', { count: shownCount }));
  $('transactions-bulk-set').disabled = selected.size === 0;
  $('transactions-bulk-delete').disabled = selected.size === 0;
  $('transactions-total-label').colSpan = bulk ? 5 : 4;
  for (const box of document.querySelectorAll('#transactions-rows input[data-select]')) box.checked = selected.has(box.dataset.select);
}

function selectBox(transaction) {
  const box = h('input', {
    type: 'checkbox',
    'data-select': transaction.id,
    'aria-label': t('Select {name}', { name: transactionLabel(transaction) }),
  });
  box.addEventListener('change', () => {
    if (box.checked) selected.add(transaction.id);
    else selected.delete(transaction.id);
    syncBulk();
  });
  return h('td', { class: 'select-col' }, box);
}

/** Switches bulk select on or off; off forgets the selection. */
function toggleBulk(on) {
  bulk = on;
  selected.clear();
  loadList().catch((error) => showToast(errorText(error)));
}

/** Back to the plain list, when another project is opened. */
function endBulk() {
  bulk = false;
  selected.clear();
}

/** The header checkbox: selects every transaction the filters show, also those not loaded yet, or none. */
async function selectAll() {
  try {
    if (selectHead().firstElementChild.checked) {
      for (const id of await invoke('list_transaction_ids', filterArgs())) selected.add(id);
    } else {
      selected.clear();
    }
  } catch (error) {
    showToast(errorText(error));
  }
  syncBulk();
}

async function setSelectedCategory() {
  try {
    const { value } = $('transactions-bulk-category');
    const count = await invoke('set_transactions_category', { transactionIds: [...selected], categoryId: value || null });
    selected.clear();
    showResult(t('{count} changed', { count }));
    await changed();
  } catch (error) {
    showToast(errorText(error));
  }
}

function deleteSelected() {
  const count = selected.size;
  return deleteWithConfirm({
    title: tn('Delete {count} transaction?', 'Delete {count} transactions?', count),
    message: tn(
      'The selected transaction will be deleted. Importing the file again adds it again. You can undo this.',
      'The {count} selected transactions will be deleted. Importing the file again adds them again. You can undo this.',
      count,
    ),
    confirmLabel: t('Delete selected'),
    command: 'delete_transactions',
    args: { transactionIds: [...selected] },
    changed: async () => {
      selected.clear();
      await changed();
    },
  });
}

/** The category choice of the bulk bar: Uncategorized, then the categories. */
function renderBulkCategories() {
  const select = $('transactions-bulk-category');
  const kept = select.value;
  select.replaceChildren(
    h('option', { value: '' }, t('Uncategorized')),
    ...categories.map((category) => h('option', { value: category.id }, category.name)),
  );
  select.value = categories.some((category) => category.id === kept) ? kept : '';
}

const categoryName = (id) => categories.find((category) => category.id === id)?.name ?? t('Uncategorized');

const transactionsFilter = createStatementFilter({
  periodHost: 'transaction-period-filter',
  entityHost: 'transaction-entity-filter',
  amountHost: 'transaction-amount-filter',
  textHosts: { counterparty: 'transaction-counterparty-filter', description: 'transaction-description-filter' },
  period: 'day',
  entityTitle: 'Category',
  entities: () => [
    { id: UNCATEGORIZED, label: t('Uncategorized') },
    ...categories.map((category) => ({ id: category.id, label: category.name })),
  ],
  onChange: () => {
    limit = PAGE_ROWS;
    selected.clear();
    loadList().catch((error) => showToast(errorText(error)));
  },
});

// ---- Yearly totals ----

function renderTotals(totals) {
  renderTable({ empty: 'transaction-totals-empty', table: 'transaction-totals-table', body: 'transaction-totals-rows' }, totals, (total) =>
    h('tr', {}, rowHeader(String(total.year)), numCell(formatAmount(total.income)), numCell(formatAmount(total.spent)), numCell(formatAmount(total.net))),
  );
}

// ---- The list ----

/** Loads the transactions through the filter and shows them. */
async function loadList() {
  if (bulk && selected.size > 0) {
    // A change may have moved a selected transaction out of the filters, or deleted it.
    const shown = new Set(await invoke('list_transaction_ids', filterArgs()));
    for (const id of [...selected]) if (!shown.has(id)) selected.delete(id);
  }
  const { result, hasData } = await transactionsFilter.load('list_transactions', (list) => list.count, { limit }, manualOnly() ? { manualOnly: true } : {});
  transactionsFilter.show(hasData);
  renderList(result, hasData);
}

function categorySelect(transaction) {
  const select = h(
    'select',
    { class: 'table-select', 'aria-label': t('Category of {name}', { name: transactionLabel(transaction) }) },
    h('option', { value: '' }, t('Uncategorized')),
    categories.map((category) => h('option', { value: category.id }, category.name)),
  );
  select.value = transaction.category_id ?? '';
  select.addEventListener('change', () => setCategory(transaction, select.value));
  return select;
}

/** A name for a row in labels and messages: the counterparty, else the description. */
const transactionLabel = (transaction) => transaction.counterparty_name || transaction.description || formatDate(transaction.date);

/** Marks a category that was set by hand: "Apply rules" leaves it alone (CSD-032 REQ-11). */
const manualBadge = (transaction) =>
  h(
    'button',
    {
      type: 'button',
      class: 'badge manual-badge',
      title: t('Set by hand. Applying the rules leaves it as it is.'),
      'aria-label': t('Manual: {name}. Choose to unset it.', { name: transactionLabel(transaction) }),
      onClick: () => unsetManual(transaction),
    },
    t('Manual'),
  );

/** Asks first, then takes the mark off and keeps the category (CSD-032 REQ-11, AC-11.5). */
async function unsetManual(transaction) {
  const confirmed = await confirmAction({
    title: t('Unset manual?'),
    message: t('The category {category} stays. Applying the rules to all non-manual transactions may change it.', {
      category: categoryName(transaction.category_id),
    }),
    confirmLabel: t('Unset manual'),
  });
  if (!confirmed) return;
  try {
    await invoke('unset_transaction_manual', { transactionId: transaction.id });
    await changed();
  } catch (error) {
    showToast(errorText(error));
    await reloadTransactionsOverview().catch((reloadError) => showToast(errorText(reloadError)));
  }
}

function renderList(list, hasData) {
  if (list.count === 0 && !transactionsFilter.payload() && !manualOnly()) $('transactions-empty').textContent = t('No transactions yet');
  // The total of every transaction that passes the filter, as Rust added it up (CSD-032 REQ-15).
  $('transactions-total').textContent = formatAmount(list.total);
  $('transactions-total-label').textContent = transactionsFilter.payload() || manualOnly() ? t('Filtered total') : t('Total');
  renderFilteredTable(
    { empty: 'transactions-empty', nomatch: 'transactions-nomatch', table: 'transactions-table', body: 'transactions-rows' },
    { rows: list.rows, hasData },
    (transaction) =>
      h(
        'tr',
        {},
        bulk ? selectBox(transaction) : null,
        rowHeader(formatDate(transaction.date)),
        h('td', { class: 'wrap' }, transaction.counterparty_name),
        h('td', { class: 'wrap' }, transaction.description),
        h('td', {}, categorySelect(transaction), transaction.manual ? manualBadge(transaction) : null),
        numCell(formatAmount(transaction.amount)),
        h(
          'td',
          { class: 'row-actions' },
          iconButton('add-rule', t('Add a rule from {name}', { name: transactionLabel(transaction) }), () => openRuleFromTransaction(transaction)),
          iconButton('delete', t('Delete {name}', { name: transactionLabel(transaction) }), () => deleteTransaction(transaction)),
        ),
      ),
  );
  shownCount = list.count;
  syncBulk();
  const more = list.rows.length < list.count;
  $('transactions-more').hidden = !more;
  $('transactions-shown').textContent = t('Showing {shown} of {count}', { shown: list.rows.length, count: list.count });
}

async function setCategory(transaction, categoryId) {
  try {
    await invoke('set_transaction_category', { transactionId: transaction.id, categoryId: categoryId || null });
    await changed();
  } catch (error) {
    showToast(errorText(error));
    await reloadTransactionsOverview().catch((reloadError) => showToast(errorText(reloadError)));
  }
}

function deleteTransaction(transaction) {
  return deleteWithConfirm({
    title: t('Delete {name}?', { name: transactionLabel(transaction) }),
    message: t('The transaction of {date} will be deleted. Importing the file again adds it again.', { date: formatDate(transaction.date) }),
    confirmLabel: t('Delete transaction'),
    command: 'delete_transaction',
    args: { transactionId: transaction.id },
    changed,
  });
}

/** Removes every transaction after a confirmation; categories and rules stay (CSD-032 REQ-25). */
function deleteAllTransactions() {
  return deleteWithConfirm({
    title: t('Delete all transactions?'),
    message: t('All transactions will be deleted. The categories and the rules stay. You can undo this.'),
    confirmLabel: t('Delete all transactions'),
    command: 'delete_all_transactions',
    args: {},
    changed,
  });
}

// ---- Messages of an import or a re-apply ----

/** The result as a yellow notification bottom right, like the one after an undo (CSD-032 REQ-16). */
const showResult = (text) => showToast(text, 'notice');

async function importTransactions() {
  try {
    const summary = await invoke('import_transactions');
    if (summary === null) return;
    showResult(t('{added} added, {skipped} skipped as duplicates', summary));
    await changed();
  } catch (error) {
    showToast(errorText(error));
  }
}

async function reapply(scope) {
  if (scope === 'all') {
    const confirmed = await confirmAction({
      title: t('Apply rules including manually set?'),
      message: t('The rules will also change the categories you set by hand. You can undo this.'),
      confirmLabel: t('Apply rules'),
    });
    if (!confirmed) return;
  }
  try {
    const { changed: count } = await invoke('reapply_transaction_rules', { scope });
    showResult(t('{count} changed', { count }));
    await changed();
  } catch (error) {
    showToast(errorText(error));
  }
}

// ---- Categories ----

function renderCategories() {
  $('transaction-categories').replaceChildren(
    h('li', { class: 'job-row' }, h('span', { class: 'job-name' }, t('Uncategorized')), h('span', { class: 'job-meta' }, t('Built in'))),
    ...categories.map((category) =>
      h(
        'li',
        { class: 'job-row' },
        h('span', { class: 'job-name' }, category.name),
        h('span', { class: 'job-meta rule-count' }, tn('{count} transaction', '{count} transactions', category.transaction_count)),
        h(
          'span',
          { class: 'job-actions' },
          iconButton('edit', t('Edit {name}', { name: category.name }), () => openCategoryDialog(category)),
          iconButton('delete', t('Delete {name}', { name: category.name }), () => deleteCategory(category)),
        ),
      ),
    ),
  );
}

let editingCategory = null;

function openCategoryDialog(category = null) {
  editingCategory = category;
  openDialog({
    dialog: 'transaction-category-dialog',
    form: 'transaction-category-form',
    error: 'transaction-category-error',
    title: 'transaction-category-dialog-title',
    titleText: category ? t('Edit category') : t('Add category'),
    focus: 'name',
    fill(form) {
      if (category) form.elements.name.value = category.name;
    },
  });
}

function submitCategory() {
  const name = $('transaction-category-form').elements.name.value;
  return saveDialog({
    dialog: 'transaction-category-dialog',
    error: 'transaction-category-error',
    changed,
    save: () =>
      editingCategory
        ? invoke('rename_transaction_category', { categoryId: editingCategory.id, name })
        : invoke('add_transaction_category', { name }),
  });
}

function deleteCategory(category) {
  return deleteWithConfirm({
    title: t('Delete {name}?', { name: category.name }),
    message: t('Its transactions become Uncategorized and the rules that name it are deleted.'),
    confirmLabel: t('Delete category'),
    command: 'delete_transaction_category',
    args: { categoryId: category.id },
    changed,
  });
}

// ---- Rules ----

/** The share of categorized transactions, as Rust labelled it; hidden while there are no transactions (CSD-032 REQ-31). */
function renderCategorisation(summary) {
  const note = $('transaction-categorisation');
  note.hidden = summary.percent_label === null;
  if (note.hidden) return;
  note.textContent = t('{percent} of the transactions are categorized ({categorized} of {total})', {
    percent: formatLabel(summary.percent_label),
    categorized: summary.categorized,
    total: summary.total,
  });
}

/** The rule whose handle gets the focus back after the list is drawn again, so the keys can move it on. */
let focusRuleHandle = null;

function renderRules() {
  $('transaction-rules-empty').hidden = rules.length > 0;
  $('transaction-rules').replaceChildren(
    ...rules.map((rule) => {
      const handle = iconButton('grip', t('Reorder {name}', { name: rule.text }), () => {});
      handle.classList.add('drag-handle');
      handle.dataset.ruleId = rule.id;
      return h(
        'li',
        { class: 'job-row' },
        handle,
        h('span', { class: 'job-name' }, rule.text),
        h('span', { class: 'job-meta' }, `→ ${categoryName(rule.category_id)}`),
        h('span', { class: 'job-meta rule-count' }, tn('{count} transaction', '{count} transactions', rule.transaction_count)),
        h(
          'span',
          { class: 'job-actions' },
          iconButton('edit', t('Edit {name}', { name: rule.text }), () => openRuleDialog(rule)),
          iconButton('delete', t('Delete {name}', { name: rule.text }), () => deleteRule(rule)),
        ),
      );
    }),
  );
  if (focusRuleHandle !== null) {
    $('transaction-rules').querySelector(`.drag-handle[data-rule-id="${focusRuleHandle}"]`)?.focus();
    focusRuleHandle = null;
  }
}

async function moveRule(rule, position) {
  try {
    await invoke('move_transaction_rule', { ruleId: rule.id, position });
    await changed();
  } catch (error) {
    showToast(errorText(error));
  }
}

let editingRule = null;
/** The value of the last option of the category choice of a new rule: the rule creates its category. */
const NEW_CATEGORY = '__new__';

/** Shows the name field while "New category…" is chosen, and only then (CSD-032 REQ-9). */
function syncNewCategory() {
  const form = $('transaction-rule-form');
  const creating = form.elements.category.value === NEW_CATEGORY;
  $('transaction-rule-new-category').hidden = !creating;
  if (creating) form.elements.name.focus();
}

/**
 * The rule dialog for a new rule that starts from a transaction: its counterparty (its description
 * when it has none) as the text and its category as the category, both still editable (CSD-032 REQ-23).
 */
function openRuleFromTransaction(transaction) {
  openRuleDialog(null, {
    text: transaction.counterparty_name || transaction.description,
    categoryId: transaction.category_id,
  });
}

function openRuleDialog(rule = null, start = null) {
  editingRule = rule;
  openDialog({
    dialog: 'transaction-rule-dialog',
    form: 'transaction-rule-form',
    error: 'transaction-rule-error',
    title: 'transaction-rule-dialog-title',
    titleText: rule ? t('Edit rule') : t('Add rule'),
    focus: 'text',
    fill(form) {
      form.elements.category.replaceChildren(
        ...categories.map((category) => h('option', { value: category.id }, category.name)),
        // A new rule may create its category; an edit chooses among the existing ones.
        rule ? null : h('option', { value: NEW_CATEGORY }, t('New category…')),
      );
      form.elements.name.value = '';
      if (rule) {
        form.elements.text.value = rule.text;
        form.elements.category.value = rule.category_id;
      } else if (start) {
        form.elements.text.value = start.text;
        if (start.categoryId) form.elements.category.value = start.categoryId;
      }
      syncNewCategory();
    },
  });
}

function submitRule() {
  const form = $('transaction-rule-form');
  const text = form.elements.text.value;
  const categoryId = form.elements.category.value;
  const creating = categoryId === NEW_CATEGORY;
  return saveDialog({
    dialog: 'transaction-rule-dialog',
    error: 'transaction-rule-error',
    changed,
    save: () =>
      editingRule
        ? invoke('edit_transaction_rule', { ruleId: editingRule.id, text, categoryId })
        : invoke('add_transaction_rule', creating ? { text, newCategory: form.elements.name.value } : { text, categoryId }),
  });
}

function deleteRule(rule) {
  return deleteWithConfirm({
    title: t('Delete {name}?', { name: rule.text }),
    message: t('Transactions that already have a category keep it.'),
    confirmLabel: t('Delete rule'),
    command: 'delete_transaction_rule',
    args: { ruleId: rule.id },
    changed,
  });
}

// ---- Loading ----

/** Reloads everything from Rust. */
export async function reloadTransactionsOverview() {
  categories = await invoke('list_transaction_categories');
  rules = await invoke('list_transaction_rules');
  renderCategories();
  renderBulkCategories();
  renderRules();
  renderCategorisation(await invoke('get_categorisation_summary'));
  await loadList();
  renderTotals(await invoke('yearly_transaction_totals'));
}

async function changed() {
  await reloadTransactionsOverview();
  await onChanged();
}

export function initTransactionsOverview({ onChanged: callback }) {
  onChanged = callback;
  initTransactionSuggest({ categories: () => categories, changed });
  makeSortable($('transaction-rules'), {
    handle: '.drag-handle',
    onMove: (from, to) => {
      const rule = rules[from];
      focusRuleHandle = rule.id;
      return moveRule(rule, to);
    },
    onCancel: renderRules,
  });
  $('transaction-import').addEventListener('click', importTransactions);
  $('transaction-bulk-toggle').addEventListener('click', () => toggleBulk(!bulk));
  $('transactions-bulk-set').addEventListener('click', setSelectedCategory);
  $('transactions-bulk-delete').addEventListener('click', deleteSelected);
  $('transaction-manual-only').addEventListener('change', () => {
    limit = PAGE_ROWS;
    selected.clear();
    loadList().catch((error) => showToast(errorText(error)));
  });
  $('transactions-load-more').addEventListener('click', () => {
    limit += PAGE_ROWS;
    loadList().catch((error) => showToast(errorText(error)));
  });
  $('transaction-category-add').addEventListener('click', () => openCategoryDialog(null));
  submitOnce($('transaction-category-form'), submitCategory);
  $('transaction-rule-add').addEventListener('click', () => openRuleDialog(null));
  submitOnce($('transaction-rule-form'), submitRule);
  $('transaction-rule-form').elements.category.addEventListener('change', syncNewCategory);
  $('transaction-apply-rules').addEventListener('click', () => reapply('all_non_manual'));
  $('transaction-apply-rules-all').addEventListener('click', () => reapply('all'));
  $('transaction-delete-all').addEventListener('click', deleteAllTransactions);
}
