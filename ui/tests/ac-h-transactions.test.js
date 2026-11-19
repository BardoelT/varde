// DOM tests of CSD-032 (the list, import, filters, totals, manual badge and the page itself). Rust computes every figure; the tests feed canned answers and check only what is displayed.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { BANK, SAVINGS, GROCERIES, SHOPPING, transaction, readsWith, transactionsPage, rowCells, tableRows, bulkRows, bulkHandlers } from './transactions-helpers.js';

test('test_csd032_ac_16_1_transactions_is_in_the_left_pane_after_expenses_and_shows_the_transactions_tab', async () => {
  const app = await openApp();
  const items = [...app.document.querySelectorAll('.nav-item')].map((item) => app.text(item));
  assert.ok(items.indexOf('Transactions') === items.indexOf('Expenses') + 1, 'after Expenses');
  await app.click(app.item('transactions'));
  assert.ok(app.isShown(app.$('transactions')));
  assert.equal(app.textOf('transactions-tab-transactions'), 'Transactions');
  assert.equal(app.$('transactions-tab-transactions').getAttribute('aria-selected'), 'true', 'the page opens on the Transactions tab');
  assert.deepEqual([...app.document.querySelectorAll('#transactions [role="tab"]')].map((tab) => app.text(tab)), ['Accounts', 'Categories', 'Rules', 'Yearly totals', 'Transactions', 'Insights']);
  app.close();
});

test('test_csd032_ac_15_1_the_rows_are_shown_as_rust_returned_them_and_the_list_is_cut_by_a_limit', async () => {
  const rows = [
    transaction('t1', '2026-10-30', '-23.97', 'Example Market Town', GROCERIES),
    transaction('t2', '2026-10-28', '2500.00', 'Example Employer'),
  ];
  const app = await transactionsPage(readsWith({ rows, count: 450, total: '1172.44' }));
  const shown = tableRows(app).map((row) => rowCells(app, row));
  assert.equal(shown.length, 2);
  assert.equal(shown[0][0], '30/10/2026');
  assert.equal(shown[0][1], 'Example Bank', 'the account directly after the date (CSD-034 REQ-6)');
  assert.equal(shown[0][2], 'Example Market Town');
  assert.ok(shown[0].includes('-23.97'));
  assert.equal(app.callsOf('list_transactions')[0].args.limit, 200, 'the first answer holds 200 rows');
  assert.equal(app.textOf('transactions-shown'), 'Showing 2 of 450');
  assert.ok(app.isShown(app.$('transactions-more')));
  app.close();
});

test('test_csd032_ac_15_1_load_more_asks_for_200_more_and_goes_when_all_are_shown', async () => {
  const all = Array.from({ length: 450 }, (_, n) => transaction(`t${n}`, '2026-10-01', '-1.00', `Example ${n}`));
  let limit = 0;
  const app = await transactionsPage(readsWith({ rows: all.slice(0, 200), count: 450 }), {
    list_transactions: (args) => {
      limit = args?.limit ?? 0;
      return { rows: all.slice(0, limit), count: 450, total: '-450.00' };
    },
  });
  await app.click(app.$('transactions-load-more'));
  assert.equal(limit, 400);
  assert.equal(tableRows(app).length, 400);
  await app.click(app.$('transactions-load-more'));
  assert.equal(limit, 600);
  assert.equal(tableRows(app).length, 450);
  assert.equal(app.isShown(app.$('transactions-more')), false, 'Load more is gone');
  app.close();
});

test('test_csd032_ac_15_2_the_filtered_total_is_the_one_rust_returned', async () => {
  const rows = [transaction('t1', '2026-10-30', '-1.00', 'Example Market Town', GROCERIES)];
  const app = await transactionsPage(readsWith({ rows, count: 450, total: '-450.00' }), {
    list_transactions: (args) => ({ rows, count: args?.filter ? 450 : 450, total: args?.filter ? '-450.00' : '-900.00' }),
  });
  assert.equal(app.textOf('transactions-total-label'), 'Total');
  assert.equal(app.textOf('transactions-total'), '-900.00');
  await app.click(app.$('transaction-entity-filter').querySelector('button'));
  const box = [...app.document.querySelectorAll('.filter-check input')].find((input) => input.value === GROCERIES);
  box.checked = true;
  box.dispatchEvent(new app.window.Event('change', { bubbles: true }));
  await app.settle();
  assert.equal(app.textOf('transactions-total-label'), 'Filtered total');
  assert.equal(app.textOf('transactions-total'), '-450.00', 'not the sum of the loaded rows');
  app.close();
});

