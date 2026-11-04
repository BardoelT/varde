// Display ACs of CSD-005 (Investments): sections, rows, forms, deletion confirmations, the note
// about an account's first statement. The screens are fed canned answers from the golden file.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { emptyReads, exampleReads } from './fixtures.js';

const BROKERAGE = 'id-16';
const PENSION = 'id-17';

const statement = (accountId, accountName, period, balance, note = '') => ({
  account_id: accountId,
  account_name: accountName,
  balance,
  capital_growth: '0.00',
  net_deposits: '0.00',
  note,
  period,
});

/** The example project with the statements of the Brokerage replaced by `periods`. */
function readsWithBrokerageStatements(periods) {
  const reads = exampleReads();
  const pension = reads.list_investment_statements.filter((s) => s.account_id === PENSION);
  reads.list_investment_statements = [
    ...periods.map((period) => statement(BROKERAGE, 'Example Brokerage', period, '1000.00')),
    ...pension,
  ];
  return reads;
}

async function openInvestments(options) {
  const app = await openApp(options);
  await app.click(app.item('investments'));
  return app;
}

const panelOf = (app, title) =>
  [...app.document.querySelectorAll('#investments-overview .panel')].find((panel) => app.text(panel.querySelector('.panel-title')) === title);

const panelTitles = (app) => [...app.document.querySelectorAll('#investments-overview .panel-title')].map((el) => app.text(el));

test('test_csd005_ac_17_1_sections_in_order_and_an_item_in_the_left_pane', async () => {
  const app = await openInvestments();
  assert.notEqual(app.item('investments'), null, 'Investments is an item in the left pane');
  assert.deepEqual(panelTitles(app), ['Accounts', 'Yearly totals', 'Statements']);
  assert.equal(app.isShown(app.$('investments-content')), true);
  app.close();
});

test('test_csd005_ac_17_4_a_long_note_is_cut_to_one_line_and_editing_shows_all_of_it', async () => {
  const reads = exampleReads();
  const note = 'first line\nsecond line\nthird line';
  reads.list_investment_statements = [statement(BROKERAGE, 'Example Brokerage', '2026-08', '12000.00', note)];
  const app = await openInvestments({ reads });
  const cell = app.document.querySelector('#statements-rows .note-cell');
  assert.equal(app.text(cell), 'first line …', 'one line of the note');
  assert.ok(!app.text(cell).includes('second line'));

  await app.click(app.document.querySelector('[aria-label="Edit Example Brokerage 2026-08"]'));
  assert.equal(app.$('statement-form').elements.note.value, note, 'editing shows the whole note');
  app.close();
});

test('test_csd005_ac_17_5_clicking_an_account_row_changes_nothing', async () => {
  const app = await openInvestments();
  const snapshot = () => [app.textOf('investment-totals-rows'), app.textOf('statements-rows')];
  const before = snapshot();
  const callsBefore = app.calls.length;
  for (const row of app.document.querySelectorAll('#account-list .job-row')) await app.click(row);
  assert.deepEqual(snapshot(), before, 'the totals and the statements are unchanged');
  assert.equal(app.calls.length, callsBefore, 'nothing is asked of Rust');
  app.close();
});

test('test_csd005_ac_17_6_a_note_is_shown_as_plain_text', async () => {
  const reads = exampleReads();
  reads.list_investment_statements = [statement(BROKERAGE, 'Example Brokerage', '2026-08', '12000.00', '<b>bold</b>')];
  const app = await openInvestments({ reads });
  const cell = app.document.querySelector('#statements-rows .note-cell');
  assert.equal(cell.textContent, '<b>bold</b>');
  assert.equal(cell.querySelector('b'), null, 'no formatting');
  app.close();
});

test('test_csd005_ac_17_7_the_add_buttons_sit_in_the_section_headers', async () => {
  const app = await openInvestments();
  const accountsHeader = panelOf(app, 'Accounts').querySelector('.panel-header');
  const statementsHeader = panelOf(app, 'Statements').querySelector('.panel-header');
  assert.equal(accountsHeader.lastElementChild, app.$('account-add'), 'at the right edge of the Accounts header');
  assert.equal(app.text(app.$('account-add')), 'Add account');
  assert.equal(statementsHeader.lastElementChild, app.$('statement-add'), 'at the right edge of the Statements header');
  assert.equal(app.text(app.$('statement-add')), 'Add statement');
  assert.equal(app.document.querySelectorAll('#investments > .section-header button').length, 0, 'the page title row holds no button');
  app.close();
});

