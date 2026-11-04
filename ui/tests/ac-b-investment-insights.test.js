// Display ACs of CSD-007 (Investment insights): the tabs, the chart of the received series, the
// tooltip, the legend highlight. Each test feeds the chart the series of its own scenario.
// What needs real layout (where the tooltip sits beside a bar) is on the checked-by-hand list.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { openApp } from './harness.js';
import { emptyReads, exampleReads } from './fixtures.js';
import {
  afterTooltipCloses, axisOf, groupOf, groups, hover, investmentYear, near, over, shapes, span, tooltipOf, tooltipRows, yScale,
} from './ac-b-chart-helpers.js';

const read = (file) => readFileSync(fileURLToPath(new URL(`../styles/${file}`, import.meta.url)), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
const charts = read('charts.css');
const base = read('base.css');

const series = (years, ...axisValues) => ({ axis: axisOf(...(axisValues.length ? axisValues : [0, 10000, 20000])), years });

/** The Investments page with the Insights tab open, fed with `insights`. */
async function openInsights(insights, { handlers = {}, reads = exampleReads() } = {}) {
  reads.get_investment_insights = insights;
  const app = await openApp({ reads, handlers });
  await app.click(app.item('investments'));
  await app.click(app.$('investments-tab-insights'));
  return { app, chart: app.$('investments-insights-chart'), legend: app.$('investments-insights-legend') };
}

const gap = (year) => investmentYear({ year, count: 0 });

// ---- REQ-1: the tabs ----

test('test_csd007_ac_1_1_the_overview_is_shown_first', async () => {
  const app = await openApp();
  await app.click(app.item('investments'));
  assert.equal(app.$('investments-tab-overview').getAttribute('aria-selected'), 'true');
  assert.equal(app.$('investments-tab-insights').getAttribute('aria-selected'), 'false');
  assert.equal(app.isShown(app.$('investments-overview')), true);
  const titles = [...app.document.querySelectorAll('#investments-overview .panel-title')].map((el) => app.text(el));
  assert.deepEqual(titles, ['Accounts', 'Yearly totals', 'Statements']);
  assert.equal(app.isShown(app.$('investments-insights')), false);
  app.close();
});

test('test_csd007_ac_1_2_switching_to_insights_hides_the_overview_and_shows_the_chart', async () => {
  const app = await openApp();
  await app.click(app.item('investments'));
  await app.click(app.$('investments-tab-insights'));
  assert.equal(app.isShown(app.$('investments-overview')), false, 'the Overview sections are hidden');
  assert.equal(app.isShown(app.$('investments-insights')), true);
  assert.equal(app.isShown(app.$('investments-insights-scroll')), true);
  assert.ok(app.$('investments-insights-chart').querySelector('svg'), 'the insights chart is drawn');
  app.close();
});

test('test_csd007_ac_1_3_without_accounts_there_is_a_prompt_and_no_tabs', async () => {
  const app = await openApp({ reads: emptyReads() });
  await app.click(app.item('investments'));
  assert.equal(app.isShown(app.$('investments-empty')), true);
  assert.equal(app.isShown(app.$('account-add-first')), true, 'the prompt to add the first investment account');
  assert.equal(app.text(app.$('account-add-first')), 'Add your first investment account');
  const tablist = app.document.querySelector('#investments [role="tablist"]');
  assert.equal(app.isShown(tablist), false, 'there are no tabs');
  app.close();
});

// ---- REQ-8: the bars ----

test('test_csd007_ac_8_2_the_growth_colour_follows_the_sign', async () => {
  const data = series(
    [
      investmentYear({ year: 2025, opening: '10000.00', growth: '300.00', label: '+3.00%' }),
      investmentYear({ year: 2026, opening: '10300.00', growth: '-500.00', label: '-4.85%' }),
    ],
    0, 5000, 10000, 15000,
  );
  const { app, chart } = await openInsights(data);
  const growthOf = (year) => shapes(groupOf(chart, year)).find((shape) => shape.dataset.legend.startsWith('growth'));
  assert.ok(growthOf(2025).classList.contains('bar-growth-up'), '2025 uses the positive colour');
  assert.ok(growthOf(2026).classList.contains('bar-growth-down'), '2026 uses the negative colour');
  assert.match(charts, /\.bar-growth-up \{[^}]*fill: var\(--chart-gain\)/);
  assert.match(charts, /\.bar-growth-down \{[^}]*fill: var\(--chart-loss\)/);
  app.close();
});

test('test_csd007_ac_8_3_negative_segments_hang_down_from_the_running_total', async () => {
  const axis = axisOf(0, 5000, 10000, 15000);
  const data = { axis, years: [investmentYear({ year: 2026, opening: '12000.00', deposits: '-500.00', growth: '-500.00', label: '-4.17%' })] };
  const { app, chart } = await openInsights(data);
  const yOf = yScale(chart, axis);
  const byClass = (name) => shapes(groupOf(chart, 2026)).find((shape) => shape.classList.contains(name));
  const deposits = span(byClass('bar-deposits'));
  const growth = span(byClass('bar-growth-down'));
  near(deposits.top, yOf(12000), 'the net deposits segment hangs from 12000.00');
  near(deposits.bottom, yOf(11500), 'down to 11500.00');
  near(growth.top, yOf(11500), 'the growth segment hangs from 11500.00');
  near(growth.bottom, yOf(11000), 'down to 11000.00');
  assert.ok(byClass('bar-deposits').classList.contains('withdrawal'), 'the withdrawal is a lighter tint');
  assert.match(charts, /\.bar-deposits\.withdrawal \{[^}]*fill-opacity: \.35[^}]*stroke-width: 1[^}]*stroke-dasharray: 3 2/, 'with a thin dashed outline');
  assert.ok(!byClass('bar-growth-down').classList.contains('withdrawal'));
  app.close();
});

