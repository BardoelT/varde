// CSD-025: the yearly charts draw what Rust returns (axis ticks, pieces, markers, labels) and
// compute nothing. The answers are the golden ones of the example project, or small series made
// for one AC. jsdom has no layout, so what needs sizes in pixels stays on the manual checklist:
// AC-8.2 and AC-8.3 (placement of the tooltip), AC-10.1 (timing of the motion), AC-11.1 (height of
// the plot follows the window), AC-12.1 to AC-12.3 (radii, opacity, fonts), AC-13.1 (wrapping),
// AC-14.2 (the palette in both modes). AC-3.x, AC-4.x and AC-17.x are proven by `cargo test`.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { setTimeout as sleep } from 'node:timers/promises';
import { openApp } from './harness.js';
import { exampleReads } from './fixtures.js';

const css = readFileSync(fileURLToPath(new URL('../styles/charts.css', import.meta.url)), 'utf8');

/** Every chart: the page, its command, the ids of its chart, and how the page opens it. */
const CHARTS = [
  { page: 'income', command: 'get_income_insights', chart: 'income-insights-chart', legend: 'income-insights-legend' },
  { page: 'investments', command: 'get_investment_insights', chart: 'investments-insights-chart', legend: 'investments-insights-legend' },
  { page: 'bank', command: 'get_bank_insights', chart: 'bank-insights-chart', legend: 'bank-insights-legend' },
  { page: 'houses', command: 'get_house_insights', chart: 'houses-insights-chart', legend: 'houses-insights-legend' },
  { page: 'debts', command: 'get_debt_insights', chart: 'debts-insights-chart', legend: 'debts-insights-legend' },
  { page: 'expenses', command: 'get_expense_insights', chart: 'expenses-insights-chart', legend: 'expenses-insights-legend' },
  { page: 'net-worth', command: 'get_net_worth_insights', chart: 'net-worth-insights-chart', legend: 'net-worth-insights-legend' },
];

/** Opens the chart of a page with `payload` as what Rust returns (the golden answer when omitted). */
async function openChart({ page, command, chart, legend }, payload, reads = exampleReads()) {
  if (payload) reads[command] = payload;
  const app = await openApp({ reads });
  await app.click(app.item(page));
  if (page !== 'net-worth') await app.click(app.$(`${page}-tab-insights`));
  return { app, chartNode: app.$(chart), legendNode: app.$(legend) };
}

const golden = (command) => exampleReads()[command];
const BANK = CHARTS.find((chart) => chart.page === 'bank');

// ---- small series of banks, for the ACs that need an exact figure ----

const bankList = (...names) => names.map((name, index) => ({ id: `id-${index + 1}`, name, colour_slot: index % 8 }));
/** A piece as Rust delivers it: `[series, low, high, hangs, rounding]`. */
const piece = ([series, low, high, hangs = false, rounding = 'none']) => ({ series, low, high, hangs, rounding });
/** A bank year: `segments` as `[id, balance]`, `pieces` as above. */
const bankYear = (year, segments, pieces, marker = null, partial = false) => ({
  year,
  is_partial: partial,
  entry_count: segments.length,
  closing_balance: marker ? marker.value : '0.00',
  segments: segments.map(([id, balance]) => ({ account_id: id, balance })),
  pieces: pieces.map(piece),
  marker,
});
const ticks = (...pairs) => ({ ticks: pairs.map(([value, label = value]) => ({ value, label })) });
const bankSeries = (banks, years, axis) => ({ axis, banks, years });

// ---- reading the drawn chart ----

const groups = (chartNode) => [...chartNode.querySelectorAll('.chart-year-group')];
const shapes = (group) => [...group.querySelectorAll('rect[data-legend], path[data-legend]')];
const num = (node, name) => Number(node.getAttribute(name));
/** The gridlines (zero line included) with their label, top to bottom. */
function gridlines(chartNode) {
  const labels = [...chartNode.querySelectorAll('.chart-axis-label')];
  return [...chartNode.querySelectorAll('.chart-grid, .chart-zero')]
    .map((line, index) => ({ y: num(line, 'y1'), zero: line.classList.contains('chart-zero'), label: labels[index].textContent }))
    .sort((a, b) => a.y - b.y);
}
/** The top and the bottom of a square-cornered piece, as y positions. */
const span = (shape) => ({ top: num(shape, 'y') - 0.5, bottom: num(shape, 'y') + num(shape, 'height') + 0.5 });
const near = (actual, expected, message) => assert.ok(Math.abs(actual - expected) < 0.01, `${message}: ${actual} is not ${expected}`);
const yearTexts = (chartNode) => [...chartNode.querySelectorAll('.chart-year')].map((node) => node.textContent);
const tooltipOf = (chartNode) => chartNode.querySelector('.chart-tooltip');

