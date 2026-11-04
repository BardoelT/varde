// Display ACs of CSD-009 (Bank accounts): sections, rows, forms, deletion confirmations and the
// currency confirmation. The screens are fed canned answers from the golden file.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { emptyReads, exampleReads } from './fixtures.js';

const CURRENT = 'id-1';
const SAVINGS = 'id-2';
const OLD_BANK = 'id-8';

const entry = (accountId, accountName, period, balance, note = '') => ({ account_id: accountId, account_name: accountName, balance, note, period });

async function openBank(options) {
  const app = await openApp(options);
  await app.click(app.item('bank'));
  return app;
}

const panelOf = (app, title) =>
  [...app.document.querySelectorAll('#bank-overview .panel')].find((panel) => app.text(panel.querySelector('.panel-title')) === title);
const panelTitles = (app) => [...app.document.querySelectorAll('#bank-overview .panel-title')].map((el) => app.text(el));

/** The example project with the entries of "Example Current Account" replaced by `periods`. */
function readsWithCurrentEntries(periods) {
  const reads = exampleReads();
  const others = reads.list_bank_entries.filter((e) => e.account_id !== CURRENT);
  reads.list_bank_entries = [...periods.map((period) => entry(CURRENT, 'Example Current Account', period, '900.00')), ...others];
  return reads;
}

/** A project with one account without entries. */
function readsWithEmptyAccount() {
  const reads = exampleReads();
  reads.list_bank_accounts = {
    accounts: [{ colour_slot: 0, entry_count: 0, id: CURRENT, latest_balance: null, latest_period: null, name: 'Example Current Account', period: 'Monthly' }],
    total: null,
  };
  reads.list_bank_entries = [];
  reads.yearly_bank_totals = [];
  return reads;
}

test('test_csd009_ac_20_1_sections_in_order_and_an_item_in_the_left_pane', async () => {
  const app = await openBank();
  assert.notEqual(app.item('bank'), null, 'Bank accounts is an item in the left pane');
  assert.equal(app.text(app.item('bank')), 'Bank accounts');
  assert.deepEqual(panelTitles(app), ['Accounts', 'Yearly totals', 'Entries']);
  app.close();
});

test('test_csd009_ac_20_2_account_rows_carry_their_period_latest_balance_and_actions', async () => {
  const reads = exampleReads();
  reads.list_bank_accounts = {
    accounts: [
      { colour_slot: 0, entry_count: 2, id: CURRENT, latest_balance: '2500.00', latest_period: '2026-09', name: 'Example Current Account', period: 'Monthly' },
      { colour_slot: 1, entry_count: 1, id: SAVINGS, latest_balance: '8000.00', latest_period: '2025', name: 'Example Savings', period: 'Yearly' },
    ],
    total: '10500.00',
  };
  const app = await openBank({ reads });
  const [current, savings] = [...app.document.querySelectorAll('#bank-account-list .job-row')];
  assert.equal(app.text(current.querySelector('.job-meta')), 'Monthly');
  assert.equal(app.text(current.querySelector('.account-amount')), '2,500.002026-09', 'the latest balance with its period');
  assert.equal(app.text(current.querySelector('.account-amount small')), '2026-09');
  assert.equal(app.text(savings.querySelector('.job-meta')), 'Yearly');
  assert.equal(app.text(savings.querySelector('.account-amount')), '8,000.002025');
  assert.equal(app.text(savings.querySelector('.account-amount small')), '2025');
  assert.equal(app.text(current), 'Example Current AccountMonthly2,500.002026-09', 'no row shows a number of entries');
  assert.equal(app.text(savings), 'Example SavingsYearly8,000.002025');
  for (const row of [current, savings]) {
    const buttons = [...row.querySelector('.job-actions').children];
    assert.equal(buttons.length, 2);
    assert.match(buttons[0].getAttribute('aria-label'), /^Edit /);
    assert.match(buttons[1].getAttribute('aria-label'), /^Delete /);
    assert.equal(buttons[0].title, buttons[0].getAttribute('aria-label'), 'also in a tooltip');
    assert.equal(row.lastElementChild, row.querySelector('.job-actions'), 'at the right edge of the row');
  }
  assert.ok(savings.querySelector('[aria-label="Edit Example Savings"]'));
  assert.equal(app.textOf('bank-account-total'), 'Total of latest balances 10,500.00');
  app.close();
});