test('test_csd034_ac_6_3_an_empty_project_points_to_the_accounts_tab_and_has_no_import_button', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }));
  assert.equal(app.textOf('transactions-empty'), 'No transactions yet. Add an account on the Accounts tab and import a file into it.');
  assert.ok(app.isShown(app.$('transactions-empty')));
  assert.equal(app.isShown(app.$('transactions-table')), false);
  // The buttons moved to the Accounts tab (CSD-034 REQ-6).
  assert.equal(app.$('transaction-import'), null);
  assert.equal(app.$('transaction-delete-all'), null);
  app.close();
});

test('test_csd034_ac_6_1_the_rows_show_the_account_directly_after_the_date_and_the_filter_sends_account_ids', async () => {
  const rows = [
    transaction('t1', '2026-10-30', '-1.00', 'Example Shop', null, BANK),
    transaction('t2', '2026-10-29', '-2.00', 'Example Cafe', null, SAVINGS),
  ];
  const app = await transactionsPage(readsWith({ rows }), {
    list_transactions: (args) => {
      const kept = args?.filter?.account_ids?.length ? rows.filter((row) => args.filter.account_ids.includes(row.account_id)) : rows;
      return { rows: kept, count: kept.length, total: '0.00' };
    },
  });
  const headers = [...app.$('transactions-table').tHead.rows[0].cells].map((cell) => app.text(cell));
  assert.deepEqual(headers.slice(0, 3), ['Date', 'Account', 'Counterparty']);
  assert.deepEqual(tableRows(app).map((row) => rowCells(app, row)[1]), ['Example Bank', 'Example Savings']);
  // The filter has one checkbox per account.
  await app.click(app.$('transaction-account-filter').querySelector('button'));
  const boxes = [...app.$('transaction-account-filter').querySelectorAll('.filter-check input')];
  assert.deepEqual(boxes.map((box) => box.value), [BANK, SAVINGS]);
  boxes[1].checked = true;
  boxes[1].dispatchEvent(new app.window.Event('change', { bubbles: true }));
  await app.settle();
  assert.deepEqual(app.callsOf('list_transactions').at(-1).args.filter.account_ids, [SAVINGS]);
  assert.deepEqual(tableRows(app).map((row) => rowCells(app, row)[2]), ['Example Cafe']);
  assert.match(app.$('transaction-account-filter').querySelector('.filter-btn').getAttribute('aria-label'), /active/);
  app.close();
});

test('test_csd032_ac_15_4_changing_a_category_keeps_the_filter', async () => {
  let rows = [transaction('t1', '2026-10-30', '-1.00', 'Example Market Town', GROCERIES)];
  const app = await transactionsPage(readsWith({ rows }), {
    list_transactions: (args) => {
      const kept = args?.filter?.ids?.length ? rows.filter((row) => args.filter.ids.includes(row.category_id)) : rows;
      return { rows: kept, count: kept.length, total: '0.00' };
    },
    set_transaction_category: ({ categoryId }) => {
      rows = rows.map((row) => ({ ...row, category_id: categoryId, manual: true }));
      return null;
    },
  });
  await app.click(app.$('transaction-entity-filter').querySelector('button'));
  const box = [...app.document.querySelectorAll('.filter-check input')].find((input) => input.value === GROCERIES);
  box.checked = true;
  box.dispatchEvent(new app.window.Event('change', { bubbles: true }));
  await app.settle();
  assert.equal(tableRows(app).length, 1);
  await app.choose(app.document.querySelector('[aria-label="Category of Example Market Town"]'), SHOPPING);
  assert.deepEqual(app.callsOf('set_transaction_category').at(-1).args, { transactionId: 't1', categoryId: SHOPPING });
  assert.ok(app.$('transaction-entity-filter').querySelector('.filter-btn'), 'the filter is still there');
  assert.equal(tableRows(app).length, 0, 'the row no longer matches');
  assert.ok(app.isShown(app.$('transactions-nomatch')));
  app.close();
});

