// Display ACs of CSD-014 (expense insights): the stacked bars per category, the tooltip, the
// highlight, the legend and the refresh. The answers are small series made for one AC (what Rust
// returns for them is proven by `cargo test`). jsdom has no layout, so the placement of the
// tooltip (AC-8.5, the centring in AC-8.6), the wrapping of the legend in a real window (AC-7.2)
// and the look of the colours stay on the manual checklist.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { setTimeout as sleep } from 'node:timers/promises';
import { openApp } from './harness.js';
import { exampleReads } from './fixtures.js';

const read = (name) => readFileSync(fileURLToPath(new URL(`../styles/${name}`, import.meta.url)), 'utf8');
const chartsCss = read('charts.css');
const baseCss = read('base.css').replace(/\/\*[\s\S]*?\*\//g, '');

const CATEGORIES = ['Home', 'Vehicle', 'Holiday', 'Sport', 'Study', 'Appliances', 'Hobby', 'Other'];
const categories = CATEGORIES.map((name, colour_slot) => ({ name, colour_slot }));
const ticks = (...pairs) => ({ ticks: pairs.map(([value, label = value]) => ({ value, label })) });
const AXIS = ticks(['0.00', '0'], ['3300.00', '3.3k'], ['6600.00', '6.6k'], ['9900.00', '9.9k']);

/** A year as Rust sends it: `segments` as `[category, amount]`, stacked bottom to top in this order. */
function year(value, segments, { partial = false, count = segments.length } = {}) {
  let low = 0;
  const pieces = segments.map(([category, amount], index) => {
    const piece = { series: `category-${category}`, low: low.toFixed(2), high: (low + Number(amount)).toFixed(2), hangs: false, rounding: index === segments.length - 1 ? 'top' : 'none' };
    low += Number(amount);
    return piece;
  });
  return {
    year: value, is_partial: partial, expense_count: count, marker: null, pieces,
    segments: segments.map(([category, amount]) => ({ category, amount })),
    total: low.toFixed(2),
  };
}
const series = (years, axis = AXIS) => ({ axis, categories, years });
const twoYears = () => series([year(2025, [['Home', '5000.00'], ['Hobby', '400.00']]), year(2026, [['Home', '5700.00'], ['Hobby', '900.00']], { count: 3, partial: true })]);

async function openInsights(payload, reads = exampleReads()) {
  if (payload) reads.get_expense_insights = payload;
  const app = await openApp({ reads });
  await app.click(app.item('expenses'));
  await app.click(app.$('expenses-tab-insights'));
  return { app, chart: app.$('expenses-insights-chart'), legend: app.$('expenses-insights-legend') };
}

const groups = (chart) => [...chart.querySelectorAll('.chart-year-group')];
const pieces = (group) => [...group.querySelectorAll('rect[data-legend], path[data-legend]')];
const tooltip = (chart) => chart.querySelector('.chart-tooltip');
const num = (node, name) => Number(node.getAttribute(name));
const rowsOf = (app, chart) => {
  const labels = [...tooltip(chart).querySelectorAll('dt')].map((dt) => app.text(dt));
  const values = [...tooltip(chart).querySelectorAll('dd')].map((dd) => app.text(dd));
  return labels.map((label, index) => [label, values[index]]);
};
async function hover(app, group) {
  group.dispatchEvent(new app.window.MouseEvent('mouseenter'));
  await app.settle();
}
const over = (app, node) => node.dispatchEvent(new app.window.MouseEvent('mouseover', { bubbles: true }));
const lit = (chart, key) => marksOf(chart, key).filter((node) => node.matches('rect, path'));
const marksOf = (chart, key) => [...chart.querySelectorAll(`[data-legend="${key}"]`)];
const gridY = (chart, label) => {
  const labels = [...chart.querySelectorAll('.chart-axis-label')];
  const lines = [...chart.querySelectorAll('.chart-grid, .chart-zero')];
  return num(lines[labels.findIndex((node) => node.textContent === label)], 'y1');
};

// ---- REQ-5: bars ----

test('test_csd014_ac_5_1_stacked_bars_per_year', async () => {
  const { app, chart } = await openInsights(twoYears());
  assert.deepEqual([...chart.querySelectorAll('.chart-year')].map((node) => node.textContent), ['2025', '2026 YTD'], '2025 on the left, 2026 on the right');
  const [first, second] = groups(chart);
  assert.deepEqual(pieces(first).map((piece) => piece.dataset.legend), ['category-Home', 'category-Hobby'], 'one segment per category with an expense');
  assert.equal(pieces(second).length, 2);
  app.close();
});

test('test_csd014_ac_5_2_the_axis_covers_the_highest_total_and_starts_at_zero', async () => {
  const { app, chart } = await openInsights(twoYears());
  const labels = [...chart.querySelectorAll('.chart-axis-label')].map((node) => node.textContent);
  assert.deepEqual(labels.sort(), ['0', '3.3k', '6.6k', '9.9k'].sort(), 'the ticks received');
  assert.ok(gridY(chart, '9.9k') < gridY(chart, '6.6k'), 'the top of the axis (9900.00) is above the highest total (6600.00)');
  assert.equal(gridY(chart, '0'), num(chart.querySelector('.chart-zero'), 'y1'), 'the axis starts at 0');
  app.close();
});

test('test_csd014_ac_5_3_the_bar_height_is_the_total', async () => {
  const { app, chart } = await openInsights(twoYears());
  const box = groups(chart)[1].querySelector('.chart-bar-box');
  assert.ok(Math.abs(num(box, 'y') - gridY(chart, '6.6k')) < 0.01, 'the top of the 2026 bar is at 6600.00');
  assert.ok(Math.abs(num(box, 'y') + num(box, 'height') - gridY(chart, '0')) < 0.01, 'and its bottom at zero');
  app.close();
});

test('test_csd014_ac_5_4_a_gap_year_is_an_empty_slot', async () => {
  const { app, chart } = await openInsights(series([year(2022, [['Home', '100.00']]), year(2023, []), year(2024, [['Home', '100.00']])]));
  const gap = groups(chart)[1];
  assert.equal(gap.querySelector('.chart-year').textContent, '2023');
  assert.equal(pieces(gap).length, 0, 'no bar');
  app.close();
});

test('test_csd014_ac_5_5_the_current_year_is_marked', async () => {
  const { app, chart } = await openInsights(twoYears());
  const [full, partial] = groups(chart);
  assert.ok(partial.classList.contains('partial'));
  assert.ok(!full.classList.contains('partial'));
  assert.equal(partial.querySelector('.chart-year').textContent, '2026 YTD');
  assert.match(chartsCss, /\.partial \[class\*="bar-slot-"\][^{]*\{[^}]*opacity: \.55/, 'the style sheet gives it a lighter fill');
  app.close();
});

test('test_csd014_ac_5_6_there_is_no_total_marker', async () => {
  const { app, chart, legend } = await openInsights(twoYears());
  assert.equal(chart.querySelectorAll('.chart-marker, .chart-marker-label').length, 0, 'no marker line for a year');
  assert.equal(legend.querySelectorAll('[data-legend="marker"]').length, 0, 'and none in the legend');
  app.close();
});

// ---- REQ-6: colours ----

test('test_csd014_ac_6_1_a_category_has_one_colour_everywhere', async () => {
  const { app, chart, legend } = await openInsights(twoYears());
  await hover(app, groups(chart)[1]);
  const hobby = lit(chart, 'category-Hobby');
  assert.equal(hobby.length, 2, 'Hobby has a segment in 2025 and in 2026');
  assert.ok(hobby.every((node) => node.classList.contains('bar-slot-6')), 'the segments are slot 6');
  assert.ok(legend.querySelector('[data-legend="category-Hobby"] .swatch.slot-6'), 'the legend swatch');
  assert.ok(tooltip(chart).querySelector('dt[data-legend="category-Hobby"] .swatch.slot-6'), 'the tooltip swatch');
  app.close();
});

test('test_csd014_ac_6_3_the_palette_is_distinct_and_not_too_saturated_in_both_modes', () => {
  const all = [...baseCss.matchAll(/--bank-(\d): hsl\((\d+), (\d+)%, (\d+)%\)/g)].map((match) => ({ slot: Number(match[1]), hue: Number(match[2]), saturation: Number(match[3]) }));
  assert.equal(all.length, 24, 'light, dark by system setting and dark by choice: eight colours each');
  for (const [mode, start] of [['light', 0], ['dark (system)', 8], ['dark (chosen)', 16]]) {
    const colours = all.slice(start, start + 8);
    assert.deepEqual(colours.map((colour) => colour.slot), [0, 1, 2, 3, 4, 5, 6, 7], `${mode}: slots 0 to 7`);
    for (const colour of colours) assert.ok(colour.saturation <= 60, `${mode}: slot ${colour.slot} has ${colour.saturation}% saturation`);
    for (const a of colours) {
      for (const b of colours.filter((other) => other.slot > a.slot)) {
        const gap = Math.abs(a.hue - b.hue);
        assert.ok(Math.min(gap, 360 - gap) >= 15, `${mode}: slots ${a.slot} and ${b.slot} are distinct (hues ${a.hue} and ${b.hue})`);
      }
    }
  }
});

// ---- REQ-7: legend ----

test('test_csd014_ac_7_1_the_legend_lists_all_categories', async () => {
  const { app, legend } = await openInsights(series([year(2026, [['Home', '100.00']])]));
  const items = [...legend.querySelectorAll('li')];
  assert.deepEqual(items.map((item) => app.text(item)), CATEGORIES, 'in the fixed order, also those without expenses');
  assert.deepEqual(items.map((item) => item.querySelector('.swatch').className), CATEGORIES.map((_, slot) => `swatch slot-${slot}`));
  app.close();
});

test('test_csd014_ac_7_2_the_legend_wraps_in_a_narrow_window_and_leaves_no_category_out', async () => {
  const { app, legend } = await openInsights();
  assert.equal(legend.querySelectorAll('li').length, 8, 'no category is left out');
  assert.match(chartsCss, /\.legend\s*\{[^}]*flex-wrap:\s*wrap/, 'the style sheet lets the items wrap onto more lines');
  app.close();
});

// ---- REQ-8: tooltip ----

test('test_csd014_ac_8_1_the_tooltip_shows_the_categories_and_the_total', async () => {
  const { app, chart } = await openInsights(twoYears());
  await hover(app, groups(chart)[1]);
  assert.equal(tooltip(chart).classList.contains('is-open'), true);
  assert.deepEqual(rowsOf(app, chart), [['Hobby', '900.00'], ['Home', '5,700.00'], ['Total', '6,600.00']], 'top to bottom like the bar, no other category');
  assert.ok(tooltip(chart).querySelector('dt[data-legend="category-Home"] .swatch.slot-0'));
  assert.ok(tooltip(chart).querySelector('dt[data-legend="category-Hobby"] .swatch.slot-6'));
  assert.equal(tooltip(chart).querySelector('dt:last-of-type .swatch'), null, 'the Total row has no swatch');
  assert.doesNotMatch(app.text(tooltip(chart)), /\b3\b|expenses?\b(?! so far)/i, 'no number of expenses');
  app.close();
});

test('test_csd014_ac_8_2_the_tooltip_follows_keyboard_focus', async () => {
  const { app, chart } = await openInsights(twoYears());
  const group = groups(chart)[1];
  assert.equal(group.getAttribute('tabindex'), '0');
  group.dispatchEvent(new app.window.Event('focus'));
  assert.equal(tooltip(chart).classList.contains('is-open'), true);
  assert.deepEqual(rowsOf(app, chart), [['Hobby', '900.00'], ['Home', '5,700.00'], ['Total', '6,600.00']], 'the same tooltip');
  group.dispatchEvent(new app.window.Event('blur'));
  await sleep(150);
  assert.equal(tooltip(chart).classList.contains('is-open'), false, 'it disappears when focus leaves the bar');
  app.close();
});

test('test_csd014_ac_8_3_the_bar_has_an_accessible_label', async () => {
  const { app, chart } = await openInsights(twoYears());
  assert.equal(groups(chart)[1].getAttribute('aria-label'), '2026 YTD: Home 5,700.00, Hobby 900.00, total 6,600.00');
  app.close();
});

test('test_csd014_ac_8_4_a_partial_year_says_so', async () => {
  const { app, chart } = await openInsights(twoYears());
  await hover(app, groups(chart)[1]);
  assert.equal(app.text(tooltip(chart).querySelector('.tip-note')), 'Partial year: expenses so far');
  await hover(app, groups(chart)[0]);
  assert.equal(tooltip(chart).querySelector('.tip-note'), null, 'a full year has no such note');
  app.close();
});

test('test_csd014_ac_8_6_a_year_without_expenses_shows_a_note_and_no_rows', async () => {
  const { app, chart } = await openInsights(series([year(2022, [['Home', '100.00']]), year(2023, []), year(2024, [['Home', '100.00']])]));
  await hover(app, groups(chart)[1]);
  assert.equal(app.text(tooltip(chart).querySelector('.tip-note')), 'No expenses');
  assert.equal(tooltip(chart).querySelectorAll('dt').length, 0, 'no Total row and no category rows');
  app.close();
});

test('test_csd014_ac_8_7_one_expense', async () => {
  const { app, chart } = await openInsights(series([year(2026, [['Appliances', '650.00']], { count: 1 })]));
  await hover(app, groups(chart)[0]);
  assert.deepEqual(rowsOf(app, chart), [['Appliances', '650.00'], ['Total', '650.00']]);
  assert.doesNotMatch(app.text(tooltip(chart)), /\b1\b/, 'no number of expenses');
  app.close();
});

test('test_csd014_ac_8_8_the_highest_segment_first', async () => {
  // Rust stacks the pieces bottom to top (Home, Sport, Other); the tooltip reads like the bar, from the top.
  const { app, chart } = await openInsights(series([year(2026, [['Home', '100.00'], ['Sport', '200.00'], ['Other', '300.00']])]));
  await hover(app, groups(chart)[0]);
  assert.deepEqual(rowsOf(app, chart), [['Other', '300.00'], ['Sport', '200.00'], ['Home', '100.00'], ['Total', '600.00']]);
  app.close();
});

// ---- REQ-9: highlight ----

test('test_csd014_ac_9_1_hovering_a_segment_highlights_its_category_in_every_year', async () => {
  const { app, chart, legend } = await openInsights(twoYears());
  over(app, pieces(groups(chart)[0])[0]); // the Home segment of 2025
  const home = lit(chart, 'category-Home');
  const hobby = lit(chart, 'category-Hobby');
  assert.equal(home.length, 2);
  assert.ok(home.every((node) => node.classList.contains('hl') && !node.classList.contains('dim')), 'every Home segment keeps its colour');
  assert.ok(hobby.every((node) => node.classList.contains('dim')), 'every Hobby segment is eased down');
  assert.ok(legend.querySelector('[data-legend="category-Home"]').classList.contains('hl'));
  assert.ok(legend.querySelector('[data-legend="category-Vehicle"]').classList.contains('dim'), 'the other legend items are eased down');
  assert.match(chartsCss, /\.chart-year-group \[data-legend\]\.dim \{ opacity: \.7/, '70% opacity');
  assert.match(chartsCss, /\.legend li\.dim \{ opacity: \.7/);
  assert.match(chartsCss, /\.legend li\.hl \.legend-label[^{]*\{ font-weight: 600/, 'the highlighted item is heavier');
  app.close();
});

test('test_csd014_ac_9_2_hovering_a_legend_item_highlights_its_category', async () => {
  const { app, chart, legend } = await openInsights(twoYears());
  legend.querySelector('[data-legend="category-Home"]').dispatchEvent(new app.window.Event('mouseenter'));
  assert.ok(lit(chart, 'category-Home').every((node) => node.classList.contains('hl')), 'Home keeps its colour in every year');
  assert.ok(lit(chart, 'category-Hobby').every((node) => node.classList.contains('dim')), 'the other segments are eased down');
  app.close();
});

test('test_csd014_ac_9_3_focusing_a_legend_item_highlights_like_hovering', async () => {
  const { app, chart, legend } = await openInsights(twoYears());
  const item = legend.querySelector('[data-legend="category-Home"]');
  item.dispatchEvent(new app.window.Event('focus'));
  assert.ok(lit(chart, 'category-Home').every((node) => node.classList.contains('hl')));
  assert.ok(lit(chart, 'category-Hobby').every((node) => node.classList.contains('dim')));
  item.dispatchEvent(new app.window.Event('blur'));
  assert.equal(chart.querySelectorAll('.hl, .dim').length + legend.querySelectorAll('.hl, .dim').length, 0, 'it ends when focus leaves');
  app.close();
});

test('test_csd014_ac_9_4_leaving_the_segment_highlights_nothing_and_keeps_the_tooltip', async () => {
  const { app, chart, legend } = await openInsights(twoYears());
  const group = groups(chart)[0];
  await hover(app, group);
  over(app, pieces(group)[0]);
  assert.ok(chart.querySelectorAll('.hl').length > 0);
  over(app, group.querySelector('.chart-hit')); // empty space in the year's column
  assert.equal(chart.querySelectorAll('.hl, .dim').length + legend.querySelectorAll('.hl, .dim').length, 0);
  assert.equal(tooltip(chart).classList.contains('is-open'), true, 'the tooltip is still shown');
  app.close();
});

test('test_csd014_ac_9_5_the_highlight_is_not_colour_alone', async () => {
  const { app, chart, legend } = await openInsights(twoYears());
  const before = pieces(groups(chart)[0]).map((node) => node.getAttribute('class').replace(/ (hl|dim)/g, ''));
  over(app, pieces(groups(chart)[0])[0]);
  assert.ok(legend.querySelector('.hl'), 'one legend item is highlighted');
  assert.match(chartsCss, /\.legend li\.hl \.legend-label[^{]*\{ font-weight: 600/, 'with a heavier font weight');
  const after = pieces(groups(chart)[0]).map((node) => node.getAttribute('class').replace(/ (hl|dim)/g, ''));
  assert.deepEqual(after, before, 'the other segments keep their colour class and are only lighter');
  assert.match(chartsCss, /\[data-legend\]\.dim \{ opacity: \.7/);
  app.close();
});

test('test_csd014_ac_9_6_keyboard_focus_on_a_year_highlights_nothing', async () => {
  const { app, chart, legend } = await openInsights(twoYears());
  groups(chart)[0].dispatchEvent(new app.window.Event('focus'));
  assert.equal(chart.querySelectorAll('.hl, .dim').length + legend.querySelectorAll('.hl, .dim').length, 0);
  app.close();
});

test('test_csd014_ac_9_7_a_category_without_expenses_dims_every_segment', async () => {
  const { app, chart, legend } = await openInsights(twoYears());
  const sport = legend.querySelector('[data-legend="category-Sport"]');
  sport.dispatchEvent(new app.window.Event('mouseenter'));
  const all = chart.querySelectorAll('rect[data-legend], path[data-legend]');
  assert.ok(all.length > 0);
  assert.ok([...all].every((node) => node.classList.contains('dim')), 'all segments are eased down');
  assert.ok(sport.classList.contains('hl'), 'the Sport item is highlighted (heavier font)');
  app.close();
});

// ---- REQ-10 and REQ-11 ----

test('test_csd014_ac_10_1_the_tooltip_follows_the_hovered_year_and_respects_reduced_motion', async () => {
  const { app, chart } = await openInsights(twoYears());
  const [first, second] = groups(chart);
  await hover(app, first);
  assert.equal(app.text(tooltip(chart).querySelector('.tip-title')), '2025');
  await hover(app, second);
  assert.equal(app.text(tooltip(chart).querySelector('.tip-title')), '2026 YTD', 'hovering the next year shows that year');
  assert.equal(tooltip(chart).classList.contains('is-open'), true);
  assert.match(chartsCss, /prefers-reduced-motion: reduce\)\s*\{[^}]*\.chart-tooltip[^}]*transition: none/, 'no motion for those who ask for less');
  app.close();
});

test('test_csd014_ac_11_4_a_segment_has_an_accessible_label', async () => {
  const { app, chart } = await openInsights(twoYears());
  assert.equal(pieces(groups(chart)[1])[0].getAttribute('aria-label'), 'Home, 2026, 5,700.00');
  app.close();
});

// ---- REQ-12 and REQ-13 ----

test('test_csd014_ac_12_1_without_expenses_there_is_a_note_and_no_chart', async () => {
  const { app, chart } = await openInsights(series([]), (() => {
    const reads = exampleReads();
    reads.list_expenses = { expenses: [], total: '0.00' };
    reads.yearly_expense_totals = [];
    return reads;
  })());
  assert.equal(app.isShown(app.$('expenses-insights-empty')), true);
  assert.equal(app.textOf('expenses-insights-empty'), 'No expenses yet. The chart appears once there is an expense.');
  assert.equal(chart.querySelectorAll('svg').length, 0, 'no chart is drawn');
  app.close();
});

test('test_csd014_ac_13_1_the_figures_follow_changes', async () => {
  const { app, chart } = await openInsights(series([year(2026, [['Home', '900.00']], { partial: true })]));
  assert.match(groups(chart)[0].getAttribute('aria-label'), /total 900\.00/);
  await app.click(app.$('expenses-tab-overview'));
  app.bridge.table.get_expense_insights = series([year(2026, [['Home', '900.00'], ['Appliances', '650.00']], { partial: true })]);
  const before = app.callsOf('get_expense_insights').length;
  await app.click(app.$('expenses-tab-insights'));
  assert.equal(app.callsOf('get_expense_insights').length, before + 1, 'opening Insights asks Rust again');
  assert.match(groups(chart)[0].getAttribute('aria-label'), /total 1,550\.00/, 'the 2026 bar is 1550.00 high');
  const box = groups(chart)[0].querySelector('.chart-bar-box');
  const zero = gridY(chart, '0');
  const expectedTop = zero - ((zero - gridY(chart, '3.3k')) * 1550) / 3300;
  assert.ok(Math.abs(num(box, 'y') - expectedTop) < 0.01, 'the bar is drawn 1550.00 high from the new pieces');
  app.close();
});

test('test_csd014_ac_13_2_the_last_expense_is_deleted', async () => {
  const { app, chart } = await openInsights(series([year(2026, [['Home', '900.00']], { partial: true })]));
  assert.equal(chart.querySelectorAll('svg').length > 0, true);
  await app.click(app.$('expenses-tab-overview'));
  app.bridge.table.get_expense_insights = series([]);
  await app.click(app.$('expenses-tab-insights'));
  assert.equal(app.isShown(app.$('expenses-insights-empty')), true);
  assert.equal(app.textOf('expenses-insights-empty'), 'No expenses yet. The chart appears once there is an expense.');
  assert.equal(chart.querySelectorAll('svg').length, 0, 'no chart is drawn');
  app.close();
});

// ---- REQ-14 ----

test('test_csd014_ac_14_1_the_chart_has_the_shared_size_and_zero_line', async () => {
  const years = Array.from({ length: 20 }, (_, index) => year(2007 + index, [['Home', '1000.00']]));
  const { app, chart } = await openInsights(series(years, ticks(['0.00', '0'], ['1000.00', '1k'])));
  assert.equal(chart.querySelectorAll('.chart-zero').length, 1, 'one zero line');
  const centres = [...chart.querySelectorAll('.chart-year')].map((node) => num(node, 'x'));
  for (let i = 1; i < centres.length; i += 1) assert.ok(centres[i] - centres[i - 1] >= 64, 'each slot is at least 64 px wide');
  assert.ok(parseFloat(chart.querySelector('svg').style.minWidth) >= 20 * 64, 'so the chart scrolls horizontally');
  app.close();
});