test('test_csd009_ac_20_3_entries_of_all_accounts_with_their_period_and_account', async () => {
  const reads = exampleReads();
  reads.list_bank_entries = [
    entry(CURRENT, 'Example Current Account', '2026-09', '2500.00'),
    entry(SAVINGS, 'Example Savings', '2025', '8000.00'),
  ];
  const app = await openBank({ reads });
  const headers = [...app.document.querySelectorAll('#bank-entries-table thead th')].map((th) => app.text(th));
  assert.deepEqual(headers.slice(0, 4), ['Period', 'Account', 'Balance', 'Note'], 'in that order');
  const rows = [...app.document.querySelectorAll('#bank-entries-rows tr')];
  assert.equal(rows.length, 2);
  assert.deepEqual([...rows[0].children].slice(0, 2).map((cell) => app.text(cell)), ['2026-09', 'Example Current Account']);
  assert.deepEqual([...rows[1].children].slice(0, 2).map((cell) => app.text(cell)), ['2025', 'Example Savings']);
  app.close();
});

test('test_csd009_ac_20_4_a_long_note_is_cut_to_one_line_and_editing_shows_all_of_it', async () => {
  const reads = exampleReads();
  const note = 'first line\nsecond line\nthird line';
  reads.list_bank_entries = [entry(CURRENT, 'Example Current Account', '2026-08', '2500.50', note)];
  const app = await openBank({ reads });
  assert.equal(app.text(app.document.querySelector('#bank-entries-rows .note-cell')), 'first line …');
  await app.click(app.document.querySelector('[aria-label="Edit Example Current Account 2026-08"]'));
  assert.equal(app.$('bank-entry-form').elements.note.value, note, 'editing shows the whole note');
  app.close();
});

test('test_csd009_ac_20_5_clicking_an_account_row_changes_nothing', async () => {
  const app = await openBank();
  const snapshot = () => [app.textOf('bank-totals-rows'), app.textOf('bank-entries-rows')];
  const before = snapshot();
  const callsBefore = app.calls.length;
  for (const row of app.document.querySelectorAll('#bank-account-list .job-row')) await app.click(row);
  assert.deepEqual(snapshot(), before, 'the totals and the entries are unchanged');
  assert.equal(app.calls.length, callsBefore);
  app.close();
});

test('test_csd009_ac_20_6_a_note_is_shown_as_plain_text', async () => {
  const reads = exampleReads();
  reads.list_bank_entries = [entry(CURRENT, 'Example Current Account', '2026-08', '2500.50', '<b>bold</b>')];
  const app = await openBank({ reads });
  const cell = app.document.querySelector('#bank-entries-rows .note-cell');
  assert.equal(cell.textContent, '<b>bold</b>');
  assert.equal(cell.querySelector('b'), null, 'no formatting');
  app.close();
});

test('test_csd009_ac_20_7_the_add_buttons_sit_in_the_section_headers', async () => {
  const app = await openBank();
  assert.equal(panelOf(app, 'Accounts').querySelector('.panel-header').lastElementChild, app.$('bank-account-add'));
  assert.equal(app.text(app.$('bank-account-add')), 'Add account');
  assert.equal(panelOf(app, 'Entries').querySelector('.panel-header').lastElementChild, app.$('bank-entry-add'));
  assert.equal(app.text(app.$('bank-entry-add')), 'Add entry');
  assert.equal(app.document.querySelectorAll('#bank > .section-header button').length, 0, 'the page title row holds no button');
  app.close();
});

test('test_csd009_ac_20_8_the_yearly_totals_section_has_two_columns_and_no_buttons', async () => {
  const app = await openBank();
  const headers = [...app.document.querySelectorAll('#bank-totals-table thead th')].map((th) => app.text(th));
  assert.deepEqual(headers, ['Year', 'Closing balance'], 'no column with a number of entries');
  const section = panelOf(app, 'Yearly totals');
  assert.equal(section.querySelector('.panel-header button'), null);
  assert.equal(section.querySelectorAll('tbody button').length, 0);
  assert.equal(app.document.querySelectorAll('#bank-totals-rows tr').length, 5);
  app.close();
});

test('test_csd009_ac_20_8_the_yearly_totals_section_is_not_shown_without_accounts', async () => {
  const app = await openBank({ reads: emptyReads() });
  assert.equal(app.isShown(app.$('bank-totals-table')), false);
  assert.equal(app.isShown(app.$('bank-content')), false);
  assert.equal(app.isShown(app.$('bank-empty')), true);
  app.close();
});

test('test_csd009_ac_20_9_a_negative_balance_is_shown_with_its_sign', async () => {
  const reads = exampleReads();
  reads.list_bank_entries = [entry(CURRENT, 'Example Current Account', '2026-09', '-150.25')];
  const app = await openBank({ reads });
  const cells = [...app.document.querySelectorAll('#bank-entries-rows tr')[0].children];
  assert.equal(app.text(cells[2]), '-150.25', 'the Balance column');
  app.close();
});

