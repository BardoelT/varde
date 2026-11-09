// CSD-028: filters in the statement tables. Rust applies the filter; these tests check what the
// screens show and send: the icon buttons, the panel, the dot, the notes and the total. The fake
// bridge answers like Rust would for the rows of the example project.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { readFileSync } from 'node:fs';
import { exampleReads } from './fixtures.js';

const SAVED = { has_path: true, unsaved_changes: false, undo_label: null, redo_label: null };

/** Rows the way Rust would return them for a filter on the account and the month. */
function bankAnswer(rows) {
  return (args) => {
    const filter = args?.filter;
    if (!filter) return rows;
    return rows.filter((row) => {
      const month = /^\d{4}-\d{2}$/.test(row.period) ? row.period : null;
      const inRange = month === null || ((!filter.from || month >= filter.from) && (!filter.to || month <= filter.to));
      return inRange && (filter.ids.length === 0 || filter.ids.includes(row.account_id));
    });
  };
}

async function openBank({ reads = exampleReads(), handlers = {} } = {}) {
  const app = await openApp({
    reads,
    handlers: { list_bank_entries: bankAnswer(reads.list_bank_entries), ...handlers },
  });
  await app.click(app.item('bank'));
  return app;
}

const PREFIX = 'bank-entry';
const periodHost = (prefix = PREFIX) => `${prefix}-period-filter`;
const entityHost = (prefix = PREFIX) => `${prefix}-entity-filter`;
const buttonIn = (app, host) => app.$(host).querySelector('.filter-btn');
const periodButton = (app, prefix) => buttonIn(app, periodHost(prefix));
const entityButton = (app, prefix) => buttonIn(app, entityHost(prefix));
const panelIn = (app, host) => app.$(host).querySelector('.filter-panel');
const checkboxes = (app, prefix) => [...panelIn(app, entityHost(prefix)).querySelectorAll('input[type=checkbox]')];
const monthInputs = (app, prefix) => [...panelIn(app, periodHost(prefix)).querySelectorAll('input[data-mask=month]')];
const entryPeriods = (app) => [...app.$('bank-entries-rows').querySelectorAll('tr')].map((row) => app.text(row.querySelector('th')));
const lastFilterSent = (app) => app.callsOf('list_bank_entries').at(-1).args?.filter;
const labelOf = (button) => button.getAttribute('aria-label');
const escape = (app, panel) => panel.dispatchEvent(new app.window.KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));

/** Edit > Undo with the focus out of the filter fields: in a field the menu item undoes the typing (CSD-027 REQ-11). */
async function undo(app) {
  app.document.activeElement?.blur();
  await app.emit('menu-action', 'project-undo');
  assert.equal(app.callsOf('undo').length > 0, true, 'a project step was undone');
}

/** Opens the panel of a column and chooses "Reset filter". */
async function reset(app, host) {
  const button = buttonIn(app, host);
  if (!panelIn(app, host)) await app.click(button);
  await app.click(panelIn(app, host).querySelector('.filter-reset'));
}

async function tick(app, id, prefix) {
  const box = checkboxes(app, prefix).find((b) => b.value === id);
  box.checked = !box.checked;
  box.dispatchEvent(new app.window.Event('change', { bubbles: true }));
  await app.settle();
}

/** Opens the entity panel of the bank entries and ticks `ids`. */
async function filterAccounts(app, ...ids) {
  await app.click(entityButton(app));
  for (const id of ids) await tick(app, id);
}

test('test_csd028_ac_1_1_two_icon_only_filter_buttons_sit_next_to_the_column_titles', async () => {
  const app = await openBank();
  const [period, account] = [...app.$('bank-entries-table').querySelectorAll('thead th')];
  assert.equal(app.text(period), 'Period');
  assert.equal(app.text(account), 'Account');
  assert.ok(period.contains(periodButton(app)), 'the period button is in the Period header');
  assert.ok(account.contains(entityButton(app)), 'the account button is in the Account header');
  assert.equal(labelOf(periodButton(app)), 'Filter by period');
  assert.equal(labelOf(entityButton(app)), 'Filter by account');
  for (const button of [periodButton(app), entityButton(app)]) {
    assert.ok(app.isShown(button));
    assert.equal(button.getAttribute('title'), labelOf(button));
    assert.ok(button.querySelector('svg'), 'an icon');
    assert.equal(button.textContent, '', 'and no text');
  }
  app.close();
});

