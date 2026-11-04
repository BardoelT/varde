// CSD-018 REQ-10: the Ratios table shows the percentage labels Rust builds, as received, and builds none itself.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { exampleReads } from './fixtures.js';

const UNICODE_MINUS = String.fromCharCode(0x2212);

/** A year of the `get_ratios` answer with its labels, as Rust returns it. */
const year = (overrides) => ({
  year: 2025,
  is_partial: false,
  total_income: '48000.00',
  wealth_accumulation: '6000.00',
  wealth_percent: '12.50',
  wealth_percent_label: '+12.50%',
  investment_growth_percent: null,
  investment_growth_percent_label: '–',
  wealth_growth_percent: '-5.00',
  wealth_growth_percent_label: '-5.00%',
  disposable_income: '10000.00',
  disposable_percent: '0.00',
  disposable_percent_label: '0.00%',
  ...overrides,
});

/** The cells of one row of the drawn table, by the field of that row. */
async function openRatios(years, reads = exampleReads()) {
  reads.get_ratios = { years };
  const app = await openApp({ reads });
  await app.click(app.item('ratios'));
  const row = (index) => [...app.$('ratios-rows').querySelectorAll('tr:not(.group-row)')[index].querySelectorAll('td')].map((cell) => app.text(cell));
  return { app, row };
}

// Rows without the group rows: 0 total income, 1 wealth accumulation, 2 wealth percent, 3 investment growth, 4 wealth growth, 5 disposable income, 6 disposable percent.

test('test_csd018_ac_10_1_percentages_show_the_labels_received_from_rust', async () => {
  const { app, row } = await openRatios([year({}), year({ year: 2024, wealth_percent: null, wealth_percent_label: '–' })]);
  assert.deepEqual(row(2), ['+12.50%', '–']);
  assert.deepEqual(row(3), ['–', '–']);
  assert.deepEqual(row(4), ['-5.00%', '-5.00%']);
  assert.deepEqual(row(6), ['0.00%', '0.00%']);
  app.close();
});

test('test_csd018_ac_10_1_a_label_is_shown_as_received_not_rebuilt_from_the_figure', async () => {
  // The figure says 12.50 but the label is another text: the table shows the label.
  const { app, row } = await openRatios([year({ wealth_percent: '12.50', wealth_percent_label: 'label of Rust' })]);
  assert.deepEqual(row(2), ['label of Rust']);
  app.close();
});

test('test_csd018_ac_10_1_the_example_project_shows_hyphen_minus_and_never_the_unicode_minus', async () => {
  const reads = exampleReads();
  const labels = reads.get_ratios.years.flatMap((entry) => Object.entries(entry).filter(([key]) => key.endsWith('_label')).map(([, label]) => label));
  assert.ok(labels.some((label) => label.startsWith('-')), 'the example has a negative percentage');
  const { app } = await openRatios(reads.get_ratios.years, reads);
  const table = app.text(app.$('ratios-rows'));
  for (const label of labels.filter((label) => label.startsWith('-'))) assert.ok(table.includes(label), label);
  assert.equal(table.includes(UNICODE_MINUS), false);
  app.close();
});

test('test_csd018_ac_10_2_a_negative_percentage_is_shown_in_the_negative_colour', async () => {
  const { app } = await openRatios([year({})]);
  const cells = [...app.$('ratios-rows').querySelectorAll('tr:not(.group-row)')[4].querySelectorAll('td')];
  assert.equal(cells[0].classList.contains('negative'), true);
  app.close();
});
