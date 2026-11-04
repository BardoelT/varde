// CSD-017 display ACs: the Net worth page, its bars, legend, highlight, tooltip and accessible texts.
// The answers are what Rust returns for small series made for one AC; the page only displays them.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { openApp } from './harness.js';
import { exampleReads } from './fixtures.js';

const read = (name) => readFileSync(fileURLToPath(new URL(`../styles/${name}`, import.meta.url)), 'utf8');
const baseCss = read('base.css');

const piece = (series, low, high, hangs = false) => ({ series, low, high, hangs, rounding: 'none' });
const ticks = (...values) => ({ ticks: values.map((value) => ({ value, label: value })) });

/** The year with houses 300000.00, investments 20000.00, cash 5000.00 and debts 9000.00. */
const fullYear = (year, partial = false) => ({
  year,
  is_partial: partial,
  has_records: true,
  houses: '300000.00',
  investments: '20000.00',
  cash: '5000.00',
  debts: '9000.00',
  net_worth: '316000.00',
  pieces: [
    piece('houses', '0.00', '300000.00'),
    piece('investments', '300000.00', '320000.00'),
    piece('cash', '320000.00', '325000.00'),
    piece('debts', '-9000.00', '0.00', true),
  ],
  marker: { value: '316000.00', label: '316.00k' },
});

const emptyYear = (year) => ({
  year,
  is_partial: false,
  has_records: false,
  houses: '0.00',
  investments: '0.00',
  cash: '0.00',
  debts: '0.00',
  net_worth: '0.00',
  pieces: [],
  marker: null,
});

const cashYear = (year, cash, partial = false) => ({
  year,
  is_partial: partial,
  has_records: true,
  houses: '0.00',
  investments: '0.00',
  cash,
  debts: '0.00',
  net_worth: cash,
  pieces: [piece('cash', '0.00', cash)],
  marker: { value: cash, label: cash },
});

async function openNetWorth(years, axis = ticks('-9000.00', '0.00', '325000.00'), reads = exampleReads()) {
  reads.get_net_worth_insights = { axis, years };
  const app = await openApp({ reads });
  await app.click(app.item('net-worth'));
  const chart = app.$('net-worth-insights-chart');
  const groups = () => [...chart.querySelectorAll('.chart-year-group')];
  const shapes = (group) => [...group.querySelectorAll('rect[data-legend], path[data-legend]')];
  const hover = async (group) => {
    group.dispatchEvent(new app.window.MouseEvent('mouseenter'));
    await app.settle();
  };
  const tooltip = () => chart.querySelector('.chart-tooltip');
  return { app, chart, groups, shapes, hover, tooltip, legend: app.$('net-worth-insights-legend') };
}

test('test_csd017_ac_1_1_one_panel_and_no_tabs', async () => {
  const { app } = await openNetWorth([cashYear(2025, '1000.00')]);
  const section = app.$('net-worth');
  const panels = section.querySelectorAll('.panel');
  assert.equal(panels.length, 1);
  assert.equal(app.text(panels[0].querySelector('.panel-title')), 'Net worth per year');
  assert.equal(section.querySelectorAll('[role="tab"], [role="tablist"], .tabs').length, 0, 'there are no tabs');
  assert.ok(panels[0].contains(app.$('net-worth-insights-legend')) && panels[0].contains(app.$('net-worth-insights-chart')));
  app.close();
});

test('test_csd017_ac_5_2_a_past_year_is_labelled_plainly_and_drawn_at_full_opacity', async () => {
  const { app, groups } = await openNetWorth([cashYear(2025, '1000.00'), cashYear(2026, '2000.00', true)]);
  const [past, running] = groups();
  assert.equal(past.querySelector('.chart-year').textContent, '2025');
  assert.equal(past.classList.contains('partial'), false);
  assert.equal(running.querySelector('.chart-year').textContent, '2026 YTD');
  assert.equal(running.classList.contains('partial'), true);
  app.close();
});

test('test_csd017_ac_6_1_the_bar_rises_from_houses_to_investments_to_cash_and_debts_hang_below_zero', async () => {
  const { app, groups, shapes } = await openNetWorth([fullYear(2025)]);
  const bars = shapes(groups()[0]);
  const keys = bars.map((bar) => bar.getAttribute('data-legend'));
  assert.deepEqual(keys, ['houses', 'investments', 'cash', 'debts']);
  const y = (bar) => Number(bar.getAttribute('y'));
  const height = (bar) => Number(bar.getAttribute('height'));
  const [houses, investments, cash, debts] = bars;
  // Higher on the screen means a smaller y: Houses is the lowest piece above zero, Cash the highest.
  assert.ok(y(houses) > y(investments) && y(investments) > y(cash), 'Houses at the bottom, then Investments, then Cash on top');
  const zeroLine = Number(app.$('net-worth-insights-chart').querySelector('.chart-zero').getAttribute('y1'));
  assert.ok(Math.abs(y(houses) + height(houses) - zeroLine) < 1.5, 'Houses stands on the zero line');
  assert.ok(y(debts) >= zeroLine - 1, 'Debts hangs below the zero line');
  app.close();
});