test('test_csd028_ac_1_2_a_table_without_data_has_no_filter_buttons', async () => {
  const reads = exampleReads();
  reads.list_bank_accounts = {
    accounts: [{ colour_slot: 0, entry_count: 0, id: 'id-1', latest_balance: null, latest_period: null, name: 'Example Current Account', period: 'Monthly' }],
    total: null,
  };
  reads.list_bank_entries = [];
  reads.yearly_bank_totals = [];
  const app = await openBank({ reads });
  assert.equal(app.isShown(app.$(periodHost())), false);
  assert.equal(app.isShown(app.$(entityHost())), false);
  assert.equal(app.text(app.$('bank-entries-empty')), 'No entries yet.');
  assert.ok(app.isShown(app.$('bank-entries-empty')));
  app.close();
});

test('test_csd028_ac_2_1_the_entity_panel_lists_every_account_with_entries', async () => {
  const app = await openBank();
  await app.click(entityButton(app));
  assert.deepEqual(checkboxes(app).map((b) => app.text(b.closest('label'))), ['Example Current Account', 'Example Old Bank', 'Example Savings']);
  assert.equal(panelIn(app, entityHost()).querySelector('input[data-mask]'), null, 'no month fields');
  app.close();
});

test('test_csd028_ac_2_4_the_period_panel_holds_the_range_only', async () => {
  const app = await openBank();
  await app.click(periodButton(app));
  assert.deepEqual(monthInputs(app).map((input) => input.value), ['', '']);
  assert.deepEqual(monthInputs(app).map((input) => app.text(input.closest('label'))), ['From month', 'To month']);
  assert.equal(panelIn(app, periodHost()).querySelector('input[type=checkbox]'), null, 'no checkboxes');
  app.close();
});

test('test_csd028_ac_2_2_changes_apply_live_and_the_panel_closes_with_the_filter_kept', async () => {
  const app = await openBank();
  await app.click(entityButton(app));
  await tick(app, 'id-1');
  assert.deepEqual(lastFilterSent(app), { from: null, to: null, ids: ['id-1'] });
  assert.deepEqual(entryPeriods(app), ['2026-08', '2025-12'], 'the table changed while the panel is open');
  assert.ok(panelIn(app, entityHost()), 'the panel is still open');

  escape(app, panelIn(app, entityHost()));
  assert.equal(panelIn(app, entityHost()), null, 'Escape closes it');
  assert.deepEqual(entryPeriods(app), ['2026-08', '2025-12'], 'the filter is kept');
  app.close();
});

test('test_csd028_ac_2_2_a_click_outside_the_panel_closes_it', async () => {
  const app = await openBank();
  await app.click(entityButton(app));
  assert.ok(panelIn(app, entityHost()));
  await app.click(app.$('bank-entries-rows'));
  assert.equal(panelIn(app, entityHost()), null);
  app.close();
});

test('test_csd028_ac_2_2_opening_one_panel_closes_the_other', async () => {
  const app = await openBank();
  await app.click(entityButton(app));
  await app.click(periodButton(app));
  assert.equal(panelIn(app, entityHost()), null);
  assert.ok(panelIn(app, periodHost()));
  app.close();
});

test('test_csd028_ac_2_5_the_two_filters_combine', async () => {
  const app = await openBank();
  await filterAccounts(app, 'id-1');
  await app.click(periodButton(app));
  await app.type(monthInputs(app)[0], '202601');
  assert.deepEqual(lastFilterSent(app), { from: '2026-01', to: null, ids: ['id-1'] });
  assert.deepEqual(entryPeriods(app), ['2026-08', '2025-12'].filter((p) => p >= '2026-01'));
  app.close();
});

test('test_csd028_ac_3_1_an_incomplete_month_is_not_sent', async () => {
  const app = await openBank();
  await app.click(periodButton(app));
  const calls = app.callsOf('list_bank_entries').length;
  await app.type(monthInputs(app)[0], '20260');
  assert.equal(monthInputs(app)[0].value, '2026-0', 'the dash is typed for the user');
  assert.equal(lastFilterSent(app), undefined, 'a month that is not complete filters nothing');
  assert.equal(labelOf(periodButton(app)), 'Filter by period', 'so the filter icon stays');
  await app.type(monthInputs(app)[0], '8');
  assert.deepEqual(lastFilterSent(app), { from: '2026-08', to: null, ids: [] });
  assert.ok(app.callsOf('list_bank_entries').length > calls);
  app.close();
});