test('test_csd032_ac_11_2_the_category_select_offers_uncategorized_and_sends_null_for_it', async () => {
  const rows = [transaction('t1', '2026-10-30', '-1.00', 'Example Market Town', GROCERIES)];
  const app = await transactionsPage(readsWith({ rows }), { set_transaction_category: null });
  const select = app.document.querySelector('[aria-label="Category of Example Market Town"]');
  assert.deepEqual([...select.options].map((option) => app.text(option)), ['Uncategorized', 'Groceries', 'Shopping']);
  assert.equal(select.value, GROCERIES);
  await app.choose(select, '');
  assert.deepEqual(app.callsOf('set_transaction_category').at(-1).args, { transactionId: 't1', categoryId: null });
  app.close();
});

test('test_csd032_ac_14_2_a_cancelled_deletion_changes_nothing', async () => {
  const rows = [transaction('t1', '2026-10-30', '-1.00', 'Example Shop')];
  const app = await transactionsPage(readsWith({ rows }));
  await app.click(app.document.querySelector('[aria-label="Delete Example Shop"]'));
  assert.equal(app.$('confirm-dialog').open, true);
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.callsOf('delete_transaction').length, 0);
  assert.equal(tableRows(app).length, 1);
  app.close();
});

test('test_csd032_ac_14_1_a_confirmed_deletion_sends_the_id', async () => {
  const rows = [transaction('t1', '2026-10-30', '-1.00', 'Example Shop')];
  const app = await transactionsPage(readsWith({ rows }), { delete_transaction: null });
  await app.click(app.document.querySelector('[aria-label="Delete Example Shop"]'));
  await app.click(app.$('confirm-ok'));
  assert.deepEqual(app.callsOf('delete_transaction').at(-1).args, { transactionId: 't1' });
  app.close();
});

test('test_csd032_ac_31_2_the_categories_tab_shows_the_categorized_share', async () => {
  const reads = readsWith({ rows: [], count: 0 });
  reads.get_transactions_overview.summary = { total: 40, categorized: 35, percent_label: '87.50%' };
  const app = await transactionsPage(reads);
  const note = app.$('transaction-categorisation');
  assert.equal(app.text(note), '87.50% of the transactions are categorized (35 of 40)');
  assert.equal(app.isShown(note), false, 'not on the Transactions tab');
  await app.click(app.$('transactions-tab-categories'));
  assert.ok(app.isShown(note));
  assert.equal(note.closest('.panel').querySelector('.panel-title').textContent, 'Categories', 'in the header of the Categories panel');
  assert.ok(note.closest('.panel-header'));
  assert.equal(app.$('transactions-totals').contains(note), false, 'not on the Yearly totals tab');
  assert.equal(app.$('transactions-rules').contains(note), false, 'not on the Rules tab');
  app.close();
  const empty = readsWith({ rows: [], count: 0 });
  empty.get_transactions_overview.summary = { total: 0, categorized: 0, percent_label: null };
  const none = await transactionsPage(empty);
  assert.ok(none.$('transaction-categorisation').hidden, 'no line without transactions');
  none.close();
});

test('test_csd032_ac_20_1_discrete_mode_hides_the_amounts_and_not_the_names', async () => {
  const rows = [transaction('t1', '2026-10-30', '-23.97', 'Example Shop')];
  const reads = readsWith({ rows, total: '-23.97' });
  reads.get_preferences = { theme: 'system', language: 'en', number_format: 'point_comma', discrete_mode: true };
  const app = await transactionsPage(reads);
  const cells = rowCells(app, tableRows(app)[0]);
  assert.ok(cells.includes('•••••'), 'the amount is hidden');
  assert.ok(!cells.some((cell) => cell.includes('23.97')));
  assert.ok(cells.includes('Example Shop'), 'the name stays');
  assert.equal(app.textOf('transactions-total'), '•••••');
  app.close();
});

test('test_csd032_ac_21_1_dutch_labels_and_stored_names_unchanged', async () => {
  const rows = [transaction('t1', '2026-10-30', '-23.97', 'Example Shop', GROCERIES)];
  const reads = readsWith({ rows });
  reads.get_preferences = { theme: 'system', language: 'nl', number_format: 'point_comma', discrete_mode: false };
  const app = await transactionsPage(reads);
  assert.equal(app.text(app.item('transactions')), 'Transacties');
  assert.equal(app.textOf('transactions-tab-accounts'), 'Rekeningen');
  const select = app.document.querySelector('[aria-label="Categorie van Example Shop"]');
  assert.deepEqual([...select.options].map((option) => app.text(option)), ['Zonder categorie', 'Groceries', 'Shopping']);
  app.close();
});

