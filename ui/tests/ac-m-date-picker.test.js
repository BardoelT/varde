// Display ACs of CSD-036: the date picker of every date, month and year field. Today is fixed to 22/11/2026
// through the clock of the test, so the days, the marks and the months are the same on every day it runs.
import test, { mock } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { openApp } from './harness.js';
import { exampleReads } from './fixtures.js';

const NOW = new Date(2026, 10, 22, 12, 0, 0);

/** Runs `body` with the clock at 22/11/2026; the clock is given back afterwards. */
async function withClock(body) {
  mock.timers.enable({ apis: ['Date'], now: NOW });
  try {
    await body();
  } finally {
    mock.timers.reset();
  }
}

/** The Add job dialog: a date field (the start date) in a modal dialog. */
async function jobDialog(options = {}) {
  const app = await openApp(options);
  await app.click(app.item('income'));
  await app.click(app.$('job-add'));
  return app;
}

const startDate = (app) => app.$('job-form').elements.start_date;
const picker = (app) => app.document.querySelector('.date-picker');
const title = (app) => app.text(picker(app).querySelector('.dp-title'));
const click = async (app, element) => app.click(element);
const key = (app, target, name, extra = {}) => target.dispatchEvent(new app.window.KeyboardEvent('keydown', { key: name, bubbles: true, cancelable: true, ...extra }));
const cellOf = (app, iso) => picker(app).querySelector(`[data-date="${iso}"]`);
const choices = (app) => [...picker(app).querySelectorAll('.dp-choice')].map((cell) => app.text(cell));
const arrows = (app) => [...picker(app).querySelectorAll('.dp-arrow')];

/** The month filter of the bank entries: a month field in a filter panel, not in a dialog. */
async function bankPeriodFilter() {
  const app = await openApp();
  await app.click(app.item('bank'));
  await app.click(app.$('bank-tab-entries'));
  await app.click(app.$('bank-entry-period-filter').querySelector('.filter-btn'));
  return app;
}
const monthField = (app) => app.$('bank-entry-period-filter').querySelector('input[data-mask=month]');

test('test_csd036_ac_1_1_every_field_has_a_picker_on_its_own_level', async () => {
  await withClock(async () => {
    const app = await jobDialog();
    await click(app, startDate(app));
    assert.equal(title(app), 'November 2026', 'a date field opens on the days');
    assert.equal(picker(app).querySelectorAll('.dp-days').length, 1);
    app.close();

    const filter = await bankPeriodFilter();
    await click(filter, monthField(filter));
    assert.equal(title(filter), '2026', 'a month field opens on the months');
    assert.deepEqual(choices(filter).slice(0, 3), ['Jan', 'Feb', 'Mar']);
    filter.close();

    const yearApp = await jobDialog();
    const { maskInput } = await yearApp.load('shared/date-mask.js');
    const year = yearApp.document.createElement('input');
    year.dataset.mask = 'month';
    year.maxLength = 4;
    yearApp.$('job-form').append(year);
    maskInput(year);
    await yearApp.click(year);
    assert.equal(title(yearApp), '2020 – 2029', 'a year field opens on the years');
    yearApp.close();
  });
});

test('test_csd036_ac_1_3_a_field_that_becomes_a_year_field_shows_only_the_years', async () => {
  await withClock(async () => {
    const app = await jobDialog();
    const { maskInput } = await app.load('shared/date-mask.js');
    const field = app.document.createElement('input');
    field.dataset.mask = 'month';
    field.maxLength = 7;
    app.$('job-form').append(field);
    maskInput(field);
    // The period of a statement is a month until the user chooses a yearly statement (the form changes its length).
    field.maxLength = 4;
    await app.click(field);
    assert.equal(title(app), '2020 – 2029');
    assert.equal(picker(app).querySelector('button.dp-title'), null, 'no way to the months');
    await click(app, arrows(app)[1]);
    assert.equal(title(app), '2030 – 2039');
    await click(app, picker(app).querySelector('[data-choice="2031"]'));
    assert.equal(field.value, '2031');
    assert.equal(picker(app), null, 'the pick is the year, no months are shown');
    app.close();
  });
});

test('test_csd036_ac_1_2_no_field_is_left_without_a_calendar_button', async () => {
  const app = await openApp();
  const fields = [...app.document.querySelectorAll('input[data-mask]')];
  assert.ok(fields.length >= 10);
  for (const field of fields) assert.ok(field.parentElement.querySelector('.date-open'), `${field.name} has a calendar button`);
  app.close();
});

