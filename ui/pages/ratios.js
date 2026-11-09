// Ratios: the table on the Ratios page, with the years as columns (CSD-018). The
// figures come from Rust; this file only lays them out.
import { invoke, errorText } from '../shared/ipc.js';
import { $, h, iconButton, showToast } from '../shared/dom.js';
import { formatAmount, formatLabel } from '../shared/format.js';
import { t } from '../shared/i18n.js';
import { isNegative, yearLabel } from '../shared/yearly-chart.js';

/** An amount, or a dash when Rust has none (CSD-018 REQ-10). */
const amountCell = (value) => (value === null ? '–' : formatAmount(value));
/** A percentage as Rust labels it ("+6.09%", "-2.50%", "0.00%" or a dash): the `<field>_label` of the row, shown as received. */
const percentCell = (_value, row, field) => formatLabel(row[`${field}_label`]);

/** The rows of the table under their sub-headings: label, field, how to show it and how it is calculated (CSD-018 REQ-3, REQ-13). The texts are English; they are shown through `t`. */
const GROUPS = [
  ['Wealth', [
    ['Total income (gross)', 'total_income', amountCell,
      'The gross income of the year from all paychecks, holiday pay and bonuses. 0.00 when the year has no income.'],
    ['Wealth accumulation from cash', 'wealth_accumulation', amountCell,
      'The increase of the bank accounts, plus the net deposits to the investments, plus the debt repaid in the year. Houses are not counted. Shown as – when none of these has data.'],
    ['Percentage of income to wealth', 'wealth_percent', percentCell,
      'The wealth accumulation from cash divided by the total income, times 100. Shown as – when either is missing or the total income is 0.00.'],
  ]],
  ['Growth', [
    ['Growth of investments', 'investment_growth_percent', percentCell,
      'The growth percentage of the investments, the same figure as on the Investments chart. Shown as – where that chart has none.'],
    ['Growth of total wealth', 'wealth_growth_percent', percentCell,
      'The net worth of the year minus that of the year before, divided by the net worth of the year before, times 100. Shown as – when the year or the year before has no records, or when the net worth of the year before is not positive.'],
  ]],
  ['Free income', [
    ['Disposable income', 'disposable_income', amountCell,
      'The wealth accumulation from cash plus the expenses of the year: what was set aside plus what was spent. Shown as – when there is no wealth accumulation from cash.'],
    ['Percentage of disposable income', 'disposable_percent', percentCell,
      'The disposable income divided by the total income, times 100. Shown as – when either is missing or the total income is 0.00.'],
  ]],
];

const infoPopover = () => $('ratios-info');
let infoOwner = null;

function closeInfo() {
  infoPopover().hidden = true;
  infoOwner?.setAttribute('aria-expanded', 'false');
  infoOwner = null;
}

/** Opens the explanation under its icon, or closes it when that icon's own is open (CSD-018 REQ-13). */
function toggleInfo(button, text) {
  const wasOwner = infoOwner === button;
  closeInfo();
  if (wasOwner) return;
  const popover = infoPopover();
  popover.textContent = text;
  popover.hidden = false;
  const box = button.getBoundingClientRect();
  const width = popover.offsetWidth;
  popover.style.left = `${Math.max(8, Math.min(box.left, window.innerWidth - width - 8))}px`;
  popover.style.top = `${box.bottom + 6}px`;
  button.setAttribute('aria-expanded', 'true');
  button.setAttribute('aria-describedby', 'ratios-info');
  infoOwner = button;
}

/** The info icon after a row label. */
function infoButton(label, text) {
  const button = iconButton('info', t('How "{label}" is calculated', { label }), (event) => {
    event.stopPropagation();
    toggleInfo(button, text);
  });
  button.classList.add('info-btn');
  button.setAttribute('aria-expanded', 'false');
  return button;
}

function renderTable(years) {
  closeInfo();
  $('ratios-head').replaceChildren(
    h('tr', {}, h('th', { scope: 'col' }, ''), ...years.map((row) => h('th', { scope: 'col', class: 'num' }, yearLabel(row)))),
  );
  const rows = GROUPS.flatMap(([title, items]) => [
    // The title is its own sticky cell; one spanning cell would scroll away with the years.
    h('tr', { class: 'group-row' }, h('th', { scope: 'row' }, t(title)), h('td', { colspan: String(years.length) })),
    ...items.map(([label, field, show, info]) =>
      h(
        'tr',
        {},
        h('th', { scope: 'row' }, t(label), infoButton(t(label), t(info))),
        ...years.map((row) => {
          const value = row[field];
          const negative = value !== null && isNegative(value);
          return h('td', { class: `num${negative ? ' negative' : ''}` }, show(value, row, field));
        }),
      ),
    ),
  ]);
  $('ratios-rows').replaceChildren(...rows);
}

/** Loads the figures from Rust and shows them, or the note when there is nothing yet. */
export async function reloadRatios() {
  const { years } = await invoke('get_ratios');
  $('ratios-empty').hidden = years.length > 0;
  $('ratios-table').hidden = years.length === 0;
  if (years.length > 0) renderTable(years);
}

/** Reloads the table only while the page is showing. */
export async function reloadRatiosIfOpen() {
  if ($('ratios').classList.contains('active')) await reloadRatios();
}

export function initRatios() {
  document.addEventListener('click', (event) => {
    if (infoOwner && !infoPopover().contains(event.target)) closeInfo();
  });
  document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' && infoOwner) {
      infoOwner.focus();
      closeInfo();
    }
  });
  document.querySelector('#ratios .table-wrap').addEventListener('scroll', closeInfo);
  // Opening the page recomputes it, so it never shows stale figures (CSD-018 REQ-12).
  document.querySelector('[data-section="ratios"]').addEventListener('click', () => reloadRatios().catch((error) => showToast(errorText(error))));
}