test('test_csd028_ac_5_1_each_table_filters_on_its_own_entity', async () => {
  const app = await openApp();
  const cases = [
    ['income', 'paycheck', 'Job'],
    ['investments', 'statement', 'Account'],
    ['bank', 'bank-entry', 'Account'],
    ['debts', 'debt-statement', 'Debt'],
    ['houses', 'house-valuation', 'House'],
    ['expenses', 'expense', 'Category'],
  ];
  for (const [section, prefix, entity] of cases) {
    await app.click(app.item(section));
    const header = entityButton(app, prefix).closest('th');
    assert.equal(app.text(header), entity, `${section}: the button sits next to the column title ${entity}`);
    assert.equal(labelOf(entityButton(app, prefix)), `Filter by ${entity.toLowerCase()}`);
    await app.click(entityButton(app, prefix));
    assert.ok(checkboxes(app, prefix).length > 0, `${section} lists its ${entity}s`);
    escape(app, panelIn(app, entityHost(prefix)));
    assert.ok(periodButton(app, prefix).closest('th'), `${section} has a period button too`);
  }
  app.close();
});

test('test_csd028_ac_6_1_an_active_entity_filter_shows_a_filled_icon', async () => {
  const app = await openBank();
  await filterAccounts(app, 'id-1');
  assert.equal(labelOf(entityButton(app)), 'Filter by account (active)');
  assert.equal(entityButton(app).getAttribute('title'), 'Filter by account (active)');
  assert.equal(entityButton(app).textContent, '', 'icon only');
  assert.equal(entityButton(app).querySelector('svg').getAttribute('fill'), 'currentColor', 'a filled icon shows the filter is in use');
  assert.equal(periodButton(app).querySelector('svg').getAttribute('fill'), 'none', 'the other one is an outline');
  assert.equal(labelOf(periodButton(app)), 'Filter by period', 'the period column keeps its own state');
  app.close();
});

test('test_csd028_ac_6_2_an_active_period_filter_shows_a_filled_icon', async () => {
  const app = await openBank();
  await app.click(periodButton(app));
  await app.type(monthInputs(app)[0], '202601');
  assert.equal(labelOf(periodButton(app)), 'Filter by period (active)');
  assert.equal(labelOf(entityButton(app)), 'Filter by account');
  app.close();
});

test('test_csd028_ac_6_3_without_a_filter_both_buttons_show_the_outline_icon', async () => {
  const app = await openBank();
  assert.equal(labelOf(periodButton(app)), 'Filter by period');
  assert.equal(labelOf(entityButton(app)), 'Filter by account');
  assert.equal(app.$('bank-entries-table').querySelectorAll('.filter-dot').length, 0);
  app.close();
});

test('test_csd028_ac_7_1_resetting_one_column_keeps_the_other', async () => {
  const app = await openBank();
  await filterAccounts(app, 'id-1');
  await app.click(periodButton(app));
  await app.type(monthInputs(app)[0], '202601');
  await app.click(panelIn(app, periodHost()).querySelector('input')); // the period panel is still open
  await reset(app, entityHost());
  assert.deepEqual(lastFilterSent(app), { from: '2026-01', to: null, ids: [] }, 'only the period is sent');
  assert.equal(labelOf(entityButton(app)), 'Filter by account', 'the outline icon is back');
  assert.equal(labelOf(periodButton(app)), 'Filter by period (active)');
  assert.equal(panelIn(app, entityHost()), null, 'the panel is closed');
  app.close();
});

test('test_csd028_ac_7_2_resetting_the_last_filter_shows_every_row', async () => {
  const app = await openBank();
  await filterAccounts(app, 'id-1');
  assert.equal(entryPeriods(app).length, 2);
  await reset(app, entityHost());
  assert.equal(entryPeriods(app).length, 6);
  assert.equal(lastFilterSent(app), undefined, 'asked for without a filter');
  assert.equal(labelOf(entityButton(app)), 'Filter by account');
  app.close();
});