test('test_csd005_ac_17_9_the_yearly_totals_section_has_four_columns_and_no_buttons', async () => {
  const app = await openInvestments();
  const headers = [...app.document.querySelectorAll('#investment-totals-table thead th')].map((th) => app.text(th));
  assert.deepEqual(headers, ['Year', 'Closing balance', 'Net deposits', 'Capital growth'], 'no column with a number of statements');
  const section = panelOf(app, 'Yearly totals');
  assert.equal(section.querySelector('.panel-header button'), null, 'its header holds no button');
  assert.equal(section.querySelectorAll('tbody button').length, 0, 'its rows hold no Edit or Delete button');
  assert.equal(app.document.querySelectorAll('#investment-totals-rows tr').length, 3);
  app.close();
});

test('test_csd005_ac_17_9_the_yearly_totals_section_is_not_shown_without_accounts', async () => {
  const app = await openInvestments({ reads: emptyReads() });
  assert.equal(app.isShown(app.$('investment-totals-table')), false);
  assert.equal(app.isShown(app.$('investments-content')), false, 'the whole content, with the Yearly totals section, is hidden');
  assert.equal(app.isShown(app.$('investments-empty')), true);
  app.close();
});

test('test_csd005_ac_18_2_an_account_without_statements_shows_a_dash', async () => {
  const reads = exampleReads();
  reads.list_investment_accounts = {
    accounts: [{ id: BROKERAGE, latest_balance: null, latest_period: null, name: 'Example Brokerage', period: 'Monthly', statement_count: 0 }],
    total: '0.00',
  };
  reads.list_investment_statements = [];
  reads.yearly_investment_totals = [];
  const app = await openInvestments({ reads });
  const row = app.document.querySelector('#account-list .job-row');
  assert.equal(app.text(row.querySelector('.account-amount')), '—', 'a dash as latest balance');
  assert.equal(app.text(row.querySelector('.job-meta')), 'Monthly', 'only its period as meta');
  assert.ok(!app.text(row).includes('No statements yet'));
  assert.equal(app.isShown(app.$('statements-empty')), true);
  assert.equal(app.textOf('statements-empty'), 'No statements yet.');
  assert.equal(app.isShown(app.$('toast')), false, 'no error is shown');
  assert.deepEqual(app.errors, []);
  app.close();
});

test('test_csd005_ac_18_3_no_statements_in_any_account_shows_the_totals_note', async () => {
  const reads = exampleReads();
  reads.list_investment_accounts = {
    accounts: [{ id: BROKERAGE, latest_balance: null, latest_period: null, name: 'Example Brokerage', period: 'Monthly', statement_count: 0 }],
    total: '0.00',
  };
  reads.list_investment_statements = [];
  reads.yearly_investment_totals = [];
  const app = await openInvestments({ reads });
  assert.equal(app.isShown(app.$('investment-totals-empty')), true);
  assert.equal(app.textOf('investment-totals-empty'), 'No totals yet. Totals appear once there is a statement.');
  assert.equal(app.isShown(app.$('investment-totals-table')), false, 'no table');
  assert.equal(app.isShown(app.$('toast')), false, 'no error');
  app.close();
});

test('test_csd005_ac_20_1_no_currency_in_the_sections', async () => {
  const app = await openInvestments();
  assert.equal(app.$('setting-currency').value, 'EUR', 'the project is in EUR');
  assert.deepEqual(panelTitles(app), ['Accounts', 'Yearly totals', 'Statements']);
  const shown = app.text(app.$('investments-overview'));
  for (const currency of ['EUR', 'USD', 'GBP', '€', '$', '£']) assert.ok(!shown.includes(currency), `no ${currency} on the page`);
  app.close();
});

test('test_csd005_ac_20_2_changing_the_currency_changes_no_title', async () => {
  const handlers = { set_currency: ({ currency }) => ({ currency, entry_count: 34 }) };
  const app = await openApp({ handlers });
  await app.click(app.item('investments'));
  const balances = () => [app.textOf('account-list'), app.textOf('statements-rows')];
  const before = balances();

  await app.click(app.item('settings'));
  await app.choose(app.$('setting-currency'), 'USD');
  assert.equal(app.$('confirm-dialog').open, true, 'the change is confirmed first');
  await app.click(app.$('confirm-ok'));
  assert.deepEqual(app.callsOf('set_currency').map((call) => call.args.currency), ['USD']);

  await app.click(app.item('investments'));
  assert.deepEqual(panelTitles(app), ['Accounts', 'Yearly totals', 'Statements']);
  assert.deepEqual(balances(), before, 'the balances are unchanged');
  app.close();
});