async function hover(app, group) {
  group.dispatchEvent(new app.window.MouseEvent('mouseenter'));
  await app.settle();
}

// ---- REQ-1: bars and pieces ----

test('test_csd025_ac_1_1_one_bar_per_year_in_the_order_received', async () => {
  for (const spec of CHARTS) {
    const data = golden(spec.command);
    const { app, chartNode } = await openChart(spec);
    const drawn = groups(chartNode);
    assert.equal(drawn.length, data.years.length, `${spec.page}: one slot per year`);
    assert.deepEqual(
      yearTexts(chartNode),
      data.years.map((row) => (row.is_partial ? `${row.year} YTD` : String(row.year))),
      `${spec.page}: the years in the order received`,
    );
    drawn.forEach((group, index) => {
      assert.equal(shapes(group).length, data.years[index].pieces.length, `${spec.page} ${data.years[index].year}: the pieces received`);
    });
    app.close();
  }
});

test('test_csd025_ac_1_1_pieces_keep_their_series_and_order', async () => {
  for (const spec of CHARTS) {
    const data = golden(spec.command);
    const { app, chartNode } = await openChart(spec);
    groups(chartNode).forEach((group, index) => {
      assert.deepEqual(
        shapes(group).map((shape) => shape.dataset.legend),
        data.years[index].pieces.map((p) => p.series),
        `${spec.page} ${data.years[index].year}: the series of the pieces, in the order received`,
      );
    });
    app.close();
  }
});

test('test_csd025_ac_1_2_a_piece_hangs_below_the_zero_line', async () => {
  const series = bankSeries(
    bankList('Example Bank', 'Example Overdraft'),
    [bankYear(2026, [['id-1', '5000.00'], ['id-2', '-1000.00']], [['bank-id-1', '0.00', '5000.00'], ['bank-id-2', '-1000.00', '0.00', true]])],
    ticks(['-1000.00'], ['0.00'], ['5000.00']),
  );
  const { app, chartNode } = await openChart(BANK, series);
  const zero = num(chartNode.querySelector('.chart-zero'), 'y1');
  const [rising, hanging] = shapes(groups(chartNode)[0]).map(span);
  near(rising.bottom, zero, 'the first piece rises from the zero line');
  assert.ok(rising.top < zero);
  near(hanging.top, zero, 'the second piece hangs from the zero line');
  assert.ok(hanging.bottom > zero);
  app.close();
});

test('test_csd025_ac_1_3_pieces_are_drawn_at_the_received_values', async () => {
  const series = bankSeries(
    bankList('Example Bank'),
    [bankYear(2026, [['id-1', '8000.00']], [['bank-id-1', '2500.00', '10500.00']])],
    ticks(['0.00'], ['2500.00'], ['5000.00'], ['10500.00']),
  );
  const { app, chartNode } = await openChart(BANK, series);
  const lines = gridlines(chartNode);
  const yOf = (label) => lines.find((line) => line.label === label).y;
  const only = span(shapes(groups(chartNode)[0])[0]);
  near(only.top, yOf('10500.00'), 'the piece ends at the tick that holds 10500.00');
  near(only.bottom, yOf('2500.00'), 'the piece starts at the tick that holds 2500.00');
  app.close();
});

// ---- REQ-2: axis and zero line ----

test('test_csd025_ac_2_1_gridlines_follow_the_ticks_with_their_labels', async () => {
  for (const spec of CHARTS) {
    const data = golden(spec.command);
    const { app, chartNode } = await openChart(spec);
    const lines = gridlines(chartNode);
    assert.deepEqual(lines.map((line) => line.label), [...data.axis.ticks].reverse().map((tick) => tick.label), `${spec.page}: a line per tick, with its label`);
    assert.equal(lines.filter((line) => line.zero).length, 1, `${spec.page}: one zero line`);
    app.close();
  }
  const series = bankSeries(
    bankList('Example Bank'),
    [bankYear(2026, [['id-1', '5000.00']], [['bank-id-1', '0.00', '5000.00', false, 'top']])],
    ticks(['0.00', '0'], ['5000.00', '5k'], ['10000.00', '10.00k'], ['15000.00', '15.00k']),
  );
  const { app, chartNode } = await openChart(BANK, series);
  const lines = gridlines(chartNode);
  assert.deepEqual(lines.map((line) => line.label), ['15.00k', '10.00k', '5k', '0']);
  assert.equal(lines.find((line) => line.zero).label, '0');
  app.close();
});