test('test_csd028_ac_8_1_a_filter_matching_nothing_keeps_the_header_and_has_its_own_note', async () => {
  const app = await openBank();
  await filterAccounts(app, 'id-1');
  await app.click(periodButton(app));
  await app.type(monthInputs(app)[0], '203001');
  assert.ok(app.isShown(app.$('bank-entries-nomatch')));
  assert.equal(app.text(app.$('bank-entries-nomatch')), 'No entries match the filters.');
  assert.equal(app.isShown(app.$('bank-entries-empty')), false, 'not the note of an empty table');
  assert.ok(app.isShown(app.$('bank-entries-table')), 'the header row stays');
  assert.equal(entryPeriods(app).length, 0);
  assert.equal(labelOf(entityButton(app)), 'Filter by account (active)', 'with the filled icons');
  assert.equal(labelOf(periodButton(app)), 'Filter by period (active)');
  await reset(app, entityHost());
  assert.equal(app.isShown(app.$('bank-entries-nomatch')), false);
  app.close();
});

test('test_csd028_ac_9_1_the_total_is_the_filtered_total_while_a_filter_is_on', async () => {
  const reads = exampleReads();
  const expenses = reads.list_expenses;
  const app = await openApp({
    reads,
    handlers: {
      list_expenses: (args) =>
        args?.filter
          ? { expenses: expenses.expenses.filter((e) => args.filter.ids.includes(e.category)), total: '800.00' }
          : expenses,
    },
  });
  await app.click(app.item('expenses'));
  assert.equal(app.textOf('expenses-total-label'), 'Total', 'AC-9.2: the label without a filter');
  assert.equal(app.textOf('expenses-total'), '2,770.50');

  await app.click(entityButton(app, 'expense'));
  await tick(app, 'Home', 'expense');
  assert.deepEqual(app.callsOf('list_expenses').at(-1).args, { filter: { from: null, to: null, ids: ['Home'] } });
  assert.equal(app.textOf('expenses-total-label'), 'Filtered total');
  assert.equal(app.textOf('expenses-total'), '800.00', 'the total Rust returned for the rows');
  app.close();
});

test('test_csd028_ac_11_2_deleting_a_row_keeps_the_filter', async () => {
  const reads = exampleReads();
  let rows = reads.list_bank_entries;
  const app = await openBank({
    reads,
    handlers: {
      list_bank_entries: (args) => bankAnswer(rows)(args),
      delete_bank_entry: () => {
        rows = rows.filter((row) => row.period !== '2026-08');
      },
    },
  });
  await filterAccounts(app, 'id-1');
  assert.deepEqual(entryPeriods(app), ['2026-08', '2025-12']);
  await app.click(app.document.querySelector('[aria-label="Delete Example Current Account 2026-08"]'));
  await app.click(app.$('confirm-ok'));
  assert.equal(app.callsOf('delete_bank_entry').length, 1);
  assert.deepEqual(lastFilterSent(app), { from: null, to: null, ids: ['id-1'] }, 'the table is loaded through the same filter');
  assert.deepEqual(entryPeriods(app), ['2025-12'], 'one row is left');
  assert.equal(labelOf(entityButton(app)), 'Filter by account (active)');
  app.close();
});

test('test_csd028_ac_12_1_a_deleted_account_drops_out_of_the_selection', async () => {
  const reads = exampleReads();
  let accountsAfterDelete = false;
  const app = await openBank({
    reads,
    handlers: {
      list_bank_accounts: () => ({
        accounts: reads.list_bank_accounts.accounts.filter((a) => !(accountsAfterDelete && a.id === 'id-1')),
        total: reads.list_bank_accounts.total,
      }),
      undo: { step: 'Delete Bank Account', status: SAVED },
    },
  });
  await filterAccounts(app, 'id-1');
  await app.click(periodButton(app));
  await app.type(monthInputs(app)[0], '202601');
  accountsAfterDelete = true;
  await undo(app);
  assert.deepEqual(lastFilterSent(app), { from: '2026-01', to: null, ids: [] }, 'the account is gone from the selection, the range stays');
  assert.equal(labelOf(entityButton(app)), 'Filter by account', 'so that column is not filtered any more');
  app.close();
});

test('test_csd028_ac_13_1_the_filter_survives_a_page_switch', async () => {
  const app = await openBank();
  await filterAccounts(app, 'id-2');
  await app.click(app.item('debts'));
  await app.click(app.item('bank'));
  assert.equal(labelOf(entityButton(app)), 'Filter by account (active)');
  app.close();
});