test('test_csd017_ac_6_2_the_chart_has_the_frame_of_the_bank_accounts_chart', async () => {
  const reads = exampleReads();
  const app = await openApp({ reads });
  await app.click(app.item('bank'));
  await app.click(app.$('bank-tab-insights'));
  const bank = app.$('bank-insights-chart');
  await app.click(app.item('net-worth'));
  const worth = app.$('net-worth-insights-chart');
  const frame = (chart) => ({
    svg: chart.querySelector('svg').getAttribute('viewBox').split(/\s+/).slice(2).join('x'),
    classes: ['.chart-zero', '.chart-grid', '.chart-axis-label', '.chart-year-group', '.chart-year', '.chart-hit', '.chart-tooltip'].map((selector) => chart.querySelector(selector) !== null),
  });
  assert.deepEqual(frame(worth), frame(bank));
  assert.deepEqual(frame(worth).classes, [true, true, true, true, true, true, true]);
  app.close();
});

test('test_csd017_ac_6_4_a_negative_net_worth_reaches_below_the_deepest_debts_on_the_axis', async () => {
  const year = {
    ...cashYear(2025, '1000.00'),
    debts: '9000.00',
    net_worth: '-8000.00',
    pieces: [piece('cash', '0.00', '1000.00'), piece('debts', '-9000.00', '0.00', true)],
    marker: { value: '-8000.00', label: '-8.00k' },
  };
  const { app, chart } = await openNetWorth([year], ticks('-10000.00', '0.00', '1000.00'));
  const labels = [...chart.querySelectorAll('.chart-axis-label')].map((node) => node.textContent);
  assert.ok(labels.includes('-10000.00'), `the axis reaches below -9000.00: ${labels}`);
  const lowest = Math.max(...[...chart.querySelectorAll('.chart-grid, .chart-zero')].map((line) => Number(line.getAttribute('y1'))));
  const bottomOfDebts = Math.max(...[...chart.querySelectorAll('rect[data-legend="debts"]')].map((bar) => Number(bar.getAttribute('y')) + Number(bar.getAttribute('height'))));
  assert.ok(lowest >= bottomOfDebts - 1, 'the plot reaches the bottom of the Debts piece');
  app.close();
});

test('test_csd017_ac_8_1_the_legend_lists_the_four_series_and_the_net_worth_marker', async () => {
  const { app, legend } = await openNetWorth([fullYear(2025)]);
  const items = [...legend.querySelectorAll('[data-legend]')];
  assert.deepEqual(items.map((item) => app.text(item)), ['Houses', 'Investments', 'Cash', 'Debts', 'Net worth']);
  assert.deepEqual(items.map((item) => item.getAttribute('data-legend')), ['houses', 'investments', 'cash', 'debts', 'marker']);
  assert.ok(items.every((item) => item.querySelector('.swatch')), 'each item has its swatch');
  app.close();
});

/** The `hsl(h, s%, l%)` of a token in the block that starts at `selector`, as [h, s, l]. */
function token(selector, name) {
  const start = baseCss.indexOf(selector);
  assert.ok(start >= 0, selector);
  const block = baseCss.slice(baseCss.indexOf('{', start), baseCss.indexOf('}', start));
  const match = block.match(new RegExp(`${name}:\\s*hsl\\(([\\d.]+),\\s*([\\d.]+)%,\\s*([\\d.]+)%\\)`));
  assert.ok(match, `${name} in ${selector}`);
  return match.slice(1).map(Number);
}

test('test_csd017_ac_8_2_dark_mode_has_four_lighter_colours_that_stay_distinct', async () => {
  // Houses, Investments and Cash are palette slots 4, 0 and 1 and Debts is the loss colour (base.css).
  assert.match(baseCss, /--nw-houses: var\(--bank-4\); --nw-investments: var\(--bank-0\); --nw-cash: var\(--bank-1\); --nw-debts: var\(--chart-loss\);/);
  const names = ['--bank-4', '--bank-0', '--bank-1', '--chart-loss'];
  const light = names.map((name) => token(':root {', name));
  const dark = names.map((name) => token(':root[data-theme="dark"]', name));
  dark.forEach((colour, index) => assert.ok(colour[2] > light[index][2], `${names[index]} is lighter in dark mode`));
  for (const colours of [light, dark]) {
    const hues = colours.map(([hue]) => hue);
    assert.equal(new Set(hues).size, 4, 'the four hues differ');
    for (let a = 0; a < 4; a += 1) for (let b = a + 1; b < 4; b += 1) assert.ok(Math.abs(hues[a] - hues[b]) >= 25, `${names[a]} and ${names[b]} stay apart`);
  }
});

