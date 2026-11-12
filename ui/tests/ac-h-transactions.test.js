// Display ACs of CSD-032 (transactions): the page, the rows, the paging, the filtered total, the
// import message, the categories and rules forms, Discrete mode and the Dutch labels. Rust parses,
// categorises, filters and adds up; the fake bridge answers with the golden example project or a
// small list made for one AC.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { exampleReads } from './fixtures.js';

const GROCERIES = 'cat-groceries';
const SHOPPING = 'cat-shopping';
const CATEGORIES = [
  { id: GROCERIES, name: 'Groceries' },
  { id: SHOPPING, name: 'Shopping' },
];
const RULES = [
  { id: 'rule-1', text: 'Example Market', category_id: GROCERIES, transaction_count: 3 },
  { id: 'rule-2', text: 'Example', category_id: SHOPPING, transaction_count: 1 },
];

const transaction = (id, date, amount, name, category = null) => ({
  id,
  date,
  amount,
  counterparty_name: name,
  counterparty_iban: '',
  description: 'Example description',
  own_account: 'NL00EXAM0000000001',
  reference: `R-${id}`,
  payment_method: 'Betaalautomaat',
  category_id: category,
  manual: false,
});

/** The golden example project with this list, the count and total Rust would return for it. */
function readsWith({ rows, count = rows.length, total = '0.00', categories = CATEGORIES, rules = RULES }) {
  const reads = exampleReads();
  reads.list_transactions = { rows, count, total };
  reads.list_transaction_categories = categories;
  reads.list_transaction_rules = rules;
  return reads;
}

async function transactionsPage(reads, handlers) {
  const app = await openApp({ reads, handlers });
  await app.click(app.item('transactions'));
  return app;
}

const rowCells = (app, row) => [...row.querySelectorAll('th, td')].map((cell) => app.text(cell));
const tableRows = (app) => [...app.$('transactions-rows').querySelectorAll('tr')];

test('test_csd032_ac_16_1_transactions_is_in_the_left_pane_after_expenses_and_shows_the_overview_tab', async () => {
  const app = await openApp();
  const items = [...app.document.querySelectorAll('.nav-item')].map((item) => app.text(item));
  assert.ok(items.indexOf('Transactions') === items.indexOf('Expenses') + 1, 'after Expenses');
  await app.click(app.item('transactions'));
  assert.ok(app.isShown(app.$('transactions')));
  assert.equal(app.textOf('transactions-tab-overview'), 'Overview');
  assert.deepEqual([...app.document.querySelectorAll('#transactions [role="tab"]')].map((tab) => app.text(tab)), ['Categories', 'Rules', 'Overview', 'Insights']);
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
  assert.equal(shown[0][1], 'Example Market Town');
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

test('test_csd032_ac_15_3_an_empty_project_shows_the_empty_state_and_the_import_button', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }));
  assert.equal(app.textOf('transactions-empty'), 'No transactions yet');
  assert.ok(app.isShown(app.$('transactions-empty')));
  assert.equal(app.isShown(app.$('transactions-table')), false);
  assert.ok(app.isShown(app.$('transaction-import')));
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

test('test_csd032_ac_25_3_delete_all_asks_first_and_cancel_changes_nothing', async () => {
  const rows = [transaction('t1', '2026-10-30', '-1.00', 'Example Shop')];
  const app = await transactionsPage(readsWith({ rows }));
  await app.click(app.$('transaction-delete-all'));
  assert.equal(app.$('confirm-dialog').open, true);
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.callsOf('delete_all_transactions').length, 0);
  assert.equal(tableRows(app).length, 1);
  app.close();
});

test('test_csd032_ac_25_1_a_confirmed_delete_all_sends_the_command', async () => {
  const rows = [transaction('t1', '2026-10-30', '-1.00', 'Example Shop')];
  const app = await transactionsPage(readsWith({ rows }), { delete_all_transactions: null });
  await app.click(app.$('transaction-delete-all'));
  await app.click(app.$('confirm-ok'));
  assert.equal(app.callsOf('delete_all_transactions').length, 1);
  app.close();
});

test('test_csd032_ac_16_2_the_import_result_is_a_yellow_notification', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }), {
    import_transactions: { added: 3, skipped: 2 },
  });
  await app.click(app.$('transaction-import'));
  assert.equal(app.textOf('toast'), '3 added, 2 skipped as duplicates');
  assert.ok(app.isShown(app.$('toast')));
  assert.ok(app.$('toast').classList.contains('toast-notice'), 'yellow, not the red of an error');
  app.close();
});

