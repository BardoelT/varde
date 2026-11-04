// Display ACs of CSD-010 (Bank account insights): the tabs, the stacked chart of the received
// series, legend, tooltip and refresh. Each test feeds the chart the series of its own scenario.
// What needs real layout (the tooltip centred over a slot) is on the checked-by-hand list.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { openApp } from './harness.js';
import { emptyReads, exampleReads } from './fixtures.js';
import {
  afterTooltipCloses, axisOf, groupOf, groups, hover, near, piece, shapes, span, tooltipOf, tooltipRows, yScale,
} from './ac-b-chart-helpers.js';

const charts = readFileSync(fileURLToPath(new URL('../styles/charts.css', import.meta.url)), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');

const bankList = (...names) => names.map((name, index) => ({ id: `id-${index + 1}`, name, colour_slot: index % 8 }));
/**
 * A bank year as Rust delivers it. `segments` are `[id, balance]` in creation order; the pieces stack
 * them: a positive one from the running total up, a negative one hanging from zero.
 */
function bankYear(year, segments, { partial = false, count = segments.length } = {}) {
  let up = 0;
  let down = 0;
  const pieces = [];
  for (const [id, balance] of segments) {
    const amount = Number(balance);
    if (amount >= 0) {
      pieces.push(piece(`bank-${id}`, up.toFixed(2), (up + amount).toFixed(2)));
      up += amount;
    } else {
      pieces.push(piece(`bank-${id}`, (down + amount).toFixed(2), down.toFixed(2), true));
      down += amount;
    }
  }
  const closing = segments.reduce((sum, [, balance]) => sum + Number(balance), 0).toFixed(2);
  return {
    year,
    is_partial: partial,
    entry_count: count,
    closing_balance: count === 0 ? '0.00' : closing,
    segments: segments.map(([id, balance]) => ({ account_id: id, balance })),
    pieces,
    marker: count === 0 ? null : { value: closing, label: `${(Number(closing) / 1000).toFixed(2)}k` },
  };
}
const gapYear = (year) => bankYear(year, [], { count: 0 });
const insightsOf = (banks, years, ...axisValues) => ({ axis: axisOf(...(axisValues.length ? axisValues : [-2000, 0, 5000, 10000])), banks, years });

async function openInsights(insights, { handlers = {}, reads = exampleReads() } = {}) {
  reads.get_bank_insights = insights;
  const app = await openApp({ reads, handlers });
  await app.click(app.item('bank'));
  await app.click(app.$('bank-tab-insights'));
  return { app, chart: app.$('bank-insights-chart'), legend: app.$('bank-insights-legend') };
}

// ---- REQ-1: the tabs ----

test('test_csd010_ac_1_1_the_overview_is_shown_first', async () => {
  const app = await openApp();
  await app.click(app.item('bank'));
  assert.equal(app.$('bank-tab-overview').getAttribute('aria-selected'), 'true');
  assert.equal(app.isShown(app.$('bank-overview')), true);
  const titles = [...app.document.querySelectorAll('#bank-overview .panel-title')].map((el) => app.text(el));
  assert.deepEqual(titles, ['Accounts', 'Yearly totals', 'Entries']);
  assert.equal(app.isShown(app.$('bank-insights')), false);
  app.close();
});

test('test_csd010_ac_1_2_switching_to_insights_hides_the_overview_and_shows_the_chart', async () => {
  const app = await openApp();
  await app.click(app.item('bank'));
  await app.click(app.$('bank-tab-insights'));
  assert.equal(app.isShown(app.$('bank-overview')), false, 'the Overview sections are hidden');
  assert.equal(app.isShown(app.$('bank-insights')), true);
  assert.equal(app.isShown(app.$('bank-insights-scroll')), true);
  assert.ok(app.$('bank-insights-chart').querySelector('svg'), 'the insights chart is drawn');
  app.close();
});

test('test_csd010_ac_1_3_without_accounts_there_is_a_prompt_and_no_tabs', async () => {
  const app = await openApp({ reads: emptyReads() });
  await app.click(app.item('bank'));
  assert.equal(app.isShown(app.$('bank-empty')), true);
  assert.equal(app.text(app.$('bank-account-add-first')), 'Add your first bank account');
  assert.equal(app.isShown(app.document.querySelector('#bank [role="tablist"]')), false, 'there are no tabs');
  app.close();
});

// ---- REQ-6, REQ-7: bars and markers ----

test('test_csd010_ac_6_1_one_stacked_bar_per_year_with_a_segment_per_bank', async () => {
  const banks = bankList('Example Current Account', 'Example Savings', 'Example Unused');
  const data = insightsOf(banks, [bankYear(2025, [['id-1', '2000.00'], ['id-2', '8000.00']]), bankYear(2026, [['id-1', '2500.00']])]);
  const { app, chart } = await openInsights(data);
  assert.equal(groups(chart).length, 2, 'two bars');
  const [left, right] = groups(chart);
  assert.equal(app.text(left.querySelector('.chart-year')), '2025', '2025 on the left');
  assert.equal(app.text(right.querySelector('.chart-year')), '2026', '2026 on the right');
  assert.deepEqual(shapes(left).map((shape) => shape.dataset.legend), ['bank-id-1', 'bank-id-2'], 'one segment per bank with an entry in that year');
  assert.deepEqual(shapes(right).map((shape) => shape.dataset.legend), ['bank-id-1']);
  app.close();
});

test('test_csd010_ac_6_2_negative_segments_hang_down_from_the_zero_line', async () => {
  const axis = axisOf(-2000, 0, 5000);
  const data = {
    axis,
    banks: bankList('Example Current Account', 'Example Overdraft'),
    years: [bankYear(2026, [['id-1', '5000.00'], ['id-2', '-1000.00']])],
  };
  const { app, chart } = await openInsights(data);
  const yOf = yScale(chart, axis);
  const [rising, hanging] = shapes(groupOf(chart, 2026)).map(span);
  near(rising.bottom, yOf(0), 'the segment of 5000.00 rises from the zero line');
  near(rising.top, yOf(5000), 'up to 5000.00');
  near(hanging.top, yOf(0), 'the segment of -1000.00 hangs from the zero line');
  near(hanging.bottom, yOf(-1000), 'down to -1000.00');
  app.close();
});

test('test_csd010_ac_6_6_the_current_year_is_lighter_and_labelled_ytd', async () => {
  const data = insightsOf(bankList('Example Savings'), [bankYear(2025, [['id-1', '100.00']]), bankYear(2026, [['id-1', '200.00']], { partial: true })]);
  const { app, chart } = await openInsights(data);
  assert.ok(groupOf(chart, 2026).classList.contains('partial'));
  assert.ok(!groupOf(chart, 2025).classList.contains('partial'));
  assert.equal(app.text(groupOf(chart, 2026).querySelector('.chart-year')), '2026 YTD');
  assert.match(charts, /\.partial \[class\*="bar-slot-"\][^{]*\{ opacity: \.55; \}/, 'the partial bar has a lighter fill');
  app.close();
});

test('test_csd010_ac_7_1_a_marker_is_drawn_across_the_bar_at_the_closing_balance', async () => {
  const axis = axisOf(0, 5000, 10000, 15000);
  const data = { axis, banks: bankList('Example Current Account', 'Example Savings'), years: [bankYear(2025, [['id-1', '2000.00'], ['id-2', '8000.00']])] };
  const { app, chart } = await openInsights(data);
  const group = groupOf(chart, 2025);
  const marker = group.querySelector('.chart-marker');
  const yOf = yScale(chart, axis);
  near(Number(marker.getAttribute('y1')), yOf(10000), 'the marker line is at 10000.00');
  const bar = group.querySelector('.chart-bar-box');
  assert.ok(Number(marker.getAttribute('x1')) < Number(bar.getAttribute('x')), 'it is wider than the bar, so it is drawn across it');
  assert.ok(Number(marker.getAttribute('x2')) > Number(bar.getAttribute('x')) + Number(bar.getAttribute('width')));
  assert.equal(app.text(group.querySelector('.chart-marker-label')), '10.00k');
  app.close();
});

test('test_csd010_ac_7_3_markers_are_not_connected_and_the_current_year_is_dashed', async () => {
  const data = insightsOf(bankList('Example Savings'), [bankYear(2024, [['id-1', '100.00']]), gapYear(2025), bankYear(2026, [['id-1', '300.00']], { partial: true })]);
  const { app, chart } = await openInsights(data);
  const markers = [...chart.querySelectorAll('.chart-marker')];
  assert.equal(markers.length, 2, 'a marker for 2024 and for 2026 only');
  assert.equal(chart.querySelectorAll('polyline, polygon, path.chart-marker').length, 0, 'no line joins the markers');
  assert.ok(markers[1].closest('.chart-year-group').classList.contains('partial'));
  assert.match(charts, /\.partial \.chart-marker \{ stroke-dasharray: 5 3; \}/, 'the marker of 2026 is dashed');
  assert.equal(groupOf(chart, 2025).querySelector('.chart-marker'), null, '2025 has no marker');
  app.close();
});

// ---- REQ-8, REQ-9: colours and legend ----

test('test_csd010_ac_8_1_a_bank_has_one_colour_everywhere', async () => {
  const banks = [{ id: 'id-1', name: 'Example Current Account', colour_slot: 0 }, { id: 'id-2', name: 'Example Savings', colour_slot: 1 }];
  const data = insightsOf(banks, [bankYear(2025, [['id-1', '100.00'], ['id-2', '200.00']]), bankYear(2026, [['id-1', '150.00'], ['id-2', '250.00']])]);
  const { app, chart, legend } = await openInsights(data);
  for (const year of [2025, 2026]) {
    const segment = shapes(groupOf(chart, year)).find((shape) => shape.dataset.legend === 'bank-id-2');
    assert.ok(segment.classList.contains('bar-slot-1'), `the segment of ${year} has the colour of slot 1`);
  }
  assert.ok(legend.querySelector('li[data-legend="bank-id-2"] .swatch').classList.contains('slot-1'), 'the legend swatch');
  await hover(app, groupOf(chart, 2026));
  const savings = tooltipRows(app, chart).find((row) => row.label === 'Example Savings');
  assert.equal(savings.swatch, 'slot-1', 'the tooltip swatch');
  assert.match(charts, /\.swatch\.slot-1, \.bar-slot-1 \{ background: var\(--bank-1\); fill: var\(--bank-1\); \}/, 'one colour for all three');
  app.close();
});

test('test_csd010_ac_8_2_colours_survive_a_deletion', async () => {
  const reads = exampleReads();
  const all = [
    { id: 'id-1', name: 'Example Current Account', colour_slot: 0 },
    { id: 'id-2', name: 'Example Savings', colour_slot: 1 },
    { id: 'id-3', name: 'Example Old Bank', colour_slot: 2 },
  ];
  let banks = all;
  const handlers = {
    get_bank_insights: () => insightsOf(banks, [bankYear(2026, banks.map((bank) => [bank.id, '100.00']))]),
    delete_bank_account: () => {
      banks = all.filter((bank) => bank.id !== 'id-1');
      return null;
    },
  };
  const app = await openApp({ reads, handlers });
  await app.click(app.item('bank'));
  await app.click(app.document.querySelector('[aria-label="Delete Example Current Account"]'));
  await app.click(app.$('confirm-ok'));
  assert.equal(app.callsOf('delete_bank_account').length, 1);
  await app.click(app.$('bank-tab-insights'));
  const chart = app.$('bank-insights-chart');
  const slots = shapes(groupOf(chart, 2026)).map((shape) => [shape.dataset.legend, [...shape.classList].find((name) => name.startsWith('bar-slot-'))]);
  assert.deepEqual(slots, [['bank-id-2', 'bar-slot-1'], ['bank-id-3', 'bar-slot-2']], 'the other banks keep the colours of slots 1 and 2');
  app.close();
});

test('test_csd010_ac_8_4_a_ninth_bank_shares_the_colour_of_the_first_and_is_named', async () => {
  const banks = Array.from({ length: 9 }, (_, index) => ({ id: `id-${index + 1}`, name: `Example Bank ${index + 1}`, colour_slot: index % 8 }));
  const data = insightsOf(banks, [bankYear(2026, banks.map((bank) => [bank.id, '100.00']))], 0, 500, 1000);
  const { app, chart, legend } = await openInsights(data);
  const colour = (id) => [...shapes(groupOf(chart, 2026)).find((shape) => shape.dataset.legend === `bank-${id}`).classList].find((name) => name.startsWith('bar-slot-'));
  assert.equal(colour('id-1'), 'bar-slot-0');
  assert.equal(colour('id-9'), colour('id-1'), 'the first and the ninth share a colour');
  const named = [...legend.querySelectorAll('li')].map((item) => app.text(item));
  assert.ok(named.includes('Example Bank 1') && named.includes('Example Bank 9'), 'each is named in the legend');
  await hover(app, groupOf(chart, 2026));
  const rows = tooltipRows(app, chart).map((row) => row.label);
  assert.ok(rows.includes('Example Bank 1') && rows.includes('Example Bank 9'), 'and in the tooltip');
  app.close();
});

test('test_csd010_ac_9_1_the_legend_lists_the_banks_in_creation_order_and_the_marker', async () => {
  const banks = [{ id: 'id-1', name: 'Example Savings', colour_slot: 0 }, { id: 'id-2', name: 'Example Current Account', colour_slot: 1 }];
  const { app, legend } = await openInsights(insightsOf(banks, [bankYear(2026, [['id-1', '100.00'], ['id-2', '200.00']])]));
  const items = [...legend.querySelectorAll('li')];
  assert.deepEqual(items.map((item) => app.text(item)), ['Example Savings', 'Example Current Account', 'Closing balance']);
  assert.ok(items[0].querySelector('.swatch.slot-0') && items[1].querySelector('.swatch.slot-1'), 'each with its colour swatch');
  assert.ok(items[2].querySelector('.swatch.marker'), 'the legend shows the closing-balance marker');
  app.close();
});

test('test_csd010_ac_9_5_a_deleted_bank_leaves_the_legend', async () => {
  const both = [{ id: 'id-1', name: 'Example Savings', colour_slot: 0 }, { id: 'id-2', name: 'Example Current Account', colour_slot: 1 }];
  let banks = both;
  const handlers = {
    get_bank_insights: () => insightsOf(banks, [bankYear(2026, banks.map((bank) => [bank.id, '100.00']))]),
    delete_bank_account: () => {
      banks = both.slice(1);
      return null;
    },
  };
  const reads = exampleReads();
  reads.list_bank_accounts.accounts[0].name = 'Example Savings';
  const app = await openApp({ reads, handlers });
  await app.click(app.item('bank'));
  await app.click(app.$('bank-tab-insights'));
  assert.deepEqual([...app.$('bank-insights-legend').querySelectorAll('li')].map((item) => app.text(item)).slice(0, 2), ['Example Savings', 'Example Current Account']);
  await app.click(app.$('bank-tab-overview'));
  await app.click(app.document.querySelector('[aria-label="Delete Example Savings"]'));
  await app.click(app.$('confirm-ok'));
  await app.click(app.$('bank-tab-insights'));
  assert.deepEqual([...app.$('bank-insights-legend').querySelectorAll('li')].map((item) => app.text(item)), ['Example Current Account', 'Closing balance']);
  app.close();
});

// ---- REQ-10: the tooltip ----

const banksOfTwo = () => bankList('Example Current Account', 'Example Savings');

test('test_csd010_ac_10_1_the_tooltip_shows_both_banks_and_the_closing_balance_and_no_count', async () => {
  const { app, chart } = await openInsights(insightsOf(banksOfTwo(), [bankYear(2026, [['id-1', '2500.00'], ['id-2', '8000.00']], { count: 7 })]));
  await hover(app, groups(chart)[0]);
  assert.equal(tooltipOf(chart).classList.contains('is-open'), true);
  const rows = tooltipRows(app, chart);
  const byLabel = Object.fromEntries(rows.map((row) => [row.label, row]));
  assert.equal(byLabel['Example Current Account'].value, '2,500.00');
  assert.equal(byLabel['Example Current Account'].swatch, 'slot-0');
  assert.equal(byLabel['Example Savings'].value, '8,000.00');
  assert.equal(byLabel['Example Savings'].swatch, 'slot-1');
  assert.equal(byLabel['Closing balance'].value, '10,500.00');
  assert.ok(!/entries|entry/i.test(app.text(tooltipOf(chart))), 'no number of entries');
  app.close();
});

test('test_csd010_ac_10_2_the_tooltip_follows_keyboard_focus', async () => {
  const data = insightsOf(banksOfTwo(), [bankYear(2025, [['id-1', '100.00']]), bankYear(2026, [['id-1', '2500.00'], ['id-2', '8000.00']])]);
  const { app, chart } = await openInsights(data);
  const second = groups(chart)[1];
  assert.equal(second.getAttribute('tabindex'), '0', 'a bar can take focus with Tab');
  second.dispatchEvent(new app.window.Event('focus'));
  assert.equal(tooltipOf(chart).classList.contains('is-open'), true);
  assert.equal(app.text(tooltipOf(chart).querySelector('.tip-title')), '2026');
  assert.equal(tooltipRows(app, chart).length, 3, 'the same tooltip');
  second.dispatchEvent(new app.window.Event('blur'));
  await afterTooltipCloses();
  assert.equal(tooltipOf(chart).classList.contains('is-open'), false, 'it disappears when focus leaves the bar');
  app.close();
});

test('test_csd010_ac_10_3_the_accessible_label_names_each_bank_and_says_negative', async () => {
  const banks = bankList('Example Current Account', 'Example Overdraft');
  const { app, chart } = await openInsights(insightsOf(banks, [bankYear(2026, [['id-1', '5000.00'], ['id-2', '-1000.00']], { count: 2 })]));
  const spoken = groups(chart)[0].getAttribute('aria-label');
  assert.ok(spoken.startsWith('2026'), 'the year');
  assert.ok(spoken.includes('Example Current Account 5,000.00'), 'each bank with its balance');
  assert.ok(spoken.includes('Example Overdraft -1,000.00 (negative)'), 'Example Overdraft is announced as negative');
  assert.ok(spoken.includes('closing balance 4,000.00'), 'and the closing balance');
  assert.ok(!/entries|entry|\b2 /.test(spoken.replace('2026', '')), 'no number of entries');
  app.close();
});

test('test_csd010_ac_10_4_a_partial_year_says_so_in_the_tooltip', async () => {
  const data = insightsOf(banksOfTwo(), [bankYear(2025, [['id-1', '100.00']]), bankYear(2026, [['id-1', '200.00']], { partial: true })]);
  const { app, chart } = await openInsights(data);
  await hover(app, groupOf(chart, 2026));
  assert.equal(app.text(tooltipOf(chart).querySelector('.tip-title')), '2026 YTD');
  assert.equal(app.text(tooltipOf(chart).querySelector('.tip-note')), 'Partial year: entries so far');
  await hover(app, groupOf(chart, 2025));
  assert.equal(tooltipOf(chart).querySelector('.tip-note'), null, 'a full year has no such note');
  app.close();
});

test('test_csd010_ac_10_6_a_year_without_a_bar_shows_the_note_and_no_bank_rows', async () => {
  const data = insightsOf(banksOfTwo(), [bankYear(2022, [['id-1', '100.00']]), gapYear(2023), bankYear(2024, [['id-1', '100.00']])]);
  const { app, chart } = await openInsights(data);
  await hover(app, groupOf(chart, 2023));
  const tooltip = tooltipOf(chart);
  assert.equal(tooltip.classList.contains('is-open'), true);
  assert.equal(tooltip.querySelectorAll('dt').length, 0, 'no bank rows');
  assert.equal(app.text(tooltip.querySelector('.tip-note')), 'No entries');
  app.close();
});

test('test_csd010_ac_10_7_the_rows_follow_the_bar_from_top_to_bottom', async () => {
  const banks = bankList('Example Current Account', 'Example Savings', 'Example Overdraft');
  const data = insightsOf(banks, [bankYear(2026, [['id-1', '2500.00'], ['id-2', '8000.00'], ['id-3', '-1000.00']])]);
  const { app, chart } = await openInsights(data);
  await hover(app, groups(chart)[0]);
  const rows = tooltipRows(app, chart);
  assert.deepEqual(rows.map((row) => row.label), ['Example Savings', 'Example Current Account', 'Example Overdraft', 'Closing balance']);
  assert.equal(rows.at(-1).value, '9,500.00', 'the closing balance is listed after them');
  app.close();
});

test('test_csd010_ac_10_8_with_only_positive_segments_the_highest_is_listed_first', async () => {
  const data = insightsOf(banksOfTwo(), [bankYear(2025, [['id-1', '2000.00'], ['id-2', '8000.00']])]);
  const { app, chart } = await openInsights(data);
  await hover(app, groups(chart)[0]);
  assert.deepEqual(tooltipRows(app, chart).map((row) => row.label), ['Example Savings', 'Example Current Account', 'Closing balance']);
  app.close();
});

// ---- REQ-13, REQ-14, REQ-15 ----

test('test_csd010_ac_13_4_a_segment_is_announced_with_its_bank_year_and_balance', async () => {
  const data = insightsOf(banksOfTwo(), [bankYear(2026, [['id-1', '2500.00'], ['id-2', '8000.00']])]);
  const { chart, app } = await openInsights(data);
  const savings = shapes(groupOf(chart, 2026)).find((shape) => shape.dataset.legend === 'bank-id-2');
  assert.equal(savings.getAttribute('aria-label'), 'Example Savings, 2026, 8,000.00');
  app.close();
});

test('test_csd010_ac_14_1_accounts_without_entries_show_the_note_and_no_chart', async () => {
  const reads = exampleReads();
  reads.list_bank_entries = [];
  reads.yearly_bank_totals = [];
  const { app, chart } = await openInsights({ axis: axisOf(0, 1), banks: [], years: [] }, { reads });
  assert.equal(app.isShown(app.$('bank-insights-empty')), true);
  assert.equal(app.textOf('bank-insights-empty'), 'No entries yet. The chart appears once there is an entry.');
  assert.equal(app.isShown(app.$('bank-insights-scroll')), false);
  assert.equal(chart.querySelector('svg'), null, 'no chart is drawn');
  app.close();
});

test('test_csd010_ac_15_1_the_figures_follow_a_change_on_the_overview', async () => {
  let closing = '2500.00';
  const handlers = {
    get_bank_insights: () => insightsOf(bankList('Example Current Account'), [bankYear(2026, [['id-1', closing]])]),
    add_bank_entry: () => {
      closing = '3000.00';
      return {};
    },
  };
  const app = await openApp({ handlers });
  await app.click(app.item('bank'));
  await app.click(app.$('bank-tab-insights'));
  const marker = () => app.text(app.$('bank-insights-chart').querySelector('.chart-marker-label'));
  assert.equal(marker(), '2.50k');

  await app.click(app.$('bank-tab-overview'));
  await app.click(app.$('bank-entry-add'));
  const form = app.$('bank-entry-form');
  form.elements.period.value = '';
  await app.type(form.elements.period, '202610');
  await app.type(form.elements.balance, '3000.00');
  form.requestSubmit();
  await app.settle();
  assert.equal(app.callsOf('add_bank_entry').length, 1);

  await app.click(app.$('bank-tab-insights'));
  assert.equal(marker(), '3.00k', 'the 2026 marker shows 3000.00');
  app.close();
});

test('test_csd010_ac_15_2_deleting_the_last_entry_shows_the_note_again', async () => {
  let insights = insightsOf(bankList('Example Current Account'), [bankYear(2026, [['id-1', '2500.00']])]);
  const reads = exampleReads();
  reads.list_bank_entries = reads.list_bank_entries.filter((e) => e.account_id === 'id-1').slice(0, 1);
  const handlers = {
    get_bank_insights: () => insights,
    delete_bank_entry: () => {
      insights = { axis: axisOf(0, 1), banks: [], years: [] };
      return null;
    },
  };
  const app = await openApp({ reads, handlers });
  await app.click(app.item('bank'));
  await app.click(app.$('bank-tab-insights'));
  assert.ok(app.$('bank-insights-chart').querySelector('svg'), 'the Insights tab showed a chart');

  await app.click(app.$('bank-tab-overview'));
  await app.click(app.document.querySelector('#bank-entries-rows [aria-label^="Delete"]'));
  await app.click(app.$('confirm-ok'));
  assert.equal(app.callsOf('delete_bank_entry').length, 1);

  await app.click(app.$('bank-tab-insights'));
  assert.equal(app.textOf('bank-insights-empty'), 'No entries yet. The chart appears once there is an entry.');
  assert.equal(app.$('bank-insights-chart').querySelector('svg'), null, 'no chart is drawn');
  app.close();
});
