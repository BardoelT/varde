// Bulk select on the Transactions list (CSD-032 REQ-28): a button switches on a checkbox per row and in the
// header, and a bar sets the category of, or deletes, the selected transactions. Rust names the ids that the
// filters show and makes the changes; this file keeps the selection and sends it.
import { invoke } from '../shared/ipc.js';
import { $, h } from '../shared/dom.js';
import { t, tn } from '../shared/i18n.js';
import { deleteWithConfirm } from '../shared/crud.js';
import { state, changed, run, showResult, transactionLabel } from './transactions-shared.js';

/** Bulk select is switched on by the user; then every row has a checkbox. */
let bulk = false;
/** The ids of the selected transactions: always some of those the filters show. */
const selected = new Set();
/** How many transactions the filters show, as Rust counted them, for the checkbox of the header. */
let shownCount = 0;
let selectHeadCell = null;
/** The arguments that say which transactions the filters show, as `list_transactions` takes them. */
let filterArgs = () => undefined;
let reloadList = async () => {};

export const isBulk = () => bulk;

/** Forgets the selection, such as when a filter changes. */
export function clearSelection() {
  selected.clear();
}

/** Back to the plain list, when another project is opened. */
export function endBulk() {
  bulk = false;
  selected.clear();
}

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

/** Shows how many are selected, and the state of the checkbox of the header; `count` is how many the filters show. */
export function syncBulk(count = shownCount) {
  shownCount = count;
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

/** The checkbox cell of a row of the list. */
export function selectBox(transaction) {
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

/** Drops the selected transactions that the filters no longer show, or that a change deleted. */
export async function pruneSelection() {
  if (!bulk || selected.size === 0) return;
  const shown = new Set(await invoke('list_transaction_ids', filterArgs()));
  for (const id of [...selected]) if (!shown.has(id)) selected.delete(id);
}

/** Switches bulk select on or off; off forgets the selection. */
function toggleBulk(on) {
  bulk = on;
  selected.clear();
  return run(reloadList);
}

/** The header checkbox: selects every transaction the filters show, also those not loaded yet, or none. */
async function selectAll() {
  await run(async () => {
    if (selectHead().firstElementChild.checked) {
      for (const id of await invoke('list_transaction_ids', filterArgs())) selected.add(id);
    } else {
      selected.clear();
    }
  });
  syncBulk();
}

function setSelectedCategory() {
  return run(async () => {
    const { value } = $('transactions-bulk-category');
    const count = await invoke('set_transactions_category', { transactionIds: [...selected], categoryId: value || null });
    selected.clear();
    showResult(t('{count} changed', { count }));
    await changed();
  });
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
export function renderBulkCategories() {
  const select = $('transactions-bulk-category');
  const kept = select.value;
  select.replaceChildren(
    h('option', { value: '' }, t('Uncategorized')),
    ...state.categories.map((category) => h('option', { value: category.id }, category.name)),
  );
  select.value = state.categories.some((category) => category.id === kept) ? kept : '';
}

/** `args()` says which transactions the filters show; `reload()` redraws the list. */
export function initBulk({ args, reload }) {
  filterArgs = args;
  reloadList = reload;
  $('transaction-bulk-toggle').addEventListener('click', () => toggleBulk(!bulk));
  $('transactions-bulk-set').addEventListener('click', setSelectedCategory);
  $('transactions-bulk-delete').addEventListener('click', deleteSelected);
}
