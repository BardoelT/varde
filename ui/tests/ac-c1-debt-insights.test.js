// CSD-012: the display ACs of the Debts Insights tab (tabs, stacked bars, markers, colours, legend,
// tooltip, highlight, accessible labels, empty note). Rust computes the series, the pieces, the
// axis and the labels, so the tests feed canned answers of that shape and check only what is drawn.
// Amounts are shown with the digit grouping of CSD-022 ("158,000.00"). What needs real layout
// (tooltip placement, the plot size in pixels, the wrapping of the legend) cannot be seen in jsdom.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { setTimeout as sleep } from 'node:timers/promises';
import { openApp } from './harness.js';
import { emptyReads, exampleReads } from './fixtures.js';

const read = (file) => readFileSync(fileURLToPath(new URL(`../styles/${file}`, import.meta.url)), 'utf8');
const css = read('charts.css');
const baseCss = read('base.css');

// ---- canned answers of get_debt_insights ----

/** Debts in creation order: `[name, colour slot]` (the slot follows the creation order when omitted). */
const debtList = (...specs) => specs.map(([name, slot], index) => ({ id: `id-${index + 1}`, name, colour_slot: slot ?? index % 8 }));

const money = (value) => value.toFixed(2);
const short = (value) => `${(value / 1000).toFixed(2)}k`;

/**
 * A year as Rust returns it. `balances` lists `[debt id, balance]` in creation order (the stacking
 * order); a repaid debt (0.00) has a segment but no piece. `repaid` is the repaid text or null.
 */
function debtYear(year, balances, { partial = false, repaid = null, count = balances.length } = {}) {
  let low = 0;
  const visible = balances.filter(([, balance]) => Number(balance) > 0);
  const pieces = visible.map(([id, balance], index) => {
    const piece = { series: `debt-${id}`, low: money(low), high: money(low + Number(balance)), hangs: false, rounding: index === visible.length - 1 ? 'top' : 'none' };
    low += Number(balance);
    return piece;
  });
  const total = balances.reduce((sum, [, balance]) => sum + Number(balance), 0);
  return {
    year,
    is_partial: partial,
    statement_count: balances.length === 0 ? 0 : count,
    closing_balance: money(total),
    repaid,
    segments: balances.map(([id, balance]) => ({ debt_id: id, balance })),
    pieces,
    marker: balances.length === 0 ? null : { value: money(total), label: short(total) },
  };
}

const axisUpTo = (max) => ({ ticks: [{ value: '0.00', label: '0' }, { value: money(max), label: short(max) }] });
const series = (debts, years, max = 160000) => ({ axis: axisUpTo(max), debts, years });

/** The default answer: "Example Mortgage" and "Example Student Loan", in 2025 and 2026. */
function twoYears() {
  return series(debtList(['Example Mortgage'], ['Example Student Loan']), [
    debtYear(2025, [['id-1', '151000.00'], ['id-2', '8000.00']]),
    debtYear(2026, [['id-1', '150000.00'], ['id-2', '8000.00']], { partial: true, repaid: '1500.00' }),
  ]);
}

async function openChart(payload, reads = exampleReads()) {
  if (payload) reads.get_debt_insights = payload;
  const app = await openApp({ reads });
  await app.click(app.item('debts'));
  await app.click(app.$('debts-tab-insights'));
  return { app, chart: app.$('debts-insights-chart'), legend: app.$('debts-insights-legend') };
}

// ---- reading the drawn chart ----

const groups = (chart) => [...chart.querySelectorAll('.chart-year-group')];
const shapes = (group) => [...group.querySelectorAll('rect[data-legend], path[data-legend]')];
const num = (node, name) => Number(node.getAttribute(name));
const tooltipOf = (chart) => chart.querySelector('.chart-tooltip');
const yearTexts = (chart) => [...chart.querySelectorAll('.chart-year')].map((node) => node.textContent);
const marks = (chart, key) => [...chart.querySelectorAll(`[data-legend="${key}"]`)].filter((node) => node.matches('rect, path'));
const rowsOf = (app, chart) => [...tooltipOf(chart).querySelectorAll('dt')].map((dt) => app.text(dt));
const mouseover = (app, node) => node.dispatchEvent(new app.window.MouseEvent('mouseover', { bubbles: true }));