test('test_csd032_ac_1_2_cancelling_the_file_dialog_shows_nothing', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }), { import_transactions: null });
  await app.click(app.$('transaction-import'));
  assert.equal(app.$('toast').hidden, true);
  app.close();
});

test('test_csd032_ac_3_1_a_rejected_import_shows_its_error_in_the_toast', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }), {
    import_transactions: () => {
      throw { code: 'TRANSACTION_IMPORT_DATE', message: 'Line 4: the date is not a real date as dd-mm-yyyy.' };
    },
  });
  await app.click(app.$('transaction-import'));
  assert.equal(app.$('toast').hidden, false);
  assert.match(app.textOf('toast'), /Line 4/);
  assert.equal(app.$('toast').classList.contains('toast-notice'), false, 'an error stays red');
  app.close();
});

test('test_csd032_ac_12_1_applying_rules_reports_how_many_changed', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }), {
    reapply_transaction_rules: ({ scope }) => ({ changed: scope === 'all' ? 3 : 2 }),
  });
  await app.click(app.$('transactions-tab-rules'));
  await app.click(app.$('transaction-apply-rules'));
  assert.equal(app.textOf('toast'), '2 changed');
  assert.deepEqual(app.callsOf('reapply_transaction_rules').at(-1).args, { scope: 'all_non_manual' });
  app.close();
});

test('test_csd032_ac_12_4_applying_including_manual_asks_first', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }), {
    reapply_transaction_rules: { changed: 3 },
  });
  await app.click(app.$('transactions-tab-rules'));
  await app.click(app.$('transaction-apply-rules-all'));
  assert.equal(app.$('confirm-dialog').open, true);
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.callsOf('reapply_transaction_rules').length, 0);
  await app.click(app.$('transaction-apply-rules-all'));
  await app.click(app.$('confirm-ok'));
  assert.deepEqual(app.callsOf('reapply_transaction_rules').at(-1).args, { scope: 'all' });
  assert.equal(app.textOf('toast'), '3 changed');
  app.close();
});

test('test_csd032_ac_8_1_uncategorized_is_listed_first_and_cannot_be_edited', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }));
  const items = [...app.$('transaction-categories').querySelectorAll('li')];
  assert.equal(app.text(items[0].querySelector('.job-name')), 'Uncategorized');
  assert.equal(app.text(items[0].querySelector('.job-meta')), 'Built in');
  assert.equal(items[0].querySelectorAll('button').length, 0, 'no edit or delete');
  assert.deepEqual(items.slice(1).map((item) => app.text(item.querySelector('.job-name'))), ['Groceries', 'Shopping']);
  app.close();
});

test('test_csd032_ac_8_2_adding_a_category_sends_the_name_and_shows_a_rejection_in_the_dialog', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }), {
    add_transaction_category: ({ name }) => {
      if (name === 'groceries') throw { code: 'TRANSACTION_CATEGORY_NAME_TAKEN', message: 'A category with this name already exists.' };
      return { id: 'c-new', name };
    },
  });
  await app.click(app.$('transaction-category-add'));
  const form = app.$('transaction-category-form');
  form.elements.name.value = 'groceries';
  form.requestSubmit();
  await app.settle();
  assert.equal(app.$('transaction-category-dialog').open, true);
  assert.equal(app.textOf('transaction-category-error'), 'A category with this name already exists.');
  assert.equal(form.elements.name.getAttribute('aria-invalid'), 'true');
  form.elements.name.value = 'Pets';
  form.requestSubmit();
  await app.settle();
  assert.equal(app.$('transaction-category-dialog').open, false);
  assert.deepEqual(app.callsOf('add_transaction_category').at(-1).args, { name: 'Pets' });
  app.close();
});

test('test_csd032_ac_13_1_deleting_a_category_asks_first_and_names_the_consequence', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }), { delete_transaction_category: null });
  await app.click(app.document.querySelector('[aria-label="Delete Groceries"]'));
  assert.match(app.textOf('confirm-message'), /become Uncategorized.*rules/);
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.callsOf('delete_transaction_category').length, 0);
  await app.click(app.document.querySelector('[aria-label="Delete Groceries"]'));
  await app.click(app.$('confirm-ok'));
  assert.deepEqual(app.callsOf('delete_transaction_category').at(-1).args, { categoryId: GROCERIES });
  app.close();
});

