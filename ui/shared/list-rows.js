// Building blocks of the overview pages: the list of accounts, the tables of totals and records,
// their empty states and the edit and delete buttons of a row. They only lay out what Rust sent.
import { $, h, iconButton } from './dom.js';
import { formatAmount } from './format.js';

/** A table cell holding a number, aligned right. */
export const numCell = (text) => h('td', { class: 'num' }, text);

/** The first cell of a table row (the period or the year), as the row header. */
export const rowHeader = (text) => h('th', { scope: 'row' }, text);

/** Shows `contentId` when there are items, else the empty state `emptyId`. */
export function toggleEmpty(emptyId, contentId, hasItems) {
  $(emptyId).hidden = hasItems;
  $(contentId).hidden = !hasItems;
}

/** The total line under an account list: hidden without accounts, a dash when Rust has no total. */
export function showTotal(boxId, valueId, total, hasItems) {
  $(boxId).hidden = !hasItems;
  $(valueId).textContent = total === null ? '—' : formatAmount(total);
}

/**
 * Fills a table from `items`, one `buildRow(item)` each, and shows the table or its empty note.
 * `ids` names the three elements: `{ empty, table, body }`.
 */
export function renderTable({ empty, table, body }, items, buildRow) {
  $(empty).hidden = items.length > 0;
  $(table).hidden = items.length === 0;
  $(body).replaceChildren(...items.map(buildRow));
}

/** The latest amount of an account with the period it is from, or a dash when it has none. */
export function latestAmount(amount, period) {
  if (amount === null) return h('span', { class: 'account-amount' }, '—');
  return h('span', { class: 'account-amount' }, formatAmount(amount), h('small', {}, period));
}

/**
 * The edit and delete icon buttons of a row, in a cell of the given class. `edit` and `remove`
 * are `{ label, onClick }`; `locked` leaves the cell empty (a record that cannot change).
 */
function actions(tag, cssClass, { edit, remove, locked = false }) {
  if (locked) return h(tag, { class: cssClass });
  return h(
    tag,
    { class: cssClass },
    iconButton('edit', edit.label, edit.onClick),
    iconButton('delete', remove.label, remove.onClick),
  );
}

/** The actions cell (`td.row-actions`) of a record table row. */
export const recordActions = (options) => actions('td', 'row-actions', options);

/**
 * A row of an account list (`li.job-row`): name, a meta line (text or nodes), the optional
 * latest amount and the edit and delete buttons.
 */
export function accountRow({ name, meta, amount, edit, remove }) {
  return h(
    'li',
    { class: 'job-row' },
    h('span', { class: 'job-name' }, name),
    h('span', { class: 'job-meta' }, meta),
    amount,
    actions('span', 'job-actions', { edit, remove }),
  );
}
