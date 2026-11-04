// CSD-015 display ACs: the Houses page (sections, rows, tables, dialogs, empty states) and the
// currency confirmation that counts house valuations. The answers are what Rust returns; the page
// only displays them. Amounts are shown with the digit grouping of CSD-022 ("300,000.00").
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { emptyReads, exampleReads } from './fixtures.js';

const house = (overrides) => ({
  colour_slot: 0,
  held: true,
  id: 'id-1',
  latest_period: '2026-09',
  latest_value: '300000.00',
  linked_debts: [],
  name: 'Example House',
  period: 'Monthly',
  purchase_amount: '250000.00',
  purchase_date: '2024-03-15',
  result: null,
  sell_amount: null,
  sell_date: null,
  valuation_count: 2,
  ...overrides,
});
const valuation = (overrides) => ({ change: null, house_id: 'id-1', house_name: 'Example House', note: '', period: '2026-09', value: '300000.00', ...overrides });

const HOUSE = house({});
const LODGE = house({ id: 'id-2', name: 'Example Lodge', period: 'Yearly', latest_period: '2025', latest_value: '80000.00', colour_slot: 1, valuation_count: 1 });

/** Opens the Houses page with the given answers of the three list commands. */
async function openHouses({ houses = [HOUSE, LODGE], total = '380000.00', valuations = [], totals = [], handlers = {} } = {}) {
  const reads = exampleReads();
  reads.list_houses = { houses, total };
  reads.list_house_valuations = valuations;
  reads.yearly_house_values = totals;
  const app = await openApp({ reads, handlers });
  await app.click(app.item('houses'));
  const panels = () => [...app.$('houses-overview').querySelectorAll('.panel')];
  const panel = (title) => panels().find((node) => app.text(node.querySelector('.panel-title')) === title);
  return { app, panels, panel };
}

test('test_csd015_ac_21_1_the_sections_come_in_order_and_houses_has_an_icon_in_the_left_pane', async () => {
  const { app, panels } = await openHouses();
  assert.deepEqual(panels().map((node) => app.text(node.querySelector('.panel-title'))), ['Houses', 'Yearly values', 'Valuations']);
  assert.ok(app.item('houses').querySelector('svg'), 'a house icon in the left pane');
  assert.equal(app.text(app.item('houses')), 'Houses');
  app.close();
});

test('test_csd015_ac_21_2_house_rows_carry_period_latest_value_and_actions', async () => {
  const { app, panel } = await openHouses();
  const [first, second] = [...panel('Houses').querySelectorAll('.job-list li')];
  const parts = (row) => [...row.children].filter((node) => !node.classList.contains('job-actions')).map((node) => app.text(node));
  assert.deepEqual(parts(first), ['Example House', 'Monthly · Held', '300,000.002026-09']);
  assert.deepEqual(parts(second), ['Example Lodge', 'Yearly · Held', '80,000.002025']);
  assert.equal(app.text(first.querySelector('.account-amount').firstChild), '300,000.00');
  const small = first.querySelector('.account-amount small');
  assert.equal(app.text(small), '2026-09', 'the period follows the value in smaller text');
  assert.doesNotMatch(app.text(first) + app.text(second), /valuations?/i, 'no number of valuations');
  const actions = second.querySelectorAll('.job-actions button');
  assert.deepEqual([...actions].map((node) => node.getAttribute('aria-label')), ['Edit Example Lodge', 'Delete Example Lodge']);
  assert.deepEqual([...actions].map((node) => node.getAttribute('title')), ['Edit Example Lodge', 'Delete Example Lodge'], 'also in a tooltip');
  assert.equal(app.text(app.$('house-total')), 'Total value 380,000.00');
  app.close();
});

test('test_csd015_ac_21_3_the_valuations_table_shows_period_house_value_change_and_note', async () => {
  const { app, panel } = await openHouses({
    valuations: [valuation({ period: '2026-09', change: '500.00' }), valuation({ period: '2026-08', value: '299500.00' })],
  });
  const section = panel('Valuations');
  assert.deepEqual([...section.querySelectorAll('thead th')].map((node) => app.text(node)).slice(0, 5), ['Period', 'House', 'Value', 'Change', 'Note']);
  const rows = [...section.querySelectorAll('tbody tr')].map((row) => [...row.children].map((cell) => app.text(cell)).slice(0, 4));
  assert.deepEqual(rows, [
    ['2026-09', 'Example House', '300,000.00', '500.00'],
    ['2026-08', 'Example House', '299,500.00', '—'],
  ]);
  app.close();
});