async function hover(app, group) {
  group.dispatchEvent(new app.window.MouseEvent('mouseenter'));
  await app.settle();
}

/** The gridlines with their label, top to bottom. */
function gridlines(chart) {
  const labels = [...chart.querySelectorAll('.chart-axis-label')];
  return [...chart.querySelectorAll('.chart-grid, .chart-zero')]
    .map((line, index) => ({ y: num(line, 'y1'), label: labels[index].textContent }))
    .sort((a, b) => a.y - b.y);
}

// ---- REQ-1: the tabs ----

test('test_csd012_ac_1_1_overview_is_shown_first', async () => {
  const app = await openApp();
  await app.click(app.item('debts'));
  assert.equal(app.$('debts-tab-overview').getAttribute('aria-selected'), 'true', 'the Overview tab is selected');
  assert.equal(app.$('debts-tab-insights').getAttribute('aria-selected'), 'false');
  const titles = [...app.$('debts-overview').querySelectorAll('.panel-title')].map((node) => app.text(node));
  assert.deepEqual(titles, ['Debts', 'Yearly totals', 'Statements']);
  assert.equal(app.isShown(app.$('debts-overview')), true);
  assert.equal(app.isShown(app.$('debts-insights')), false);
  app.close();
});

test('test_csd012_ac_1_2_switching_to_insights', async () => {
  const app = await openApp();
  await app.click(app.item('debts'));
  await app.click(app.$('debts-tab-insights'));
  assert.equal(app.isShown(app.$('debts-overview')), false, 'the Overview sections are hidden');
  assert.equal(app.isShown(app.$('debts-insights-scroll')), true, 'the insights chart is shown');
  assert.ok(app.$('debts-insights-chart').querySelector('svg .chart-year-group'));
  app.close();
});

test('test_csd012_ac_1_3_no_debts_yet_means_a_prompt_and_no_tabs', async () => {
  const app = await openApp({ reads: emptyReads() });
  await app.click(app.item('debts'));
  assert.equal(app.isShown(app.$('debts-empty')), true);
  assert.equal(app.text(app.$('debts-empty').querySelector('p')), 'No debts yet. Add your first debt to start tracking what you owe.');
  assert.equal(app.text(app.$('debt-add-first')), 'Add your first debt');
  assert.equal(app.isShown(app.$('debts-tab-overview')), false, 'there are no tabs');
  assert.equal(app.isShown(app.$('debts-tab-insights')), false);
  app.close();
});

// ---- REQ-6: the chart ----

test('test_csd012_ac_6_1_stacked_bars_per_year', async () => {
  const { app, chart } = await openChart(twoYears());
  const drawn = groups(chart);
  assert.equal(drawn.length, 2, 'two bars');
  assert.deepEqual(yearTexts(chart), ['2025', '2026 YTD'], '2025 on the left and 2026 on the right');
  assert.ok(num(drawn[0].querySelector('.chart-hit'), 'x') < num(drawn[1].querySelector('.chart-hit'), 'x'));
  for (const group of drawn) {
    assert.deepEqual(shapes(group).map((shape) => shape.dataset.legend), ['debt-id-1', 'debt-id-2'], 'one segment per debt, stacked in order');
  }
  const [mortgage, student] = shapes(drawn[0]);
  assert.ok(num(student, 'y') < num(mortgage, 'y'), 'stacked upwards: the second debt sits on top of the first');
  app.close();
});

