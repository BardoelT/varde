// The donut charts of the Categories tab (CSD-032 REQ-35): what was spent per category in one year and in one
// month. Rust sends the periods for the dropdowns and, per period, the slices with their amount, percentage,
// colour slot and place on the ring; this file only draws them.
import { $, h, showToast } from '../shared/dom.js';
import { formatAmount, formatLabel } from '../shared/format.js';
import { invoke, errorText } from '../shared/ipc.js';
import { t, currentLanguage } from '../shared/i18n.js';
import { svg, swatch, legendLabel, noSwatch, legendItem, bindHighlight, tooltipRow, createTooltip } from '../shared/chart.js';
import { isDiscrete } from '../shared/discrete-mode.js';

/** The ring in a 200 x 200 box. */
const CENTRE = 100;
const OUTER = 90;
const INNER = 58;

const MONTHS = () => [t('January'), t('February'), t('March'), t('April'), t('May'), t('June'), t('July'), t('August'), t('September'), t('October'), t('November'), t('December')];

/** "November 2026" for `2026-11`; Dutch writes month names in lower case ("november 2026"), and the key "May" is shared with the short name, which is capitalised. */
const monthLabel = (period) => {
  const [year, month] = period.split('-');
  const name = MONTHS()[Number(month) - 1];
  return `${currentLanguage() === 'en' ? name : name.toLocaleLowerCase(currentLanguage())} ${year}`;
};

/** The two donuts: the id stem of their elements, the period shown now and the numbers of the latest requests. */
const donuts = {
  year: { stem: 'transaction-donut-year', period: null, request: 0, label: (period) => period },
  month: { stem: 'transaction-donut-month', period: null, request: 0, label: monthLabel },
};

const categoryName = (slice) => (slice.id === 'uncategorized' ? t('Uncategorized') : slice.name);
const keyOf = (slice) => `category-${slice.id}`;

/** The point on the ring at `fraction` of the circle (0 is 12 o'clock, clockwise) and `radius`. */
function point(fraction, radius) {
  const angle = Number(fraction) * 2 * Math.PI;
  return [CENTRE + radius * Math.sin(angle), CENTRE - radius * Math.cos(angle)].map((value) => value.toFixed(3));
}

/** The outline of a ring segment from `start` to `end`; a full ring is drawn as two halves. */
function segmentPath(start, end) {
  const arc = (from, to) => {
    const large = Number(to) - Number(from) > 0.5 ? 1 : 0;
    const [ox1, oy1] = point(from, OUTER);
    const [ox2, oy2] = point(to, OUTER);
    const [ix1, iy1] = point(from, INNER);
    const [ix2, iy2] = point(to, INNER);
    return `M${ox1} ${oy1}A${OUTER} ${OUTER} 0 ${large} 1 ${ox2} ${oy2}L${ix2} ${iy2}A${INNER} ${INNER} 0 ${large} 0 ${ix1} ${iy1}Z`;
  };
  if (Number(end) - Number(start) >= 0.999999) return `${arc('0', '0.5')}${arc('0.5', '1')}`;
  return arc(start, end);
}

/** The accessible name of a slice; the amount is left out while Discrete mode is on. */
function sliceLabel(donut, slice) {
  const parts = [categoryName(slice), donut.label(donut.period)];
  if (!isDiscrete()) parts.push(formatAmount(slice.amount));
  parts.push(formatLabel(slice.percent_label));
  return parts.join(', ');
}

/** Draws one donut with its legend from the shares Rust sent. */
function draw(donut, shares) {
  const chart = $(`${donut.stem}-chart`);
  const legend = $(`${donut.stem}-legend`);
  const card = $(donut.stem);
  const scopes = () => [card];
  const group = svg('g', { class: 'donut-slices' });
  const inner = h('div', { class: 'donut-inner' });
  const tooltip = createTooltip(inner);
  for (const slice of shares.slices) {
    const path = svg('path', {
      d: segmentPath(slice.start, slice.end),
      class: `donut-slice bar-slot-${slice.colour_slot}`,
      'data-legend': keyOf(slice),
      role: 'img',
      tabindex: '0',
      'aria-label': sliceLabel(donut, slice),
    });
    // The title is the category; the rows are its amount (hidden in Discrete mode) and its percentage.
    const show = () => {
      const rows = [
        ...(isDiscrete() ? [] : tooltipRow(keyOf(slice), noSwatch(), t('Amount'), formatAmount(slice.amount))),
        ...tooltipRow(keyOf(slice), noSwatch(), t('Percentage'), formatLabel(slice.percent_label)),
      ];
      tooltip.show([h('div', { class: 'tip-title tip-category' }, swatch(`slot-${slice.colour_slot}`), categoryName(slice)), h('dl', {}, ...rows)], path, path);
    };
    path.addEventListener('mouseenter', show);
    path.addEventListener('focus', show);
    for (const off of ['mouseleave', 'blur']) path.addEventListener(off, () => tooltip.hide());
    group.append(path);
  }
  bindHighlight(group, [card]);
  const centre = svg(
    'text',
    { class: 'donut-centre', x: CENTRE, y: CENTRE, 'text-anchor': 'middle' },
    svg('tspan', { class: 'donut-total', x: CENTRE, dy: '-0.1em' }, formatAmount(shares.total)),
    svg('tspan', { class: 'donut-total-label', x: CENTRE, dy: '1.6em' }, t('Total spent')),
  );
  inner.append(svg('svg', { viewBox: '0 0 200 200', class: 'donut-svg', role: 'group', 'aria-label': `${donut.label(donut.period)}: ${t('Spending per category')}` }, group, centre), tooltip.element);
  chart.replaceChildren(inner);
  legend.replaceChildren(
    ...shares.slices.map((slice) => legendItem(keyOf(slice), swatch(`slot-${slice.colour_slot}`), legendLabel(categoryName(slice)), scopes)),
  );
}

/** Loads the shares of the period the donut shows and draws them; an answer that is no longer the latest is dropped. */
async function load(donut) {
  const request = ++donut.request;
  const period = donut.period;
  const shares = await invoke('transaction_spending_shares', { period });
  if (request === donut.request && period === donut.period) draw(donut, shares);
}

/** Fills the dropdown with the periods, keeping the choice while it is still there, else the newest. */
function fillSelect(donut, periods) {
  const select = $(`${donut.stem}-select`);
  const values = periods.map(String);
  donut.period = values.includes(donut.period) ? donut.period : values[0];
  select.replaceChildren(...values.map((value) => h('option', { value, selected: value === donut.period }, donut.label(value))));
  select.value = donut.period;
}

/** Reloads both donuts from Rust, or shows the empty note while nothing was spent. */
export async function reloadTransactionsSpending() {
  const periods = await invoke('transaction_spending_periods');
  const empty = periods.years.length === 0;
  $('transaction-spending-empty').hidden = !empty;
  $('transaction-spending-donuts').hidden = empty;
  if (empty) {
    donuts.year.period = null;
    donuts.month.period = null;
    return;
  }
  fillSelect(donuts.year, periods.years);
  fillSelect(donuts.month, periods.months);
  await Promise.all([load(donuts.year), load(donuts.month)]);
}

export function initTransactionsSpending() {
  for (const donut of Object.values(donuts)) {
    $(`${donut.stem}-select`).addEventListener('change', async (event) => {
      donut.period = event.target.value;
      try {
        await load(donut);
      } catch (error) {
        showToast(errorText(error));
      }
    });
  }
}
