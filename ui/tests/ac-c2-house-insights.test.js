// CSD-016 display ACs: the Insights tab of the Houses page (tabs, bars, marker, colours, legend,
// tooltip, highlight, accessible labels, fresh figures). The answers are what Rust returns for small
// series made for one AC; the page only displays them. jsdom has no layout, so the placement of the
// tooltip (AC-10.6, and its centring in AC-10.7) and the wrapping of the legend (AC-9.3) stay by hand.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { openApp } from './harness.js';
import { emptyReads, exampleReads } from './fixtures.js';

const read = (name) => readFileSync(fileURLToPath(new URL(`../styles/${name}`, import.meta.url)), 'utf8');
const charts = read('charts.css');
const base = read('base.css');

const house = (id, name, slot) => ({ id, name, colour_slot: slot });
const LODGE = house('id-1', 'Example Lodge', 1);
const HOUSE = house('id-2', 'Example House', 0);

/** One year as Rust returns it: `segments` as `[house id, value]`, stacked in that order (a value of 0.00 gets no piece). */
function year(number, segments, { change = null, partial = false, count = segments.length } = {}) {
  let low = 0;
  const pieces = [];
  for (const [id, value] of segments) {
    const high = low + Number(value);
    if (Number(value) !== 0) pieces.push({ series: `house-${id}`, low: low.toFixed(2), high: high.toFixed(2), hangs: false, rounding: 'none' });
    low = high;
  }
  const total = low.toFixed(2);
  return {
    year: number,
    is_partial: partial,
    closing_value: total,
    change,
    valuation_count: segments.length === 0 ? 0 : count,
    segments: segments.map(([id, value]) => ({ house_id: id, value })),
    pieces,
    marker: segments.length === 0 ? null : { value: total, label: `${(low / 1000).toFixed(2)}k` },
  };
}

const axis = (...values) => ({ ticks: values.map((value) => ({ value: `${value}.00`, label: `${value / 1000}k` })) });

async function openInsights(houses, years, ticks = axis(0, 400000), reads = exampleReads()) {
  reads.get_house_insights = { axis: ticks, houses, years };
  const app = await openApp({ reads });
  await app.click(app.item('houses'));
  await app.click(app.$('houses-tab-insights'));
  const chart = app.$('houses-insights-chart');
  const groups = () => [...chart.querySelectorAll('.chart-year-group')];
  const shapes = (group) => [...group.querySelectorAll('rect[data-legend], path[data-legend]')];
  const hover = async (group) => {
    group.dispatchEvent(new app.window.MouseEvent('mouseenter'));
    await app.settle();
  };
  const tooltip = () => chart.querySelector('.chart-tooltip');
  const rows = () => [...tooltip().querySelectorAll('dt')].map((dt, index) => [app.text(dt), app.text(tooltip().querySelectorAll('dd')[index])]);
  const reopen = async (answer) => {
    app.bridge.table.get_house_insights = answer;
    await app.click(app.$('houses-tab-overview'));
    await app.click(app.$('houses-tab-insights'));
  };
  return { app, chart, groups, shapes, hover, tooltip, rows, reopen, legend: app.$('houses-insights-legend') };
}

const num = (node, name) => Number(node.getAttribute(name));
const marks = (root, key) => [...root.querySelectorAll(`[data-legend="${key}"]`)];
const pieces = (root, key) => marks(root, key).filter((node) => node.matches('rect, path'));
const gridlines = (chart) => {
  const labels = [...chart.querySelectorAll('.chart-axis-label')];
  return [...chart.querySelectorAll('.chart-grid, .chart-zero')]
    .map((line, index) => ({ y: num(line, 'y1'), zero: line.classList.contains('chart-zero'), label: labels[index].textContent }))
    .sort((a, b) => a.y - b.y);
};

// ---- REQ-1: tabs ----

test('test_csd016_ac_1_1_overview_is_shown_first_with_its_three_sections', async () => {
  const app = await openApp();
  await app.click(app.item('houses'));
  assert.equal(app.$('houses-tab-overview').getAttribute('aria-selected'), 'true');
  assert.equal(app.isShown(app.$('houses-overview')), true);
  assert.equal(app.isShown(app.$('houses-insights')), false);
  const titles = [...app.$('houses-overview').querySelectorAll('.panel-title')].map((node) => app.text(node));
  assert.deepEqual(titles, ['Houses', 'Yearly values', 'Valuations']);
  app.close();
});