test('test_csd028_ac_13_2_a_new_project_resets_every_filter', async () => {
  const app = await openBank({ handlers: { new_project: SAVED } });
  await filterAccounts(app, 'id-2');
  assert.equal(labelOf(entityButton(app)), 'Filter by account (active)');
  await app.emit('menu-action', 'project-new');
  assert.equal(labelOf(entityButton(app)), 'Filter by account');
  assert.equal(lastFilterSent(app), undefined);
  assert.equal(entryPeriods(app).length, 6);
  app.close();
});

test('test_csd028_ac_13_3_nothing_is_sent_to_be_saved', async () => {
  const app = await openBank();
  await filterAccounts(app, 'id-2');
  const commands = new Set(app.calls.map((call) => call.command));
  for (const name of commands) assert.ok(!/filter/i.test(name), `${name} stores no filter`);
  assert.equal(app.window.localStorage.length, 0, 'and the page stores none in the browser');
  app.close();
});

test('test_csd028_ac_14_2_undo_keeps_the_filter', async () => {
  const app = await openBank({ handlers: { undo: { step: 'Delete Bank Entry', status: SAVED } } });
  await filterAccounts(app, 'id-1');
  const loads = app.callsOf('list_bank_entries').length;
  await undo(app);
  assert.ok(app.callsOf('list_bank_entries').length > loads, 'the table was loaded again');
  assert.deepEqual(lastFilterSent(app), { from: null, to: null, ids: ['id-1'] });
  assert.deepEqual(entryPeriods(app), ['2026-08', '2025-12']);
  app.close();
});

test('test_csd028_ac_15_1_the_texts_in_dutch', async () => {
  const reads = exampleReads();
  reads.get_preferences = { theme: 'system', language: 'nl' };
  const app = await openBank({ reads });
  assert.equal(labelOf(periodButton(app)), 'Filteren op periode');
  assert.equal(labelOf(entityButton(app)), 'Filteren op rekening');
  await app.click(periodButton(app));
  assert.deepEqual(monthInputs(app).map((input) => app.text(input.closest('label'))), ['Vanaf maand', 'Tot en met maand']);
  await app.click(entityButton(app));
  await tick(app, 'id-1');
  assert.equal(labelOf(entityButton(app)), 'Filteren op rekening (actief)');
  await app.click(periodButton(app));
  await app.type(monthInputs(app)[0], '203001');
  assert.equal(labelOf(periodButton(app)), 'Filteren op periode (actief)');
  assert.equal(app.text(app.$('bank-entries-nomatch')), 'Geen boekingen voldoen aan de filters.');
  app.close();
});

test('test_csd028_ac_15_2_the_buttons_work_with_the_keyboard', async () => {
  const app = await openBank();
  await app.press(periodButton(app), 'Enter');
  assert.ok(panelIn(app, periodHost()), 'Enter opens the period panel');
  assert.equal(app.document.activeElement, monthInputs(app)[0], 'the first control has the focus');
  escape(app, panelIn(app, periodHost()));
  assert.equal(app.document.activeElement, periodButton(app), 'Escape returns the focus to the button');
  assert.equal(periodButton(app).tagName, 'BUTTON');
  app.close();
});

test('test_csd028_ac_2_6_a_filter_in_use_can_be_adjusted', async () => {
  const app = await openBank();
  await filterAccounts(app, 'id-1');
  escape(app, panelIn(app, entityHost()));
  assert.equal(panelIn(app, entityHost()), null);

  await app.click(entityButton(app));
  assert.ok(panelIn(app, entityHost()), 'the icon opens the panel again');
  assert.deepEqual(checkboxes(app).map((box) => box.checked), [true, false, false], 'with the choice as it was');
  await tick(app, 'id-2');
  assert.deepEqual(lastFilterSent(app), { from: null, to: null, ids: ['id-1', 'id-2'] });
  app.close();
});

test('test_csd028_ac_2_7_the_panel_holds_a_reset_button_that_is_off_without_a_filter', async () => {
  const app = await openBank();
  await app.click(entityButton(app));
  const resetButton = panelIn(app, entityHost()).querySelector('.filter-reset');
  assert.equal(app.text(resetButton), 'Reset filter');
  assert.equal(resetButton.disabled, true, 'nothing to reset yet');
  await tick(app, 'id-1');
  assert.equal(resetButton.disabled, false, 'on once the filter is');
  await app.click(resetButton);
  assert.equal(panelIn(app, entityHost()), null, 'the panel closes');
  assert.equal(entryPeriods(app).length, 6);
  assert.equal(app.document.activeElement, entityButton(app), 'the focus goes back to the icon');
  app.close();
});