test('test_csd012_ac_6_3_a_repaid_debt_adds_no_height_but_is_still_named', async () => {
  const payload = series(
    debtList(['Example Mortgage'], ['Example Car Loan']),
    [debtYear(2026, [['id-1', '150000.00'], ['id-2', '0.00']])],
    150000,
  );
  const { app, chart, legend } = await openChart(payload);
  const [group] = groups(chart);
  const bar = shapes(group);
  assert.equal(bar.length, 1, 'only one segment is drawn');
  const zero = num(chart.querySelector('.chart-zero'), 'y1');
  const top = gridlines(chart)[0];
  assert.equal(top.label, '150.00k');
  const box = group.querySelector('.chart-bar-box');
  assert.ok(Math.abs(num(box, 'y') - top.y) < 0.01, 'the bar reaches the tick of 150000.00');
  assert.ok(Math.abs(num(box, 'y') + num(box, 'height') - zero) < 0.01, 'and stands on the zero line, so the repaid debt added no height');
  assert.ok(app.text(legend).includes('Example Car Loan'), 'still named in the legend');
  await hover(app, group);
  assert.ok(rowsOf(app, chart).includes('Example Car Loan'), 'and in the tooltip');
  assert.ok(app.text(tooltipOf(chart)).includes('0.00'));
  app.close();
});

test('test_csd012_ac_6_4_a_gap_year_is_an_empty_labelled_slot', async () => {
  const payload = series(debtList(['Example Mortgage']), [
    debtYear(2022, [['id-1', '1000.00']]),
    debtYear(2023, []),
    debtYear(2024, [['id-1', '900.00']]),
  ], 1000);
  const { app, chart } = await openChart(payload);
  const gap = groups(chart)[1];
  assert.equal(gap.querySelector('.chart-year').textContent, '2023', 'a labelled slot');
  assert.equal(shapes(gap).length, 0, 'without a bar');
  assert.equal(gap.querySelector('.chart-marker'), null);
  app.close();
});

test('test_csd012_ac_6_5_the_current_year_is_marked', async () => {
  const { app, chart } = await openChart(twoYears());
  const [past, current] = groups(chart);
  assert.ok(current.classList.contains('partial'), 'the 2026 bar carries the class of the lighter fill');
  assert.ok(!past.classList.contains('partial'));
  assert.match(css, /\.chart-inner \.partial \[class\*="bar-slot-"\][^}]*opacity: \.55/, 'which lightens the fill');
  assert.equal(current.querySelector('.chart-year').textContent, '2026 YTD');
  app.close();
});

// ---- REQ-7: the markers ----

test('test_csd012_ac_7_3_markers_are_not_connected_and_the_current_year_is_dashed', async () => {
  const payload = series(debtList(['Example Mortgage']), [
    debtYear(2024, [['id-1', '1000.00']]),
    debtYear(2025, []),
    debtYear(2026, [['id-1', '900.00']], { partial: true }),
  ], 1000);
  const { app, chart } = await openChart(payload);
  const [first, gap, last] = groups(chart);
  assert.equal(chart.querySelectorAll('.chart-marker').length, 2, 'one line per marker and none joining them');
  assert.ok(first.querySelector('.chart-marker') && last.querySelector('.chart-marker'));
  assert.ok(last.classList.contains('partial'), 'the marker of 2026 sits in the partial group');
  assert.match(css, /\.partial \.chart-marker\s*\{[^}]*stroke-dasharray/, 'and the style sheet dashes it');
  assert.ok(!first.classList.contains('partial'), '2024 is solid');
  assert.equal(gap.querySelector('.chart-marker'), null, '2025 has no marker');
  app.close();
});

// ---- REQ-8: a colour per debt ----

test('test_csd012_ac_8_1_a_debt_has_one_colour_everywhere', async () => {
  const { app, chart, legend } = await openChart(twoYears());
  await hover(app, groups(chart)[1]);
  const pieces = marks(chart, 'debt-id-2');
  assert.equal(pieces.length, 2, 'its segments in 2025 and 2026');
  assert.ok(pieces.every((node) => node.classList.contains('bar-slot-1')), 'have the colour of slot 1');
  assert.ok(legend.querySelector('[data-legend="debt-id-2"] .swatch.slot-1'), 'the legend swatch');
  assert.ok(tooltipOf(chart).querySelector('dt[data-legend="debt-id-2"] .swatch.slot-1'), 'the tooltip swatch');
  app.close();
});

