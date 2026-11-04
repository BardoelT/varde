// Acceptance tests (batch D) for CSD-025 that charts.test.js did not name: the placement of the
// tooltip (AC-8.2, AC-8.3), motion (AC-10.1), the height of the plot (AC-11.1), the look of bars,
// lines and tooltip (AC-12.1 to AC-12.3), the legend (AC-13.1) and the palette (AC-14.2).
//
// jsdom has no layout. What depends on measured boxes is tested with boxes the test puts on the
// elements (the placement rule is the app's own code), and the look is tested on the style sheets
// the browser applies (`charts.css`, `base.css`): the rules and the numbers in them. That the
// browser then paints those rules, and that the tooltip really moves, stays on the manual list.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { openApp } from './harness.js';
import { exampleReads } from './fixtures.js';

const read = (file) => readFileSync(fileURLToPath(new URL(`../styles/${file}`, import.meta.url)), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
const chartsCss = read('charts.css');
const baseCss = read('base.css');

// ---- helpers (the series are as Rust delivers them) ----

const bankList = (...names) => names.map((name, index) => ({ id: `id-${index + 1}`, name, colour_slot: index % 8 }));
const piece = ([series, low, high, hangs = false, rounding = 'none']) => ({ series, low, high, hangs, rounding });
const bankYear = (year, segments, pieces) => ({
  year,
  is_partial: false,
  entry_count: segments.length,
  closing_balance: '0.00',
  segments: segments.map(([id, balance]) => ({ account_id: id, balance })),
  pieces: pieces.map(piece),
  marker: null,
});
const ticks = (...values) => ({ ticks: values.map((value) => ({ value, label: value })) });

async function openBank(series) {
  const reads = exampleReads();
  reads.get_bank_insights = series;
  const app = await openApp({ reads });
  await app.click(app.item('bank'));
  await app.click(app.$('bank-tab-insights'));
  return { app, chartNode: app.$('bank-insights-chart'), legendNode: app.$('bank-insights-legend') };
}

const groups = (chartNode) => [...chartNode.querySelectorAll('.chart-year-group')];
const num = (node, name) => Number(node.getAttribute(name));
const tooltipOf = (chartNode) => chartNode.querySelector('.chart-tooltip');

/** The declarations of the first rule of `css` whose selector list contains `selector`, as a map. */
function rule(css, selector) {
  for (const [, selectors, body] of css.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    if (selectors.split(',').some((part) => part.trim() === selector)) {
      return Object.fromEntries(
        body.split(';').map((declaration) => declaration.split(/:(.*)/s)).filter(([name]) => name?.trim()).map(([name, value]) => [name.trim(), value.trim()]),
      );
    }
  }
  assert.fail(`no rule for ${selector}`);
}

const box = (left, top, width, height) => ({ left, top, width, height, right: left + width, bottom: top + height, x: left, y: top });
const stubBox = (element, rect) => {
  element.getBoundingClientRect = () => rect;
};
const stubSize = (element, width, height) => {
  Object.defineProperty(element, 'offsetWidth', { configurable: true, get: () => width });
  Object.defineProperty(element, 'offsetHeight', { configurable: true, get: () => height });
};
async function hover(app, group) {
  group.dispatchEvent(new app.window.MouseEvent('mouseenter'));
  await app.settle();
}

const threeYears = () => ({
  axis: ticks('0.00', '1500.00'),
  banks: bankList('Example Bank'),
  years: [2024, 2025, 2026].map((year) => bankYear(year, [['id-1', '1000.00']], [['bank-id-1', '0.00', '1000.00', false, 'top']])),
});

// ---- REQ-8: placement of the tooltip ----

test('test_csd025_ac_8_2_the_tooltip_sits_right_of_the_bar_or_left_when_there_is_no_room', async () => {
  const { app, chartNode } = await openBank(threeYears());
  const tooltip = tooltipOf(chartNode);
  stubBox(chartNode, box(0, 0, 800, 400));
  stubSize(tooltip, 200, 100);
  const [first, , last] = groups(chartNode);
  stubBox(first.querySelector('.chart-bar-box'), box(100, 50, 40, 250));
  stubBox(last.querySelector('.chart-bar-box'), box(700, 80, 40, 220));

  await hover(app, first);
  assert.equal(tooltip.style.left, '148px', 'right of the bar, 8 px from it');
  assert.equal(tooltip.style.top, '50px', 'its top level with the top of the bar');

  await hover(app, last);
  assert.equal(tooltip.style.left, '492px', 'no room on the right: left of the bar (700 - 8 - 200)');
  assert.equal(tooltip.style.top, '80px');
  assert.ok(Number.parseFloat(tooltip.style.left) >= 0 && Number.parseFloat(tooltip.style.left) + 200 <= 800, 'inside the chart');
  app.close();
});

test('test_csd025_ac_8_3_a_year_without_a_bar_gets_the_tooltip_centred_over_its_slot', async () => {
  const series = threeYears();
  series.years[0] = bankYear(2023, [], []);
  const { app, chartNode } = await openBank(series);
  const tooltip = tooltipOf(chartNode);
  stubBox(chartNode, box(0, 0, 800, 400));
  stubSize(tooltip, 200, 100);
  const gap = groups(chartNode)[0];
  assert.equal(gap.querySelector('.chart-bar-box'), null, 'the year has no bar');
  stubBox(gap.querySelector('.chart-hit'), box(200, 28, 100, 300));

  await hover(app, gap);
  assert.equal(tooltip.style.left, '150px', 'centre of the slot (250) minus half the tooltip (100)');
  assert.equal(tooltip.style.top, '0px');
  app.close();
});

// ---- REQ-10: motion ----

test('test_csd025_ac_10_1_the_tooltip_fades_in_within_150_ms_and_glides_to_the_next_bar', async () => {
  const open = rule(chartsCss, '.chart-tooltip.is-open').transition;
  const closed = rule(chartsCss, '.chart-tooltip').transition;
  for (const transition of [open, closed]) {
    const durations = Object.fromEntries([...transition.matchAll(/(opacity|left|top)\s+(\d+)ms/g)].map(([, name, ms]) => [name, Number(ms)]));
    assert.ok(durations.opacity >= 120 && durations.opacity <= 150, `fade: ${durations.opacity} ms`);
    assert.ok(durations.left > 0 && durations.top > 0, 'left and top are animated, so the tooltip glides');
  }

  const { app, chartNode } = await openBank(threeYears());
  const tooltip = tooltipOf(chartNode);
  const [first, second] = groups(chartNode);
  await hover(app, first);
  assert.equal(tooltip.classList.contains('is-open'), true);
  assert.equal(tooltip.classList.contains('no-glide'), false, 'it fades in, it does not jump');
  await hover(app, second);
  assert.equal(tooltipOf(chartNode), tooltip, 'the same tooltip moves on to the next bar');
  assert.equal(tooltip.classList.contains('no-glide'), false, 'moving along the bars glides');
  app.close();
});

// ---- REQ-11: the plot follows the window ----

test('test_csd025_ac_11_1_the_plot_follows_the_height_of_the_window_and_stays_at_260_px', async () => {
  const { app, chartNode } = await openBank(threeYears());
  const scroll = app.$('bank-insights-scroll');
  Object.defineProperty(scroll, 'offsetParent', { configurable: true, get: () => app.document.body });
  /** The plot height: the bottom of the plot is 18 px above the first year label, its top is the 28 px margin. */
  const plotHeight = () => num(chartNode.querySelector('.chart-year'), 'y') - 18 - 28;
  const resizeTo = (height) => {
    app.window.innerHeight = height;
    app.window.dispatchEvent(new app.window.Event('resize'));
  };

  resizeTo(900);
  const base = plotHeight();
  assert.ok(base > 260, `a high window gives a plot above the minimum (${base})`);
  resizeTo(1100);
  assert.equal(plotHeight() - base, 200, 'a window 200 px higher gives a plot 200 px higher');
  resizeTo(900);
  assert.equal(plotHeight(), base, 'and back');

  resizeTo(300);
  assert.equal(plotHeight(), 260, 'a low window keeps the plot at 260 px');
  resizeTo(200);
  assert.equal(plotHeight(), 260);
  app.close();
});

// ---- REQ-12: the look ----

test('test_csd025_ac_12_1_bars_are_at_most_44_px_with_3_px_rounding_and_a_1_px_gap', async () => {
  const series = {
    axis: ticks('-1000.00', '0.00', '10500.00'),
    banks: bankList('A', 'B', 'C'),
    years: [2024, 2025, 2026].map((year) =>
      bankYear(year, [['id-1', '2500.00'], ['id-2', '8000.00'], ['id-3', '-1000.00']], [
        ['bank-id-1', '0.00', '2500.00'],
        ['bank-id-2', '2500.00', '10500.00', false, 'top'],
        ['bank-id-3', '-1000.00', '0.00', true, 'bottom'],
      ]),
    ),
  };
  const { app, chartNode } = await openBank(series);
  const [year] = groups(chartNode);
  const centres = [...chartNode.querySelectorAll('.chart-year')].map((node) => num(node, 'x'));
  const slot = centres[1] - centres[0];
  const shape = (id) => year.querySelector(`[data-legend="bank-${id}"]`);
  const [lower, upper, hanging] = ['id-1', 'id-2', 'id-3'].map(shape);

  // Width: at most 50% of the slot or 44 px.
  const barWidth = num(lower, 'width');
  assert.ok(barWidth <= 44 && barWidth <= slot * 0.5 + 0.001, `bar ${barWidth} px in a slot of ${slot} px`);

  // Rounding: the top piece has its top corners rounded by 3 px, the hanging one its bottom corners.
  assert.equal(upper.tagName, 'path');
  const corner = upper.getAttribute('d').match(/^M([\d.-]+),([\d.-]+) V([\d.-]+) Q([\d.-]+),([\d.-]+) ([\d.-]+),([\d.-]+) H/);
  assert.equal(Number(corner[3]) - Number(corner[5]), 3, 'top corners: 3 px high');
  assert.equal(Number(corner[6]) - Number(corner[4]), 3, 'top corners: 3 px wide');
  assert.equal(lower.tagName, 'rect', 'a piece with rounding none is square');
  assert.equal(hanging.tagName, 'path');
  const low = hanging.getAttribute('d').match(/^M([\d.-]+),([\d.-]+) H([\d.-]+) V([\d.-]+) Q([\d.-]+),([\d.-]+) ([\d.-]+),([\d.-]+) H/);
  assert.equal(Number(low[6]) - Number(low[4]), 3, 'bottom corners: 3 px high');
  assert.equal(Number(low[5]) - Number(low[7]), 3, 'bottom corners: 3 px wide');

  // Gap: stacked pieces are separated by 1 px and have no outline.
  const lowerTop = num(lower, 'y');
  const upperBottom = Number(upper.getAttribute('d').match(/^M[\d.-]+,([\d.-]+)/)[1]);
  assert.ok(Math.abs(lowerTop - upperBottom - 1) < 0.001, `gap ${lowerTop - upperBottom}`);
  for (const node of [lower, upper, hanging]) assert.equal(node.getAttribute('stroke'), null, 'no outline');
  for (const [, selectors, body] of chartsCss.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    if (/\.bar-/.test(selectors) && !/withdrawal/.test(selectors)) assert.ok(!/stroke/.test(body), `no outline in ${selectors.trim()}`);
  }
  app.close();
});

test('test_csd025_ac_12_2_gridlines_zero_line_and_labels', async () => {
  const series = {
    axis: ticks('-1000.00', '0.00', '1000.00'),
    banks: bankList('A'),
    years: [bankYear(2026, [['id-1', '-500.00']], [['bank-id-1', '-500.00', '0.00', true, 'bottom']])],
  };
  const { app, chartNode } = await openBank(series);
  assert.equal(chartNode.querySelectorAll('.chart-zero').length, 1, 'a zero line is drawn');
  assert.equal(chartNode.querySelectorAll('.chart-grid').length, 2, 'and the gridlines');

  const grid = rule(chartsCss, '.chart-grid');
  assert.deepEqual([grid.stroke, grid['stroke-opacity'], grid['stroke-width']], ['var(--border)', '.55', '1'], '1 px, the border colour at 55%');
  const zero = rule(chartsCss, '.chart-zero');
  assert.deepEqual([zero.stroke, zero['stroke-opacity'], zero['stroke-width']], ['var(--muted)', '.7', '1'], '1 px, the muted colour at 70%');
  const label = rule(chartsCss, '.chart-axis-label');
  const year = rule(chartsCss, '.chart-year');
  for (const text of [label, year]) assert.deepEqual([text.fill, text['font-weight']], ['var(--muted)', '400'], 'muted, regular weight');
  app.close();
});

test('test_csd025_ac_12_3_the_tooltip_has_a_rounded_padded_soft_look', async () => {
  const { app, chartNode } = await openBank(threeYears());
  await hover(app, groups(chartNode)[0]);
  const tooltip = tooltipOf(chartNode);
  assert.ok(tooltip.querySelector('.tip-title'), 'the tooltip has a title');
  assert.ok(tooltip.querySelector('dt'), 'and row labels');

  const style = rule(chartsCss, '.chart-tooltip');
  assert.equal(style['border-radius'], '10px');
  assert.equal(style.padding, '12px 14px');
  const alphas = [...style['box-shadow'].matchAll(/rgba\([^)]*,\s*([\d.]+)\)/g)].map(([, alpha]) => Number(alpha));
  assert.ok(alphas.length > 0 && alphas.every((alpha) => alpha <= 0.15), `a soft shadow: ${style['box-shadow']}`);
  assert.equal(rule(chartsCss, '.chart-tooltip .tip-title')['font-weight'], '600', 'the title is semibold');
  assert.equal(rule(chartsCss, '.chart-tooltip dt').color, 'var(--muted)', 'row labels are muted');
  app.close();
});

