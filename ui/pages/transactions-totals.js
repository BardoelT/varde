// The Yearly totals of the Transactions page (CSD-032 REQ-24): one collapsible block per year with the income,
// spent and net in its header and, when open, the grid of the net per category and month. It only lays out what
// Rust returned; every figure, including which month is empty, is Rust's.
import { $, h } from '../shared/dom.js';
import { formatAmount } from '../shared/format.js';
import { t } from '../shared/i18n.js';
import { numCell } from '../shared/list-rows.js';
import { isNegative } from '../shared/yearly-chart.js';

/** The years the user opened (true) or closed (false) in this session. A year not in it follows the default. */
const chosen = new Map();

/** The short month names, January first, in the chosen language. */
const monthNames = () => [t('Jan'), t('Feb'), t('Mar'), t('Apr'), t('May'), t('Jun'), t('Jul'), t('Aug'), t('Sep'), t('Oct'), t('Nov'), t('Dec')];

/** Whether the block of `year` is open: the choice of the user, else only the newest year (REQ-24). */
const isOpen = (year, newest) => chosen.get(year) ?? year === newest;

/** One amount cell: a dimmed dash for a month Rust sent as empty, else the signed net, red when negative. */
function netCell(amount, extra = '') {
  if (amount === null) return h('td', { class: `num empty ${extra}`.trim() }, '–');
  return h('td', { class: `num ${isNegative(amount) ? 'negative ' : ''}${extra}`.trim() }, formatAmount(amount));
}

const categoryName = (category) => (category.id === 'uncategorized' ? t('Uncategorized') : category.name);

/** The grid of one year: a row per category, then the Total row; Total column on the right. */
function gridOf(total) {
  const head = h('tr', {}, h('th', { scope: 'col' }, t('Category')), monthNames().map((name) => h('th', { scope: 'col', class: 'num' }, name)), h('th', { scope: 'col', class: 'num' }, t('Total')));
  const rows = total.categories.map((category) =>
    h('tr', {}, h('th', { scope: 'row' }, categoryName(category)), category.months.map((amount) => netCell(amount)), netCell(category.total, 'total-cell')),
  );
  const totalRow = h('tr', { class: 'total-row' }, h('th', { scope: 'row' }, t('Total')), total.months.map((amount) => netCell(amount)), netCell(total.total, 'total-cell'));
  return h('table', { class: 'data-table year-grid' }, h('thead', {}, head), h('tbody', {}, rows, totalRow));
}

/** A labelled figure of the header of a year. */
const figure = (label, amount) => h('span', { class: `year-figure${isNegative(amount) ? ' negative' : ''}` }, h('span', { class: 'year-figure-label' }, label), h('span', { class: 'year-figure-value' }, formatAmount(amount)));

/** One block: the header button and the grid below it. */
function blockOf(total, newest) {
  const open = isOpen(total.year, newest);
  const gridId = `transaction-year-grid-${total.year}`;
  const grid = h('div', { class: 'table-wrap year-grid-wrap', id: gridId, hidden: !open }, gridOf(total));
  const button = h(
    'button',
    { type: 'button', class: 'year-toggle', 'aria-expanded': String(open), 'aria-controls': gridId },
    h('span', { class: 'year-chevron', 'aria-hidden': 'true' }),
    h('span', { class: 'year-title' }, String(total.year)),
    figure(t('Income'), total.income),
    figure(t('Spent'), total.spent),
    figure(t('Net'), total.net),
  );
  button.addEventListener('click', () => {
    const next = button.getAttribute('aria-expanded') !== 'true';
    chosen.set(total.year, next);
    button.setAttribute('aria-expanded', String(next));
    grid.hidden = !next;
  });
  return h('section', { class: 'year-block' }, button, grid);
}

/** Shows the blocks of `totals` (newest year first), or the empty note when there are none. */
export function renderYearlyTotals(totals) {
  $('transaction-totals-empty').hidden = totals.length > 0;
  const newest = totals[0]?.year;
  $('transaction-totals-years').replaceChildren(...totals.map((total) => blockOf(total, newest)));
}