test('test_csd012_ac_8_2_colours_survive_a_deletion', async () => {
  // The debt with slot 0 was deleted on the Overview tab: Rust still sends the slots 1 and 2.
  const payload = series(debtList(['Example Student Loan', 1], ['Example Car Loan', 2]), [
    debtYear(2026, [['id-1', '8000.00'], ['id-2', '3000.00']]),
  ], 8000 + 3000);
  const { app, chart, legend } = await openChart(payload);
  assert.ok(marks(chart, 'debt-id-1')[0].classList.contains('bar-slot-1'));
  assert.ok(marks(chart, 'debt-id-2')[0].classList.contains('bar-slot-2'));
  assert.ok(legend.querySelector('[data-legend="debt-id-1"] .swatch.slot-1'));
  assert.ok(legend.querySelector('[data-legend="debt-id-2"] .swatch.slot-2'));
  app.close();
});

/** The `--bank-n` colours of the light block and of the two dark blocks of base.css, as `[h, s, l]`. */
function palette(block) {
  const colours = [...block.matchAll(/--bank-(\d): hsl\((\d+), (\d+)%, (\d+)%\)/g)];
  return colours.map((match) => ({ slot: Number(match[1]), h: Number(match[2]), s: Number(match[3]), l: Number(match[4]) }));
}

test('test_csd012_ac_8_3_the_palette_in_both_modes', async () => {
  const blocks = {
    light: baseCss.slice(baseCss.indexOf(':root {'), baseCss.indexOf('@media (prefers-color-scheme: dark)')),
    dark: baseCss.slice(baseCss.indexOf(':root[data-theme="dark"]')),
  };
  for (const [mode, block] of Object.entries(blocks)) {
    const colours = palette(block);
    assert.equal(colours.length, 8, `${mode}: eight colours`);
    assert.equal(new Set(colours.map((c) => `${c.h},${c.s},${c.l}`)).size, 8, `${mode}: distinct from each other`);
    assert.ok(colours.every((c) => c.s <= 60), `${mode}: at most 60% HSL saturation`);
  }
  assert.match(css, /\.swatch\.slot-7, \.bar-slot-7 \{[^}]*var\(--bank-7\)/, 'the chart uses the palette tokens');
});

test('test_csd012_ac_8_4_more_than_eight_debts_share_colours_but_are_all_named', async () => {
  const names = Array.from({ length: 9 }, (_, index) => `Example Debt ${index + 1}`);
  const debts = debtList(...names.map((name) => [name]));
  const payload = series(debts, [debtYear(2026, debts.map((debt) => [debt.id, '100.00']))], 900);
  const { app, chart, legend } = await openChart(payload);
  const colourOf = (id) => marks(chart, `debt-${id}`)[0].getAttribute('class');
  assert.equal(colourOf('id-1'), colourOf('id-9'), 'the first and the ninth share a colour');
  assert.notEqual(colourOf('id-1'), colourOf('id-2'));
  for (const name of names) assert.ok(app.text(legend).includes(name), `${name} is in the legend`);
  await hover(app, groups(chart)[0]);
  assert.ok(rowsOf(app, chart).includes('Example Debt 9'), 'and in the tooltip');
  app.close();
});

// ---- REQ-9: the legend ----

test('test_csd012_ac_9_1_the_legend_lists_the_debts_in_creation_order_and_the_marker', async () => {
  const payload = series(debtList(['Example Student Loan'], ['Example Mortgage']), [
    debtYear(2026, [['id-1', '8000.00'], ['id-2', '150000.00']]),
  ]);
  const { app, legend } = await openChart(payload);
  const items = [...legend.querySelectorAll('li')];
  assert.deepEqual(items.map((item) => app.text(item)), ['Example Student Loan', 'Example Mortgage', 'Total debt']);
  assert.ok(items[0].querySelector('.swatch.slot-0') && items[1].querySelector('.swatch.slot-1'), 'each with its colour swatch');
  assert.ok(items[2].querySelector('.swatch.marker'), 'the legend shows the total-debt marker');
  app.close();
});