test('test_csd025_ac_2_2_axis_below_zero', async () => {
  const series = bankSeries(
    bankList('Example Bank'),
    [bankYear(2026, [['id-1', '-1000.00']], [['bank-id-1', '-1000.00', '0.00', true, 'bottom']])],
    ticks(['-1000.00', '-1k'], ['0.00', '0'], ['1000.00', '1k'], ['2000.00', '2k']),
  );
  const { app, chartNode } = await openChart(BANK, series);
  const lines = gridlines(chartNode);
  assert.equal(lines.at(-1).label, '-1k', 'the axis starts at -1000.00');
  assert.equal(lines[0].label, '2k', 'and ends at 2000.00');
  assert.ok(lines.find((line) => line.zero).y < lines.at(-1).y, 'the zero line is above the bottom of the axis');
  app.close();
});

test('test_csd025_ac_2_3_only_positive_values_zero_line_at_the_bottom', async () => {
  for (const spec of CHARTS.filter((chart) => ['income', 'houses', 'expenses', 'debts'].includes(chart.page))) {
    const { app, chartNode } = await openChart(spec);
    const lines = gridlines(chartNode);
    const zero = lines.find((line) => line.zero);
    assert.ok(zero, `${spec.page}: the zero line is drawn`);
    assert.equal(zero.y, Math.max(...lines.map((line) => line.y)), `${spec.page}: it is at the bottom of the plot`);
    app.close();
  }
});

// ---- REQ-5: the frontend displays, it does not compute ----

test('test_csd025_ac_5_1_a_received_marker_label_is_shown_as_it_is', async () => {
  // The value 12500.00 would read "12.50k"; Rust says "9.99k", and that is what is shown.
  const series = bankSeries(
    bankList('Example Bank'),
    [bankYear(2026, [['id-1', '12500.00']], [['bank-id-1', '0.00', '12500.00', false, 'top']], { value: '12500.00', label: '9.99k' })],
    ticks(['0.00', '0'], ['12500.00', '12.5k']),
  );
  const { app, chartNode } = await openChart(BANK, series);
  assert.deepEqual([...chartNode.querySelectorAll('.chart-marker-label')].map((node) => node.textContent), ['9.99k']);
  app.close();
});

test('test_csd025_ac_5_1_every_area_shows_the_marker_and_growth_labels_it_received', async () => {
  for (const spec of CHARTS) {
    const data = golden(spec.command);
    const row = data.years.find((year) => year.marker) ?? data.years.find((year) => 'growth_label' in year);
    if (!row) continue; // expenses has neither a marker nor a growth label
    const received = 'received-as-is';
    if (row.marker) row.marker.label = received;
    else row.growth_label = received;
    const { app, chartNode } = await openChart(spec, data);
    const texts = [...chartNode.querySelectorAll('.chart-marker-label, .chart-growth, .chart-percent')].map((node) => node.textContent);
    assert.ok(texts.includes(received), `${spec.page}: shows ${received}`);
    app.close();
  }
});

test('test_csd025_ac_5_1_marker_labels_and_lines_are_the_ones_received', async () => {
  for (const spec of CHARTS) {
    const data = golden(spec.command);
    const { app, chartNode } = await openChart(spec);
    const markers = data.years.filter((row) => row.marker);
    assert.deepEqual(
      [...chartNode.querySelectorAll('.chart-marker-label')].map((node) => node.textContent),
      markers.map((row) => row.marker.label),
      `${spec.page}: the labels as received`,
    );
    assert.equal(chartNode.querySelectorAll('.chart-marker').length, markers.length, `${spec.page}: a line for every marker, none for a year without one`);
    app.close();
  }
});

test('test_csd025_ac_5_2_a_received_tick_label_is_shown_as_it_is', async () => {
  for (const spec of CHARTS) {
    const data = golden(spec.command);
    data.axis.ticks[1].label = 'five thousand';
    const { app, chartNode } = await openChart(spec, data);
    const lines = gridlines(chartNode);
    assert.ok(lines.some((line) => line.label === 'five thousand'), `${spec.page}: the gridline shows the received label`);
    app.close();
  }
});