// ---- REQ-13: many series ----

test('test_csd025_ac_13_1_twelve_series_are_all_in_the_legend_and_it_wraps', async () => {
  const banks = bankList(...Array.from({ length: 12 }, (_, index) => `Example Bank ${index + 1}`));
  const year = bankYear(2026, banks.map((bank) => [bank.id, '100.00']), banks.map((bank, index) => [`bank-${bank.id}`, `${index * 100}.00`, `${index * 100 + 100}.00`]));
  const { app, legendNode } = await openBank({ axis: ticks('0.00', '1200.00'), banks, years: [year] });
  const names = [...legendNode.querySelectorAll('li')].map((item) => app.text(item));
  for (let index = 1; index <= 12; index += 1) assert.ok(names.includes(`Example Bank ${index}`), `Example Bank ${index} is in the legend`);
  const legend = rule(chartsCss, '.legend');
  assert.equal(legend.display, 'flex');
  assert.equal(legend['flex-wrap'], 'wrap', 'a narrow window wraps the names onto more lines');
  app.close();
});

// ---- REQ-14: the palette in both modes ----

test('test_csd025_ac_14_2_the_eight_colours_are_distinct_and_calm_in_light_and_dark', () => {
  const sets = {
    light: baseCss.slice(baseCss.indexOf(':root {')),
    'dark (system)': baseCss.slice(baseCss.indexOf('@media (prefers-color-scheme: dark)')),
    'dark (chosen)': baseCss.slice(baseCss.indexOf(':root[data-theme="dark"]')),
  };
  for (const [mode, text] of Object.entries(sets)) {
    const colours = [0, 1, 2, 3, 4, 5, 6, 7].map((slot) => {
      const match = text.match(new RegExp(`--bank-${slot}:\\s*hsl\\((\\d+),\\s*(\\d+)%,\\s*(\\d+)%\\)`));
      assert.ok(match, `--bank-${slot} in ${mode}`);
      return { slot, hue: Number(match[1]), saturation: Number(match[2]), lightness: Number(match[3]) };
    });
    const distinct = new Set(colours.map(({ hue, saturation, lightness }) => `${hue}/${saturation}/${lightness}`));
    assert.equal(distinct.size, 8, `eight distinct colours in ${mode}`);
    for (const colour of colours) assert.ok(colour.saturation <= 60, `--bank-${colour.slot} in ${mode}: saturation ${colour.saturation}% is over 60%`);
  }
});