test('test_csd032_ac_27_2_every_rule_shows_how_many_transactions_it_categorises', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }));
  const counts = [...app.$('transaction-rules').querySelectorAll('li')].map((row) => app.text(row.querySelector('.rule-count')));
  assert.deepEqual(counts, ['3 transactions', '1 transaction']);
  app.close();
});

test('test_csd032_ac_9_1_the_rules_are_listed_in_order_with_their_category_and_can_move', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }), { move_transaction_rule: null });
  const rows = [...app.$('transaction-rules').querySelectorAll('li')];
  assert.deepEqual(rows.map((row) => app.text(row.querySelector('.job-name'))), ['Example Market', 'Example']);
  assert.equal(app.text(rows[0].querySelector('.job-meta')), '→ Groceries');
  assert.equal(rows[0].querySelector('[aria-label="Move Example Market up"]'), null, 'the first cannot move up');
  await app.click(rows[1].querySelector('[aria-label="Move Example up"]'));
  assert.deepEqual(app.callsOf('move_transaction_rule').at(-1).args, { ruleId: 'rule-2', position: 0 });
  app.close();
});

test('test_csd032_ac_9_3_the_rule_form_offers_the_existing_categories_and_sends_them', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }), { add_transaction_rule: { id: 'rule-3' } });
  await app.click(app.$('transaction-rule-add'));
  const form = app.$('transaction-rule-form');
  assert.deepEqual([...form.elements.category.options].map((option) => app.text(option)), ['Groceries', 'Shopping', 'New category…']);
  form.elements.text.value = 'Example Gym';
  form.elements.category.value = SHOPPING;
  form.requestSubmit();
  await app.settle();
  assert.deepEqual(app.callsOf('add_transaction_rule').at(-1).args, { text: 'Example Gym', categoryId: SHOPPING });
  app.close();
});

test('test_csd032_ac_9_10_a_duplicate_rule_text_marks_its_text_field_in_dutch', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }), {
    add_transaction_rule: () => {
      throw { code: 'TRANSACTION_RULE_TEXT_TAKEN', message: 'A rule with this text already exists.' };
    },
  });
  await app.click(app.$('transaction-rule-add'));
  const form = app.$('transaction-rule-form');
  form.elements.text.value = 'Example Market';
  form.requestSubmit();
  await app.settle();
  assert.equal(form.elements.text.getAttribute('aria-invalid'), 'true');
  assert.match(app.document.body.textContent, /A rule with this text already exists\./);
  app.close();
});

test('test_csd032_ac_9_3_a_rejected_rule_marks_its_text_field', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }), {
    add_transaction_rule: () => {
      throw { code: 'TRANSACTION_RULE_TEXT_REQUIRED', message: 'Enter the text the rule looks for.' };
    },
  });
  await app.click(app.$('transaction-rule-add'));
  const form = app.$('transaction-rule-form');
  form.requestSubmit();
  await app.settle();
  assert.equal(form.elements.text.getAttribute('aria-invalid'), 'true');
  assert.equal(app.$('transaction-rule-dialog').open, true);
  app.close();
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
  assert.equal(app.textOf('transaction-import'), 'Transacties importeren');
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
  await app.click(app.$('transaction-import'));
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
  const overview = ['transactions-table', 'transaction-import'];
  assert.equal(app.$('transactions-tab-overview').getAttribute('aria-selected'), 'true');
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
  assert.equal(app.$('transaction-manual-only').checked, false);
  assert.equal(tableRows(app).length, 2);
  await app.click(app.$('transaction-manual-only'));
  assert.equal(app.callsOf('list_transactions').at(-1).args.manualOnly, true);
  assert.equal(tableRows(app).length, 1);
  assert.equal(app.textOf('transactions-total-label'), 'Filtered total');
  await app.click(app.$('transaction-manual-only'));
  assert.equal(app.callsOf('list_transactions').at(-1).args?.manualOnly, undefined);
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
  assert.equal(app.isShown(app.$('transactions-table')), true, 'the header stays, so the checkbox can be unticked');
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

