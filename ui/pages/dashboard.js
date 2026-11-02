// Dashboard: income this year to date and last month, over all jobs (CSD-004).
// Entries are paychecks and annual statements.
// The figures come from Rust; this file only displays them.
import { invoke } from '../shared/ipc.js';
import { h } from '../shared/dom.js';
import { formatAmount } from '../shared/format.js';
import { t } from '../shared/i18n.js';

const $ = (id) => document.getElementById(id);

function cardsFor(totals) {
  const card = (label, value, primary = false) =>
    h(
      'div',
      { class: primary ? 'card primary' : 'card' },
      h('div', { class: 'card-label' }, label),
      h('div', { class: 'card-value' }, value),
    );
  return [
    card(t('Total gross'), formatAmount(totals.total_gross), true),
    card(t('Gross salary'), formatAmount(totals.gross_salary)),
    card(t('Holiday pay'), formatAmount(totals.holiday_pay)),
    card(t('Bonus'), formatAmount(totals.bonus)),
  ];
}

/** Shows one block: its cards, or a note when the period has no paychecks. */
function renderBlock(prefix, title, totals, emptyText) {
  $(`dashboard-${prefix}-title`).textContent = title;
  $(`dashboard-${prefix}-empty`).textContent = emptyText;
  $(`dashboard-${prefix}-empty`).hidden = Boolean(totals);
  $(`dashboard-${prefix}-cards`).replaceChildren(...(totals ? cardsFor(totals) : []));
}

/** Loads the figures from Rust and shows them. */
export async function reloadDashboard() {
  const d = await invoke('get_dashboard');
  $('dashboard-empty').hidden = d.has_jobs;
  $('dashboard-content').hidden = !d.has_jobs;
  if (!d.has_jobs) return;

  renderBlock(
    'year',
    t('This year · {year}', { year: d.year }),
    d.year_to_date,
    t('No entries yet in {year}.', { year: d.year }),
  );
  const month = d.last_month ?? '';
  renderBlock(
    'month',
    t('Last month · {month}', { month }),
    d.last_month_totals,
    t('No paycheck for {month}.', { month }),
  );
}