test('test_csd009_ac_21_2_an_account_without_entries_shows_a_dash', async () => {
  const app = await openBank({ reads: readsWithEmptyAccount() });
  const row = app.document.querySelector('#bank-account-list .job-row');
  assert.equal(app.text(row.querySelector('.account-amount')), '—');
  assert.equal(app.text(row.querySelector('.job-meta')), 'Monthly');
  assert.ok(!app.text(row).includes('No entries yet'));
  assert.equal(app.isShown(app.$('bank-entries-empty')), true);
  assert.equal(app.textOf('bank-entries-empty'), 'No entries yet.');
  assert.equal(app.isShown(app.$('toast')), false, 'no error is shown');
  assert.deepEqual(app.errors, []);
  app.close();
});

test('test_csd009_ac_21_3_no_entries_in_any_account_shows_the_totals_note', async () => {
  const app = await openBank({ reads: readsWithEmptyAccount() });
  assert.equal(app.isShown(app.$('bank-totals-empty')), true);
  assert.equal(app.textOf('bank-totals-empty'), 'No totals yet. Totals appear once there is an entry.');
  assert.equal(app.isShown(app.$('bank-totals-table')), false, 'no table');
  assert.equal(app.isShown(app.$('toast')), false, 'no error');
  app.close();
});

test('test_csd009_ac_22_1_no_currency_in_the_sections', async () => {
  const app = await openBank();
  assert.equal(app.$('setting-currency').value, 'EUR', 'the project is in EUR');
  assert.deepEqual(panelTitles(app), ['Accounts', 'Yearly totals', 'Entries']);
  const shown = app.text(app.$('bank-overview'));
  for (const currency of ['EUR', 'USD', 'GBP', '€', '$', '£']) assert.ok(!shown.includes(currency), `no ${currency} on the page`);
  app.close();
});

test('test_csd009_ac_6_2_a_cancelled_account_deletion_changes_nothing', async () => {
  const reads = exampleReads();
  reads.list_bank_accounts.accounts.find((account) => account.id === SAVINGS).entry_count = 3;
  const app = await openBank({ reads });
  const savingsRows = () => [...app.document.querySelectorAll('#bank-entries-rows tr')].filter((row) => app.text(row).includes('Example Savings')).length;
  assert.equal(savingsRows(), 3);
  await app.click(app.document.querySelector('[aria-label="Delete Example Savings"]'));
  assert.equal(app.$('confirm-dialog').open, true);
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.$('confirm-dialog').open, false);
  assert.equal(app.callsOf('delete_bank_account').length, 0);
  assert.ok(app.document.querySelector('[aria-label="Delete Example Savings"]'), 'the account still exists');
  assert.equal(savingsRows(), 3, 'its 3 entries still exist');
  app.close();
});

for (const count of [0, 3]) {
  test(`test_csd009_ac_6_3_the_deletion_confirmation_states_no_count_with_${count}_entries`, async () => {
    const reads = exampleReads();
    reads.list_bank_accounts.accounts[0].entry_count = count;
    const app = await openBank({ reads });
    await app.click(app.document.querySelector('[aria-label="Delete Example Current Account"]'));
    assert.equal(app.textOf('confirm-message'), 'This account and all its entries will be deleted.');
    assert.ok(!/\d/.test(app.textOf('confirm-message')), 'it shows no number');
    assert.ok(!/\d/.test(app.textOf('confirm-title')));
    app.close();
  });
}

test('test_csd009_ac_14_2_a_cancelled_entry_deletion_changes_nothing', async () => {
  const app = await openBank({ reads: readsWithCurrentEntries(['2026-09', '2026-08']) });
  const periodsOfCurrent = () =>
    [...app.document.querySelectorAll('#bank-entries-rows tr')].filter((row) => app.text(row).includes('Example Current Account')).map((row) => app.text(row.querySelector('th')));
  assert.deepEqual(periodsOfCurrent(), ['2026-09', '2026-08']);
  await app.click(app.document.querySelector('[aria-label="Delete Example Current Account 2026-09"]'));
  assert.equal(app.$('confirm-dialog').open, true);
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.callsOf('delete_bank_entry').length, 0);
  assert.deepEqual(periodsOfCurrent(), ['2026-09', '2026-08'], 'both entries still exist');
  app.close();
});