test('test_csd025_ac_5_3_pieces_are_not_stacked_in_the_frontend', async () => {
  const build = (secondLow) =>
    bankSeries(
      bankList('Example Bank', 'Example Savings'),
      [bankYear(2026, [['id-1', '1000.00'], ['id-2', '500.00']], [['bank-id-1', '0.00', '1000.00'], ['bank-id-2', secondLow, '1500.00']])],
      ticks(['0.00'], ['1000.00'], ['1500.00']),
    );
  let { app, chartNode } = await openChart(BANK, build('1000.00'));
  let [first, second] = shapes(groups(chartNode)[0]).map(span);
  near(second.bottom, first.top, 'the second piece starts where the first ends because the received values say so');
  app.close();

  ({ app, chartNode } = await openChart(BANK, build('1200.00')));
  const [movedFirst, moved] = shapes(groups(chartNode)[0]).map(span);
  near(movedFirst.top, first.top, 'the first piece does not change');
  assert.ok(moved.bottom < second.bottom, 'the second piece moved up with the received low');
  const lines = gridlines(chartNode);
  const at = (label) => lines.find((line) => line.label === label).y;
  near(moved.bottom, at('1000.00') + (at('1500.00') - at('1000.00')) * 0.4, 'it stands at 1200.00');
  app.close();
});

// ---- REQ-6: year labels, partial years and gap years ----

test('test_csd025_ac_6_1_the_current_year_is_marked', async () => {
  const data = golden('get_investment_insights');
  const partial = data.years.find((row) => row.is_partial);
  const { app, chartNode } = await openChart(CHARTS[1], data);
  const group = groups(chartNode)[data.years.indexOf(partial)];
  assert.ok(group.classList.contains('partial'), 'the bar of a partial year carries the class of the lighter fill');
  assert.ok(css.includes('.partial'), 'and the style sheet has the lighter fill');
  assert.equal(group.querySelector('.chart-year').textContent, `${partial.year} YTD`);
  assert.ok(groups(chartNode).filter((g) => g !== group).every((g) => !g.classList.contains('partial')));
  app.close();
});

test('test_csd025_ac_6_2_a_gap_year_is_a_labelled_slot_without_a_bar', async () => {
  const series = bankSeries(
    bankList('Example Bank'),
    [
      bankYear(2022, [['id-1', '1000.00']], [['bank-id-1', '0.00', '1000.00', false, 'top']]),
      bankYear(2023, [], []),
      bankYear(2024, [['id-1', '1000.00']], [['bank-id-1', '0.00', '1000.00', false, 'top']]),
    ],
    ticks(['0.00'], ['1000.00']),
  );
  const { app, chartNode } = await openChart(BANK, series);
  const gap = groups(chartNode)[1];
  assert.equal(gap.querySelector('.chart-year').textContent, '2023');
  assert.equal(shapes(gap).length, 0, 'no bar');
  assert.equal(gap.querySelector('.chart-bar-box'), null);
  app.close();
});

// ---- REQ-7: marker ----

test('test_csd025_ac_7_1_a_marker_is_drawn_at_its_value_with_its_label', async () => {
  const series = bankSeries(
    bankList('Example Bank'),
    [bankYear(2025, [['id-1', '10000.00']], [['bank-id-1', '0.00', '10000.00']], { value: '10000.00', label: '10.00k' })],
    ticks(['0.00'], ['5000.00'], ['10000.00']),
  );
  const { app, chartNode } = await openChart(BANK, series);
  const line = chartNode.querySelector('.chart-marker');
  const tenThousand = gridlines(chartNode).find((gridline) => gridline.label === '10000.00').y;
  near(num(line, 'y1'), tenThousand, 'the line stands at 10000.00');
  const bar = shapes(groups(chartNode)[0])[0];
  assert.ok(num(line, 'x2') - num(line, 'x1') > num(bar, 'width'), 'the line is wider than the bar');
  assert.equal(chartNode.querySelector('.chart-marker-label').textContent, '10.00k');
  app.close();
});

test('test_csd025_ac_7_2_markers_are_not_connected_and_dashed_when_partial', async () => {
  const one = ['bank-id-1', '0.00', '1000.00', false, 'top'];
  const series = bankSeries(
    bankList('Example Bank'),
    [
      bankYear(2024, [['id-1', '1000.00']], [one], { value: '1000.00', label: '1.00k' }),
      bankYear(2025, [], []),
      bankYear(2026, [['id-1', '1000.00']], [one], { value: '1000.00', label: '1.00k' }, true),
    ],
    ticks(['0.00'], ['1000.00']),
  );
  const { app, chartNode } = await openChart(BANK, series);
  const [first, gap, last] = groups(chartNode);
  assert.equal(chartNode.querySelectorAll('.chart-marker').length, 2, 'one line per marker, none joining them');
  assert.equal(gap.querySelector('.chart-marker'), null, 'the gap year has no marker');
  assert.ok(last.classList.contains('partial') && last.querySelector('.chart-marker'), 'the marker of the partial year is inside its partial group');
  assert.ok(!first.classList.contains('partial'));
  assert.match(css, /\.partial \.chart-marker\s*\{[^}]*stroke-dasharray/, 'the style sheet dashes it');
  app.close();
});