test('test_csd012_ac_9_3_many_debts_wrap_and_none_is_left_out', async () => {
  const debts = debtList(...Array.from({ length: 12 }, (_, index) => [`Example Debt ${index + 1}`]));
  const payload = series(debts, [debtYear(2026, debts.map((debt) => [debt.id, '100.00']))], 1200);
  const { app, legend } = await openChart(payload);
  const names = [...legend.querySelectorAll('li:not([data-legend="marker"])')].map((item) => app.text(item));
  assert.equal(names.length, 12, 'all twelve debts are shown');
  assert.match(css, /\.legend\s*\{[^}]*flex-wrap:\s*wrap/, 'and the legend wraps onto more lines');
  app.close();
});

test('test_csd012_ac_9_4_a_long_name_is_cut_with_an_ellipsis_and_kept_in_full', async () => {
  const name = 'Example '.repeat(12).trim().padEnd(100, 'x');
  assert.equal(name.length, 100);
  const payload = series(debtList([name]), [debtYear(2026, [['id-1', '1000.00']])], 1000);
  const { app, chart, legend } = await openChart(payload);
  const label = legend.querySelector('[data-legend="debt-id-1"] .legend-name');
  assert.equal(label.dataset.text, name, 'the full name is in the legend item');
  assert.equal(label.textContent, name, 'and for screen readers');
  assert.ok(label.querySelector('.legend-name-text'));
  assert.match(css, /\.legend-name-text\s*\{[^}]*text-overflow:\s*ellipsis/, 'cut with an ellipsis');
  await hover(app, groups(chart)[0]);
  assert.ok(rowsOf(app, chart).includes(name), 'the tooltip shows it in full');
  app.close();
});

test('test_csd012_ac_9_5_a_deleted_debt_leaves_the_legend', async () => {
  const { app, legend } = await openChart(twoYears());
  assert.deepEqual([...legend.querySelectorAll('li')].map((item) => app.text(item)), ['Example Mortgage', 'Example Student Loan', 'Total debt']);
  // The student loan is deleted on the Overview tab: Rust now answers without it.
  await app.click(app.$('debts-tab-overview'));
  app.bridge.table.get_debt_insights = series(debtList(['Example Mortgage']), [debtYear(2026, [['id-1', '150000.00']])]);
  await app.click(app.$('debts-tab-insights'));
  assert.deepEqual([...app.$('debts-insights-legend').querySelectorAll('li')].map((item) => app.text(item)), ['Example Mortgage', 'Total debt']);
  app.close();
});

// ---- REQ-10: the tooltip ----

/** 2026: "Example Mortgage" 150000.00 first in stacking order, "Example Student Loan" 8000.00 on top. */
const detailYear = (repaid = '1500.00', extra = {}) => series(debtList(['Example Mortgage'], ['Example Student Loan']), [
  debtYear(2026, [['id-1', '150000.00'], ['id-2', '8000.00']], { repaid, ...extra }),
]);

test('test_csd012_ac_10_1_the_tooltip_shows_the_debts_the_total_and_the_repaid_amount', async () => {
  const { app, chart } = await openChart(detailYear('1500.00', { count: 5 }));
  await hover(app, groups(chart)[0]);
  const tooltip = tooltipOf(chart);
  assert.equal(tooltip.classList.contains('is-open'), true);
  assert.equal(app.text(tooltip.querySelector('.tip-title')), '2026');
  const pairs = [...tooltip.querySelectorAll('dt')].map((dt) => [app.text(dt), app.text(dt.nextElementSibling)]);
  assert.deepEqual(pairs, [
    ['Example Student Loan', '8,000.00'],
    ['Example Mortgage', '150,000.00'],
    ['Total debt', '158,000.00'],
    ['Repaid', '1,500.00'],
  ]);
  assert.ok(tooltip.querySelector('dt[data-legend="debt-id-1"] .swatch.slot-0'), 'each debt with its swatch');
  assert.doesNotMatch(app.text(tooltip), /\b5\b|statements/i, 'no number of statements');
  app.close();
});

