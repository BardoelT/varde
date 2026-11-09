// Dashboard: one tile per area with a headline figure and its change against the same moment last
// year (CSD-004). The figures, the change, its colour and the percentage label come from Rust; this
// file only displays them.
import { invoke } from '../shared/ipc.js';
import { $, h } from '../shared/dom.js';
import { formatAmount } from '../shared/format.js';
import { t } from '../shared/i18n.js';

/** The tiles in display order: payload key, title, left-pane page it opens, and its label (CSD-004 REQ-8, REQ-13). */
function tilesFor() {
  return [
    { key: 'net_worth', title: t('Net worth'), page: 'net-worth', hero: true },
    { key: 'income', title: t('Income'), page: 'income', label: t('Year to date') },
    { key: 'expenses', title: t('Expenses'), page: 'expenses', label: t('Year to date') },
    { key: 'bank_accounts', title: t('Bank accounts'), page: 'bank' },
    { key: 'investments', title: t('Investments'), page: 'investments' },
    { key: 'houses', title: t('Houses'), page: 'houses' },
    { key: 'debts', title: t('Debts'), page: 'debts' },
  ];
}

const ARROWS = { up: '▲', down: '▼', stable: '–' };

/** The change of a tile: an arrow, coloured by whether it is good, and the percentage; a grey dash without a comparison. */
function changeOf(change) {
  if (!change) {
    return h('span', { class: 'tile-change neutral', title: t('No comparison with last year') }, h('span', { class: 'tile-arrow', 'aria-hidden': 'true' }, '–'));
  }
  const words = { up: t('Up'), down: t('Down'), stable: t('Stable') };
  return h(
    'span',
    { class: `tile-change ${change.sentiment}`, 'aria-label': `${words[change.direction]} ${change.percent_label}`, title: t('Compared with last year') },
    h('span', { class: 'tile-arrow', 'aria-hidden': 'true' }, ARROWS[change.direction]),
    h('span', { class: 'tile-percent' }, change.percent_label),
  );
}

/** One tile, a button that opens the page of its area (CSD-004 REQ-15). */
function renderTile(spec, tile) {
  const label = tile.as_of ? t('as of {period}', { period: tile.as_of }) : spec.label;
  const body = tile.has_data
    ? [h('div', { class: 'tile-figure' }, h('span', { class: 'tile-value' }, formatAmount(tile.value)), changeOf(tile.change))]
    : [h('div', { class: 'tile-empty' }, t('No data yet'))];
  return h(
    'button',
    {
      type: 'button',
      class: `tile${spec.hero ? ' tile-hero' : ''}${tile.has_data ? '' : ' no-data'}`,
      'data-tile': spec.key,
      onClick: () => document.querySelector(`.nav-item[data-section="${spec.page}"]`).click(),
    },
    h('div', { class: 'tile-title' }, spec.title),
    label && tile.has_data ? h('div', { class: 'tile-label' }, label) : null,
    ...body,
  );
}

/** Loads the tiles from Rust and shows them. */
export async function reloadDashboard() {
  const d = await invoke('get_dashboard');
  $('dashboard-tiles').replaceChildren(...tilesFor().map((spec) => renderTile(spec, d[spec.key])));
}
