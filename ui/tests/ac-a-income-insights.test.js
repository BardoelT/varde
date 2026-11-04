// Display ACs of CSD-006 (income insights): the tabs of the Income page, the stacked bars, the
// tooltip, the legend and the refresh. The answers are the golden ones of the example project
// (2023 to 2026, 2026 partial) or small series made for one AC. jsdom has no layout, so the
// placement of the tooltip (AC-12.2 to AC-12.4) stays on the manual checklist.
import test from 'node:test';
import assert from 'node:assert/strict';
import { setTimeout as sleep } from 'node:timers/promises';
import { openApp } from './harness.js';
import { emptyReads, exampleReads } from './fixtures.js';

const golden = () => exampleReads().get_income_insights;

/** A year as Rust sends it: `[salary, holiday, bonus]` stacked bottom to top. */
function year(value, [salary, holiday, bonus], { growth = '–', partial = false } = {}) {
  const amount = (text) => Number(text);
  const total = amount(salary) + amount(holiday) + amount(bonus);
  const pieces = [];
  let low = 0;
  for (const [series, text] of [['salary', salary], ['holiday', holiday], ['bonus', bonus]]) {
    if (amount(text) === 0) continue;
    pieces.push({ series, low: low.toFixed(2), high: (low + amount(text)).toFixed(2), hangs: false, rounding: 'none' });
    low += amount(text);
  }
  if (pieces.length) pieces.at(-1).rounding = 'top';
  return {
    year: value, entry_count: total === 0 ? 0 : 1, is_partial: partial, marker: null, pieces,
    gross_salary: salary, holiday_pay: holiday, bonus, total_gross: total.toFixed(2),
    growth_label: growth, growth_percent: growth === '–' ? null : growth.replace(/[+%]/g, ''),
  };
}
const axis = { ticks: [{ value: '0.00', label: '0' }, { value: '50000.00', label: '50k' }] };

async function openInsights(payload) {
  const reads = exampleReads();
  if (payload) reads.get_income_insights = payload;
  const app = await openApp({ reads });
  await app.click(app.item('income'));
  await app.click(app.$('income-tab-insights'));
  return { app, chart: app.$('income-insights-chart'), legend: app.$('income-insights-legend') };
}

const groups = (chart) => [...chart.querySelectorAll('.chart-year-group')];
const pieces = (group) => [...group.querySelectorAll('rect[data-legend], path[data-legend]')];
const tooltip = (chart) => chart.querySelector('.chart-tooltip');
const rows = (app, chart) => {
  const list = tooltip(chart);
  const labels = [...list.querySelectorAll('dt')].map((dt) => app.text(dt));
  const values = [...list.querySelectorAll('dd')].map((dd) => app.text(dd));
  return labels.map((label, index) => [label, values[index]]);
};
async function hover(app, group) {
  group.dispatchEvent(new app.window.MouseEvent('mouseenter'));
  await app.settle();
}
const y2025 = (data) => groups(data)[2];

// ---- REQ-1: the tabs ----

test('test_csd006_ac_1_1_overview_is_shown_first', async () => {
  const app = await openApp();
  await app.click(app.item('income'));
  assert.equal(app.$('income-tab-overview').getAttribute('aria-selected'), 'true');
  assert.equal(app.$('income-tab-insights').getAttribute('aria-selected'), 'false');
  assert.equal(app.isShown(app.$('income-overview')), true);
  assert.equal(app.isShown(app.$('income-insights')), false);
  const titles = [...app.$('income-overview').querySelectorAll('.panel-title')].map((node) => app.text(node));
  assert.deepEqual(titles, ['Jobs', 'Yearly totals', 'Paychecks and statements']);
  app.close();
});