test('test_csd005_ac_5_2_a_cancelled_account_deletion_changes_nothing', async () => {
  const app = await openInvestments();
  const brokerageRows = () => [...app.document.querySelectorAll('#statements-rows tr')].filter((row) => app.text(row).includes('Example Brokerage')).length;
  assert.equal(brokerageRows(), 3);
  await app.click(app.document.querySelector('[aria-label="Delete Example Brokerage"]'));
  assert.equal(app.$('confirm-dialog').open, true);
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.$('confirm-dialog').open, false);
  assert.equal(app.callsOf('delete_investment_account').length, 0, 'nothing is deleted');
  assert.ok(app.document.querySelector('[aria-label="Delete Example Brokerage"]'), 'the account still exists');
  assert.equal(brokerageRows(), 3, 'its 3 statements still exist');
  app.close();
});

for (const count of [0, 3]) {
  test(`test_csd005_ac_5_3_the_deletion_confirmation_states_no_count_with_${count}_statements`, async () => {
    const reads = exampleReads();
    reads.list_investment_accounts.accounts[0].statement_count = count;
    const app = await openInvestments({ reads });
    await app.click(app.document.querySelector('[aria-label="Delete Example Brokerage"]'));
    assert.equal(app.textOf('confirm-message'), 'This account and all its statements will be deleted.');
    assert.ok(!/\d/.test(app.textOf('confirm-message')), 'it shows no number');
    assert.ok(!/\d/.test(app.textOf('confirm-title')), 'not in the title either');
    app.close();
  });
}

test('test_csd005_ac_12_2_a_cancelled_statement_deletion_changes_nothing', async () => {
  const reads = readsWithBrokerageStatements(['2026-09', '2026-08']);
  const app = await openInvestments({ reads });
  const periods = () => [...app.document.querySelectorAll('#statements-rows tr th')].map((th) => app.text(th));
  assert.deepEqual(periods().slice(0, 2), ['2026-09', '2026-08']);
  await app.click(app.document.querySelector('[aria-label="Delete Example Brokerage 2026-09"]'));
  assert.equal(app.$('confirm-dialog').open, true);
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.callsOf('delete_investment_statement').length, 0);
  assert.deepEqual(periods().slice(0, 2), ['2026-09', '2026-08'], 'both statements still exist');
  app.close();
});

test('test_csd005_ac_14_1_a_monthly_account_asks_for_a_month', async () => {
  const app = await openInvestments();
  await app.click(app.$('statement-add'));
  const form = app.$('statement-form');
  assert.equal(form.elements.account_id.value, BROKERAGE, 'the monthly account is chosen');
  assert.equal(app.textOf('statement-period-label'), 'Month');
  assert.equal(form.elements.period.placeholder, 'yyyy-mm');
  assert.equal(form.elements.period.maxLength, 7);
  assert.equal(app.textOf('statement-copy'), 'Copy previous month');
  app.close();
});

test('test_csd005_ac_14_2_a_yearly_account_asks_for_a_year', async () => {
  const app = await openInvestments();
  await app.click(app.$('statement-add'));
  const form = app.$('statement-form');
  await app.choose(form.elements.account_id, PENSION);
  assert.equal(app.textOf('statement-period-label'), 'Year');
  assert.equal(form.elements.period.maxLength, 4);
  assert.match(form.elements.period.value, /^\d{4}$/);
  assert.equal(app.textOf('statement-copy'), 'Copy previous year');
  app.close();
});

test('test_csd005_ac_14_3_switching_account_switches_the_form', async () => {
  const app = await openInvestments();
  await app.click(app.$('statement-add'));
  const form = app.$('statement-form');
  await app.choose(form.elements.account_id, PENSION);
  assert.equal(app.textOf('statement-period-label'), 'Year', 'a year after the yearly account');
  await app.choose(form.elements.account_id, BROKERAGE);
  assert.equal(app.textOf('statement-period-label'), 'Month', 'a month again after the monthly account');
  assert.match(form.elements.period.value, /^\d{4}-\d{2}$/);
  assert.equal(app.textOf('statement-copy'), 'Copy previous month');
  app.close();
});