test('test_csd015_ac_21_4_a_long_note_is_cut_to_one_line_and_editing_shows_all_of_it', async () => {
  const note = 'first line\nsecond line\nthird line';
  const { app, panel } = await openHouses({ valuations: [valuation({ note })] });
  const cell = panel('Valuations').querySelector('td.note-cell');
  assert.equal(app.text(cell), 'first line …');
  await app.click(panel('Valuations').querySelector('[aria-label="Edit Example House 2026-09"]'));
  assert.equal(app.$('house-valuation-form').elements.note.value, note, 'the whole note is in the edit form');
  app.close();
});

test('test_csd015_ac_21_5_clicking_a_house_row_changes_nothing', async () => {
  const { app, panel } = await openHouses({ valuations: [valuation({})], totals: [{ year: 2026, closing_value: '300000.00', change: null, valuation_count: 1 }] });
  const before = [app.text(app.$('house-total')), app.text(panel('Yearly values')), app.text(panel('Valuations'))];
  const calls = app.calls.length;
  await app.click(panel('Houses').querySelector('.job-row'));
  assert.deepEqual([app.text(app.$('house-total')), app.text(panel('Yearly values')), app.text(panel('Valuations'))], before);
  assert.equal(app.calls.length, calls, 'nothing is requested or changed');
  assert.equal(app.$('house-dialog').open, false);
  app.close();
});

test('test_csd015_ac_21_6_a_note_is_shown_as_plain_text', async () => {
  const { app, panel } = await openHouses({ valuations: [valuation({ note: '<b>bold</b>' })] });
  const cell = panel('Valuations').querySelector('td.note-cell');
  assert.equal(cell.textContent, '<b>bold</b>');
  assert.equal(cell.querySelector('b'), null, 'no formatting');
  app.close();
});

test('test_csd015_ac_21_7_the_add_buttons_sit_in_the_section_headers', async () => {
  const { app, panel } = await openHouses();
  assert.deepEqual([...panel('Houses').querySelectorAll('.panel-header button')].map((node) => app.text(node)), ['Add house']);
  assert.deepEqual([...panel('Valuations').querySelectorAll('.panel-header button')].map((node) => app.text(node)), ['Add valuation']);
  assert.equal(app.$('houses').querySelector('.section-header').querySelectorAll('button').length, 0, 'the title row holds no button');
  app.close();
});

test('test_csd015_ac_21_8_the_yearly_values_section_has_year_closing_value_and_change_only', async () => {
  const { app, panel } = await openHouses({
    totals: [
      { year: 2026, closing_value: '300000.00', change: '40000.00', valuation_count: 1 },
      { year: 2025, closing_value: '260000.00', change: null, valuation_count: 1 },
    ],
  });
  const section = panel('Yearly values');
  assert.deepEqual([...section.querySelectorAll('thead th')].map((node) => app.text(node)), ['Year', 'Closing value', 'Change']);
  const rows = [...section.querySelectorAll('tbody tr')].map((row) => [...row.children].map((cell) => app.text(cell)));
  assert.deepEqual(rows, [['2026', '300,000.00', '40,000.00'], ['2025', '260,000.00', '—']]);
  assert.equal(section.querySelectorAll('button').length, 0, 'no button in its header or rows');
  app.close();
  // Not shown while no houses exist.
  const empty = await openApp({ reads: emptyReads() });
  await empty.click(empty.item('houses'));
  assert.equal(empty.isShown(empty.$('house-totals-table')), false);
  assert.equal(empty.isShown(empty.$('houses-content')), false);
  empty.close();
});

test('test_csd015_ac_22_2_a_house_without_valuations_shows_a_dash_and_the_empty_note', async () => {
  const bare = house({ latest_period: null, latest_value: null, valuation_count: 0 });
  const { app, panel } = await openHouses({ houses: [bare], total: null });
  const row = panel('Houses').querySelector('.job-row');
  assert.equal(app.text(row.querySelector('.account-amount')), '—');
  assert.match(app.text(row.querySelector('.job-meta')), /^Monthly/);
  assert.doesNotMatch(app.text(row), /No valuations yet/);
  assert.equal(app.isShown(app.$('house-valuations-empty')), true);
  assert.equal(app.textOf('house-valuations-empty'), 'No valuations yet.');
  assert.equal(app.$('toast').hidden, true, 'no error is shown');
  app.close();
});