test('test_csd032_ac_21_2_an_error_code_is_shown_in_dutch', async () => {
  const reads = readsWith({ rows: [], count: 0 });
  reads.get_preferences = { theme: 'system', language: 'nl', number_format: 'point_comma', discrete_mode: false };
  const app = await transactionsPage(reads, {
    import_transactions: () => {
      throw { code: 'TRANSACTION_IMPORT_CURRENCY', message: 'Line 3: the currency is not the currency of the project.' };
    },
  });
  await app.click(app.$('transactions-tab-accounts'));
  await app.click(app.document.querySelector('#transaction-accounts [aria-label^="Transacties importeren in"]'));
  assert.equal(app.textOf('toast'), 'De valuta is niet de valuta van het project.');
  app.close();
});

test('test_csd032_ac_18_1_a_change_reloads_the_settings_for_the_currency_confirmation', async () => {
  const rows = [transaction('t1', '2026-10-30', '-1.00', 'Example Shop')];
  const app = await transactionsPage(readsWith({ rows }), { delete_transaction: null });
  const before = app.callsOf('get_settings').length;
  await app.click(app.document.querySelector('[aria-label="Delete Example Shop"]'));
  await app.click(app.$('confirm-ok'));
  assert.ok(app.callsOf('get_settings').length > before);
  app.close();
});

test('test_csd032_ac_16_3_categories_and_rules_are_on_their_own_tabs', async () => {
  const app = await transactionsPage(readsWith({ rows: [transaction('t1', '2026-10-30', '-1.00', 'Example Shop')] }));
  const own = ['transaction-categories', 'transaction-rules', 'transaction-apply-rules', 'transaction-apply-rules-all'];
  const overview = ['transactions-table'];
  assert.equal(app.$('transactions-tab-transactions').getAttribute('aria-selected'), 'true');
  for (const id of overview) assert.ok(app.isShown(app.$(id)), `${id} is on the Overview`);
  for (const id of own) assert.equal(app.isShown(app.$(id)), false, `${id} is not on the Overview`);
  await app.click(app.$('transactions-tab-categories'));
  assert.ok(app.isShown(app.$('transaction-categories')), 'the categories are on their tab');
  for (const id of ['transaction-rules', 'transaction-apply-rules', 'transaction-apply-rules-all']) assert.equal(app.isShown(app.$(id)), false, `${id} is not on the Categories tab`);
  for (const id of overview) assert.equal(app.isShown(app.$(id)), false, `${id} is not on the other tab`);
  await app.click(app.$('transactions-tab-rules'));
  for (const id of ['transaction-rules', 'transaction-apply-rules', 'transaction-apply-rules-all']) assert.ok(app.isShown(app.$(id)), `${id} is on the Rules tab`);
  assert.equal(app.isShown(app.$('transaction-categories')), false, 'the categories are not on the Rules tab');
  app.close();
});

test('test_csd032_ac_26_2_manual_only_asks_rust_for_the_hand_set_transactions', async () => {
  const manual = { ...transaction('t1', '2026-10-30', '-1.00', 'Example Shop'), manual: true };
  const app = await transactionsPage(readsWith({ rows: [manual, transaction('t2', '2026-10-29', '-2.00', 'Example Cafe')], count: 2 }), {
    list_transactions: (args) => (args?.manualOnly ? { rows: [manual], count: 1, total: '-1.00' } : { rows: [manual, transaction('t2', '2026-10-29', '-2.00', 'Example Cafe')], count: 2, total: '-3.00' }),
  });
  assert.equal(app.$('transaction-manual-only').getAttribute('aria-pressed'), 'false');
  assert.equal(tableRows(app).length, 2);
  await app.click(app.$('transaction-manual-only'));
  assert.equal(app.callsOf('list_transactions').at(-1).args.manualOnly, true);
  assert.equal(app.$('transaction-manual-only').getAttribute('aria-pressed'), 'true', 'the button looks pressed while it is on');
  assert.equal(tableRows(app).length, 1);
  assert.equal(app.textOf('transactions-total-label'), 'Filtered total');
  await app.click(app.$('transaction-manual-only'));
  assert.equal(app.callsOf('list_transactions').at(-1).args?.manualOnly, undefined);
  assert.equal(app.$('transaction-manual-only').getAttribute('aria-pressed'), 'false');
  assert.equal(tableRows(app).length, 2);
  app.close();
});