test('test_csd017_ac_9_1_hovering_a_legend_item_highlights_that_series_in_every_year', async () => {
  const { app, chart, legend } = await openNetWorth([cashYear(2024, '1000.00'), fullYear(2025), fullYear(2026, true)]);
  const item = legend.querySelector('[data-legend="cash"]');
  item.dispatchEvent(new app.window.Event('mouseenter'));
  const cash = [...chart.querySelectorAll('rect[data-legend="cash"]')];
  const others = [...chart.querySelectorAll('rect[data-legend]')].filter((bar) => bar.getAttribute('data-legend') !== 'cash');
  assert.equal(cash.length, 3, 'a Cash segment in each of the three years');
  assert.ok(cash.every((bar) => bar.classList.contains('hl') && !bar.classList.contains('dim')));
  assert.ok(others.length > 0 && others.every((bar) => bar.classList.contains('dim')), 'the other segments are dimmed');
  assert.ok(item.classList.contains('hl'), 'the legend item is highlighted');
  app.close();
});

test('test_csd017_ac_9_2_leaving_the_legend_item_returns_every_segment_to_full_opacity', async () => {
  const { app, chart, legend } = await openNetWorth([fullYear(2025), fullYear(2026)]);
  const item = legend.querySelector('[data-legend="cash"]');
  item.dispatchEvent(new app.window.Event('mouseenter'));
  assert.ok(chart.querySelectorAll('.dim').length > 0);
  item.dispatchEvent(new app.window.Event('mouseleave'));
  assert.equal(chart.querySelectorAll('.hl, .dim').length + legend.querySelectorAll('.hl, .dim').length, 0);
  app.close();
});

test('test_csd017_ac_10_1_the_tooltip_rows_read_like_the_bar_with_debts_as_a_minus', async () => {
  const { app, groups, hover, tooltip } = await openNetWorth([fullYear(2025)]);
  await hover(groups()[0]);
  const box = tooltip();
  assert.equal(box.classList.contains('is-open'), true);
  assert.equal(app.text(box.querySelector('.tip-title')), '2025');
  const rows = [...box.querySelectorAll('dt')].map((dt, index) => [app.text(dt), app.text(box.querySelectorAll('dd')[index])]);
  assert.deepEqual(rows, [
    ['Cash', '5,000.00'],
    ['Investments', '20,000.00'],
    ['Houses', '300,000.00'],
    ['Debts', '-9,000.00'],
    ['Net worth', '316,000.00'],
  ]);
  app.close();
});

test('test_csd017_ac_10_2_a_year_without_records_shows_its_title_and_the_note', async () => {
  const { app, groups, hover, tooltip } = await openNetWorth([cashYear(2024, '1000.00'), emptyYear(2025), cashYear(2026, '1000.00')]);
  await hover(groups()[1]);
  const box = tooltip();
  assert.equal(app.text(box.querySelector('.tip-title')), '2025');
  assert.match(app.text(box), /No records/);
  assert.equal(box.querySelectorAll('dt').length, 0, 'no figures are listed');
  app.close();
});

test('test_csd017_ac_10_3_focusing_a_year_with_the_keyboard_shows_the_same_tooltip', async () => {
  const { app, groups, hover, tooltip } = await openNetWorth([fullYear(2025), fullYear(2026)]);
  await hover(groups()[0]);
  const hovered = app.text(tooltip());
  groups()[0].dispatchEvent(new app.window.Event('mouseleave'));
  groups()[0].dispatchEvent(new app.window.Event('blur'));
  groups()[0].dispatchEvent(new app.window.Event('focus'));
  assert.equal(tooltip().classList.contains('is-open'), true);
  assert.equal(app.text(tooltip()), hovered);
  assert.equal(groups()[0].getAttribute('tabindex'), '0', 'a bar can take the focus');
  app.close();
});

test('test_csd017_ac_11_1_a_segment_announces_its_series_year_and_value', async () => {
  const { app, groups, shapes } = await openNetWorth([cashYear(2025, '5000.00')]);
  const [segment] = shapes(groups()[0]);
  assert.equal(segment.getAttribute('aria-label'), 'Cash, 2025, 5,000.00');
  const described = groups()[0].getAttribute('aria-label');
  for (const part of ['2025', 'Houses 0.00', 'Investments 0.00', 'Cash 5,000.00', 'Debts 0.00', 'net worth 5,000.00']) assert.ok(described.includes(part), `${part} in "${described}"`);
  app.close();
});

test('test_csd017_ac_13_1_the_figures_follow_changes_when_the_page_is_opened_again', async () => {
  const { app, chart } = await openNetWorth([cashYear(2026, '5000.00', true)]);
  const cashLabel = () => chart.querySelector('rect[data-legend="cash"]').getAttribute('aria-label');
  assert.equal(cashLabel(), 'Cash, 2026, 5,000.00');
  // A bank account entry of 6000.00 is added elsewhere: Rust now answers with the new figure.
  app.bridge.table.get_net_worth_insights = { axis: ticks('0.00', '6000.00'), years: [cashYear(2026, '6000.00', true)] };
  await app.click(app.item('bank'));
  await app.click(app.item('net-worth'));
  assert.equal(cashLabel(), 'Cash, 2026, 6,000.00');
  app.close();
});