test('test_csd016_ac_1_2_switching_to_insights_hides_the_overview', async () => {
  const app = await openApp();
  await app.click(app.item('houses'));
  await app.click(app.$('houses-tab-insights'));
  assert.equal(app.isShown(app.$('houses-overview')), false);
  assert.equal(app.isShown(app.$('houses-insights')), true);
  assert.ok(app.$('houses-insights-chart').querySelector('.chart-year-group'), 'the chart is drawn');
  app.close();
});

test('test_csd016_ac_1_3_no_houses_shows_the_prompt_and_no_tabs', async () => {
  const app = await openApp({ reads: emptyReads() });
  await app.click(app.item('houses'));
  assert.equal(app.isShown(app.$('houses-empty')), true);
  assert.match(app.text(app.$('houses-empty').querySelector('button')), /^Add your first /);
  assert.equal(app.isShown(app.$('houses-tab-overview')), false);
  assert.equal(app.isShown(app.$('houses-tab-insights')), false);
  app.close();
});

// ---- REQ-6: chart ----

test('test_csd016_ac_6_1_one_stacked_bar_per_year_with_a_segment_per_house', async () => {
  const { app, groups, shapes } = await openInsights(
    [LODGE, HOUSE],
    [year(2025, [['id-1', '50000.00'], ['id-2', '300000.00']]), year(2026, [['id-2', '310000.00']])],
  );
  const [first, second] = groups();
  assert.deepEqual(groups().map((group) => group.querySelector('.chart-year').textContent), ['2025', '2026'], '2025 on the left, 2026 on the right');
  assert.deepEqual(shapes(first).map((shape) => shape.dataset.legend), ['house-id-1', 'house-id-2']);
  assert.deepEqual(shapes(second).map((shape) => shape.dataset.legend), ['house-id-2'], 'a house without a valuation has no segment');
  app.close();
});

test('test_csd016_ac_6_2_the_axis_starts_at_zero_and_reaches_the_highest_total', async () => {
  const { app, chart } = await openInsights([LODGE, HOUSE], [year(2025, [['id-1', '50000.00'], ['id-2', '300000.00']])], axis(0, 100000, 200000, 300000, 400000));
  const lines = gridlines(chart);
  assert.equal(lines.at(-1).zero, true, 'the zero line is the lowest line');
  assert.equal(lines.length, 5, 'nothing below zero');
  assert.ok(Number.parseFloat(lines[0].label) * 1000 >= 350000, `the top of the axis (${lines[0].label}) reaches 350000.00`);
  const top = Math.min(...[...chart.querySelectorAll('rect[data-legend]')].map((bar) => num(bar, 'y')));
  assert.ok(top >= lines[0].y - 1, 'the bar fits under the top of the axis');
  app.close();
});

test('test_csd016_ac_6_3_a_house_at_zero_adds_no_height_but_stays_named', async () => {
  const { app, chart, groups, hover, tooltip, legend } = await openInsights(
    [HOUSE, house('id-3', 'Example Cabin', 2)],
    [year(2026, [['id-2', '300000.00'], ['id-3', '0.00']])],
    axis(0, 300000),
  );
  const bars = [...chart.querySelectorAll('rect[data-legend]')];
  assert.deepEqual(bars.map((bar) => bar.dataset.legend), ['house-id-2'], 'only one piece is drawn');
  const lines = gridlines(chart);
  assert.ok(Math.abs(num(bars[0], 'y') - lines[0].y) <= 1.5, 'the bar reaches 300000.00 and no more');
  assert.ok(legend.querySelector('[data-legend="house-id-3"]'), 'the second house is in the legend');
  await hover(groups()[0]);
  assert.ok(app.text(tooltip()).includes('Example Cabin'), 'and in the tooltip');
  app.close();
});

test('test_csd016_ac_6_4_a_gap_year_is_a_labelled_slot_without_a_bar', async () => {
  const { app, groups, shapes } = await openInsights([HOUSE], [year(2022, [['id-2', '1000.00']]), year(2023, []), year(2024, [['id-2', '1000.00']])]);
  const gap = groups()[1];
  assert.equal(gap.querySelector('.chart-year').textContent, '2023');
  assert.equal(shapes(gap).length, 0);
  assert.equal(gap.querySelector('.chart-bar-box'), null);
  app.close();
});