test('test_csd036_ac_2_1_click_button_and_key_open_it', async () => {
  await withClock(async () => {
    const app = await jobDialog();
    await click(app, startDate(app));
    assert.equal(title(app), 'November 2026');
    key(app, startDate(app), 'Escape');
    assert.equal(picker(app), null);
    await click(app, startDate(app).parentElement.querySelector('.date-open'));
    assert.equal(title(app), 'November 2026', 'the calendar button');
    await click(app, startDate(app).parentElement.querySelector('.date-open'));
    assert.equal(picker(app), null, 'the button closes it again');
    key(app, startDate(app), 'ArrowDown');
    assert.equal(title(app), 'November 2026', 'Down');
    assert.equal(app.document.activeElement, cellOf(app, '2026-11-22'), 'the focus moves to the grid');
    app.close();
  });
});

test('test_csd036_ac_2_2_the_date_of_the_field_is_shown', async () => {
  await withClock(async () => {
    const app = await jobDialog();
    startDate(app).value = '15/03/2025';
    await click(app, startDate(app));
    assert.equal(title(app), 'March 2025');
    assert.ok(cellOf(app, '2025-03-15').classList.contains('selected'));
    app.close();
  });
});

test('test_csd036_ac_2_3_tab_alone_does_not_open_it', async () => {
  const app = await jobDialog();
  startDate(app).focus();
  assert.equal(picker(app), null);
  app.close();
});

test('test_csd036_ac_3_1_escape_closes_only_the_picker', async () => {
  const app = await jobDialog();
  await click(app, startDate(app));
  const event = new app.window.KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
  startDate(app).dispatchEvent(event);
  assert.equal(picker(app), null);
  assert.ok(event.defaultPrevented, 'the dialog does not get the Escape');
  assert.ok(app.$('job-dialog').open, 'the dialog is still open');
  assert.equal(app.document.activeElement, startDate(app));
  app.close();
});

test('test_csd036_ac_3_2_other_ways_to_close_it', async () => {
  const app = await jobDialog();
  const field = startDate(app);
  await click(app, field);
  field.ownerDocument.body.dispatchEvent(new app.window.MouseEvent('mousedown', { bubbles: true }));
  assert.equal(picker(app), null, 'a click outside');
  await click(app, field);
  key(app, field, 'Tab');
  assert.equal(picker(app), null, 'Tab');
  await click(app, field);
  await click(app, cellOf(app, '2026-11-10'));
  assert.equal(picker(app), null, 'a pick');
  await click(app, field);
  app.$('job-dialog').close();
  assert.equal(picker(app), null, 'closing the dialog');
  app.close();
});