test('test_csd012_ac_10_2_the_tooltip_follows_keyboard_focus', async () => {
  const { app, chart } = await openChart(twoYears());
  const [first, second] = groups(chart);
  const tooltip = tooltipOf(chart);
  first.dispatchEvent(new app.window.Event('focus'));
  assert.equal(tooltip.classList.contains('is-open'), true);
  assert.equal(app.text(tooltip.querySelector('.tip-title')), '2025');
  second.dispatchEvent(new app.window.Event('focus'));
  assert.equal(app.text(tooltip.querySelector('.tip-title')), '2026 YTD', 'the same tooltip of the focused year');
  assert.ok(rowsOf(app, chart).includes('Total debt'));
  second.dispatchEvent(new app.window.Event('blur'));
  await sleep(150);
  assert.equal(tooltip.classList.contains('is-open'), false, 'it disappears when focus leaves the bar');
  assert.equal(first.getAttribute('tabindex'), '0', 'every bar is in the tab order');
  app.close();
});

test('test_csd012_ac_10_3_the_accessible_label_says_a_negative_repaid_amount_is_negative', async () => {
  const { app, chart } = await openChart(detailYear('-200.00'));
  const spoken = groups(chart)[0].getAttribute('aria-label');
  assert.ok(spoken.startsWith('2026'), 'the year');
  assert.ok(spoken.includes('Example Mortgage 150,000.00') && spoken.includes('Example Student Loan 8,000.00'), 'each debt with its balance');
  assert.ok(spoken.includes('total debt 158,000.00'), 'the total debt');
  assert.ok(spoken.includes('repaid -200.00 (negative)'), 'the amount repaid and that it is negative');
  assert.doesNotMatch(spoken, /statement/i, 'no number of statements');
  app.close();
});

test('test_csd012_ac_10_4_a_partial_year_says_so_in_the_tooltip', async () => {
  const { app, chart } = await openChart(detailYear('1500.00', { partial: true }));
  await hover(app, groups(chart)[0]);
  assert.equal(app.text(tooltipOf(chart).querySelector('.tip-title')), '2026 YTD');
  assert.equal(app.text(tooltipOf(chart).querySelector('.tip-note')), 'Partial year: statements so far');
  app.close();
});

test('test_csd012_ac_10_5_repaid_none_is_a_dash', async () => {
  const payload = series(debtList(['Example Mortgage']), [debtYear(2025, [['id-1', '10000.00']], { repaid: null })], 10000);
  const { app, chart } = await openChart(payload);
  await hover(app, groups(chart)[0]);
  const repaid = [...tooltipOf(chart).querySelectorAll('dt')].find((dt) => app.text(dt) === 'Repaid');
  assert.equal(app.text(repaid.nextElementSibling), '–', 'a dash as repaid');
  assert.ok(groups(chart)[0].getAttribute('aria-label').includes('repaid none'));
  app.close();
});

test('test_csd012_ac_10_7_a_year_without_statements_shows_the_note_and_no_rows', async () => {
  const payload = series(debtList(['Example Mortgage']), [
    debtYear(2022, [['id-1', '1000.00']]),
    debtYear(2023, []),
    debtYear(2024, [['id-1', '900.00']]),
  ], 1000);
  const { app, chart } = await openChart(payload);
  await hover(app, groups(chart)[1]);
  const tooltip = tooltipOf(chart);
  assert.equal(app.text(tooltip.querySelector('.tip-title')), '2023');
  assert.equal(app.text(tooltip.querySelector('.tip-note')), 'No statements', 'the note');
  assert.equal(tooltip.querySelectorAll('dt').length, 0, 'no debt rows and no row with a number of statements');
  assert.equal(groups(chart)[1].getAttribute('aria-label'), '2023: no statements');
  app.close();
});

test('test_csd012_ac_10_8_rows_follow_the_bar_from_top_to_bottom', async () => {
  const { app, chart } = await openChart(detailYear('1500.00'));
  await hover(app, groups(chart)[0]);
  assert.deepEqual(rowsOf(app, chart), ['Example Student Loan', 'Example Mortgage', 'Total debt', 'Repaid']);
  const [mortgage, student] = shapes(groups(chart)[0]);
  assert.equal(mortgage.dataset.legend, 'debt-id-1');
  assert.ok(num(student, 'y') < num(mortgage, 'y'), 'the student loan is at the top of the bar');
  app.close();
});