test('test_csd006_ac_1_2_switching_to_insights_hides_the_overview_and_shows_the_chart', async () => {
  const { app, chart } = await openInsights();
  assert.equal(app.$('income-tab-insights').getAttribute('aria-selected'), 'true');
  assert.equal(app.isShown(app.$('income-overview')), false, 'the Overview sections are hidden');
  assert.equal(app.isShown(app.$('income-insights')), true);
  assert.equal(app.isShown(app.$('income-insights-scroll')), true);
  assert.ok(chart.querySelector('svg'), 'the chart is drawn');
  app.close();
});

test('test_csd006_ac_1_3_without_jobs_there_is_a_prompt_and_no_tabs', async () => {
  const app = await openApp({ reads: emptyReads() });
  await app.click(app.item('income'));
  assert.equal(app.isShown(app.$('income-empty')), true, 'the prompt to add the first job');
  assert.equal(app.isShown(app.$('job-add-first')), true);
  assert.equal(app.isShown(app.$('income-tab-overview')), false, 'no tabs');
  assert.equal(app.isShown(app.$('income-tab-insights')), false);
  app.close();
});

// ---- REQ-7: the bars ----

test('test_csd006_ac_7_1_stacked_bars_per_year', async () => {
  const data = golden();
  const { app, chart, legend } = await openInsights();
  const drawn = groups(chart);
  assert.deepEqual(
    [...chart.querySelectorAll('.chart-year')].map((node) => node.textContent),
    ['2023', '2024', '2025', '2026 YTD'],
    'the earliest year on the left, the latest on the right',
  );
  const xs = drawn.map((group) => Number(group.querySelector('.chart-year').getAttribute('x')));
  assert.deepEqual(xs, [...xs].sort((a, b) => a - b));
  drawn.forEach((group, index) => {
    assert.deepEqual(pieces(group).map((piece) => piece.dataset.legend), data.years[index].pieces.map((p) => p.series), `${data.years[index].year}: stacked from the pieces received`);
  });
  assert.deepEqual(pieces(drawn[2]).map((piece) => piece.dataset.legend), ['salary', 'holiday', 'bonus'], 'gross salary, holiday pay and bonus, bottom to top');
  assert.deepEqual([...legend.querySelectorAll('li')].map((item) => app.text(item)), ['Gross salary', 'Holiday pay', 'Bonus']);
  app.close();
});

test('test_csd006_ac_7_2_the_axis_covers_the_largest_year_and_starts_at_zero', async () => {
  const data = golden();
  const { app, chart } = await openInsights();
  const labels = [...chart.querySelectorAll('.chart-axis-label')].map((node) => node.textContent);
  const lines = [...chart.querySelectorAll('.chart-grid, .chart-zero')].map((line) => Number(line.getAttribute('y1')));
  const top = Math.min(...lines);
  const topLabel = labels[lines.indexOf(top)];
  assert.equal(topLabel, data.axis.ticks.at(-1).label, 'the top of the axis is the last tick received');
  assert.ok(Number(data.axis.ticks.at(-1).value) >= Math.max(...data.years.map((row) => Number(row.total_gross))), 'and it holds the largest total gross (48100.00)');
  const zero = chart.querySelector('.chart-zero');
  assert.equal(labels[[...chart.querySelectorAll('.chart-grid, .chart-zero')].indexOf(zero)], '0', 'the axis starts at 0');
  assert.equal(Number(zero.getAttribute('y1')), Math.max(...lines), 'at the bottom');
  const tallest = Math.min(...pieces(groups(chart)[2]).map((piece) => Number(piece.getAttribute('y') ?? 0)).filter((y) => y > 0));
  assert.ok(tallest >= top, 'the tallest bar stays inside the axis');
  app.close();
});

test('test_csd006_ac_7_3_a_gap_year_is_an_empty_slot', async () => {
  const series = { axis, years: [year(2022, ['30000.00', '0.00', '0.00']), year(2023, ['0.00', '0.00', '0.00']), year(2024, ['40000.00', '0.00', '0.00'], { growth: '+33.33%' })] };
  const { app, chart } = await openInsights(series);
  const gap = groups(chart)[1];
  assert.equal(gap.querySelector('.chart-year').textContent, '2023', 'a labelled slot');
  assert.equal(pieces(gap).length, 0, 'without a bar');
  app.close();
});