test('test_csd016_ac_6_5_the_current_year_has_a_lighter_fill_and_ytd_in_its_label', async () => {
  const { app, groups } = await openInsights([HOUSE], [year(2025, [['id-2', '1000.00']]), year(2026, [['id-2', '1000.00']], { partial: true })]);
  const [past, current] = groups();
  assert.equal(current.querySelector('.chart-year').textContent, '2026 YTD');
  assert.equal(past.querySelector('.chart-year').textContent, '2025');
  assert.equal(current.classList.contains('partial'), true);
  assert.equal(past.classList.contains('partial'), false);
  assert.match(charts, /\.partial \[class\*="bar-slot-"\][^{]*\{[^}]*opacity: \.55/);
  app.close();
});

// ---- REQ-7: marker ----

test('test_csd016_ac_7_2_the_marker_sits_at_the_top_of_the_stack', async () => {
  const { app, chart, groups, shapes } = await openInsights([LODGE, HOUSE], [year(2025, [['id-1', '2000.00'], ['id-2', '8000.00']])], axis(0, 5000, 10000));
  const topOfStack = Math.min(...shapes(groups()[0]).map((bar) => num(bar, 'y')));
  const marker = chart.querySelector('.chart-marker');
  assert.ok(Math.abs(num(marker, 'y1') - topOfStack) <= 1.5, `marker ${num(marker, 'y1')} at the top of the stack ${topOfStack}`);
  assert.equal(chart.querySelector('.chart-marker-label').textContent, '10.00k');
  app.close();
});

test('test_csd016_ac_7_3_markers_are_not_joined_and_the_current_year_is_dashed', async () => {
  const { app, chart, groups } = await openInsights(
    [HOUSE],
    [year(2024, [['id-2', '1000.00']]), year(2025, []), year(2026, [['id-2', '1000.00']], { partial: true })],
    axis(0, 1000),
  );
  const [, gap, current] = groups();
  assert.equal(chart.querySelectorAll('.chart-marker').length, 2, 'one line per marker, none between them');
  assert.equal(gap.querySelector('.chart-marker'), null, '2025 has no marker');
  assert.ok(current.classList.contains('partial') && current.querySelector('.chart-marker'));
  assert.match(charts, /\.partial \.chart-marker\s*\{[^}]*stroke-dasharray/);
  app.close();
});

// ---- REQ-8: colours ----

test('test_csd016_ac_8_1_a_house_has_one_colour_everywhere', async () => {
  const { app, chart, groups, hover, tooltip, legend } = await openInsights(
    [LODGE, HOUSE],
    [year(2025, [['id-1', '50000.00'], ['id-2', '300000.00']]), year(2026, [['id-1', '55000.00'], ['id-2', '310000.00']])],
  );
  const lodge = pieces(chart, 'house-id-1');
  assert.equal(lodge.length, 2, 'its segments in 2025 and 2026');
  assert.ok(lodge.every((bar) => bar.classList.contains('bar-slot-1')));
  assert.ok(legend.querySelector('[data-legend="house-id-1"] .swatch.slot-1'), 'the legend swatch');
  await hover(groups()[0]);
  assert.ok(tooltip().querySelector('dt[data-legend="house-id-1"] .swatch.slot-1'), 'the tooltip swatch');
  assert.match(charts, /\.swatch\.slot-1, \.bar-slot-1 \{[^}]*var\(--bank-1\)/);
  app.close();
});

test('test_csd016_ac_8_2_colours_survive_a_deletion', async () => {
  const slots = [house('id-1', 'Example A', 0), house('id-2', 'Example B', 1), house('id-3', 'Example C', 2)];
  const rowsOf = (ids) => [year(2026, ids.map((id) => [id, '1000.00']))];
  const { app, chart, legend, reopen } = await openInsights(slots, rowsOf(['id-1', 'id-2', 'id-3']));
  // The house with slot 0 is deleted on the Overview tab: Rust answers without it.
  await reopen({ axis: axis(0, 3000), houses: slots.slice(1), years: rowsOf(['id-2', 'id-3']) });
  assert.ok(pieces(chart, 'house-id-2').every((bar) => bar.classList.contains('bar-slot-1')));
  assert.ok(pieces(chart, 'house-id-3').every((bar) => bar.classList.contains('bar-slot-2')));
  assert.ok(legend.querySelector('[data-legend="house-id-2"] .swatch.slot-1'));
  assert.ok(legend.querySelector('[data-legend="house-id-3"] .swatch.slot-2'));
  app.close();
});

/** The `hsl(h, s%, l%)` tokens `--bank-0` to `--bank-7` of the block that starts at `selector`, as strings and saturations. */
function palette(selector) {
  const start = base.indexOf(selector);
  assert.ok(start >= 0, selector);
  const block = base.slice(base.indexOf('{', start), base.indexOf('}', start));
  return [...block.matchAll(/--bank-\d:\s*(hsl\(([\d.]+),\s*([\d.]+)%,\s*([\d.]+)%\))/g)].map((match) => ({ colour: match[1], hue: Number(match[2]), saturation: Number(match[3]) }));
}

test('test_csd016_ac_8_3_the_palette_is_distinct_and_muted_in_both_modes', async () => {
  for (const selector of [':root {', ':root[data-theme="dark"]']) {
    const colours = palette(selector);
    assert.equal(colours.length, 8, selector);
    assert.equal(new Set(colours.map((entry) => entry.colour)).size, 8, `${selector}: the eight colours are distinct`);
    assert.ok(colours.every((entry) => entry.saturation <= 60), `${selector}: at most 60% saturation`);
  }
});

test('test_csd016_ac_8_4_more_than_eight_houses_share_colours_and_stay_named', async () => {
  const nine = Array.from({ length: 9 }, (_, index) => house(`id-${index + 1}`, `Example ${index + 1}`, index % 8));
  const { app, chart, groups, hover, tooltip, legend } = await openInsights(nine, [year(2026, nine.map((entry) => [entry.id, '1000.00']))], axis(0, 9000));
  const slotOf = (id) => [...pieces(chart, `house-${id}`)[0].classList].find((name) => name.startsWith('bar-slot-'));
  assert.equal(slotOf('id-1'), slotOf('id-9'));
  assert.notEqual(slotOf('id-1'), slotOf('id-2'));
  assert.ok(app.text(legend).includes('Example 9'));
  await hover(groups()[0]);
  assert.ok(app.text(tooltip()).includes('Example 9') && app.text(tooltip()).includes('Example 1'));
  app.close();
});

// ---- REQ-9: legend ----

test('test_csd016_ac_9_1_the_legend_lists_the_houses_in_creation_order_and_the_marker', async () => {
  const { app, legend } = await openInsights([LODGE, HOUSE], [year(2026, [['id-1', '50000.00'], ['id-2', '300000.00']])]);
  const items = [...legend.querySelectorAll('li')];
  assert.deepEqual(items.map((item) => app.text(item)), ['Example Lodge', 'Example House', 'Total value']);
  assert.ok(items.every((item) => item.querySelector('.swatch')), 'each with a swatch');
  assert.equal(items.at(-1).dataset.legend, 'marker');
  app.close();
});

test('test_csd016_ac_9_3_many_houses_are_all_in_a_legend_that_wraps', async () => {
  const twelve = Array.from({ length: 12 }, (_, index) => house(`id-${index + 1}`, `Example ${index + 1}`, index % 8));
  const { app, legend } = await openInsights(twelve, [year(2026, twelve.map((entry) => [entry.id, '1000.00']))], axis(0, 12000));
  assert.equal(legend.querySelectorAll('li').length, 13, 'twelve houses and the marker, none left out');
  assert.match(charts, /\.legend\s*\{[^}]*flex-wrap:\s*wrap/);
  app.close();
});

test('test_csd016_ac_9_4_a_long_name_is_cut_in_the_legend_and_kept_in_full', async () => {
  const name = `Example ${'x'.repeat(92)}`;
  assert.equal(name.length, 100);
  const { app, groups, hover, tooltip, legend } = await openInsights([house('id-1', name, 0)], [year(2026, [['id-1', '1000.00']])], axis(0, 1000));
  const label = legend.querySelector('[data-legend="house-id-1"] .legend-name');
  assert.equal(label.textContent, name, 'the full name is in the text, so a screen reader reads it');
  assert.equal(label.dataset.text, name);
  assert.ok(label.querySelector('.legend-name-text'));
  assert.match(charts, /\.legend-name-text\s*\{[^}]*text-overflow:\s*ellipsis/);
  await hover(groups()[0]);
  assert.ok(app.text(tooltip()).includes(name), 'the tooltip names it in full');
  app.close();
});

test('test_csd016_ac_9_5_a_deleted_house_leaves_the_legend', async () => {
  const { app, legend, reopen } = await openInsights([LODGE, HOUSE], [year(2026, [['id-1', '50000.00'], ['id-2', '300000.00']])]);
  assert.equal(legend.querySelectorAll('li').length, 3);
  await reopen({ axis: axis(0, 400000), houses: [HOUSE], years: [year(2026, [['id-2', '300000.00']])] });
  assert.deepEqual([...legend.querySelectorAll('li')].map((item) => app.text(item)), ['Example House', 'Total value']);
  app.close();
});

// ---- REQ-10: tooltip and accessible label ----

const TWO_HOUSES = () => [year(2026, [['id-2', '300000.00'], ['id-1', '50000.00']], { change: '10000.00' })];

test('test_csd016_ac_10_1_the_tooltip_shows_the_houses_the_total_and_the_change_and_no_count', async () => {
  const { app, groups, hover, tooltip, rows } = await openInsights([HOUSE, LODGE], TWO_HOUSES());
  await hover(groups()[0]);
  assert.equal(tooltip().classList.contains('is-open'), true);
  assert.equal(app.text(tooltip().querySelector('.tip-title')), '2026');
  assert.ok(tooltip().querySelector('dt[data-legend="house-id-2"] .swatch.slot-0') && tooltip().querySelector('dt[data-legend="house-id-1"] .swatch.slot-1'), 'swatches');
  assert.deepEqual(rows().map(([, value]) => value), ['50,000.00', '300,000.00', '350,000.00', '10,000.00']);
  assert.doesNotMatch(app.text(tooltip()), /\bvaluations?\b/i, 'no number of valuations');
  app.close();
});

test('test_csd016_ac_10_2_the_tooltip_follows_keyboard_focus_and_goes_with_it', async () => {
  const { app, groups, tooltip, rows } = await openInsights([HOUSE, LODGE], TWO_HOUSES());
  const [group] = groups();
  assert.equal(group.getAttribute('tabindex'), '0', 'the bar can take the focus');
  group.dispatchEvent(new app.window.Event('focus'));
  assert.equal(tooltip().classList.contains('is-open'), true);
  assert.equal(rows().length, 4);
  group.dispatchEvent(new app.window.Event('blur'));
  await new Promise((resolve) => setTimeout(resolve, 150));
  assert.equal(tooltip().classList.contains('is-open'), false);
  app.close();
});

test('test_csd016_ac_10_3_the_accessible_label_has_the_values_and_says_negative', async () => {
  const { app, groups } = await openInsights([HOUSE, LODGE], [year(2026, [['id-2', '300000.00'], ['id-1', '50000.00']], { change: '-200.00' })]);
  const spoken = groups()[0].getAttribute('aria-label');
  assert.ok(spoken.startsWith('2026'));
  for (const part of ['Example House 300,000.00', 'Example Lodge 50,000.00', 'total value 350,000.00', 'change -200.00 (negative)']) assert.ok(spoken.includes(part), `${part} in "${spoken}"`);
  assert.doesNotMatch(spoken, /\bvaluations?\b/i, 'no number of valuations');
  app.close();
});

test('test_csd016_ac_10_4_the_tooltip_says_a_partial_year_is_partial', async () => {
  const { app, groups, hover, tooltip } = await openInsights([HOUSE], [year(2026, [['id-2', '1000.00']], { partial: true })], axis(0, 1000));
  await hover(groups()[0]);
  assert.equal(app.text(tooltip().querySelector('.tip-note')), 'Partial year: valuations so far');
  app.close();
});

test('test_csd016_ac_10_5_a_change_of_none_is_a_dash', async () => {
  const { app, groups, hover, rows } = await openInsights([HOUSE], [year(2025, [['id-2', '1000.00']], { change: null })], axis(0, 1000));
  await hover(groups()[0]);
  assert.deepEqual(rows().at(-1), ['Change', '–']);
  app.close();
});

test('test_csd016_ac_10_7_a_year_without_valuations_shows_the_note_and_no_rows', async () => {
  const { app, groups, hover, tooltip } = await openInsights([HOUSE], [year(2022, [['id-2', '1000.00']]), year(2023, []), year(2024, [['id-2', '1000.00']])], axis(0, 1000));
  await hover(groups()[1]);
  assert.equal(app.text(tooltip().querySelector('.tip-title')), '2023');
  assert.match(app.text(tooltip().querySelector('.tip-note')), /^No valuations$/);
  assert.equal(tooltip().querySelectorAll('dt').length, 0, 'no house rows and no row with a number of valuations');
  app.close();
});

test('test_csd016_ac_10_8_the_rows_follow_the_bar_from_top_to_bottom', async () => {
  const { app, groups, hover, rows } = await openInsights([HOUSE, LODGE], TWO_HOUSES());
  await hover(groups()[0]);
  assert.deepEqual(rows().map(([label]) => label), ['Example Lodge', 'Example House', 'Total value', 'Change']);
  app.close();
});

test('test_csd016_ac_10_9_the_accessible_label_lists_the_houses_in_the_same_order', async () => {
  const { app, groups } = await openInsights([HOUSE, LODGE], TWO_HOUSES());
  const spoken = groups()[0].getAttribute('aria-label');
  assert.ok(spoken.indexOf('Example House') < spoken.indexOf('Example Lodge'), 'in the order of the segments');
  assert.ok(spoken.indexOf('Example Lodge') < spoken.indexOf('total value') && spoken.indexOf('total value') < spoken.indexOf('change 10,000.00'));
  assert.doesNotMatch(spoken, /\bvaluations?\b/i);
  app.close();
});

// ---- REQ-11: highlight ----

const TWO_YEARS = () => [
  year(2025, [['id-1', '50000.00'], ['id-2', '300000.00']]),
  year(2026, [['id-1', '55000.00'], ['id-2', '310000.00']]),
];
const over = (app, node) => node.dispatchEvent(new app.window.MouseEvent('mouseover', { bubbles: true }));

test('test_csd016_ac_11_1_hovering_a_segment_highlights_its_house_in_every_year', async () => {
  const { app, chart, legend } = await openInsights([LODGE, HOUSE], TWO_YEARS());
  over(app, pieces(chart, 'house-id-1')[0]);
  assert.ok(pieces(chart, 'house-id-1').every((bar) => bar.classList.contains('hl') && !bar.classList.contains('dim')), 'every Lodge segment keeps its colour');
  assert.ok(pieces(chart, 'house-id-2').every((bar) => bar.classList.contains('dim')), 'every House segment is eased down');
  assert.ok(legend.querySelector('[data-legend="house-id-1"]').classList.contains('hl'));
  assert.ok(legend.querySelector('[data-legend="house-id-2"]').classList.contains('dim'));
  assert.match(charts, /\.legend li\.hl \.legend-label[^{]*\{[^}]*font-weight: 600/, 'a heavier weight');
  assert.match(charts, /\.chart-year-group \[data-legend\]\.dim \{ opacity: \.7/);
  assert.match(charts, /\.legend li\.dim \{ opacity: \.7/);
  app.close();
});

test('test_csd016_ac_11_2_hovering_a_legend_item_highlights_its_house', async () => {
  const { app, chart, legend } = await openInsights([LODGE, HOUSE], TWO_YEARS());
  legend.querySelector('[data-legend="house-id-1"]').dispatchEvent(new app.window.Event('mouseenter'));
  assert.ok(pieces(chart, 'house-id-1').every((bar) => bar.classList.contains('hl')));
  assert.ok(pieces(chart, 'house-id-2').every((bar) => bar.classList.contains('dim')));
  app.close();
});

test('test_csd016_ac_11_3_focusing_a_legend_item_highlights_like_hovering_and_ends_on_blur', async () => {
  const { app, chart, legend } = await openInsights([LODGE, HOUSE], TWO_YEARS());
  const item = legend.querySelector('[data-legend="house-id-1"]');
  assert.equal(item.getAttribute('tabindex'), '0');
  item.dispatchEvent(new app.window.Event('focus'));
  assert.ok(pieces(chart, 'house-id-1').every((bar) => bar.classList.contains('hl')));
  assert.ok(pieces(chart, 'house-id-2').every((bar) => bar.classList.contains('dim')));
  item.dispatchEvent(new app.window.Event('blur'));
  assert.equal(chart.querySelectorAll('.hl, .dim').length + legend.querySelectorAll('.hl, .dim').length, 0);
  app.close();
});

test('test_csd016_ac_11_4_leaving_the_segment_highlights_nothing_and_keeps_the_tooltip', async () => {
  const { app, chart, groups, hover, tooltip, legend } = await openInsights([LODGE, HOUSE], TWO_YEARS());
  const [group] = groups();
  await hover(group);
  over(app, pieces(group, 'house-id-1')[0]);
  assert.ok(chart.querySelectorAll('.hl').length > 0);
  over(app, group.querySelector('.chart-hit'));
  assert.equal(chart.querySelectorAll('.hl, .dim').length + legend.querySelectorAll('.hl, .dim').length, 0);
  assert.equal(tooltip().classList.contains('is-open'), true);
  app.close();
});

test('test_csd016_ac_11_5_the_highlight_is_not_colour_alone', async () => {
  const { app, chart, legend } = await openInsights([LODGE, HOUSE], TWO_YEARS());
  over(app, pieces(chart, 'house-id-1')[0]);
  // The dimmed segments keep their own colour class (they are lighter, not recoloured) ...
  assert.ok(pieces(chart, 'house-id-2').every((bar) => bar.classList.contains('bar-slot-0') && bar.classList.contains('dim')));
  // ... and the highlighted legend item is set heavier than the others by the style sheet.
  assert.ok(legend.querySelector('[data-legend="house-id-1"]').classList.contains('hl'));
  assert.match(charts, /\.legend li\.hl \.legend-label[^{]*\{[^}]*font-weight: 600/);
  app.close();
});

test('test_csd016_ac_11_6_keyboard_focus_on_a_year_highlights_nothing', async () => {
  const { app, chart, groups, legend } = await openInsights([LODGE, HOUSE], TWO_YEARS());
  groups()[0].dispatchEvent(new app.window.Event('focus'));
  assert.equal(chart.querySelectorAll('.hl, .dim').length + legend.querySelectorAll('.hl, .dim').length, 0);
  app.close();
});

// ---- REQ-12 and REQ-13: motion, accessible segments ----

test('test_csd016_ac_12_1_the_tooltip_moves_from_year_to_year_with_the_shared_motion', async () => {
  const { app, groups, hover, tooltip } = await openInsights([LODGE, HOUSE], TWO_YEARS());
  await hover(groups()[0]);
  assert.equal(app.text(tooltip().querySelector('.tip-title')), '2025');
  await hover(groups()[1]);
  assert.equal(app.text(tooltip().querySelector('.tip-title')), '2026', 'the tooltip moved to the other year');
  assert.equal(tooltip().classList.contains('is-open'), true);
  assert.match(charts, /\.chart-tooltip\s*\{[^}]*transition:/, 'it animates');
  assert.match(charts, /prefers-reduced-motion: reduce\)\s*\{[^}]*\.chart-tooltip[^}]*transition: none/, 'and not for a user who prefers reduced motion');
  app.close();
});

test('test_csd016_ac_13_4_a_segment_announces_the_house_the_year_and_the_value', async () => {
  const { app, groups, shapes } = await openInsights([LODGE], [year(2026, [['id-1', '50000.00']])], axis(0, 50000));
  assert.equal(shapes(groups()[0])[0].getAttribute('aria-label'), 'Example Lodge, 2026, 50,000.00');
  app.close();
});

// ---- REQ-15: current figures ----

test('test_csd016_ac_15_1_the_figures_follow_changes_when_the_tab_opens_again', async () => {
  const { app, chart, reopen } = await openInsights([HOUSE], [year(2026, [['id-2', '300000.00']])], axis(0, 400000));
  assert.equal(chart.querySelector('.chart-marker-label').textContent, '300.00k');
  await reopen({ axis: axis(0, 400000), houses: [HOUSE], years: [year(2026, [['id-2', '305000.00']])] });
  assert.equal(chart.querySelector('.chart-marker-label').textContent, '305.00k');
  assert.equal(app.callsOf('get_house_insights').length >= 2, true);
  app.close();
});

test('test_csd016_ac_15_2_the_last_valuation_is_deleted_and_the_note_shows', async () => {
  const { app, chart, reopen } = await openInsights([HOUSE], [year(2026, [['id-2', '300000.00']])], axis(0, 400000));
  assert.equal(app.isShown(app.$('houses-insights-empty')), false);
  await reopen({ axis: { ticks: [] }, houses: [], years: [] });
  assert.equal(app.isShown(app.$('houses-insights-empty')), true);
  assert.equal(app.textOf('houses-insights-empty'), 'No valuations yet. The chart appears once there is a valuation.');
  assert.equal(chart.querySelectorAll('svg').length, 0, 'no chart is drawn');
  app.close();
});