test('test_csd012_ac_10_9_the_accessible_label_has_the_values_without_a_count', async () => {
  const { app, chart } = await openChart(detailYear('1500.00', { count: 7 }));
  const spoken = groups(chart)[0].getAttribute('aria-label');
  assert.equal(spoken, '2026: Example Mortgage 150,000.00, Example Student Loan 8,000.00, total debt 158,000.00, repaid 1,500.00');
  assert.doesNotMatch(spoken, /\b7\b|statement/i);
  app.close();
});

// ---- REQ-11: highlight one debt ----

test('test_csd012_ac_11_1_hovering_a_segment_highlights_its_debt_in_every_year', async () => {
  const { app, chart, legend } = await openChart(twoYears());
  mouseover(app, shapes(groups(chart)[0])[1]); // the student loan of 2025
  const student = marks(chart, 'debt-id-2');
  const mortgage = marks(chart, 'debt-id-1');
  assert.equal(student.length, 2);
  assert.ok(student.every((node) => node.classList.contains('hl') && !node.classList.contains('dim')), 'every student loan segment keeps its colour');
  assert.ok(mortgage.every((node) => node.classList.contains('dim')), 'every mortgage segment is eased down');
  assert.ok(legend.querySelector('[data-legend="debt-id-2"]').classList.contains('hl'), 'the student loan legend item is highlighted');
  assert.ok(legend.querySelector('[data-legend="debt-id-1"]').classList.contains('dim'), 'the other legend items are eased down');
  assert.match(css, /\.legend li\.hl \.legend-label[^}]*font-weight: 600/, 'in a heavier font weight');
  assert.match(css, /\.legend li\.hl[^{]*\{[^}]*color: var\(--text\)/, 'and the full text colour');
  assert.match(css, /\.legend li\.dim \{ opacity: \.7;? \}/, 'the others at 70% opacity');
  assert.match(css, /\.chart-year-group \[data-legend\]\.dim \{ opacity: \.7;? \}/, 'also the segments');
  app.close();
});

test('test_csd012_ac_11_2_hovering_a_legend_item_highlights_its_debt', async () => {
  const { app, chart, legend } = await openChart(twoYears());
  legend.querySelector('[data-legend="debt-id-2"]').dispatchEvent(new app.window.Event('mouseenter'));
  assert.ok(marks(chart, 'debt-id-2').every((node) => node.classList.contains('hl')), 'its segments keep their full colour in every year');
  assert.ok(marks(chart, 'debt-id-1').every((node) => node.classList.contains('dim')), 'the other segments are eased down');
  legend.querySelector('[data-legend="debt-id-2"]').dispatchEvent(new app.window.Event('mouseleave'));
  assert.equal(chart.querySelectorAll('.hl, .dim').length, 0);
  app.close();
});

test('test_csd012_ac_11_3_focusing_a_legend_item_highlights_like_hovering', async () => {
  const { app, chart, legend } = await openChart(twoYears());
  const item = legend.querySelector('[data-legend="debt-id-2"]');
  assert.equal(item.getAttribute('tabindex'), '0', 'it can take keyboard focus');
  item.dispatchEvent(new app.window.Event('focus'));
  assert.ok(marks(chart, 'debt-id-2').every((node) => node.classList.contains('hl')));
  assert.ok(marks(chart, 'debt-id-1').every((node) => node.classList.contains('dim')));
  assert.ok(item.classList.contains('hl'));
  item.dispatchEvent(new app.window.Event('blur'));
  assert.equal(chart.querySelectorAll('.hl, .dim').length + legend.querySelectorAll('.hl, .dim').length, 0, 'it ends when focus leaves');
  app.close();
});

test('test_csd012_ac_11_4_leaving_the_segment_highlights_nothing_and_keeps_the_tooltip', async () => {
  const { app, chart, legend } = await openChart(twoYears());
  const [group] = groups(chart);
  await hover(app, group);
  mouseover(app, shapes(group)[0]);
  assert.ok(chart.querySelectorAll('.hl').length > 0);
  mouseover(app, group.querySelector('.chart-hit')); // empty space in the year's column
  assert.equal(chart.querySelectorAll('.hl, .dim').length + legend.querySelectorAll('.hl, .dim').length, 0, 'nothing is highlighted or dimmed');
  assert.equal(tooltipOf(chart).classList.contains('is-open'), true, 'the tooltip is still shown');
  app.close();
});