// ---- REQ-8: tooltip and screen-reader text ----

test('test_csd025_ac_8_1_tooltip_on_hover_and_on_keyboard_focus', async () => {
  const { app, chartNode } = await openChart(CHARTS[0]);
  const [first, second] = groups(chartNode);
  const tooltip = tooltipOf(chartNode);
  assert.equal(tooltip.classList.contains('is-open'), false);
  await hover(app, first);
  assert.equal(tooltip.classList.contains('is-open'), true, 'hover shows the tooltip');
  assert.equal(app.text(tooltip.querySelector('.tip-title')), '2023');
  assert.ok(tooltip.querySelectorAll('dt').length > 0, 'with its rows');

  second.dispatchEvent(new app.window.Event('focus'));
  assert.equal(app.text(tooltip.querySelector('.tip-title')), '2024', 'focus shows the same kind of tooltip for that year');
  second.dispatchEvent(new app.window.Event('blur'));
  await sleep(150);
  assert.equal(tooltip.classList.contains('is-open'), false, 'it disappears when focus leaves the bar');
  app.close();
});

test('test_csd025_ac_8_4_rows_read_like_the_bar', async () => {
  const series = bankSeries(
    bankList('A', 'B', 'C'),
    [bankYear(2026, [['id-1', '2500.00'], ['id-2', '8000.00'], ['id-3', '-1000.00']], [
      ['bank-id-1', '0.00', '2500.00'],
      ['bank-id-2', '2500.00', '10500.00', false, 'top'],
      ['bank-id-3', '-1000.00', '0.00', true, 'bottom'],
    ], { value: '9500.00', label: '9.50k' })],
    ticks(['-1000.00'], ['0.00'], ['10500.00']),
  );
  const { app, chartNode } = await openChart(BANK, series);
  await hover(app, groups(chartNode)[0]);
  const rows = [...tooltipOf(chartNode).querySelectorAll('dt')].map((dt) => app.text(dt));
  assert.deepEqual(rows.slice(0, 3), ['B', 'A', 'C'], 'highest first, the hanging piece last');
  app.close();
});

test('test_csd025_ac_8_5_screen_reader_text_has_the_values_and_says_negative', async () => {
  const series = bankSeries(
    bankList('Example Bank', 'Example Overdraft'),
    [bankYear(2026, [['id-1', '5000.00'], ['id-2', '-1000.00']], [['bank-id-1', '0.00', '5000.00'], ['bank-id-2', '-1000.00', '0.00', true, 'bottom']])],
    ticks(['-1000.00'], ['0.00'], ['5000.00']),
  );
  const { app, chartNode } = await openChart(BANK, series);
  const group = groups(chartNode)[0];
  const spoken = group.getAttribute('aria-label');
  assert.ok(spoken.startsWith('2026'), 'the year');
  assert.ok(spoken.includes('Example Bank 5,000.00'), 'and the values of the tooltip');
  assert.ok(spoken.includes('Example Overdraft -1,000.00 (negative)'), 'it announces that the overdraft is negative');
  assert.equal(shapes(group)[1].getAttribute('aria-label'), 'Example Overdraft, 2026, -1,000.00');
  await hover(app, group);
  const tooltip = app.text(tooltipOf(chartNode));
  assert.ok(tooltip.includes('-1,000.00') && tooltip.includes('5,000.00'), 'the tooltip shows the same values');
  app.close();
});

// ---- REQ-9: highlight one series ----

const twoYears = () =>
  bankSeries(
    bankList('A', 'B'),
    ['2025', '2026'].map((year) =>
      bankYear(Number(year), [['id-1', '1000.00'], ['id-2', '500.00']], [['bank-id-1', '0.00', '1000.00'], ['bank-id-2', '1000.00', '1500.00', false, 'top']]),
    ),
    ticks(['0.00'], ['1500.00']),
  );
const marks = (chartNode, key) => [...chartNode.querySelectorAll(`[data-legend="${key}"]`)];