test('test_csd032_ac_26_3_no_manual_transactions_is_no_matches_not_no_data', async () => {
  const rows = [transaction('t2', '2026-10-29', '-2.00', 'Example Cafe')];
  const app = await transactionsPage(readsWith({ rows, count: 1 }), {
    list_transactions: (args) => (args?.manualOnly ? { rows: [], count: 0, total: '0.00' } : { rows, count: 1, total: '-2.00' }),
  });
  await app.click(app.$('transaction-manual-only'));
  assert.equal(app.isShown(app.$('transactions-nomatch')), true);
  assert.equal(app.isShown(app.$('transactions-empty')), false);
  assert.equal(app.isShown(app.$('transactions-table')), true, 'the header stays, so the button stays reachable');
  app.close();
});

test('test_csd032_ac_15_7_the_period_filter_of_the_list_asks_for_days', async () => {
  const app = await transactionsPage(readsWith({ rows: bulkRows(), count: 3 }), bulkHandlers());
  await app.click(app.document.querySelector('#transaction-period-filter button'));
  const fields = [...app.document.querySelectorAll('#transaction-period-filter input[data-mask]')];
  assert.deepEqual(fields.map((input) => app.text(input.closest('label'))), ['From date', 'To date']);
  assert.deepEqual(fields.map((input) => input.getAttribute('placeholder')), ['dd/mm/yyyy', 'dd/mm/yyyy']);
  await app.typeFilter(fields[0], '29102026');
  assert.equal(fields[0].value, '29/10/2026', 'typed digits are laid out as a day');
  assert.deepEqual(app.callsOf('list_transactions').at(-1).args.filter, { from: '2026-10-29', to: null, ids: [], account_ids: [], min_amount: null, max_amount: null, counterparty: null, description: null }, 'Rust gets the ISO day');
  await app.typeFilter(fields[1], '31022026');
  assert.deepEqual(app.callsOf('list_transactions').at(-1).args.filter.to, null, 'a day that is not on the calendar is not sent');
  app.close();
});

test('test_csd032_ac_15_8_the_amount_filter_sends_the_typed_range', async () => {
  const app = await transactionsPage(readsWith({ rows: bulkRows(), count: 3 }), bulkHandlers());
  await app.click(app.document.querySelector('#transaction-amount-filter button'));
  const panel = app.document.querySelector('#transaction-amount-filter .filter-panel');
  const fields = [...panel.querySelectorAll('input')];
  assert.deepEqual(fields.map((input) => app.text(input.closest('label'))), ['From amount', 'To amount']);
  await app.typeFilter(fields[0], '-50,5');
  assert.deepEqual(app.callsOf('list_transactions').at(-1).args.filter, { from: null, to: null, ids: [], account_ids: [], min_amount: '-50,5', max_amount: null, counterparty: null, description: null }, 'the typed text goes to Rust');
  await app.typeFilter(fields[1], 'abc');
  assert.equal(app.callsOf('list_transactions').at(-1).args.filter.max_amount, null, 'text without a digit is not sent');
  assert.match(app.document.querySelector('#transaction-amount-filter button').getAttribute('aria-label'), /active/);
  app.close();
});