test('test_csd012_ac_11_5_the_highlight_is_not_colour_alone', async () => {
  const { app, chart, legend } = await openChart(twoYears());
  const before = marks(chart, 'debt-id-1')[0].getAttribute('class');
  mouseover(app, shapes(groups(chart)[0])[1]);
  const dimmed = marks(chart, 'debt-id-1')[0];
  assert.equal(dimmed.getAttribute('class').replace(' dim', ''), before, 'the other segments are lighter, not differently coloured');
  assert.ok(legend.querySelector('[data-legend="debt-id-2"]').classList.contains('hl'), 'the highlighted item');
  assert.match(css, /\.legend li\.hl \.legend-label[^}]*font-weight: 600/, 'has a heavier font weight than the other items');
  app.close();
});

test('test_csd012_ac_11_6_keyboard_focus_on_a_year_highlights_nothing', async () => {
  const { app, chart, legend } = await openChart(twoYears());
  groups(chart)[0].dispatchEvent(new app.window.Event('focus'));
  assert.equal(chart.querySelectorAll('.hl, .dim').length + legend.querySelectorAll('.hl, .dim').length, 0);
  app.close();
});

// ---- REQ-13: the shared look, accessible segments ----

test('test_csd012_ac_13_1_the_plot_and_the_slot_width_are_those_of_the_bank_accounts_chart', async () => {
  const debts = await openChart(twoYears());
  const debtBox = debts.chart.querySelector('svg').getAttribute('viewBox');
  const debtCentres = [...debts.chart.querySelectorAll('.chart-year')].map((node) => num(node, 'x'));
  assert.ok(debts.chart.querySelector('.chart-zero') && debts.chart.querySelector('.chart-axis-label'), 'gridlines, zero line and labels');
  debts.app.close();

  const reads = exampleReads();
  const bankYear = (year) => ({
    year, is_partial: false, entry_count: 1, closing_balance: '1000.00', marker: { value: '1000.00', label: '1.00k' },
    segments: [{ account_id: 'id-1', balance: '1000.00' }],
    pieces: [{ series: 'bank-id-1', low: '0.00', high: '1000.00', hangs: false, rounding: 'top' }],
  });
  reads.get_bank_insights = {
    axis: axisUpTo(1000), banks: [{ id: 'id-1', name: 'Example Bank', colour_slot: 0 }], years: [bankYear(2025), bankYear(2026)],
  };
  const app = await openApp({ reads });
  await app.click(app.item('bank'));
  await app.click(app.$('bank-tab-insights'));
  const bank = app.$('bank-insights-chart');
  assert.equal(bank.querySelector('svg').getAttribute('viewBox'), debtBox, 'the same plot');
  assert.deepEqual([...bank.querySelectorAll('.chart-year')].map((node) => num(node, 'x')), debtCentres, 'and the same slot width');
  app.close();
});

test('test_csd012_ac_13_4_every_segment_has_an_accessible_label', async () => {
  const { app, chart } = await openChart(twoYears());
  const [, current] = groups(chart);
  const student = shapes(current)[1];
  assert.equal(student.getAttribute('aria-label'), 'Example Student Loan, 2026, 8,000.00');
  for (const group of groups(chart)) {
    for (const shape of shapes(group)) assert.ok(shape.getAttribute('aria-label'), 'a label with the debt, the year and the balance');
  }
  app.close();
});

// ---- REQ-14: no statements yet ----

test('test_csd012_ac_14_1_debts_without_statements_show_a_note_and_no_chart', async () => {
  const { app, chart } = await openChart({ axis: axisUpTo(1), debts: [], years: [] });
  assert.equal(app.isShown(app.$('debts-insights-empty')), true);
  assert.equal(app.textOf('debts-insights-empty'), 'No statements yet. The chart appears once there is a statement.');
  assert.equal(app.isShown(app.$('debts-insights-scroll')), false);
  assert.equal(chart.querySelectorAll('svg').length, 0, 'no chart is drawn');
  app.close();
});
