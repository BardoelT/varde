// CSD-018 display ACs: the page, its rows, sideways scrolling, fresh figures and the explanations.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { openApp } from './harness.js';
import { exampleReads } from './fixtures.js';

const css = readFileSync(fileURLToPath(new URL('../styles/components.css', import.meta.url)), 'utf8');

/** A year of the `get_ratios` answer, as Rust returns it. */
const year = (overrides) => ({
  year: 2025,
  is_partial: false,
  total_income: '48000.00',
  wealth_accumulation: '6000.00',
  wealth_percent: '12.50',
  wealth_percent_label: '+12.50%',
  investment_growth_percent: null,
  investment_growth_percent_label: '–',
  wealth_growth_percent: '10.00',
  wealth_growth_percent_label: '+10.00%',
  disposable_income: '10000.00',
  disposable_percent: '20.83',
  disposable_percent_label: '+20.83%',
  ...overrides,
});

async function openRatios(years) {
  const reads = exampleReads();
  reads.get_ratios = { years };
  const app = await openApp({ reads });
  await app.click(app.item('ratios'));
  return app;
}

const rowOf = (app, label) =>
  [...app.$('ratios-rows').querySelectorAll('tr')].find((tr) => app.text(tr.querySelector('th')).startsWith(label));
const infoOf = (app, label) => rowOf(app, label).querySelector('.info-btn');

test('test_csd018_ac_1_1_one_panel_and_no_tabs', async () => {
  const app = await openRatios([year({})]);
  const section = app.$('ratios');
  assert.equal(app.isShown(section), true);
  const panels = section.querySelectorAll('.panel');
  assert.equal(panels.length, 1);
  assert.equal(app.text(panels[0].querySelector('.panel-title')), 'Ratios per year');
  assert.equal(section.querySelectorAll('[role="tab"], [role="tablist"], .tabs').length, 0, 'there are no tabs');
  assert.equal(app.isShown(app.$('ratios-table')), true);
  app.close();
});

test('test_csd018_ac_3_1_rows_and_sub_headings', async () => {
  const app = await openRatios([year({})]);
  const rows = [...app.$('ratios-rows').querySelectorAll('tr')].map((tr) => ({
    group: tr.classList.contains('group-row'),
    label: app.text(tr.querySelector('th')),
  }));
  assert.deepEqual(rows, [
    { group: true, label: 'Wealth' },
    { group: false, label: 'Total income (gross)' },
    { group: false, label: 'Wealth accumulation from cash' },
    { group: false, label: 'Percentage of income to wealth' },
    { group: true, label: 'Growth' },
    { group: false, label: 'Growth of investments' },
    { group: false, label: 'Growth of total wealth' },
    { group: true, label: 'Free income' },
    { group: false, label: 'Disposable income' },
    { group: false, label: 'Percentage of disposable income' },
  ]);
  app.close();
});

test('test_csd018_ac_10_3_the_row_labels_stay_in_view_while_the_years_scroll', async () => {
  const years = Array.from({ length: 30 }, (_, index) => year({ year: 1997 + index }));
  const app = await openRatios(years);
  // One column per year, and the first cell of every row is a row header...
  assert.equal(app.$('ratios-head').querySelectorAll('th.num').length, 30);
  for (const tr of app.$('ratios-rows').querySelectorAll('tr')) assert.equal(tr.firstElementChild.tagName, 'TH');
  // ...which the stylesheet pins to the left edge inside a wrapper that scrolls sideways.
  assert.match(css, /\.table-wrap\s*\{[^}]*overflow-x:\s*auto/);
  assert.match(css, /\.ratios-table th:first-child\s*\{[^}]*position:\s*sticky;[^}]*left:\s*0/);
  assert.equal(app.$('ratios-table').closest('.table-wrap') !== null, true);
  app.close();
});

test('test_csd018_ac_12_1_the_figures_follow_changes_when_the_page_is_opened_again', async () => {
  const app = await openRatios([year({})]);
  assert.equal(app.text(rowOf(app, 'Disposable income').querySelector('td')), '10,000.00');
  // An expense of 500.00 is added elsewhere: Rust now answers with the new figure.
  app.bridge.table.get_ratios = { years: [year({ disposable_income: '10500.00' })] };
  await app.click(app.item('income'));
  await app.click(app.item('ratios'));
  assert.equal(app.callsOf('get_ratios').length >= 2, true, 'opening the page asks Rust again');
  assert.equal(app.text(rowOf(app, 'Disposable income').querySelector('td')), '10,500.00');
  app.close();
});

test('test_csd018_ac_13_1_the_info_icon_opens_the_explanation_of_its_row', async () => {
  const app = await openRatios([year({})]);
  assert.equal(app.isShown(app.$('ratios-info')), false);
  await app.click(infoOf(app, 'Total income (gross)'));
  assert.equal(app.isShown(app.$('ratios-info')), true);
  assert.match(app.textOf('ratios-info'), /^The gross income of the year from all paychecks/);
  assert.equal(infoOf(app, 'Total income (gross)').getAttribute('aria-expanded'), 'true');
  app.close();
});

test('test_csd018_ac_13_2_the_explanation_closes_on_a_click_elsewhere_or_escape', async () => {
  const app = await openRatios([year({})]);
  const open = async () => app.click(infoOf(app, 'Total income (gross)'));
  await open();
  await app.click(app.$('ratios-title'));
  assert.equal(app.isShown(app.$('ratios-info')), false, 'a click elsewhere closes it');
  await open();
  assert.equal(app.isShown(app.$('ratios-info')), true);
  app.document.dispatchEvent(new app.window.KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  assert.equal(app.isShown(app.$('ratios-info')), false, 'Escape closes it');
  app.close();
});

test('test_csd018_ac_13_3_only_one_explanation_is_shown_at_a_time', async () => {
  const app = await openRatios([year({})]);
  await app.click(infoOf(app, 'Total income (gross)'));
  await app.click(infoOf(app, 'Disposable income'));
  assert.equal(app.isShown(app.$('ratios-info')), true);
  assert.match(app.textOf('ratios-info'), /^The wealth accumulation from cash plus the expenses of the year/);
  assert.doesNotMatch(app.textOf('ratios-info'), /gross income of the year/);
  assert.equal(infoOf(app, 'Total income (gross)').getAttribute('aria-expanded'), 'false');
  assert.equal(infoOf(app, 'Disposable income').getAttribute('aria-expanded'), 'true');
  app.close();
});