test('test_csd028_ac_2_8_clicking_the_text_of_an_item_works_like_clicking_its_checkbox', async () => {
  const app = await openBank();
  await app.click(entityButton(app));
  const text = panelIn(app, entityHost()).querySelector('.filter-check span');
  await app.click(text);
  assert.ok(panelIn(app, entityHost()), 'the panel stays open');
  assert.deepEqual(checkboxes(app).map((box) => box.checked), [true, false, false], 'and only that item is ticked');
  assert.deepEqual(lastFilterSent(app), { from: null, to: null, ids: ['id-1'] });

  // Focusing a checkbox through its text can scroll the table; that must not close the panel.
  app.$('bank-entries-table').closest('.table-wrap').dispatchEvent(new app.window.Event('scroll'));
  app.window.dispatchEvent(new app.window.Event('scroll'));
  assert.ok(panelIn(app, entityHost()), 'a scroll does not close it');
  await app.click(text);
  assert.deepEqual(checkboxes(app).map((box) => box.checked), [false, false, false], 'a second click unticks it');
  app.close();
});

test('test_csd028_ac_2_9_the_panel_texts_use_the_case_of_the_items_not_of_the_header', () => {
  // The panel sits in a header cell that is styled in capitals; jsdom loads no CSS, so the rule is read.
  const css = readFileSync(new URL('../styles/components.css', import.meta.url), 'utf8');
  const rule = css.match(/\.filter-panel \{[^}]*\}/)?.[0] ?? '';
  assert.match(rule, /text-transform:\s*none/);
  assert.match(rule, /letter-spacing:\s*normal/);
  assert.match(rule, /color:\s*var\(--text\)/);
});

/** Fakes the layout jsdom does not have: the window height, the icon's box and the panel's own height. */
function fakeLayout(app, { windowHeight, icon, panelHeight }) {
  Object.defineProperty(app.window, 'innerHeight', { configurable: true, value: windowHeight });
  Object.defineProperty(app.window, 'innerWidth', { configurable: true, value: 1000 });
  app.$(entityHost()).getBoundingClientRect = () => ({ left: 400, right: 420, ...icon });
  Object.defineProperty(app.window.HTMLElement.prototype, 'offsetHeight', { configurable: true, get: () => panelHeight });
}

test('test_csd028_ac_2_10_the_panel_stays_inside_the_window', async () => {
  const cases = [
    ['below the icon when it fits', { windowHeight: 600, icon: { top: 100, bottom: 120 }, panelHeight: 200 }, { top: '124px', maxHeight: '' }],
    ['above the icon when there is more room there', { windowHeight: 400, icon: { top: 300, bottom: 320 }, panelHeight: 200 }, { top: '96px', maxHeight: '288px' }],
    ['scrolling inside when neither side fits', { windowHeight: 300, icon: { top: 100, bottom: 120 }, panelHeight: 500 }, { top: '124px', maxHeight: '168px' }],
  ];
  for (const [name, layout, expected] of cases) {
    const app = await openBank();
    fakeLayout(app, layout);
    await app.click(entityButton(app));
    const panel = panelIn(app, entityHost());
    assert.equal(panel.style.top, expected.top, name);
    assert.equal(panel.style.maxHeight, expected.maxHeight, name);
    app.close();
  }
});

test('test_csd028_ac_2_10_the_panel_is_placed_again_when_the_window_changes', async () => {
  const app = await openBank();
  fakeLayout(app, { windowHeight: 600, icon: { top: 100, bottom: 120 }, panelHeight: 200 });
  await app.click(entityButton(app));
  assert.equal(panelIn(app, entityHost()).style.top, '124px');
  fakeLayout(app, { windowHeight: 400, icon: { top: 300, bottom: 320 }, panelHeight: 200 });
  app.window.dispatchEvent(new app.window.Event('resize'));
  assert.equal(panelIn(app, entityHost()).style.top, '96px', 'it moved above the icon');
  app.close();
});