test('test_csd032_ac_15_10_typing_in_a_filter_asks_once_after_a_pause_and_a_late_answer_is_dropped', async () => {
  let release = null;
  const slow = new Promise((resolve) => { release = resolve; });
  const fresh = { rows: [transaction('t1', '2026-10-30', '-1.00', 'Albert Heijn')], count: 1, total: '-1.00' };
  const stale = { rows: [transaction('t2', '2026-10-29', '-2.00', 'Example Cafe')], count: 1, total: '-2.00' };
  const app = await transactionsPage(readsWith({ rows: bulkRows(), count: 3 }), {
    ...bulkHandlers(),
    list_transactions: async (args) => {
      if (args?.filter?.counterparty === 'Al') {
        await slow;
        return stale;
      }
      return args?.filter?.counterparty === 'Alb' ? fresh : { rows: bulkRows(), count: 3, total: '0.00' };
    },
  });
  await app.click(app.document.querySelector('#transaction-counterparty-filter button'));
  const field = app.document.querySelector('#transaction-counterparty-filter .filter-panel input');
  const before = app.callsOf('list_transactions').length;
  await app.type(field, 'Al'); // quick typing: nothing is asked yet
  assert.equal(app.callsOf('list_transactions').length, before, 'no request while typing');
  await new Promise((resolve) => setTimeout(resolve, 260));
  assert.equal(app.callsOf('list_transactions').length, before + 1, 'one request after the pause');
  // A newer request while the first is still out (`type` would wait for it, so the input event is sent by hand).
  field.value += 'b';
  field.dispatchEvent(new app.window.InputEvent('input', { inputType: 'insertText', data: 'b', bubbles: true }));
  await new Promise((resolve) => setTimeout(resolve, 260));
  release();
  await app.settle();
  assert.deepEqual(tableRows(app).map((row) => app.text(row.querySelectorAll('td.wrap')[1])), ['Albert Heijn'], 'the late answer of the first request is dropped');
  app.close();
});

test('test_csd032_ac_15_9_the_text_filters_send_the_typed_text', async () => {
  const app = await transactionsPage(readsWith({ rows: bulkRows(), count: 3 }), bulkHandlers());
  await app.click(app.document.querySelector('#transaction-counterparty-filter button'));
  const panel = app.document.querySelector('#transaction-counterparty-filter .filter-panel');
  const field = panel.querySelector('input');
  assert.equal(app.text(field.closest('label')), 'Counterparty (* is any text)');
  await app.typeFilter(field, ' Albert ');
  assert.equal(app.callsOf('list_transactions').at(-1).args.filter.counterparty, 'Albert', 'the text is sent without surrounding spaces');
  assert.equal(app.callsOf('list_transactions').at(-1).args.filter.description, null);
  assert.match(app.document.querySelector('#transaction-counterparty-filter button').getAttribute('aria-label'), /active/);
  await app.click(app.document.querySelector('#transaction-description-filter button'));
  const second = app.document.querySelector('#transaction-description-filter .filter-panel input');
  assert.equal(app.text(second.closest('label')), 'Description (* is any text)');
  await app.typeFilter(second, 'rent');
  assert.deepEqual(
    [app.callsOf('list_transactions').at(-1).args.filter.counterparty, app.callsOf('list_transactions').at(-1).args.filter.description],
    ['Albert', 'rent'],
  );
  await app.click(app.document.querySelector('#transaction-description-filter .filter-reset'));
  assert.equal(app.callsOf('list_transactions').at(-1).args.filter.description, null, 'reset clears only that column');
  assert.equal(app.callsOf('list_transactions').at(-1).args.filter.counterparty, 'Albert');
  app.close();
});

test('test_csd032_ac_16_4_a_reapply_message_is_a_yellow_notification', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }), { reapply_transaction_rules: { changed: 2 } });
  await app.click(app.$('transactions-tab-rules'));
  await app.click(app.$('transaction-apply-rules'));
  assert.equal(app.textOf('toast'), '2 changed');
  assert.ok(app.isShown(app.$('toast')));
  assert.ok(app.$('toast').classList.contains('toast-notice'));
  app.close();
});

test('test_csd032_ac_11_4_manual_transactions_carry_a_badge', async () => {
  const manual = { ...transaction('t1', '2026-10-30', '-1.00', 'Example Shop', SHOPPING), manual: true };
  const byRule = transaction('t2', '2026-10-29', '-1.00', 'Example Cafe', GROCERIES);
  const app = await transactionsPage(readsWith({ rows: [manual, byRule] }));
  const [first, second] = tableRows(app);
  assert.equal(first.querySelector('.manual-badge') !== null, true);
  assert.equal(app.text(first.querySelector('.manual-badge')), 'Manual');
  assert.equal(first.querySelector('.manual-badge').tagName, 'BUTTON', 'a button, to unset it');
  assert.match(first.querySelector('.manual-badge').getAttribute('title'), /leaves it as it is/);
  assert.equal(second.querySelector('.manual-badge'), null);
  app.close();
});