test('test_csd025_ac_9_1_hovering_a_piece_highlights_its_series_in_every_year', async () => {
  const { app, chartNode, legendNode } = await openChart(BANK, twoYears());
  const [firstYear] = groups(chartNode);
  shapes(firstYear)[0].dispatchEvent(new app.window.MouseEvent('mouseover', { bubbles: true }));
  const a = marks(chartNode, 'bank-id-1').filter((node) => node.matches('rect, path'));
  const b = marks(chartNode, 'bank-id-2').filter((node) => node.matches('rect, path'));
  assert.equal(a.length, 2);
  assert.ok(a.every((node) => node.classList.contains('hl') && !node.classList.contains('dim')), 'every A piece keeps its colour');
  assert.ok(b.every((node) => node.classList.contains('dim')), 'every B piece is eased down');
  assert.ok(legendNode.querySelector('[data-legend="bank-id-1"]').classList.contains('hl'), 'the A legend item is highlighted');
  assert.ok(legendNode.querySelector('[data-legend="bank-id-2"]').classList.contains('dim'));
  app.close();
});

test('test_csd025_ac_9_2_hovering_or_focusing_a_legend_item_highlights_its_series', async () => {
  const { app, chartNode, legendNode } = await openChart(BANK, twoYears());
  const item = legendNode.querySelector('[data-legend="bank-id-1"]');
  for (const [on, off] of [['mouseenter', 'mouseleave'], ['focus', 'blur']]) {
    item.dispatchEvent(new app.window.Event(on));
    const a = marks(chartNode, 'bank-id-1').filter((node) => node.matches('rect, path'));
    assert.ok(a.every((node) => node.classList.contains('hl')), `${on}: the pieces of A keep their colour in every year`);
    item.dispatchEvent(new app.window.Event(off));
    assert.equal(chartNode.querySelectorAll('.hl, .dim').length + legendNode.querySelectorAll('.hl, .dim').length, 0, `${off}: it ends`);
  }
  app.close();
});

test('test_csd025_ac_9_3_leaving_the_piece_highlights_nothing_and_keeps_the_tooltip', async () => {
  const { app, chartNode } = await openChart(BANK, twoYears());
  const [group] = groups(chartNode);
  await hover(app, group);
  shapes(group)[0].dispatchEvent(new app.window.MouseEvent('mouseover', { bubbles: true }));
  assert.ok(chartNode.querySelectorAll('.hl').length > 0);
  group.querySelector('.chart-hit').dispatchEvent(new app.window.MouseEvent('mouseover', { bubbles: true }));
  assert.equal(chartNode.querySelectorAll('.hl, .dim').length, 0, 'nothing is highlighted');
  assert.equal(tooltipOf(chartNode).classList.contains('is-open'), true, 'the tooltip is still shown');
  app.close();
});

test('test_csd025_ac_9_4_keyboard_focus_on_a_year_highlights_nothing', async () => {
  const { app, chartNode, legendNode } = await openChart(BANK, twoYears());
  groups(chartNode)[0].dispatchEvent(new app.window.Event('focus'));
  assert.equal(chartNode.querySelectorAll('.hl, .dim').length + legendNode.querySelectorAll('.hl, .dim').length, 0);
  app.close();
});

// ---- REQ-10: motion (the part jsdom can see) ----

test('test_csd025_ac_10_2_reduced_motion_switches_the_transitions_off', async () => {
  assert.match(css, /prefers-reduced-motion: reduce\)\s*\{[^}]*\.chart-tooltip[^}]*transition: none/, 'tooltip, highlight and legend do not animate');
});

// ---- REQ-11: size (the part jsdom can see) ----

test('test_csd025_ac_11_2_slots_are_at_least_64_px_wide', async () => {
  const years = Array.from({ length: 20 }, (_, index) => bankYear(2000 + index, [['id-1', '1000.00']], [['bank-id-1', '0.00', '1000.00', false, 'top']]));
  const { app, chartNode } = await openChart(BANK, bankSeries(bankList('Example Bank'), years, ticks(['0.00'], ['1000.00'])));
  const centres = [...chartNode.querySelectorAll('.chart-year')].map((node) => num(node, 'x'));
  assert.equal(centres.length, 20);
  for (let i = 1; i < centres.length; i += 1) assert.ok(centres[i] - centres[i - 1] >= 64, 'each slot is at least 64 px wide');
  const bars = groups(chartNode).map((group) => shapes(group)[0]);
  assert.ok(bars.every((bar) => num(bar, 'width') <= 44 && num(bar, 'width') <= 32), 'bars are at most 44 px and 50% of the slot');
  assert.ok(parseFloat(chartNode.querySelector('svg').style.minWidth) >= 20 * 64, 'so the chart scrolls horizontally');
  app.close();
});

// ---- REQ-13 and REQ-14: legend and colour of named series ----