test('test_csd005_ac_28_6_the_edit_form_offers_the_month_and_the_account_cannot_change', async () => {
  const app = await openInvestments();
  await app.click(app.document.querySelector('[aria-label="Edit Example Brokerage 2026-08"]'));
  const form = app.$('statement-form');
  assert.equal(app.textOf('statement-period-label'), 'Month');
  assert.equal(form.elements.period.value, '2026-08');
  assert.equal(form.elements.period.disabled, false, 'the period can be changed');
  assert.equal(form.elements.account_id.value, BROKERAGE);
  assert.equal(form.elements.account_id.disabled, true, 'the account cannot be changed');
  app.close();
});

test('test_csd005_ac_28_6_the_edit_form_offers_the_year_and_the_account_cannot_change', async () => {
  const app = await openInvestments();
  await app.click(app.document.querySelector('[aria-label="Edit Example Pension 2025"]'));
  const form = app.$('statement-form');
  assert.equal(app.textOf('statement-period-label'), 'Year');
  assert.equal(form.elements.period.value, '2025');
  assert.equal(form.elements.period.disabled, false);
  assert.equal(form.elements.account_id.value, PENSION);
  assert.equal(form.elements.account_id.disabled, true);
  app.close();
});

const calculation = { calculate_investment_statement: () => ({ balance: '0.00', capital_growth: '0.00' }) };

test('test_csd005_ac_24_6_without_a_period_no_capital_growth_is_calculated', async () => {
  const reads = readsWithBrokerageStatements(['2026-08']);
  reads.list_investment_statements[0].balance = '12000.00';
  const app = await openInvestments({ reads, handlers: calculation });
  await app.click(app.$('statement-add'));
  const form = app.$('statement-form');
  form.elements.period.value = '';
  form.elements.period.dispatchEvent(new app.window.InputEvent('input', { bubbles: true }));
  await app.type(form.elements.balance, '12500.00');
  assert.equal(app.callsOf('calculate_investment_statement').length, 0, 'Rust is not asked to calculate');
  assert.equal(form.elements.growth.value, '', 'no capital growth is calculated');
  assert.equal(app.isShown(app.$('growth-calculated')), false);
  app.close();
});

test('test_csd005_ac_27_1_an_account_without_statements_shows_the_start_note', async () => {
  const reads = readsWithBrokerageStatements([]);
  const app = await openInvestments({ reads, handlers: calculation });
  await app.click(app.$('statement-add'));
  assert.equal(app.$('statement-form').elements.account_id.value, BROKERAGE);
  assert.equal(app.isShown(app.$('statement-growth-hint')), true);
  assert.match(app.textOf('statement-growth-hint'), /^No earlier statement, so the account is taken to start at 0\.00\./);
  app.close();
});

test('test_csd005_ac_27_2_a_period_before_all_statements_shows_the_start_note', async () => {
  const reads = readsWithBrokerageStatements(['2026-09']);
  const app = await openInvestments({ reads, handlers: calculation });
  await app.click(app.$('statement-add'));
  const form = app.$('statement-form');
  form.elements.period.value = '';
  await app.type(form.elements.period, '202607');
  assert.equal(form.elements.period.value, '2026-07');
  assert.equal(app.isShown(app.$('statement-growth-hint')), true);
  app.close();
});

test('test_csd005_ac_27_3_an_earlier_statement_hides_the_start_note', async () => {
  const reads = readsWithBrokerageStatements(['2026-08']);
  const app = await openInvestments({ reads, handlers: calculation });
  await app.click(app.$('statement-add'));
  const form = app.$('statement-form');
  form.elements.period.value = '';
  await app.type(form.elements.period, '202609');
  assert.equal(form.elements.period.value, '2026-09');
  assert.equal(app.isShown(app.$('statement-growth-hint')), false);
  app.close();
});

test('test_csd005_ac_27_4_editing_the_first_statement_shows_the_start_note', async () => {
  const reads = readsWithBrokerageStatements(['2026-09', '2026-08']);
  const app = await openInvestments({ reads, handlers: calculation });
  await app.click(app.document.querySelector('[aria-label="Edit Example Brokerage 2026-08"]'));
  assert.equal(app.$('statement-form').elements.period.value, '2026-08');
  assert.equal(app.isShown(app.$('statement-growth-hint')), true);
  app.close();
});