test('test_csd015_ac_22_3_no_valuations_in_any_house_shows_the_yearly_values_note_and_no_table', async () => {
  const { app } = await openHouses({ houses: [house({ latest_period: null, latest_value: null, valuation_count: 0 })], total: null });
  assert.equal(app.isShown(app.$('house-totals-empty')), true);
  assert.equal(app.textOf('house-totals-empty'), 'No yearly values yet. They appear once there is a valuation.');
  assert.equal(app.isShown(app.$('house-totals-table')), false);
  assert.equal(app.$('toast').hidden, true);
  app.close();
});

test('test_csd015_ac_23_1_the_sections_show_no_currency', async () => {
  const { app, panels } = await openHouses({
    valuations: [valuation({ change: '500.00' })],
    totals: [{ year: 2026, closing_value: '300000.00', change: '500.00', valuation_count: 1 }],
  });
  assert.deepEqual(panels().map((node) => app.text(node.querySelector('.panel-title'))), ['Houses', 'Yearly values', 'Valuations']);
  assert.doesNotMatch(app.text(app.$('houses-overview')), /EUR|€|USD|\$/);
  app.close();
});

test('test_csd015_ac_26_1_houses_is_the_last_item_of_the_wealth_group', async () => {
  const app = await openApp();
  const nav = [...app.document.querySelectorAll('.sidebar nav > *, nav.sidebar-nav > *, .nav-section-label, .nav-item')];
  const labels = [...app.document.querySelectorAll('.nav-section-label')];
  const wealth = labels.find((node) => app.text(node) === 'Wealth');
  assert.ok(wealth && nav.length > 0);
  const group = [];
  for (let node = wealth.nextElementSibling; node && !node.classList.contains('nav-section-label'); node = node.nextElementSibling) group.push(node.dataset.section);
  assert.deepEqual(group, ['bank', 'investments', 'houses']);
  app.close();
});

// ---- deleting: cancelling changes nothing, and the confirmation states no count ----

test('test_csd015_ac_6_2_cancelling_the_deletion_of_a_house_changes_nothing', async () => {
  const { app, panel } = await openHouses({ handlers: { delete_house: null } });
  await app.click(panel('Houses').querySelector('[aria-label="Delete Example House"]'));
  assert.equal(app.$('confirm-dialog').open, true);
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.$('confirm-dialog').open, false);
  assert.equal(app.callsOf('delete_house').length, 0);
  assert.equal(panel('Houses').querySelectorAll('.job-list li').length, 2);
  app.close();
});

test('test_csd015_ac_6_4_the_confirmation_states_no_number_of_valuations', async () => {
  for (const count of [0, 1, 3]) {
    const { app, panel } = await openHouses({ houses: [house({ valuation_count: count })], total: '1.00' });
    await app.click(panel('Houses').querySelector('[aria-label="Delete Example House"]'));
    assert.equal(app.textOf('confirm-message'), 'This house and all its valuations will be deleted.');
    assert.doesNotMatch(app.textOf('confirm-title') + app.textOf('confirm-message'), /\d/, 'no number');
    await app.click(app.$('confirm-cancel'));
    app.close();
  }
});

test('test_csd015_ac_14_2_cancelling_the_deletion_of_a_valuation_changes_nothing', async () => {
  const { app, panel } = await openHouses({ valuations: [valuation({ period: '2026-09' }), valuation({ period: '2026-08' })] });
  await app.click(panel('Valuations').querySelector('[aria-label="Delete Example House 2026-09"]'));
  assert.equal(app.$('confirm-dialog').open, true);
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.callsOf('delete_house_valuation').length, 0);
  assert.equal(panel('Valuations').querySelectorAll('tbody tr').length, 2, 'both valuations still exist');
  app.close();
});

// ---- the valuation form ----