test('test_csd025_ac_13_2_a_long_name_is_cut_in_the_legend_and_kept_in_full', async () => {
  const name = 'Example '.repeat(13).trim();
  const series = twoYears();
  series.banks[0].name = name;
  const { app, legendNode } = await openChart(BANK, series);
  const label = legendNode.querySelector('[data-legend="bank-id-1"] .legend-name');
  assert.equal(label.dataset.text, name, 'the full name is in the legend item');
  assert.equal(label.textContent, name);
  assert.ok(label.querySelector('.legend-name-text'), 'inside the element the style sheet cuts with an ellipsis');
  assert.match(css, /\.legend-name-text\s*\{[^}]*text-overflow:\s*ellipsis/);
  app.close();
});

test('test_csd025_ac_13_3_a_name_is_not_markup', async () => {
  const series = twoYears();
  series.banks[0].name = '<b>Example</b>';
  const { app, chartNode, legendNode } = await openChart(BANK, series);
  assert.equal(app.text(legendNode.querySelector('[data-legend="bank-id-1"]')), '<b>Example</b>');
  assert.equal(legendNode.querySelector('b'), null, 'no formatting in the legend');
  await hover(app, groups(chartNode)[0]);
  assert.equal(tooltipOf(chartNode).querySelector('b'), null, 'and none in the tooltip');
  assert.ok(app.text(tooltipOf(chartNode)).includes('<b>Example</b>'));
  app.close();
});

test('test_csd025_ac_14_1_a_series_has_one_colour_everywhere', async () => {
  const { app, chartNode, legendNode } = await openChart(BANK, twoYears());
  await hover(app, groups(chartNode)[0]);
  const second = (root, selector) => root.querySelector(selector);
  const slot = 1; // the colour slot Rust gave to the second bank
  const pieces = marks(chartNode, 'bank-id-2').filter((node) => node.matches('rect, path'));
  assert.equal(pieces.length, 2);
  assert.ok(pieces.every((node) => node.classList.contains(`bar-slot-${slot}`)), 'the pieces of both years');
  assert.ok(second(legendNode, `[data-legend="bank-id-2"] .swatch.slot-${slot}`), 'the legend swatch');
  assert.ok(second(tooltipOf(chartNode), `dt[data-legend="bank-id-2"] .swatch.slot-${slot}`), 'the tooltip swatch');
  app.close();
});

test('test_csd025_ac_14_3_a_ninth_series_shares_the_colour_of_the_first', async () => {
  const banks = Array.from({ length: 9 }, (_, index) => ({ id: `id-${index + 1}`, name: `Example ${index + 1}`, colour_slot: index % 8 }));
  const year = bankYear(2026, banks.map((bank) => [bank.id, '100.00']), banks.map((bank, index) => [`bank-${bank.id}`, `${index * 100}.00`, `${index * 100 + 100}.00`]));
  const { app, chartNode, legendNode } = await openChart(BANK, bankSeries(banks, [year], ticks(['0.00'], ['900.00'])));
  const colourOf = (id) => [...marks(chartNode, `bank-${id}`)].find((node) => node.matches('rect, path')).getAttribute('class');
  assert.equal(colourOf('id-1'), colourOf('id-9'), 'the first and the ninth share a colour');
  assert.notEqual(colourOf('id-1'), colourOf('id-2'));
  assert.ok(app.text(legendNode).includes('Example 9'), 'each is named in the legend');
  await hover(app, groups(chartNode)[0]);
  assert.ok(app.text(tooltipOf(chartNode)).includes('Example 9'), 'and in the tooltip');
  app.close();
});

// ---- REQ-6, REQ-7 and REQ-8 of the areas: what each page adds to the received pieces ----

test('test_csd025_investments_a_withdrawal_is_a_net_deposits_piece_that_hangs', async () => {
  const data = golden('get_investment_insights');
  const row = data.years[1];
  row.pieces = [
    { series: 'opening', low: '0.00', high: '5000.00', hangs: false, rounding: 'none' },
    { series: 'deposits', low: '4000.00', high: '5000.00', hangs: true, rounding: 'none' },
    { series: 'growth-up', low: '5000.00', high: '5500.00', hangs: false, rounding: 'top' },
  ];
  const { app, chartNode } = await openChart(CHARTS[1], data);
  const drawn = shapes(groups(chartNode)[1]);
  assert.deepEqual(drawn.map((shape) => shape.classList.contains('withdrawal')), [false, true, false]);
  assert.ok(drawn[1].classList.contains('bar-deposits'));
  app.close();
});