test('test_csd006_ac_7_4_the_current_year_is_marked', async () => {
  const { app, chart } = await openInsights();
  const last = groups(chart).at(-1);
  assert.ok(last.classList.contains('partial'), 'the lighter fill is the class of a partial year');
  assert.equal(last.querySelector('.chart-year').textContent, '2026 YTD');
  assert.ok(groups(chart).slice(0, -1).every((group) => !group.classList.contains('partial')));
  app.close();
});

// ---- REQ-8: growth labels ----

test('test_csd006_ac_8_2_only_decreases_are_red', async () => {
  const series = {
    axis,
    years: [
      year(2022, ['40000.00', '0.00', '0.00']),
      year(2023, ['39000.00', '0.00', '0.00'], { growth: '-2.50%' }),
      year(2024, ['41500.00', '0.00', '0.00'], { growth: '+6.42%' }),
    ],
  };
  const { app, chart } = await openInsights(series);
  const [, decrease, increase] = [...chart.querySelectorAll('.chart-year-group')].map((group) => group.querySelector('.chart-growth'));
  assert.equal(decrease.textContent, '-2.50%');
  assert.ok(decrease.classList.contains('negative'), 'the label of 2023 uses the negative colour');
  assert.equal(increase.textContent, '+6.42%');
  assert.ok(!increase.classList.contains('negative'), 'the label of 2024 does not');
  app.close();
});

// ---- REQ-9: the tooltip ----

test('test_csd006_ac_9_1_the_tooltip_shows_the_amounts_and_the_growth_without_a_count', async () => {
  const { app, chart } = await openInsights();
  await hover(app, y2025(chart));
  assert.equal(tooltip(chart).classList.contains('is-open'), true);
  assert.equal(app.text(tooltip(chart).querySelector('.tip-title')), '2025');
  assert.deepEqual(rows(app, chart), [
    ['Bonus', '2,500.00'],
    ['Holiday pay', '2,400.00'],
    ['Gross salary', '43,200.00'],
    ['Total gross', '48,100.00'],
    ['Growth', '+11.86%'],
  ]);
  assert.equal(tooltip(chart).querySelectorAll('.tip-note').length, 0, 'no note and no count of entries');
  app.close();
});

test('test_csd006_ac_9_2_the_tooltip_follows_keyboard_focus', async () => {
  const { app, chart } = await openInsights();
  const group = y2025(chart);
  assert.equal(group.getAttribute('tabindex'), '0', 'a bar is reachable with the Tab key');
  group.dispatchEvent(new app.window.Event('focus'));
  assert.equal(tooltip(chart).classList.contains('is-open'), true);
  assert.equal(app.text(tooltip(chart).querySelector('.tip-title')), '2025');
  assert.equal(rows(app, chart).length, 5, 'the same rows as on hover');
  group.dispatchEvent(new app.window.Event('blur'));
  await sleep(150);
  assert.equal(tooltip(chart).classList.contains('is-open'), false, 'it disappears when focus leaves the bar');
  app.close();
});

test('test_csd006_ac_9_3_the_bar_has_an_accessible_label', async () => {
  const { app, chart } = await openInsights();
  assert.equal(
    y2025(chart).getAttribute('aria-label'),
    '2025: gross salary 43,200.00, holiday pay 2,400.00, bonus 2,500.00, total gross 48,100.00, growth +11.86%',
  );
  app.close();
});

// ---- REQ-11: the figures follow changes ----