test('test_csd015_ac_12_7_the_edit_form_shows_the_period_and_the_house_which_cannot_change', async () => {
  const rows = [valuation({ period: '2026-09' }), valuation({ house_id: 'id-2', house_name: 'Example Lodge', period: '2025', value: '80000.00' })];
  const { app, panel } = await openHouses({ valuations: rows });
  const form = app.$('house-valuation-form');
  const edit = (label) => panel('Valuations').querySelector(`[aria-label="${label}"]`);

  await app.click(edit('Edit Example House 2026-09'));
  assert.equal(app.$('house-valuation-dialog').open, true);
  assert.equal(app.textOf('house-valuation-period-label'), 'Month');
  assert.equal(form.elements.period.value, '2026-09');
  assert.equal(form.elements.period.disabled, false, 'the period can change');
  assert.equal(form.elements.house_id.selectedOptions[0].textContent, 'Example House');
  assert.equal(form.elements.house_id.disabled, true, 'the house cannot');
  await app.escape(app.$('house-valuation-dialog'));

  await app.click(edit('Edit Example Lodge 2025'));
  assert.equal(app.textOf('house-valuation-period-label'), 'Year');
  assert.equal(form.elements.period.value, '2025');
  assert.equal(form.elements.house_id.selectedOptions[0].textContent, 'Example Lodge');
  assert.equal(form.elements.house_id.disabled, true);
  app.close();
});

/** The add form with the copy command answered by `prefill`. */
async function openAddForm(prefill) {
  const opened = await openHouses({ handlers: { copy_previous_house_valuation: prefill } });
  await opened.app.click(opened.app.$('house-valuation-add'));
  return opened;
}

test('test_csd015_ac_15_2_copying_for_a_house_without_valuations_leaves_the_form_empty', async () => {
  const { app } = await openAddForm({ value: null, note: '' });
  const form = app.$('house-valuation-form');
  await app.click(app.$('house-valuation-copy'));
  assert.equal(app.callsOf('copy_previous_house_valuation').length, 1);
  assert.equal(form.elements.value.value, '');
  assert.equal(app.isShown(app.$('house-valuation-error')), false, 'no error is shown');
  app.close();
});

test('test_csd015_ac_15_3_a_value_of_zero_is_copied_as_it_is', async () => {
  const { app } = await openAddForm({ value: '0.00', note: '' });
  await app.click(app.$('house-valuation-copy'));
  assert.equal(app.$('house-valuation-form').elements.value.value, '0.00');
  app.close();
});

test('test_csd015_ac_16_3_switching_house_switches_the_form_between_a_month_and_a_year', async () => {
  // Rust lists the houses alphabetically: Example House (Monthly) before Example Lodge (Yearly).
  const { app } = await openAddForm({ value: null, note: '' });
  const form = app.$('house-valuation-form');
  assert.equal(app.textOf('house-valuation-period-label'), 'Month', 'the first house is monthly');
  assert.match(form.elements.period.value, /^\d{4}-\d{2}$/);
  await app.choose(form.elements.house_id, 'id-2');
  assert.equal(app.textOf('house-valuation-period-label'), 'Year', 'the yearly house asks for a year');
  assert.match(form.elements.period.value, /^\d{4}$/);
  assert.equal(app.textOf('house-valuation-copy'), 'Copy previous year');
  await app.choose(form.elements.house_id, 'id-1');
  assert.equal(app.textOf('house-valuation-period-label'), 'Month', 'and a month again');
  assert.match(form.elements.period.value, /^\d{4}-\d{2}$/);
  assert.equal(app.textOf('house-valuation-copy'), 'Copy previous month');
  app.close();
});

// ---- the currency confirmation counts house valuations (governed by CSD-003 REQ-5) ----

async function openSettings(entryCount) {
  const reads = exampleReads();
  reads.get_settings = { currency: 'EUR', entry_count: entryCount };
  const handlers = { set_currency: ({ currency }) => ({ currency, entry_count: entryCount }) };
  const app = await openApp({ reads, handlers });
  await app.click(app.item('settings'));
  return app;
}

test('test_csd015_ac_25_1_a_house_valuation_asks_for_confirmation_and_cancelling_keeps_the_currency', async () => {
  const app = await openSettings(1);
  const select = app.$('setting-currency');
  await app.choose(select, 'USD');
  assert.equal(app.$('confirm-dialog').open, true, 'the app asks before changing it');
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.callsOf('set_currency').length, 0);
  assert.equal(select.value, 'EUR', 'cancelling leaves the currency EUR');
  app.close();
});

test('test_csd015_ac_25_2_a_house_without_valuations_needs_no_confirmation', async () => {
  const app = await openSettings(0);
  const select = app.$('setting-currency');
  await app.choose(select, 'USD');
  assert.equal(app.$('confirm-dialog').open, false, 'no confirmation');
  assert.deepEqual(app.callsOf('set_currency').map((call) => call.args), [{ currency: 'USD' }]);
  assert.equal(select.value, 'USD');
  app.close();
});