test('test_csd032_ac_24_3_a_filter_does_not_ask_for_other_totals', async () => {
  const rows = [transaction('t1', '2026-10-30', '-1.00', 'Example Shop', GROCERIES)];
  const app = await transactionsPage(readsWith({ rows }));
  const before = app.callsOf('get_transactions_overview').length;
  await app.click(app.$('transaction-entity-filter').querySelector('button'));
  const box = [...app.document.querySelectorAll('.filter-check input')].find((input) => input.value === GROCERIES);
  box.checked = true;
  box.dispatchEvent(new app.window.Event('change', { bubbles: true }));
  await app.settle();
  assert.equal(app.callsOf('get_transactions_overview').length, before, 'the totals are not asked for again');
  assert.ok(app.callsOf('get_transactions_overview').every((call) => call.args === undefined || !call.args.filter));
  app.close();
});

test('test_csd032_ac_24_4_a_change_reloads_the_totals', async () => {
  const rows = [transaction('t1', '2026-10-30', '-1.00', 'Example Shop')];
  const app = await transactionsPage(readsWith({ rows }), { delete_transaction: null });
  const before = app.callsOf('get_transactions_overview').length;
  await app.click(app.document.querySelector('[aria-label="Delete Example Shop"]'));
  await app.click(app.$('confirm-ok'));
  assert.ok(app.callsOf('get_transactions_overview').length > before);
  app.close();
});

test('test_csd032_ac_11_5_choosing_the_badge_asks_and_unsets_manual_keeping_the_category', async () => {
  const manual = { ...transaction('t1', '2026-10-30', '-1.00', 'Example Shop', SHOPPING), manual: true };
  const app = await transactionsPage(readsWith({ rows: [manual] }), { unset_transaction_manual: null });
  await app.click(app.$('transactions-rows').querySelector('.manual-badge'));
  assert.equal(app.$('confirm-dialog').open, true);
  assert.equal(app.textOf('confirm-title'), 'Unset manual?');
  assert.match(app.textOf('confirm-message'), /The category Shopping stays/);
  assert.equal(app.callsOf('unset_transaction_manual').length, 0, 'nothing yet');
  await app.click(app.$('confirm-ok'));
  assert.deepEqual(app.callsOf('unset_transaction_manual').at(-1).args, { transactionId: 't1' });
  app.close();
});

test('test_csd032_ac_11_6_cancelling_the_popup_changes_nothing', async () => {
  const manual = { ...transaction('t1', '2026-10-30', '-1.00', 'Example Shop', SHOPPING), manual: true };
  const app = await transactionsPage(readsWith({ rows: [manual] }), { unset_transaction_manual: null });
  await app.click(app.$('transactions-rows').querySelector('.manual-badge'));
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.callsOf('unset_transaction_manual').length, 0);
  app.close();
});

test('test_csd032_ac_11_7_a_stale_screen_shows_the_error_and_reloads', async () => {
  const manual = { ...transaction('t1', '2026-10-30', '-1.00', 'Example Shop', SHOPPING), manual: true };
  const app = await transactionsPage(readsWith({ rows: [manual] }), {
    unset_transaction_manual: () => {
      throw { code: 'TRANSACTION_NOT_FOUND', message: 'This transaction does not exist.' };
    },
  });
  const before = app.callsOf('list_transactions').length;
  await app.click(app.$('transactions-rows').querySelector('.manual-badge'));
  await app.click(app.$('confirm-ok'));
  assert.equal(app.textOf('toast'), 'This transaction does not exist.');
  assert.ok(app.callsOf('list_transactions').length > before, 'the list is loaded again');
  app.close();
});

test('test_csd032_ac_11_5_the_unset_popup_in_dutch', async () => {
  const manual = { ...transaction('t1', '2026-10-30', '-1.00', 'Example Shop', SHOPPING), manual: true };
  const reads = readsWith({ rows: [manual] });
  reads.get_preferences = { theme: 'system', language: 'nl', number_format: 'point_comma', discrete_mode: false };
  const app = await transactionsPage(reads);
  assert.equal(app.text(app.$('transactions-rows').querySelector('.manual-badge')), 'Handmatig');
  await app.click(app.$('transactions-rows').querySelector('.manual-badge'));
  assert.equal(app.textOf('confirm-title'), 'Handmatig opheffen?');
  assert.match(app.textOf('confirm-message'), /De categorie Shopping blijft/);
  app.close();
});
