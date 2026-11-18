// CSD-024 REQ-10 to REQ-16: the page header stays at the top and compresses while the page is scrolled.
// jsdom has no layout, so the scroll position and the heights are what the test puts on the content area;
// the class toggle and the chart code are the app's own. The look, the sticky position, the focus jump and
// the layers are proven on the rules of the style sheets and on the manual list (CSD-024 section 7).
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { openApp } from './harness.js';

const css = (file) => readFileSync(fileURLToPath(new URL(`../styles/${file}`, import.meta.url)), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
const layoutCss = css('layout.css');
const chartsCss = css('charts.css');
const componentsCss = css('components.css');

/** The declarations of the rule of `css` whose selector list has `selector`, as a map. */
function rule(source, selector) {
  for (const [, selectors, body] of source.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    if (selectors.split(',').some((part) => part.trim() === selector)) {
      return Object.fromEntries(body.split(';').map((d) => d.split(/:(.*)/s)).filter(([name]) => name?.trim()).map(([n, v]) => [n.trim(), v.trim()]));
    }
  }
  assert.fail(`no rule for ${selector}`);
}

/** Puts a scroll position and a height on the content area, as a browser with layout would give it. */
function layout(app, { scrollHeight, clientHeight = 600 }) {
  const scroller = app.document.querySelector('.content');
  let top = 0;
  Object.defineProperty(scroller, 'scrollTop', { configurable: true, get: () => top, set: (value) => { top = value; } });
  Object.defineProperty(scroller, 'scrollHeight', { configurable: true, value: scrollHeight });
  Object.defineProperty(scroller, 'clientHeight', { configurable: true, value: clientHeight });
  scroller.style.setProperty('--header-expanded', '77px');
  scroller.style.setProperty('--header-compressed', '40px');
  const scrollTo = (value) => {
    top = value;
    scroller.dispatchEvent(new app.window.Event('scroll'));
  };
  return { scroller, scrollTo };
}

const compressed = (scroller) => scroller.classList.contains('header-compressed');

test('test_csd024_ac_11_1_and_11_2_compressed_after_32_px', async () => {
  const app = await openApp();
  const { scroller, scrollTo } = layout(app, { scrollHeight: 1400 });
  assert.equal(compressed(scroller), false, 'a page opens expanded');
  scrollTo(32);
  assert.equal(compressed(scroller), false, 'not yet at 32 px');
  scrollTo(40);
  assert.equal(compressed(scroller), true, 'compressed at 40 px');
  app.close();
});

test('test_csd024_ac_12_1_and_12_2_expands_only_at_the_top', async () => {
  const app = await openApp();
  const { scroller, scrollTo } = layout(app, { scrollHeight: 1400 });
  scrollTo(100);
  scrollTo(20);
  assert.equal(compressed(scroller), true, 'between 0 and 32 px the state is kept');
  scrollTo(0);
  assert.equal(compressed(scroller), false, 'back at the top it expands');
  scrollTo(20);
  assert.equal(compressed(scroller), false, 'between 0 and 32 px it also stays expanded');
  app.close();
});

test('test_csd024_ac_12_3_a_short_page_never_compresses', async () => {
  const app = await openApp();
  const { scroller, scrollTo } = layout(app, { scrollHeight: 640 });
  scrollTo(40);
  assert.equal(compressed(scroller), false, 'it overflows by 40 px, less than the shrink plus 32 px');
  const long = layout(app, { scrollHeight: 669 });
  long.scrollTo(40);
  assert.equal(compressed(long.scroller), true, 'it overflows by 69 px, the shrink plus 32 px');
  app.close();
});

test('test_csd024_ac_12_4_another_page_starts_expanded_at_the_top', async () => {
  const app = await openApp();
  await app.click(app.item('income'));
  const { scroller, scrollTo } = layout(app, { scrollHeight: 1400 });
  scrollTo(400);
  assert.equal(compressed(scroller), true);
  await app.click(app.item('debts'));
  assert.equal(compressed(scroller), false);
  assert.equal(scroller.scrollTop, 0);
  app.close();
});

test('test_csd024_ac_10_4_every_page_has_its_title_and_its_tabs_in_the_pinned_header', async () => {
  const app = await openApp();
  const tabbed = ['income', 'expenses', 'transactions', 'bank', 'investments', 'houses', 'debts'];
  const plain = ['dashboard', 'net-worth', 'ratios', 'settings'];
  for (const id of [...tabbed, ...plain]) {
    const section = app.$(id);
    assert.ok(section.querySelector(':scope > .section-header > .section-title'), `${id} has a title in the header`);
    assert.equal(section.querySelector('.tabs') !== null, tabbed.includes(id), `${id}: tabs`);
  }
  app.close();
});

test('test_csd024_ac_10_1_to_10_3_the_header_and_the_tabs_are_sticky', () => {
  assert.equal(rule(layoutCss, '.section-header').position, 'sticky');
  assert.equal(rule(layoutCss, '.section-header').top, '0');
  assert.match(rule(layoutCss, '.section-header').background, /var\(--bg\)/, 'content does not show through');
  const tabs = rule(chartsCss, '.tabs');
  assert.equal(tabs.position, 'sticky');
  assert.equal(tabs.top, 'var(--header-height)', 'the tabs sit directly below the title in either state');
  assert.match(tabs.background, /var\(--bg\)/);
});

test('test_csd024_ac_11_3_the_sizes_of_the_two_states', () => {
  const expanded = rule(layoutCss, '.content');
  assert.equal(expanded['--title-size'], '20px');
  assert.equal(expanded['--header-above'], '30px');
  assert.equal(expanded['--header-below'], '22px');
  const small = rule(layoutCss, '.content.header-compressed');
  assert.equal(small['--title-size'], '16px');
  assert.equal(small['--header-above'], '10px');
  assert.equal(small['--header-below'], '10px');
  const header = rule(layoutCss, '.section-header');
  assert.equal(header.padding, 'var(--header-above) 0 var(--header-below)');
  assert.equal(rule(layoutCss, '.section-title')['font-size'], 'var(--title-size)');
  // The two stored heights are the title (1.25 times its size) plus the space around it.
  assert.equal(expanded['--header-expanded'], `${30 + 22 + 20 * 1.25}px`);
  assert.equal(expanded['--header-compressed'], `${10 + 10 + 16 * 1.25}px`);
});

test('test_csd024_ac_14_1_focus_stays_clear_of_the_header', () => {
  const clearance = Number.parseInt(rule(layoutCss, '.content')['scroll-padding-top'], 10);
  assert.ok(clearance >= 40 + 38, 'at least the compressed title plus the tabs');
});

test('test_csd024_ac_15_1_to_15_3_the_layers', () => {
  const z = (source, selector) => Number(rule(source, selector)['z-index']);
  const header = z(layoutCss, '.section-header');
  assert.ok(header > z(componentsCss, '.ratios-table th:first-child'), 'above the sticky row labels');
  assert.ok(header < z(componentsCss, '.info-popover'), 'below the popovers');
  assert.ok(header < z(chartsCss, '.chart-tooltip'), 'below the chart tooltip');
  assert.ok(header < z(componentsCss, '.filter-panel'), 'below the filter panel');
});

test('test_csd024_ac_16_1_and_16_2_the_transition_and_reduced_motion', () => {
  assert.match(rule(layoutCss, '.section-header').transition, /padding/);
  assert.match(rule(layoutCss, '.section-title').transition, /font-size/);
  const reduced = layoutCss.slice(layoutCss.indexOf('prefers-reduced-motion'));
  assert.match(reduced, /\.section-header, \.section-title \{ transition: none; \}/);
});

test('test_csd024_ac_13_1_the_chart_is_not_drawn_again_when_the_header_compresses', async () => {
  const app = await openApp();
  await app.click(app.item('income'));
  await app.click(app.$('income-tab-insights'));
  const { scroller, scrollTo } = layout(app, { scrollHeight: 1400 });
  const anchor = app.$('income-insights-chart');
  // The chart sits 100 px below the top of the page; scrolling moves it up, and a compressed header 37 px more.
  anchor.getBoundingClientRect = () => ({ top: 100 - scroller.scrollTop - (compressed(scroller) ? 37 : 0), left: 0, right: 800, bottom: 500, width: 800, height: 400 });
  window_resize(app);
  await app.settle();
  const drawn = anchor.querySelector('svg');
  assert.ok(drawn, 'the chart is drawn');
  scrollTo(400);
  assert.equal(compressed(scroller), true);
  window_resize(app);
  await app.settle();
  assert.equal(anchor.querySelector('svg'), drawn, 'the same drawing: it was not made again');
  app.close();
});

function window_resize(app) {
  app.window.dispatchEvent(new app.window.Event('resize'));
}

test('test_csd024_ac_12_2_scroll_anchoring_cannot_undo_the_compression', () => {
  assert.equal(rule(layoutCss, '.content')['overflow-anchor'], 'none');
});