test('test_csd025_net_worth_shows_a_debt_with_a_minus_sign_when_it_has_a_piece', async () => {
  const { app, chartNode } = await openChart(CHARTS.at(-1));
  const data = golden('get_net_worth_insights');
  const index = data.years.findIndex((row) => row.debts !== '0.00');
  await hover(app, groups(chartNode)[index]);
  const debts = [...tooltipOf(chartNode).querySelectorAll('dt')].findIndex((dt) => app.text(dt) === 'Debts');
  const value = app.text(tooltipOf(chartNode).querySelectorAll('dd')[debts]);
  const { formatAmount } = await app.load('shared/format.js');
  assert.equal(value, `-${formatAmount(data.years[index].debts)}`);
  const order = [...tooltipOf(chartNode).querySelectorAll('dt')].map((dt) => app.text(dt));
  assert.equal(order.indexOf('Debts'), order.length - 2, 'debts come last before the net worth');
  app.close();
});

// ---- the frontend computes nothing ----

test('test_csd025_ac_5_1_nothing_is_computed_from_the_received_values', async () => {
  // Every number Rust sends is replaced by a different one, and every label by a text that no
  // computation could give. The chart shows the received texts and places the pieces by the
  // received values; it never recomputes a label, a tick, a stack or a total from the amounts.
  const series = bankSeries(
    bankList('Example Bank'),
    [bankYear(2026, [['id-1', '1000.00']], [['bank-id-1', '0.00', '1000.00', false, 'top']], { value: '1000.00', label: 'label-from-rust' })],
    ticks(['0.00', 'zero-label'], ['500.00', 'half-label'], ['1000.00', 'full-label']),
  );
  const { app, chartNode } = await openChart(BANK, series);
  assert.deepEqual(gridlines(chartNode).map((line) => line.label), ['full-label', 'half-label', 'zero-label']);
  assert.equal(chartNode.querySelector('.chart-marker-label').textContent, 'label-from-rust');
  app.close();
});

// ---- CSD-022 REQ-10: amounts keep one format ----

test('test_csd022_ac_10_1_amounts_are_grouped_with_a_comma_in_both_languages', async () => {
  for (const language of ['en', 'nl']) {
    const reads = exampleReads();
    reads.get_preferences = { theme: 'system', language };
    reads.yearly_totals = [{ year: 2026, entry_count: 1, gross_salary: '1234.50', holiday_pay: '0.00', bonus: '0.00', total_gross: '1234.50' }];
    reads.list_entries = [{ ...reads.list_entries[0], gross_salary: '1234.50', total_gross: '1234.50' }];
    const app = await openApp({ reads });
    await app.click(app.item('income'));
    const totals = [...app.$('totals-rows').querySelectorAll('td.num')].map((cell) => app.text(cell));
    assert.ok(totals.includes('1,234.50'), `${language}: the yearly totals show 1,234.50`);
    const paychecks = [...app.$('paycheck-rows').querySelectorAll('td.num')].map((cell) => app.text(cell));
    assert.ok(paychecks.includes('1,234.50'), `${language}: so does the list of paychecks`);
    app.close();
  }
});

test('test_csd022_ac_10_2_switching_the_language_does_not_change_the_amount_format', async () => {
  const reads = exampleReads();
  reads.yearly_totals = [{ year: 2026, entry_count: 1, gross_salary: '1234.50', holiday_pay: '0.00', bonus: '0.00', total_gross: '1234.50' }];
  const app = await openApp({ reads });
  await app.click(app.item('income'));
  const cells = () => [...app.$('totals-rows').querySelectorAll('td.num')].map((cell) => app.text(cell));
  assert.ok(cells().includes('1,234.50'));
  await app.click(app.item('settings'));
  await app.choose(app.$('setting-language'), 'nl');
  await app.click(app.item('income'));
  assert.ok(cells().includes('1,234.50'), 'the amount is still shown as 1,234.50');
  app.close();
});

test('test_csd022_ac_10_1_chart_tooltips_group_the_digits_of_a_received_amount', async () => {
  const series = bankSeries(
    bankList('Example Bank'),
    [bankYear(2026, [['id-1', '1234567.50']], [['bank-id-1', '0.00', '1234567.50', false, 'top']], { value: '1234567.50', label: '1.23M' })],
    ticks(['0.00', '0'], ['1500000.00', '1500k']),
  );
  const { app, chartNode } = await openChart(BANK, series);
  await hover(app, groups(chartNode)[0]);
  const amounts = [...tooltipOf(chartNode).querySelectorAll('dd')].map((dd) => app.text(dd));
  assert.deepEqual(amounts, ['1,234,567.50', '1,234,567.50'], 'the segment and the closing balance');
  assert.equal(chartNode.querySelector('.chart-marker-label').textContent, '1.23M', 'while the marker keeps the short label of Rust');
  app.close();
});