test('test_csd009_ac_16_1_a_monthly_account_asks_for_a_month', async () => {
  const app = await openBank();
  await app.click(app.$('bank-entry-add'));
  const form = app.$('bank-entry-form');
  assert.equal(form.elements.account_id.value, CURRENT, 'the monthly account is chosen');
  assert.equal(app.textOf('bank-entry-period-label'), 'Month');
  assert.equal(form.elements.period.placeholder, 'yyyy-mm');
  assert.equal(form.elements.period.maxLength, 7);
  assert.equal(app.textOf('bank-entry-copy'), 'Copy previous month');
  app.close();
});

test('test_csd009_ac_16_2_a_yearly_account_asks_for_a_year', async () => {
  const app = await openBank();
  await app.click(app.$('bank-entry-add'));
  const form = app.$('bank-entry-form');
  await app.choose(form.elements.account_id, SAVINGS);
  assert.equal(app.textOf('bank-entry-period-label'), 'Year');
  assert.equal(form.elements.period.maxLength, 4);
  assert.match(form.elements.period.value, /^\d{4}$/);
  assert.equal(app.textOf('bank-entry-copy'), 'Copy previous year');
  app.close();
});

test('test_csd009_ac_16_3_switching_account_switches_the_form', async () => {
  const app = await openBank();
  await app.click(app.$('bank-entry-add'));
  const form = app.$('bank-entry-form');
  await app.choose(form.elements.account_id, SAVINGS);
  assert.equal(app.textOf('bank-entry-period-label'), 'Year');
  await app.choose(form.elements.account_id, CURRENT);
  assert.equal(app.textOf('bank-entry-period-label'), 'Month');
  assert.match(form.elements.period.value, /^\d{4}-\d{2}$/);
  assert.equal(app.textOf('bank-entry-copy'), 'Copy previous month');
  app.close();
});

test('test_csd009_ac_12_7_the_edit_form_offers_the_month_and_the_account_cannot_change', async () => {
  const app = await openBank();
  await app.click(app.document.querySelector('[aria-label="Edit Example Current Account 2026-08"]'));
  const form = app.$('bank-entry-form');
  assert.equal(app.textOf('bank-entry-period-label'), 'Month');
  assert.equal(form.elements.period.value, '2026-08');
  assert.equal(form.elements.period.disabled, false, 'the period can be changed');
  assert.equal(form.elements.account_id.value, CURRENT);
  assert.equal(form.elements.account_id.disabled, true, 'the account cannot be changed');
  app.close();
});

test('test_csd009_ac_12_7_the_edit_form_offers_the_year_and_the_account_cannot_change', async () => {
  const app = await openBank();
  await app.click(app.document.querySelector('[aria-label="Edit Example Savings 2025"]'));
  const form = app.$('bank-entry-form');
  assert.equal(app.textOf('bank-entry-period-label'), 'Year');
  assert.equal(form.elements.period.value, '2025');
  assert.equal(form.elements.period.disabled, false);
  assert.equal(form.elements.account_id.value, SAVINGS);
  assert.equal(form.elements.account_id.disabled, true);
  app.close();
});

// ---- REQ-24: the currency confirmation ----

test('test_csd009_ac_24_1_a_bank_entry_makes_the_currency_change_ask_for_confirmation', async () => {
  const reads = exampleReads();
  reads.get_settings = { currency: 'EUR', entry_count: 1 }; // no jobs, no investments, one bank entry
  const app = await openApp({ reads, handlers: { set_currency: ({ currency }) => ({ currency, entry_count: 1 }) } });
  await app.click(app.item('settings'));
  await app.choose(app.$('setting-currency'), 'USD');
  assert.equal(app.$('confirm-dialog').open, true, 'the app asks for confirmation before changing it');
  assert.equal(app.callsOf('set_currency').length, 0);
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.callsOf('set_currency').length, 0);
  assert.equal(app.$('setting-currency').value, 'EUR', 'cancelling leaves the currency EUR');
  app.close();
});

test('test_csd009_ac_24_2_a_bank_account_without_entries_needs_no_confirmation', async () => {
  const reads = exampleReads();
  reads.get_settings = { currency: 'EUR', entry_count: 0 };
  const app = await openApp({ reads, handlers: { set_currency: ({ currency }) => ({ currency, entry_count: 0 }) } });
  await app.click(app.item('settings'));
  await app.choose(app.$('setting-currency'), 'USD');
  assert.equal(app.$('confirm-dialog').open, false, 'no confirmation');
  assert.deepEqual(app.callsOf('set_currency').map((call) => call.args.currency), ['USD']);
  assert.equal(app.$('setting-currency').value, 'USD');
  app.close();
});