test('test_csd007_ac_8_5_a_gap_year_is_a_labelled_slot_without_a_bar', async () => {
  const data = series([investmentYear({ year: 2022, opening: '1000.00', growth: '100.00' }), gap(2023), investmentYear({ year: 2024, opening: '1100.00', growth: '100.00' })]);
  const { app, chart } = await openInsights(data);
  assert.equal(groups(chart).length, 3);
  const slot = groupOf(chart, 2023);
  assert.equal(app.text(slot.querySelector('.chart-year')), '2023', 'a labelled slot');
  assert.equal(shapes(slot).length, 0, 'without a bar');
  app.close();
});

test('test_csd007_ac_8_6_the_current_year_is_lighter_and_labelled_ytd', async () => {
  const data = series([investmentYear({ year: 2025, opening: '1000.00', growth: '100.00' }), investmentYear({ year: 2026, opening: '1100.00', growth: '100.00', partial: true })]);
  const { app, chart } = await openInsights(data);
  const partial = groupOf(chart, 2026);
  assert.ok(partial.classList.contains('partial'));
  assert.ok(!groupOf(chart, 2025).classList.contains('partial'));
  assert.equal(app.text(partial.querySelector('.chart-year')), '2026 YTD');
  assert.match(charts, /\.partial \.bar-opening[^{]*\{ opacity: \.55; \}/, 'the partial bar has a lighter fill');
  app.close();
});

// ---- REQ-9, REQ-10: marker and percentage ----

test('test_csd007_ac_9_3_markers_are_not_connected_and_the_current_year_is_dashed', async () => {
  const data = series([
    investmentYear({ year: 2024, opening: '1000.00', growth: '100.00' }),
    gap(2025),
    investmentYear({ year: 2026, opening: '1100.00', growth: '100.00', partial: true }),
  ]);
  const { app, chart } = await openInsights(data);
  const markers = [...chart.querySelectorAll('.chart-marker')];
  assert.equal(markers.length, 2, 'a marker for 2024 and for 2026 only');
  assert.equal(chart.querySelectorAll('polyline, polygon, path.chart-marker').length, 0, 'no line joins the markers');
  const slot = (marker) => marker.closest('.chart-year-group');
  assert.deepEqual(markers.map((marker) => slot(marker).querySelector('.chart-year').textContent), ['2024', '2026 YTD']);
  assert.ok(slot(markers[1]).classList.contains('partial'));
  assert.match(charts, /\.partial \.chart-marker \{ stroke-dasharray: 5 3; \}/, 'the marker of 2026 is dashed');
  assert.equal(groupOf(chart, 2025).querySelector('.chart-marker'), null, '2025 has no marker');
  app.close();
});

test('test_csd007_ac_10_2_only_decreases_are_red', async () => {
  const data = series([
    investmentYear({ year: 2024, opening: '10000.00', growth: '0.00' }),
    investmentYear({ year: 2025, opening: '10000.00', growth: '-250.00', label: '-2.50%' }),
    investmentYear({ year: 2026, opening: '9750.00', growth: '594.00', label: '+6.09%' }),
  ]);
  const { app, chart } = await openInsights(data);
  const percent = (year) => groupOf(chart, year).querySelector('.chart-percent');
  assert.equal(app.text(percent(2025)), '-2.50%');
  assert.ok(percent(2025).classList.contains('negative'), 'the percentage of 2025 uses the negative colour');
  assert.equal(app.text(percent(2026)), '+6.09%');
  assert.ok(!percent(2026).classList.contains('negative'), 'the percentage of 2026 does not');
  assert.match(charts, /\.chart-percent\.negative \{ fill: var\(--negative\); \}/);
  app.close();
});

// ---- REQ-11: the tooltip ----

const year2026 = () => investmentYear({ year: 2026, opening: '11500.00', deposits: '300.00', growth: '700.00', label: '+6.09%', count: 3 });

test('test_csd007_ac_11_1_the_tooltip_shows_five_values_and_no_statement_count', async () => {
  const { app, chart } = await openInsights(series([year2026()]));
  await hover(app, groups(chart)[0]);
  assert.equal(tooltipOf(chart).classList.contains('is-open'), true);
  const rows = Object.fromEntries(tooltipRows(app, chart).map((row) => [row.label, row.value]));
  assert.equal(rows['Opening balance'], '11,500.00');
  assert.equal(rows['Net deposits'], '300.00');
  assert.equal(rows['Capital growth'], '700.00');
  assert.equal(rows['Closing balance'], '12,500.00');
  assert.equal(rows.Growth, '+6.09%');
  assert.equal(Object.keys(rows).length, 5, 'those five values');
  assert.ok(!/statement/i.test(app.text(tooltipOf(chart))), 'no number of statements');
  app.close();
});

test('test_csd007_ac_11_2_the_tooltip_follows_keyboard_focus', async () => {
  const data = series([investmentYear({ year: 2025, opening: '1000.00', growth: '10.00' }), year2026()]);
  const { app, chart } = await openInsights(data);
  const [, second] = groups(chart);
  assert.equal(second.getAttribute('tabindex'), '0', 'a bar can take focus with Tab');
  second.dispatchEvent(new app.window.Event('focus'));
  assert.equal(tooltipOf(chart).classList.contains('is-open'), true);
  assert.equal(app.text(tooltipOf(chart).querySelector('.tip-title')), '2026');
  assert.equal(tooltipRows(app, chart).length, 5, 'the same tooltip');
  second.dispatchEvent(new app.window.Event('blur'));
  await afterTooltipCloses();
  assert.equal(tooltipOf(chart).classList.contains('is-open'), false, 'it disappears when focus leaves the bar');
  app.close();
});

test('test_csd007_ac_11_3_the_accessible_label_has_the_values_and_says_negative', async () => {
  const data = series([investmentYear({ year: 2026, opening: '12000.00', deposits: '0.00', growth: '-500.00', label: '-4.17%', count: 4 })]);
  const { app, chart } = await openInsights(data);
  const spoken = groups(chart)[0].getAttribute('aria-label');
  assert.ok(spoken.startsWith('2026'), 'the year');
  for (const part of ['opening balance 12,000.00', 'net deposits 0.00', 'capital growth -500.00 (negative)', '-4.17%', 'closing balance 11,500.00']) {
    assert.ok(spoken.includes(part), `announces "${part}"`);
  }
  assert.ok(!/statement/i.test(spoken) && !spoken.includes('4 '), 'no number of statements');
  app.close();
});

// ---- REQ-12, REQ-13: empty chart and refresh ----

test('test_csd007_ac_12_1_accounts_without_statements_show_the_note_and_no_chart', async () => {
  const reads = exampleReads();
  reads.list_investment_statements = [];
  reads.yearly_investment_totals = [];
  const { app, chart } = await openInsights({ axis: axisOf(0, 1), years: [] }, { reads });
  assert.equal(app.isShown(app.$('investments-insights-empty')), true);
  assert.equal(app.textOf('investments-insights-empty'), 'No statements yet. The chart appears once there is a statement.');
  assert.equal(app.isShown(app.$('investments-insights-scroll')), false);
  assert.equal(chart.querySelector('svg'), null, 'no chart is drawn');
  app.close();
});

test('test_csd007_ac_13_1_the_figures_follow_a_change_on_the_overview', async () => {
  let closing = '12500.00';
  const handlers = {
    get_investment_insights: () => series([investmentYear({ year: 2026, opening: '10000.00', growth: String(Number(closing) - 10000), partial: true })]),
    calculate_investment_statement: () => ({ balance: closing, capital_growth: '0.00' }),
    add_investment_statement: () => {
      closing = '13000.00';
      return {};
    },
  };
  const app = await openApp({ handlers });
  await app.click(app.item('investments'));
  await app.click(app.$('investments-tab-insights'));
  const marker = () => app.text(app.$('investments-insights-chart').querySelector('.chart-marker-label'));
  assert.equal(marker(), '12.50k', 'the Insights tab showed 12500.00 for 2026');

  await app.click(app.$('investments-tab-overview'));
  await app.click(app.$('statement-add'));
  const form = app.$('statement-form');
  form.elements.period.value = '';
  await app.type(form.elements.period, '202609');
  await app.type(form.elements.balance, '13000.00');
  form.requestSubmit();
  await app.settle();
  assert.equal(app.callsOf('add_investment_statement').length, 1);

  await app.click(app.$('investments-tab-insights'));
  assert.equal(marker(), '13.00k', 'the 2026 marker shows 13000.00');
  app.close();
});

test('test_csd007_ac_13_2_deleting_the_last_statement_shows_the_note_again', async () => {
  let years = series([investmentYear({ year: 2026, opening: '0.00', growth: '12000.00', partial: true })]);
  const reads = exampleReads();
  reads.list_investment_statements = reads.list_investment_statements.filter((s) => s.account_id === 'id-16').slice(0, 1);
  const handlers = {
    get_investment_insights: () => years,
    delete_investment_statement: () => {
      years = { axis: axisOf(0, 1), years: [] };
      return null;
    },
  };
  const app = await openApp({ reads, handlers });
  await app.click(app.item('investments'));
  await app.click(app.$('investments-tab-insights'));
  assert.ok(app.$('investments-insights-chart').querySelector('svg'), 'the Insights tab showed a chart');

  await app.click(app.$('investments-tab-overview'));
  await app.click(app.document.querySelector('#statements-rows [aria-label^="Delete"]'));
  await app.click(app.$('confirm-ok'));
  assert.equal(app.callsOf('delete_investment_statement').length, 1);

  await app.click(app.$('investments-tab-insights'));
  assert.equal(app.textOf('investments-insights-empty'), 'No statements yet. The chart appears once there is a statement.');
  assert.equal(app.isShown(app.$('investments-insights-empty')), true);
  assert.equal(app.$('investments-insights-chart').querySelector('svg'), null, 'no chart is drawn');
  app.close();
});

// ---- REQ-14: tooltip swatches, order and notes ----

const lossAndGain = () =>
  series([
    investmentYear({ year: 2023, opening: '5000.00', growth: '-800.00', label: '-16.00%' }),
    investmentYear({ year: 2024, opening: '4200.00', growth: '0.00', label: '0.00%' }),
    gap(2025),
    investmentYear({ year: 2026, opening: '4200.00', deposits: '500.00', growth: '3800.00', label: '+90.48%', partial: true }),
  ]);

test('test_csd007_ac_14_2_a_loss_shows_the_loss_swatch', async () => {
  const { app, chart } = await openInsights(lossAndGain());
  await hover(app, groupOf(chart, 2023));
  const growth = tooltipRows(app, chart).find((row) => row.label === 'Capital growth');
  assert.equal(growth.value, '-800.00');
  assert.equal(growth.swatch, 'growth-down');
  app.close();
});

test('test_csd007_ac_14_3_zero_growth_shows_the_gain_swatch', async () => {
  const { app, chart } = await openInsights(lossAndGain());
  await hover(app, groupOf(chart, 2024));
  const growth = tooltipRows(app, chart).find((row) => row.label === 'Capital growth');
  assert.equal(growth.value, '0.00');
  assert.equal(growth.swatch, 'growth-up');
  app.close();
});

test('test_csd007_ac_14_6_a_year_without_a_bar_shows_the_note_and_no_rows_or_swatch', async () => {
  const { app, chart } = await openInsights(lossAndGain());
  await hover(app, groupOf(chart, 2025));
  const tooltip = tooltipOf(chart);
  assert.equal(tooltip.classList.contains('is-open'), true);
  assert.equal(app.text(tooltip.querySelector('.tip-note')), 'No statements');
  assert.equal(tooltip.querySelectorAll('dt').length, 0, 'no rows');
  assert.equal(tooltip.querySelectorAll('.swatch').length, 0, 'no swatch');
  app.close();
});

test('test_csd007_ac_14_7_the_header_legend_stays_while_hovering_a_year', async () => {
  const { app, chart, legend } = await openInsights(lossAndGain());
  const before = [...legend.querySelectorAll('li')].map((item) => app.text(item));
  assert.deepEqual(before, ['Opening balance', 'Net deposits', 'Capital growth (gain)', 'Capital growth (loss)', 'Closing balance']);
  await hover(app, groupOf(chart, 2026));
  assert.equal(app.isShown(legend), true);
  assert.deepEqual([...legend.querySelectorAll('li')].map((item) => app.text(item)), before, 'the legend in the panel header is still shown');
  app.close();
});

test('test_csd007_ac_14_8_keyboard_focus_shows_the_same_tooltip_with_the_same_swatches', async () => {
  const { app, chart } = await openInsights(lossAndGain());
  const group = groupOf(chart, 2026);
  await hover(app, group);
  const hovered = tooltipRows(app, chart);
  group.dispatchEvent(new app.window.MouseEvent('mouseleave'));
  await afterTooltipCloses();
  assert.equal(tooltipOf(chart).classList.contains('is-open'), false);
  group.dispatchEvent(new app.window.Event('focus'));
  assert.equal(tooltipOf(chart).classList.contains('is-open'), true);
  assert.deepEqual(tooltipRows(app, chart), hovered, 'the same rows and swatches');
  assert.deepEqual(
    hovered.slice(0, 4).map((row) => row.swatch),
    ['growth-up', 'deposits', 'opening', 'marker'],
  );
  app.close();
});

// ---- REQ-15: highlight ----

test('test_csd007_ac_15_2_hovering_the_closing_balance_marker_highlights_it_everywhere', async () => {
  const { app, chart, legend } = await openInsights(lossAndGain());
  const group = groupOf(chart, 2026);
  await hover(app, group);
  over(app, group.querySelector('.chart-marker'));
  assert.ok(legend.querySelector('li[data-legend="marker"]').classList.contains('hl'), 'in the header legend');
  assert.ok(tooltipOf(chart).querySelector('dt[data-legend="marker"]').classList.contains('hl'), 'in the tooltip');
  const markers = [...chart.querySelectorAll('.chart-marker')];
  assert.ok(markers.length > 1 && markers.every((marker) => marker.classList.contains('hl')), 'the markers of every year');
  assert.match(charts, /\.legend li\.hl \.legend-label, \.chart-tooltip dt\.hl \.legend-label \{ font-weight: 600; \}/, 'a heavier font weight');
  assert.match(charts, /\.legend li\.hl, \.chart-tooltip dt\.hl \{ color: var\(--text\); \}/, 'the full text colour');
  assert.match(charts, /\.chart-marker \{[^}]*stroke-width: 2;/, 'a marker is 2 px');
  assert.match(charts, /\.chart-marker\.hl \{ stroke-width: 3; \}/, 'and 3 px when highlighted');
  app.close();
});

test('test_csd007_ac_15_3_gain_and_loss_are_separate_legend_items', async () => {
  const { app, chart, legend } = await openInsights(lossAndGain());
  const loss = shapes(groupOf(chart, 2023)).find((shape) => shape.dataset.legend === 'growth-down');
  over(app, loss);
  assert.ok(legend.querySelector('li[data-legend="growth-down"]').classList.contains('hl'), 'the loss item is highlighted');
  assert.ok(!legend.querySelector('li[data-legend="growth-up"]').classList.contains('hl'), 'the gain item is not');
  const gain = shapes(groupOf(chart, 2026)).find((shape) => shape.dataset.legend === 'growth-up');
  assert.ok(gain.classList.contains('dim'), 'the gain segment is eased down');
  assert.ok(loss.classList.contains('hl') && !loss.classList.contains('dim'), 'the loss segment keeps its full colour');
  assert.match(charts, /\.chart-year-group \[data-legend\]\.dim \{ opacity: \.7; \}/, 'to 70% opacity');
  app.close();
});

// ---- REQ-18: marker look and colour roles ----

test('test_csd007_ac_18_3_marker_and_withdrawal_styles', async () => {
  assert.match(charts, /\.chart-marker \{ stroke: var\(--muted\); stroke-width: 2; stroke-linecap: round; \}/, 'a 2 px line with round ends in the muted text colour');
  assert.match(charts, /\.chart-marker-label \{[^}]*font-size: 10px; font-weight: 500;/, 'the value is 10 px in medium weight');
  assert.match(charts, /\.bar-deposits\.withdrawal \{ fill-opacity: \.35; stroke: var\(--series-deposits\); stroke-width: 1; stroke-dasharray: 3 2; \}/, 'a lighter tint of the net deposits colour with a thin dashed outline');
  const data = series([investmentYear({ year: 2025, opening: '1000.00', deposits: '-200.00', growth: '50.00' })], 0, 1000, 2000);
  const { app, chart } = await openInsights(data);
  const deposits = shapes(groupOf(chart, 2025)).find((shape) => shape.dataset.legend === 'deposits');
  assert.ok(deposits.classList.contains('withdrawal'), 'a net withdrawal gets the withdrawal segment');
  app.close();
});

/** The tokens of a block of base.css as `{ name: [hue, saturation, lightness] }`. */
function hslTokens(selector, names) {
  const start = base.indexOf(selector);
  const body = base.slice(base.indexOf('{', start) + 1, base.indexOf('}', start));
  return Object.fromEntries(
    names.map((name) => {
      const match = body.match(new RegExp(`--${name}: hsl\\((\\d+), (\\d+)%, (\\d+)%\\)`));
      assert.ok(match, `${selector} defines --${name}`);
      return [name, match.slice(1).map(Number)];
    }),
  );
}

test('test_csd007_ac_18_6_the_colour_roles_hold_in_light_and_dark_mode', async () => {
  const names = ['series-opening', 'series-deposits', 'chart-gain', 'chart-loss'];
  for (const [mode, selector] of [['light', ':root {'], ['dark', ':root[data-theme="dark"]']]) {
    const tokens = hslTokens(selector, names);
    const [opening, deposits, gain, loss] = names.map((name) => tokens[name]);
    assert.ok(opening[0] >= 200 && opening[0] <= 230 && opening[1] <= 30, `${mode}: the opening balance is a neutral slate blue-grey`);
    assert.ok(deposits[0] >= 215 && deposits[0] <= 240 && deposits[1] >= 55, `${mode}: the net deposits are the accent blue`);
    assert.ok(gain[0] >= 120 && gain[0] <= 170, `${mode}: the capital gain is green`);
    assert.ok(loss[0] <= 15 || loss[0] >= 345, `${mode}: the capital loss is coral red`);
    for (const [left, right] of [[opening, deposits], [deposits, gain], [gain, loss]]) {
      const apart = Math.abs(left[0] - right[0]) >= 20 || Math.abs(left[1] - right[1]) >= 25;
      assert.ok(apart, `${mode}: neighbouring segments are clearly distinguishable`);
    }
  }
});
