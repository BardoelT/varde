// Transactions: bank transactions imported from a Knab export (the Overview tab), with the categories and the
// rules that categorise them on their own tabs (CSD-032). Rust parses, categorises, filters, orders, cuts and
// totals; this file only shows what Rust returns and sends what the user chose. It draws the Overview tab and
// reloads the page; bulk select, the categories and the rules have a file each, and the shared state is in
// transactions-shared.js.
import { invoke } from '../shared/ipc.js';
import { $, h, iconButton, confirmAction } from '../shared/dom.js';
import { formatAmount, formatDate, formatLabel } from '../shared/format.js';
import { t } from '../shared/i18n.js';
import { initTransactionSuggest } from './transactions-suggest.js';
import { renderYearlyTotals } from './transactions-totals.js';
import { createStatementFilter, renderFilteredTable } from '../shared/statement-filter.js';
import { deleteWithConfirm } from '../shared/crud.js';
import { numCell, rowHeader } from '../shared/list-rows.js';
import { state, accountName, categoryName, changed, connect, run, transactionLabel } from './transactions-shared.js';
import { initBulk, isBulk, clearSelection, endBulk, selectBox, syncBulk, pruneSelection, renderBulkCategories } from './transactions-bulk.js';
import { initTransactionAccounts, renderAccounts } from './transactions-accounts.js';
import { initTransactionCategories, renderCategories } from './transactions-categories.js';
import { initTransactionRules, renderRules, openRuleFromTransaction } from './transactions-rules.js';

/** How many rows one step of "Load more" adds, and the rows shown first (CSD-032 REQ-15). */
const PAGE_ROWS = 200;
/** The name the frontend and Rust give the built-in category (CSD-032 section 5). */
const UNCATEGORIZED = 'uncategorized';

let limit = PAGE_ROWS;

/** Whether the list shows only the transactions whose category was set by hand (CSD-032 REQ-26). */
const manualOnly = () => $('transaction-manual-only').getAttribute('aria-pressed') === 'true';

/** Switches "Manual only" on or off; the button looks pressed while it is on. */
function setManualOnly(on) {
  $('transaction-manual-only').setAttribute('aria-pressed', String(on));
}

/** Back to the whole list, when another project is opened. */
export function resetTransactionsManualFilter() {
  setManualOnly(false);
  endBulk();
}

/** The arguments that say which transactions the filters show, as `list_transactions` takes them. */
function filterArgs() {
  const filter = transactionsFilter.payload();
  const args = { ...(filter ? { filter } : {}), ...(manualOnly() ? { manualOnly: true } : {}) };
  return Object.keys(args).length > 0 ? args : undefined;
}

const transactionsFilter = createStatementFilter({
  periodHost: 'transaction-period-filter',
  entityHost: 'transaction-entity-filter',
  amountHost: 'transaction-amount-filter',
  textHosts: { counterparty: 'transaction-counterparty-filter', description: 'transaction-description-filter' },
  period: 'day',
  choices: [
    {
      host: 'transaction-account-filter',
      key: 'account_ids',
      title: 'Account',
      choices: () => state.accounts.map((account) => ({ id: account.id, label: account.name })),
    },
  ],
  entityTitle: 'Category',
  entities: () => [
    { id: UNCATEGORIZED, label: t('Uncategorized') },
    ...state.categories.map((category) => ({ id: category.id, label: category.name })),
  ],
  onChange: () => {
    limit = PAGE_ROWS;
    clearSelection();
    reloadList();
  },
});

// ---- Yearly totals and the share of categorized transactions ----

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

// ---- The list ----

/** The number of the latest request for the list; an older answer that arrives later is dropped. */
let listRequest = 0;

/** Loads the transactions through the filter and shows them. */
async function loadList() {
  listRequest += 1;
  const mine = listRequest;
  await pruneSelection();
  if (mine !== listRequest) return;
  const { result, hasData } = await transactionsFilter.load('list_transactions', (list) => list.count, { limit }, manualOnly() ? { manualOnly: true } : {});
  if (mine !== listRequest) return;
  transactionsFilter.show(hasData);
  renderList(result, hasData);
}

/** Loads the list and reports an error as a notification. */
const reloadList = () => run(loadList);

function categorySelect(transaction) {
  const select = h(
    'select',
    { class: 'table-select', 'aria-label': t('Category of {name}', { name: transactionLabel(transaction) }) },
    h('option', { value: '' }, t('Uncategorized')),
    state.categories.map((category) => h('option', { value: category.id }, category.name)),
  );
  select.value = transaction.category_id ?? '';
  select.addEventListener('change', () => setCategory(transaction, select.value));
  return select;
}

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
  await run(
    async () => {
      await invoke('unset_transaction_manual', { transactionId: transaction.id });
      await changed();
    },
    { reloadAfterError: true },
  );
}

function renderList(list, hasData) {
  if (list.count === 0 && !transactionsFilter.payload() && !manualOnly()) $('transactions-empty').textContent = t('No transactions yet. Add an account on the Accounts tab and import a file into it.');
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
        isBulk() ? selectBox(transaction) : null,
        rowHeader(formatDate(transaction.date)),
        h('td', { class: 'wrap' }, accountName(transaction.account_id)),
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
  syncBulk(list.count);
  const more = list.rows.length < list.count;
  $('transactions-more').hidden = !more;
  $('transactions-shown').textContent = t('Showing {shown} of {count}', { shown: list.rows.length, count: list.count });
}

function setCategory(transaction, categoryId) {
  return run(
    async () => {
      await invoke('set_transaction_category', { transactionId: transaction.id, categoryId: categoryId || null });
      await changed();
    },
    { reloadAfterError: true },
  );
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

// ---- Loading ----

/** Reloads everything from Rust. */
export async function reloadTransactionsOverview() {
  const overview = await invoke('get_transactions_overview');
  state.accounts = overview.accounts;
  state.categories = overview.categories;
  state.rules = overview.rules;
  renderAccounts();
  renderCategories();
  renderBulkCategories();
  renderRules();
  renderCategorisation(overview.summary);
  await loadList();
  renderYearlyTotals(overview.totals);
}

export function initTransactionsOverview({ onChanged }) {
  connect({ reload: reloadTransactionsOverview, onChanged });
  initTransactionSuggest({ categories: () => state.categories, changed });
  initBulk({ args: filterArgs, reload: loadList });
  initTransactionAccounts();
  initTransactionCategories();
  initTransactionRules();
  $('transaction-manual-only').addEventListener('click', () => {
    setManualOnly(!manualOnly());
    limit = PAGE_ROWS;
    clearSelection();
    reloadList();
  });
  $('transactions-load-more').addEventListener('click', () => {
    limit += PAGE_ROWS;
    reloadList();
  });
}