test('test_csd032_ac_23_1_the_dialog_starts_from_the_transaction', async () => {
  const rows = [transaction('t1', '2026-10-30', '-23.97', 'Example Market Town', SHOPPING)];
  const app = await transactionsPage(readsWith({ rows }));
  const buttons = [...app.$('transactions-rows').querySelectorAll('.row-actions button')];
  assert.deepEqual(buttons.map((button) => button.getAttribute('aria-label')), ['Add a rule from Example Market Town', 'Delete Example Market Town'], 'left of delete');
  await app.click(buttons[0]);
  const form = app.$('transaction-rule-form');
  assert.equal(app.$('transaction-rule-dialog').open, true);
  assert.equal(app.textOf('transaction-rule-dialog-title'), 'Add rule');
  assert.equal(form.elements.text.value, 'Example Market Town');
  assert.equal(form.elements.category.value, SHOPPING);
  assert.equal(app.callsOf('add_transaction_rule').length, 0, 'nothing is stored yet');
  app.close();
});

test('test_csd032_ac_23_2_without_a_counterparty_or_a_category', async () => {
  const row = { ...transaction('t1', '2026-10-30', '-1000.00', ''), description: 'Rent October 2026' };
  const app = await transactionsPage(readsWith({ rows: [row] }));
  await app.click(app.$('transactions-rows').querySelector('.row-actions button'));
  const form = app.$('transaction-rule-form');
  assert.equal(form.elements.text.value, 'Rent October 2026');
  assert.equal(form.elements.category.value, GROCERIES, 'the first category');
  app.close();
});

test('test_csd032_ac_23_3_saving_adds_the_rule_and_cancelling_adds_none', async () => {
  const rows = [transaction('t1', '2026-10-30', '-23.97', 'Example Market Town', GROCERIES)];
  const app = await transactionsPage(readsWith({ rows }), { add_transaction_rule: { id: 'rule-3' } });
  const open = () => app.click(app.$('transactions-rows').querySelector('.row-actions button'));
  await open();
  const form = app.$('transaction-rule-form');
  form.elements.text.value = 'Example Market';
  form.requestSubmit();
  await app.settle();
  assert.deepEqual(app.callsOf('add_transaction_rule').at(-1).args, { text: 'Example Market', categoryId: GROCERIES });
  assert.equal(app.$('transaction-rule-dialog').open, false);
  await open();
  await app.click(app.$('transaction-rule-dialog').querySelector('[data-cancel]'));
  assert.equal(app.callsOf('add_transaction_rule').length, 1);
  app.close();
});

test('test_csd032_ac_9_8_new_category_is_the_last_option_of_a_new_rule_only', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }));
  await app.click(app.$('transaction-rule-add'));
  const form = app.$('transaction-rule-form');
  const options = [...form.elements.category.options].map((option) => app.text(option));
  assert.equal(options.at(-1), 'New category…');
  assert.equal(app.$('transaction-rule-new-category').hidden, true);
  await app.choose(form.elements.category, '__new__');
  assert.equal(app.$('transaction-rule-new-category').hidden, false);
  await app.choose(form.elements.category, SHOPPING);
  assert.equal(app.$('transaction-rule-new-category').hidden, true);
  await app.click(app.$('transaction-rule-dialog').querySelector('[data-cancel]'));
  await app.click(app.$('transaction-rules').querySelector('[aria-label="Edit Example"]'));
  assert.ok(![...form.elements.category.options].some((option) => option.value === '__new__'), 'no new category when editing');
  app.close();
});

test('test_csd032_ac_9_6_saving_with_a_new_category_sends_the_name', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }), { add_transaction_rule: { id: 'rule-3' } });
  await app.click(app.$('transaction-rule-add'));
  const form = app.$('transaction-rule-form');
  form.elements.text.value = 'Example Gym';
  await app.choose(form.elements.category, '__new__');
  form.elements.name.value = 'Example Sport';
  form.requestSubmit();
  await app.settle();
  assert.deepEqual(app.callsOf('add_transaction_rule').at(-1).args, { text: 'Example Gym', newCategory: 'Example Sport' });
  assert.equal(app.$('transaction-rule-dialog').open, false);
  app.close();
});

test('test_csd032_ac_9_7_a_rejected_new_category_name_marks_the_name_field', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }), {
    add_transaction_rule: () => {
      throw { code: 'TRANSACTION_CATEGORY_NAME_TAKEN', message: 'A category with this name already exists.' };
    },
  });
  await app.click(app.$('transaction-rule-add'));
  const form = app.$('transaction-rule-form');
  form.elements.text.value = 'Example Gym';
  await app.choose(form.elements.category, '__new__');
  form.elements.name.value = 'groceries';
  form.requestSubmit();
  await app.settle();
  assert.equal(app.$('transaction-rule-dialog').open, true);
  assert.equal(form.elements.name.getAttribute('aria-invalid'), 'true');
  app.close();
});