test('test_csd036_ac_4_1_the_grid_of_november_2026', async () => {
  await withClock(async () => {
    const app = await jobDialog();
    await click(app, startDate(app));
    const [previous, next] = arrows(app);
    assert.equal(previous.getAttribute('aria-label'), 'Previous month');
    assert.equal(next.getAttribute('aria-label'), 'Next month');
    const header = picker(app).querySelector('.dp-header');
    assert.deepEqual([...header.children].map((node) => node === previous ? 'previous' : node === next ? 'next' : 'title'), ['previous', 'title', 'next']);
    assert.deepEqual([...picker(app).querySelectorAll('.dp-weekdays span')].map((node) => app.text(node)), ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']);
    const rows = picker(app).querySelectorAll('.dp-days .dp-row');
    assert.equal(rows.length, 6);
    assert.equal(rows[0].firstElementChild.dataset.date, '2026-10-26');
    assert.ok(rows[0].firstElementChild.classList.contains('outside'));
    assert.equal(rows[0].children[6].dataset.date, '2026-11-01', 'the 1st is under Sun');
    assert.ok(cellOf(app, '2026-11-22').classList.contains('today'));
    app.close();
  });
});

test('test_csd036_ac_4_2_a_neighbouring_day_can_be_chosen', async () => {
  await withClock(async () => {
    const app = await jobDialog();
    await click(app, startDate(app));
    await click(app, cellOf(app, '2026-12-01'));
    assert.equal(startDate(app).value, '01/12/2026');
    app.close();
  });
});

test('test_csd036_ac_4_3_the_arrows_change_the_month', async () => {
  await withClock(async () => {
    const app = await jobDialog();
    await click(app, startDate(app));
    await click(app, arrows(app)[1]);
    await click(app, arrows(app)[1]);
    assert.equal(title(app), 'January 2027');
    await click(app, arrows(app)[0]);
    assert.equal(title(app), 'December 2026');
    app.close();
  });
});

test('test_csd036_ac_5_1_from_days_to_months_to_years_and_back', async () => {
  await withClock(async () => {
    const app = await jobDialog();
    await click(app, startDate(app));
    await click(app, picker(app).querySelector('button.dp-title'));
    assert.equal(title(app), '2026');
    assert.equal(choices(app).length, 12);
    assert.ok([...picker(app).querySelectorAll('.dp-choice')][10].classList.contains('today'), 'Nov is the current month');
    await click(app, picker(app).querySelector('button.dp-title'));
    assert.equal(title(app), '2020 – 2029');
    assert.deepEqual(choices(app), ['2020', '2021', '2022', '2023', '2024', '2025', '2026', '2027', '2028', '2029']);
    assert.equal(picker(app).querySelector('button.dp-title'), null, 'the header of the years is not a button');
    await click(app, picker(app).querySelector('[data-choice="2024"]'));
    assert.equal(title(app), '2024');
    await click(app, picker(app).querySelector('[data-choice="2"]'));
    assert.equal(title(app), 'March 2024');
    app.close();
  });
});

test('test_csd036_ac_5_2_the_arrows_step_by_the_size_of_the_level', async () => {
  await withClock(async () => {
    const app = await jobDialog();
    await click(app, startDate(app));
    await click(app, picker(app).querySelector('button.dp-title'));
    await click(app, arrows(app)[1]);
    assert.equal(title(app), '2027');
    await click(app, picker(app).querySelector('button.dp-title'));
    await click(app, arrows(app)[0]);
    assert.equal(title(app), '2010 – 2019');
    app.close();
  });
});

test('test_csd036_ac_5_3_the_ends_of_the_calendar', async () => {
  const app = await jobDialog();
  startDate(app).value = '01/01/0001';
  await click(app, startDate(app));
  assert.equal(title(app), 'January 1');
  assert.equal(arrows(app)[0].disabled, true);
  key(app, startDate(app), 'Escape');
  startDate(app).value = '31/12/9999';
  await click(app, startDate(app));
  assert.equal(arrows(app)[1].disabled, true);
  app.close();
});

test('test_csd036_ac_6_1_what_a_pick_writes', async () => {
  await withClock(async () => {
    const app = await jobDialog();
    const seen = [];
    startDate(app).addEventListener('input', () => seen.push(startDate(app).value));
    await click(app, startDate(app));
    await click(app, cellOf(app, '2026-11-05'));
    assert.equal(startDate(app).value, '05/11/2026');
    assert.deepEqual(seen, ['05/11/2026'], 'an input event was fired');
    app.close();

    const filter = await bankPeriodFilter();
    await click(filter, monthField(filter));
    await click(filter, filter.document.querySelector('[data-choice="2"]'));
    assert.equal(monthField(filter).value, '2026-03');
    filter.close();

    const yearApp = await jobDialog();
    const { maskInput } = await yearApp.load('shared/date-mask.js');
    const year = yearApp.document.createElement('input');
    year.dataset.mask = 'month';
    year.maxLength = 4;
    yearApp.$('job-form').append(year);
    maskInput(year);
    await yearApp.click(year);
    await yearApp.click(yearApp.document.querySelector('[data-choice="2026"]'));
    assert.equal(year.value, '2026');
    yearApp.close();
  });
});

test('test_csd036_ac_6_2_a_filter_reacts_to_a_pick', async () => {
  await withClock(async () => {
    const app = await bankPeriodFilter();
    await click(app, monthField(app));
    await click(app, app.document.querySelector('[data-choice="6"]'));
    await new Promise((resolve) => setTimeout(resolve, 260)); // the pause after which a typed filter asks the table (CSD-028 REQ-2)
    await app.settle();
    assert.equal(app.callsOf('list_bank_entries').at(-1).args.filter.from, '2026-07');
    app.close();
  });
});

test('test_csd036_ac_7_1_the_picker_follows_what_is_typed', async () => {
  await withClock(async () => {
    const app = await jobDialog();
    await click(app, startDate(app));
    await app.type(startDate(app), '15032025');
    assert.equal(startDate(app).value, '15/03/2025');
    assert.equal(title(app), 'March 2025');
    assert.ok(cellOf(app, '2025-03-15').classList.contains('selected'));
    await app.backspace(startDate(app));
    assert.equal(title(app), 'November 2026');
    assert.equal(picker(app).querySelectorAll('.selected').length, 0);
    app.close();
  });
});

test('test_csd036_ac_8_1_every_real_date_can_be_picked', async () => {
  await withClock(async () => {
    const app = await jobDialog();
    app.$('job-form').elements.end_date.value = '';
    startDate(app).value = '01/06/2026';
    const end = app.$('job-form').elements.end_date;
    end.value = '01/05/2026';
    await click(app, end.parentElement.querySelector('.date-open'));
    assert.equal(picker(app).querySelectorAll('.dp-cell:disabled').length, 0, 'no day is disabled');
    await click(app, cellOf(app, '2026-05-01'));
    assert.equal(end.value, '01/05/2026');
    app.close();
  });
});

test('test_csd036_ac_9_1_the_picker_is_inside_the_dialog', async () => {
  const app = await jobDialog();
  await click(app, startDate(app));
  assert.ok(app.$('job-dialog').contains(picker(app)));
  app.close();
});

test('test_csd036_ac_9_2_no_room_below_opens_it_above', async () => {
  const app = await jobDialog();
  Object.defineProperty(app.window.HTMLElement.prototype, 'offsetHeight', { configurable: true, get: () => 300 });
  const field = startDate(app);
  field.getBoundingClientRect = () => ({ top: 700, bottom: 730, left: 40, right: 240, width: 200, height: 30 });
  await click(app, field);
  const top = Number.parseFloat(picker(app).style.top);
  assert.ok(picker(app).classList.contains('above'));
  assert.ok(top >= 0 && top + 300 <= app.window.innerHeight, 'inside the window');
  assert.ok(top + 300 <= 700, 'above the field');
  app.close();
});

test('test_csd036_ac_10_1_the_roles_and_names', async () => {
  const app = await jobDialog();
  await click(app, startDate(app));
  const popup = picker(app);
  assert.equal(popup.getAttribute('role'), 'dialog');
  assert.equal(popup.getAttribute('aria-labelledby'), popup.querySelector('.dp-title').id);
  assert.equal(popup.querySelectorAll('[role="grid"] [role="gridcell"]').length, 42);
  assert.deepEqual(arrows(app).map((arrow) => arrow.getAttribute('aria-label')), ['Previous month', 'Next month']);
  assert.equal(popup.querySelector('button.dp-title').getAttribute('aria-label'), 'Choose a month');
  assert.match(popup.querySelector('[role="gridcell"]').getAttribute('aria-label'), /^\d+ \w+ \d{4}$/);
  app.close();
});

test('test_csd036_ac_10_2_moving_by_keys', async () => {
  const cases = [['ArrowRight', '2026-03-16'], ['ArrowLeft', '2026-03-14'], ['ArrowDown', '2026-03-22'], ['ArrowUp', '2026-03-08'], ['PageDown', '2026-04-15'], ['PageUp', '2026-02-15']];
  for (const [name, expected] of cases) {
    const app = await jobDialog();
    startDate(app).value = '15/03/2026';
    key(app, startDate(app), 'ArrowDown');
    const focused = app.document.activeElement;
    assert.equal(focused.dataset.date, '2026-03-15');
    key(app, focused, name);
    assert.equal(app.document.activeElement.dataset.date, expected, name);
    app.close();
  }
});

test('test_csd036_ac_10_3_enter_picks', async () => {
  const app = await jobDialog();
  startDate(app).value = '15/03/2026';
  key(app, startDate(app), 'ArrowDown');
  key(app, app.document.activeElement, 'Enter');
  assert.equal(startDate(app).value, '15/03/2026');
  assert.equal(picker(app), null);
  app.close();
});

test('test_csd036_ac_11_1_dutch', async () => {
  await withClock(async () => {
    const reads = exampleReads();
    reads.get_preferences = { ...reads.get_preferences, language: 'nl' };
    const app = await jobDialog({ reads });
    await click(app, startDate(app));
    assert.equal(title(app), 'november 2026');
    assert.deepEqual([...picker(app).querySelectorAll('.dp-weekdays span')].map((node) => app.text(node)), ['ma', 'di', 'wo', 'do', 'vr', 'za', 'zo']);
    assert.equal(arrows(app)[0].getAttribute('aria-label'), 'Vorige maand');
    app.close();
  });
});

test('test_csd036_ac_11_2_discrete_mode_does_not_blur_the_picker', async () => {
  const reads = exampleReads();
  reads.get_preferences = { ...reads.get_preferences, discrete_mode: true };
  const app = await jobDialog({ reads });
  await click(app, startDate(app));
  assert.ok(app.document.documentElement.classList.contains('discrete-mode'));
  const css = readFileSync(fileURLToPath(new URL('../styles/components.css', import.meta.url)), 'utf8');
  const selectors = [...css.matchAll(/(\.discrete-mode[^{,]*)[,{]/g)].map((match) => match[1].trim());
  for (const selector of selectors) {
    assert.equal(picker(app).matches(selector) || picker(app).querySelector(selector) !== null, false, `${selector} does not reach the picker`);
  }
  assert.equal(picker(app).querySelector('.hidden-amount'), null);
  app.close();
});

// ---- REQ-12: a field that offers only some periods (the donut fields of CSD-032 REQ-35) ----

const SPENDING = { years: [2026, 2025], months: ['2026-11', '2026-10', '2025-03'] };
const SHARES = { total: '10.00', slices: [{ id: 'uncategorized', name: 'Uncategorized', colour_slot: 7, amount: '10.00', percent_label: '100.00%', start: '0.000000', end: '1.000000' }] };

async function donutFields() {
  const app = await openApp({ handlers: { transaction_spending_periods: SPENDING, transaction_spending_shares: SHARES } });
  await app.click(app.item('transactions'));
  await app.click(app.$('transactions-tab-categories'));
  return app;
}
const monthField2 = (app) => app.$('transaction-donut-month-field');
const yearField2 = (app) => app.$('transaction-donut-year-field');
const choiceCells = (app) => [...picker(app).querySelectorAll('.dp-choice')];
const available = (app) => choiceCells(app).filter((cell) => !cell.disabled).map((cell) => app.text(cell));

test('test_csd036_ac_12_1_only_the_months_with_data_can_be_chosen', async () => {
  await withClock(async () => {
    const app = await donutFields();
    await click(app, monthField2(app));
    assert.equal(title(app), '2026');
    assert.deepEqual(available(app), ['Oct', 'Nov']);
    assert.equal(choiceCells(app).filter((cell) => cell.disabled).length, 10, 'the other ten months are grey and disabled');
    assert.match(choiceCells(app)[0].getAttribute('aria-label'), /\(not available\)/);
    await click(app, picker(app).querySelector('button.dp-title'));
    assert.deepEqual(available(app), ['2025', '2026'], 'the years with data');
    assert.equal(choiceCells(app).filter((cell) => cell.disabled).length, 8);
    app.close();
  });
});

test('test_csd036_ac_12_2_the_arrows_stop_at_the_ends_of_the_data', async () => {
  await withClock(async () => {
    const app = await donutFields();
    await click(app, yearField2(app));
    assert.equal(title(app), '2020 – 2029');
    assert.deepEqual(arrows(app).map((arrow) => arrow.disabled), [true, true], 'the decade holds the earliest and the latest year');
    key(app, yearField2(app), 'Escape');
    await click(app, monthField2(app));
    assert.deepEqual(arrows(app).map((arrow) => arrow.disabled), [false, true], 'at 2026 only the previous year is offered');
    await click(app, arrows(app)[0]);
    assert.equal(title(app), '2025');
    assert.deepEqual(arrows(app).map((arrow) => arrow.disabled), [true, false], 'the previous arrow of 2025 is disabled');
    app.close();
  });
});

test('test_csd036_ac_12_3_opening_and_choosing', async () => {
  await withClock(async () => {
    const app = await donutFields();
    await click(app, yearField2(app));
    assert.ok(choiceCells(app).find((cell) => app.text(cell) === '2026').classList.contains('selected'), 'the year of the field is selected');
    await click(app, picker(app).querySelector('[data-choice="2025"]'));
    assert.equal(yearField2(app).value, '2025');
    assert.equal(picker(app), null);
    app.close();
  });
});

test('test_csd036_ac_12_4_the_field_is_read_only', async () => {
  const app = await donutFields();
  assert.equal(yearField2(app).readOnly, true);
  assert.equal(monthField2(app).readOnly, true);
  // The browser refuses typing in a read-only field, so the attribute is what the test can prove.
  assert.equal(monthField2(app).getAttribute('readonly'), '');
  assert.equal(picker(app), null, 'focusing it does not open the picker');
  app.close();
});

// ---- REQ-13: the Today button ----

const todayButton = (app) => picker(app).querySelector('.dp-today');

test('test_csd036_ac_13_1_today_on_the_days_level', async () => {
  await withClock(async () => {
    const app = await jobDialog();
    startDate(app).value = '15/03/2025';
    await click(app, startDate(app));
    assert.equal(title(app), 'March 2025');
    await click(app, todayButton(app));
    assert.equal(title(app), 'November 2026');
    assert.ok(cellOf(app, '2026-11-22').classList.contains('today'));
    assert.equal(app.document.activeElement, cellOf(app, '2026-11-22'), 'the focus moves to today');
    assert.ok(picker(app), 'the picker is still open');
    assert.equal(startDate(app).value, '15/03/2025', 'the field is unchanged');
    app.close();
  });
});

test('test_csd036_ac_13_2_today_on_the_months_and_years_levels', async () => {
  await withClock(async () => {
    const filter = await bankPeriodFilter();
    monthField(filter).value = '2019-05';
    await click(filter, monthField(filter));
    assert.equal(title(filter), '2019');
    await click(filter, todayButton(filter));
    assert.equal(title(filter), '2026');
    assert.equal(monthField(filter).value, '2019-05', 'the field is unchanged');
    await click(filter, picker(filter).querySelector('button.dp-title'));
    await click(filter, arrows(filter)[0]);
    assert.equal(title(filter), '2010 – 2019');
    await click(filter, todayButton(filter));
    assert.equal(title(filter), '2020 – 2029');
    assert.ok(picker(filter), 'still open');
    filter.close();

    const app = await jobDialog();
    const { maskInput } = await app.load('shared/date-mask.js');
    const year = app.document.createElement('input');
    year.dataset.mask = 'month';
    year.maxLength = 4;
    year.value = '2005';
    app.$('job-form').append(year);
    maskInput(year);
    await app.click(year);
    assert.equal(title(app), '2000 – 2009');
    await click(app, todayButton(app));
    assert.equal(title(app), '2020 – 2029');
    assert.equal(year.value, '2005', 'nothing is written');
    app.close();
  });
});

test('test_csd036_ac_13_3_today_in_a_limited_field', async () => {
  await withClock(async () => {
    const app = await openApp({ handlers: { transaction_spending_periods: { years: [2025], months: ['2025-03'] }, transaction_spending_shares: SHARES } });
    await app.click(app.item('transactions'));
    await app.click(app.$('transactions-tab-categories'));
    await click(app, monthField2(app));
    assert.equal(title(app), '2025');
    await click(app, todayButton(app));
    assert.equal(title(app), '2026');
    assert.equal(choiceCells(app).every((cell) => cell.disabled), true, 'all grey');
    assert.ok(picker(app));
    app.close();
  });
});

test('test_csd036_ac_13_4_the_button_is_translated', async () => {
  const reads = exampleReads();
  reads.get_preferences = { ...reads.get_preferences, language: 'nl' };
  const app = await jobDialog({ reads });
  await click(app, startDate(app));
  assert.equal(app.text(todayButton(app)), 'Vandaag');
  app.close();
});

// ---- REQ-12 with only a first period: the label of the Budgets tab (CSD-037 REQ-2) ----

test('test_csd036_ac_12_5_only_a_first_period_and_no_last_one', async () => {
  await withClock(async () => {
    const empty = { month: '2026-10', is_partial: true, earliest_month: '2026-08', rows: [], not_budgeted: '0.00', total: { budget: '0.00', spent: '0.00', remaining: '0.00' } };
    const app = await openApp({ handlers: { get_budgets: empty } });
    await app.click(app.item('transactions'));
    await app.click(app.$('transactions-tab-budgets'));
    await click(app, app.$('budgets-month'));
    assert.equal(title(app), '2026');
    assert.deepEqual(available(app), ['Aug', 'Sep', 'Oct', 'Nov', 'Dec']);
    assert.deepEqual(arrows(app).map((arrow) => arrow.disabled), [true, false], 'the previous arrow of 2026 is disabled, the next one is not');
    await click(app, arrows(app)[1]);
    await click(app, arrows(app)[1]);
    await click(app, arrows(app)[1]);
    await click(app, arrows(app)[1]);
    assert.equal(title(app), '2030');
    assert.equal(available(app).length, 12, 'the months of 2030 can be chosen');
    await click(app, picker(app).querySelector('button.dp-title'));
    assert.equal(title(app), '2030 – 2039');
    assert.equal(choiceCells(app).filter((cell) => cell.disabled).length, 0, 'every year from 2026 on can be chosen');
    await click(app, arrows(app)[0]);
    assert.equal(title(app), '2020 – 2029');
    assert.equal(choiceCells(app).filter((cell) => cell.disabled).length, 6, '2025 and every earlier year are grey');
    app.close();
  });
});