test('test_csd006_ac_11_1_the_figures_follow_changes', async () => {
  const { app, chart } = await openInsights();
  assert.match(y2025(chart).getAttribute('aria-label'), /total gross 48,100\.00/);
  await app.click(app.$('income-tab-overview'));
  const changed = golden();
  Object.assign(changed.years[2], { bonus: '3000.00', total_gross: '48600.00' });
  app.bridge.table.get_income_insights = changed;
  const before = app.callsOf('get_income_insights').length;
  await app.click(app.$('income-tab-insights'));
  assert.equal(app.callsOf('get_income_insights').length, before + 1, 'opening Insights asks Rust again');
  assert.match(y2025(chart).getAttribute('aria-label'), /bonus 3,000\.00, total gross 48,600\.00/);
  app.close();
});

test('test_csd006_ac_11_2_the_last_entry_is_deleted', async () => {
  const { app, chart } = await openInsights();
  assert.ok(chart.children.length > 0);
  await app.click(app.$('income-tab-overview'));
  app.bridge.table.get_income_insights = { ...golden(), years: [] };
  await app.click(app.$('income-tab-insights'));
  assert.equal(app.isShown(app.$('income-insights-empty')), true);
  assert.equal(app.textOf('income-insights-empty'), 'No income yet. The chart appears once there is a paycheck or an annual statement.');
  assert.equal(app.isShown(app.$('income-insights-scroll')), false);
  assert.equal(chart.querySelectorAll('svg').length, 0, 'no chart is drawn');
  app.close();
});

// ---- REQ-12: swatches and the legend while hovering ----

test('test_csd006_ac_12_1_the_swatches_match_the_legend', async () => {
  const { app, chart, legend } = await openInsights();
  await hover(app, y2025(chart));
  for (const key of ['salary', 'holiday', 'bonus']) {
    const inLegend = legend.querySelector(`[data-legend="${key}"] .swatch`);
    const inTooltip = tooltip(chart).querySelector(`dt[data-legend="${key}"] .swatch`);
    assert.ok(inLegend && inTooltip, `${key}: a swatch in both`);
    assert.equal(inTooltip.className, inLegend.className, `${key}: the same colour`);
  }
  const plain = [...tooltip(chart).querySelectorAll('dt')].filter((dt) => ['Total gross', 'Growth'].includes(app.text(dt)));
  assert.equal(plain.length, 2);
  for (const dt of plain) assert.equal(dt.querySelector('.swatch'), null, `${app.text(dt)}: no swatch`);
  app.close();
});

test('test_csd006_ac_12_5_the_header_legend_stays_while_hovering', async () => {
  const { app, chart, legend } = await openInsights();
  await hover(app, y2025(chart));
  assert.equal(app.isShown(legend), true);
  assert.equal(legend.querySelectorAll('li').length, 3);
  app.close();
});

test('test_csd006_ac_12_6_keyboard_focus_shows_the_same_tooltip_with_the_same_swatches', async () => {
  const { app, chart, legend } = await openInsights();
  y2025(chart).dispatchEvent(new app.window.Event('focus'));
  assert.equal(tooltip(chart).classList.contains('is-open'), true);
  for (const key of ['salary', 'holiday', 'bonus']) {
    assert.equal(tooltip(chart).querySelector(`dt[data-legend="${key}"] .swatch`).className, legend.querySelector(`[data-legend="${key}"] .swatch`).className);
  }
  app.close();
});

// ---- REQ-17 and REQ-18: legend and row order ----

test('test_csd006_ac_17_3_the_legend_lists_the_series_with_their_swatches', async () => {
  const { app, legend } = await openInsights();
  const items = [...legend.querySelectorAll('li')];
  assert.deepEqual(items.map((item) => app.text(item)), ['Gross salary', 'Holiday pay', 'Bonus']);
  assert.deepEqual(items.map((item) => item.querySelector('.swatch').className), ['swatch salary', 'swatch holiday', 'swatch bonus']);
  app.close();
});

test('test_csd006_ac_18_1_the_rows_read_like_the_bar', async () => {
  const { app, chart } = await openInsights();
  await hover(app, y2025(chart));
  assert.deepEqual(rows(app, chart).map(([label]) => label), ['Bonus', 'Holiday pay', 'Gross salary', 'Total gross', 'Growth']);
  app.close();
});