test('test_csd032_ac_9_8_a_rule_from_a_transaction_can_also_create_a_category', async () => {
  const rows = [transaction('t1', '2026-10-30', '-23.97', 'Example Market Town', GROCERIES)];
  const app = await transactionsPage(readsWith({ rows }));
  await app.click(app.$('transactions-rows').querySelector('.row-actions button'));
  const form = app.$('transaction-rule-form');
  assert.equal(form.elements.category.value, GROCERIES);
  assert.ok([...form.elements.category.options].some((option) => option.value === '__new__'));
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

const totalsRows = (app) => [...app.$('transaction-totals-rows').querySelectorAll('tr')].map((row) => rowCells(app, row));

test('test_csd032_ac_24_1_the_yearly_totals_are_shown_as_rust_returned_them_above_the_list', async () => {
  const reads = readsWith({ rows: [transaction('t1', '2026-10-30', '-23.97', 'Example Shop')] });
  reads.yearly_transaction_totals = [
    { year: 2026, income: '2500.00', spent: '23.97', net: '2476.03', transaction_count: 2 },
    { year: 2025, income: '0.00', spent: '150.00', net: '-150.00', transaction_count: 2 },
  ];
  const app = await transactionsPage(reads);
  assert.deepEqual(totalsRows(app), [
    ['2026', '2,500.00', '23.97', '2,476.03'],
    ['2025', '0.00', '150.00', '-150.00'],
  ]);
  const headers = [...app.$('transaction-totals-table').querySelectorAll('thead th')].map((cell) => app.text(cell));
  assert.deepEqual(headers, ['Year', 'Income', 'Spent', 'Net']);
  const panels = [...app.$('transactions-overview').querySelectorAll(':scope > .panel .panel-title')].map((node) => app.text(node));
  assert.deepEqual(panels, ['Yearly totals', 'Transactions'], 'the totals come first');
  app.close();
});

test('test_csd032_ac_24_2_no_transactions_no_totals', async () => {
  const reads = readsWith({ rows: [], count: 0 });
  reads.yearly_transaction_totals = [];
  const app = await transactionsPage(reads);
  assert.equal(app.isShown(app.$('transaction-totals-table')), false);
  assert.equal(app.textOf('transaction-totals-empty'), 'No totals yet. Totals appear once there is a transaction.');
  assert.ok(app.isShown(app.$('transaction-totals-empty')));
  app.close();
});

test('test_csd032_ac_24_3_a_filter_does_not_ask_for_other_totals', async () => {
  const rows = [transaction('t1', '2026-10-30', '-1.00', 'Example Shop', GROCERIES)];
  const app = await transactionsPage(readsWith({ rows }));
  const before = app.callsOf('yearly_transaction_totals').length;
  await app.click(app.$('transaction-entity-filter').querySelector('button'));
  const box = [...app.document.querySelectorAll('.filter-check input')].find((input) => input.value === GROCERIES);
  box.checked = true;
  box.dispatchEvent(new app.window.Event('change', { bubbles: true }));
  await app.settle();
  assert.equal(app.callsOf('yearly_transaction_totals').length, before, 'the totals are not asked for again');
  assert.ok(app.callsOf('yearly_transaction_totals').every((call) => call.args === undefined || !call.args.filter));
  app.close();
});

test('test_csd032_ac_24_4_a_change_reloads_the_totals', async () => {
  const rows = [transaction('t1', '2026-10-30', '-1.00', 'Example Shop')];
  const app = await transactionsPage(readsWith({ rows }), { delete_transaction: null });
  const before = app.callsOf('yearly_transaction_totals').length;
  await app.click(app.document.querySelector('[aria-label="Delete Example Shop"]'));
  await app.click(app.$('confirm-ok'));
  assert.ok(app.callsOf('yearly_transaction_totals').length > before);
  app.close();
});

test('test_csd032_ac_24_5_discrete_mode_hides_the_yearly_amounts', async () => {
  const reads = readsWith({ rows: [] });
  reads.yearly_transaction_totals = [{ year: 2026, income: '2500.00', spent: '23.97', net: '2476.03', transaction_count: 2 }];
  reads.get_preferences = { theme: 'system', language: 'en', number_format: 'point_comma', discrete_mode: true };
  const app = await transactionsPage(reads);
  assert.deepEqual(totalsRows(app), [['2026', '•••••', '•••••', '•••••']]);
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
