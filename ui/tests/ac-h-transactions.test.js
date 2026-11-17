// Display ACs of CSD-032 (transactions): the page, the rows, the paging, the filtered total, the
// import message, the categories and rules forms, Discrete mode and the Dutch labels. Rust parses,
// categorises, filters and adds up; the fake bridge answers with the golden example project or a
// small list made for one AC.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { exampleReads } from './fixtures.js';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const GROCERIES = 'cat-groceries';
const SHOPPING = 'cat-shopping';
const CATEGORIES = [
  { id: GROCERIES, name: 'Groceries', transaction_count: 3 },
  { id: SHOPPING, name: 'Shopping', transaction_count: 1 },
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

test('test_csd032_ac_30_2_the_categories_show_their_transaction_counts', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }));
  const items = [...app.$('transaction-categories').querySelectorAll('li')];
  assert.deepEqual(items.slice(1).map((item) => app.text(item.querySelector('.rule-count'))), ['3 transactions', '1 transaction']);
  assert.equal(items[0].querySelector('.rule-count'), null, 'Uncategorized shows no count');
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

test('test_csd032_ac_31_2_the_overview_shows_the_categorized_share_in_the_yearly_totals', async () => {
  const reads = readsWith({ rows: [], count: 0 });
  reads.get_categorisation_summary = { total: 40, categorized: 35, percent_label: '87.50%' };
  const app = await transactionsPage(reads);
  const note = app.$('transaction-categorisation');
  assert.equal(app.text(note), '87.50% of the transactions are categorized (35 of 40)');
  await app.click(app.$('transactions-tab-overview'));
  assert.ok(app.isShown(note));
  assert.equal(note.closest('.panel').querySelector('.panel-title').textContent, 'Yearly totals', 'in the panel of the yearly totals');
  assert.equal(app.$('transactions-rules').contains(note), false, 'not on the Rules tab');
  app.close();
  const empty = readsWith({ rows: [], count: 0 });
  empty.get_categorisation_summary = { total: 0, categorized: 0, percent_label: null };
  const none = await transactionsPage(empty);
  assert.ok(none.$('transaction-categorisation').hidden, 'no line without transactions');
  none.close();
});

test('test_csd032_ac_9_1_the_rules_are_listed_in_order_with_their_category_and_a_handle', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }), { move_transaction_rule: null });
  const rows = [...app.$('transaction-rules').querySelectorAll('li')];
  assert.deepEqual(rows.map((row) => app.text(row.querySelector('.job-name'))), ['Example Market', 'Example']);
  assert.equal(app.text(rows[0].querySelector('.job-meta')), '→ Groceries');
  assert.equal(rows[0].firstElementChild.classList.contains('drag-handle'), true, 'the handle is the first thing in the row');
  assert.equal(rows[0].querySelector('[aria-label^="Move "]'), null, 'no arrows any more');
  app.close();
});

/** Lays the rule rows out as 40-pixel rows from the top, since jsdom has no layout. */
function layOutRules(app) {
  [...app.$('transaction-rules').querySelectorAll('li')].forEach((row, index) => {
    row.getBoundingClientRect = () => ({ top: index * 40, height: 40, bottom: index * 40 + 40, left: 0, right: 100, width: 100 });
  });
}
const pointer = (app, target, type, clientY = 0) =>
  target.dispatchEvent(new app.window.MouseEvent(type, { bubbles: true, cancelable: true, button: 0, clientY }));

test('test_csd032_ac_9_2_a_rule_is_ordered_by_dragging_its_handle', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }), { move_transaction_rule: null });
  layOutRules(app);
  const handle = app.$('transaction-rules').querySelectorAll('.drag-handle')[1];
  pointer(app, handle, 'pointerdown', 60);
  pointer(app, app.document, 'pointermove', 5);
  const preview = [...app.$('transaction-rules').querySelectorAll('li')].map((row) => app.text(row.querySelector('.job-name')));
  assert.deepEqual(preview, ['Example', 'Example Market'], 'the row follows the pointer while dragging');
  assert.equal(app.callsOf('move_transaction_rule').length, 0, 'nothing is sent before the drop');
  pointer(app, app.document, 'pointerup', 5);
  await app.settle();
  assert.deepEqual(app.callsOf('move_transaction_rule').at(-1).args, { ruleId: 'rule-2', position: 0 });
  app.close();
});

test('test_csd032_ac_9_2_dropping_a_rule_where_it_was_or_pressing_escape_changes_nothing', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }), { move_transaction_rule: null });
  layOutRules(app);
  const handle = app.$('transaction-rules').querySelectorAll('.drag-handle')[0];
  pointer(app, handle, 'pointerdown', 10);
  pointer(app, app.document, 'pointermove', 15);
  pointer(app, app.document, 'pointerup', 15);
  await app.settle();
  assert.equal(app.callsOf('move_transaction_rule').length, 0, 'dropped on its own place');
  pointer(app, app.$('transaction-rules').querySelectorAll('.drag-handle')[0], 'pointerdown', 10);
  pointer(app, app.document, 'pointermove', 70);
  app.document.dispatchEvent(new app.window.KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
  pointer(app, app.document, 'pointerup', 70);
  await app.settle();
  assert.equal(app.callsOf('move_transaction_rule').length, 0, 'Escape cancels the drag');
  const names = [...app.$('transaction-rules').querySelectorAll('li .job-name')].map((cell) => app.text(cell));
  assert.deepEqual(names, ['Example Market', 'Example'], 'the list is as it was');
  assert.equal(app.$('transaction-rules').classList.contains('sorting'), false);
  app.close();
});

test('test_csd032_ac_9_2_the_arrow_keys_on_the_handle_move_a_rule_too', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }), { move_transaction_rule: null });
  const handles = app.$('transaction-rules').querySelectorAll('.drag-handle');
  handles[1].dispatchEvent(new app.window.KeyboardEvent('keydown', { key: 'ArrowUp', bubbles: true, cancelable: true }));
  await app.settle();
  assert.deepEqual(app.callsOf('move_transaction_rule').at(-1).args, { ruleId: 'rule-2', position: 0 });
  handles[0].dispatchEvent(new app.window.KeyboardEvent('keydown', { key: 'ArrowUp', bubbles: true, cancelable: true }));
  await app.settle();
  assert.equal(app.callsOf('move_transaction_rule').length, 1, 'the first rule cannot move up');
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
  assert.deepEqual(app.callsOf('add_transaction_rule').at(-1).args, { text: 'Example Gym', categoryId: SHOPPING, from: '', to: '' });
  app.close();
});

const HOLIDAY_RULE = { id: 'rule-3', text: '', category_id: SHOPPING, transaction_count: 4, from: '2026-07-01', to: '2026-07-15' };

test('test_csd032_ac_32_9_a_rule_with_a_period_is_listed_with_its_days', async () => {
  const reads = readsWith({ rows: [], count: 0, rules: [...RULES, HOLIDAY_RULE, { ...HOLIDAY_RULE, id: 'rule-4', to: null }, { ...HOLIDAY_RULE, id: 'rule-5', from: null, text: 'Example' }] });
  const app = await transactionsPage(reads);
  const rows = [...app.$('transaction-rules').querySelectorAll('li')];
  assert.equal(app.text(rows[2].querySelector('.job-name')), 'Any transaction', 'no text, so every transaction');
  assert.equal(app.text(rows[2].querySelector('.job-meta')), '01/07/2026 – 15/07/2026 → Shopping');
  assert.equal(app.text(rows[3].querySelector('.job-meta')), 'from 01/07/2026 → Shopping');
  assert.equal(app.text(rows[4].querySelector('.job-meta')), 'until 15/07/2026 → Shopping');
  assert.equal(app.text(rows[4].querySelector('.job-name')), 'Example');
  assert.ok(rows[2].querySelector('[aria-label="Edit Any transaction"]'), 'the buttons name the rule');
  assert.equal(app.text(rows[0].querySelector('.job-meta')), '→ Groceries', 'a rule without a period shows none');
  app.close();
});

test('test_csd032_ac_32_10_the_rule_form_has_optional_dates_and_sends_them', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }), { add_transaction_rule: { id: 'rule-3' } });
  await app.click(app.$('transaction-rule-add'));
  const form = app.$('transaction-rule-form');
  assert.deepEqual([...form.querySelectorAll('input[data-mask="date"]')].map((input) => input.name), ['from', 'to']);
  assert.equal(app.text(form.elements.from.closest('label')), 'From date (optional)');
  form.elements.category.value = SHOPPING;
  await app.type(form.elements.from, '01072026');
  await app.type(form.elements.to, '15072026');
  assert.equal(form.elements.from.value, '01/07/2026', 'typed digits are laid out as a day');
  form.requestSubmit();
  await app.settle();
  assert.deepEqual(app.callsOf('add_transaction_rule').at(-1).args, { text: '', categoryId: SHOPPING, from: '01/07/2026', to: '15/07/2026' });
  app.close();
});

test('test_csd032_ac_32_11_editing_a_period_rule_fills_the_days_and_a_bad_date_shows_in_the_dialog_in_dutch', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0, rules: [HOLIDAY_RULE] }), {
    edit_transaction_rule: () => {
      throw { code: 'TRANSACTION_RULE_PERIOD_INVALID', message: 'The first day cannot be after the last day.' };
    },
  });
  await app.click(app.$('transactions-tab-rules'));
  await app.click(app.$('transaction-rules').querySelector('[aria-label="Edit Any transaction"]'));
  const form = app.$('transaction-rule-form');
  assert.equal(form.elements.from.value, '01/07/2026');
  assert.equal(form.elements.to.value, '15/07/2026');
  assert.equal(form.elements.text.value, '');
  form.requestSubmit();
  await app.settle();
  assert.deepEqual(app.callsOf('edit_transaction_rule').at(-1).args, { ruleId: 'rule-3', text: '', categoryId: SHOPPING, from: '01/07/2026', to: '15/07/2026' });
  assert.match(app.textOf('transaction-rule-error'), /first day cannot be after the last day/);
  assert.ok(app.isShown(app.$('transaction-rule-dialog')), 'the dialog stays open');
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
      throw { code: 'TRANSACTION_RULE_TEXT_REQUIRED', message: 'Enter the text the rule looks for, or give it a period.' };
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

const bulkRows = () => [
  transaction('t1', '2026-10-30', '-1.00', 'Example Shop'),
  transaction('t2', '2026-10-29', '-2.00', 'Example Cafe'),
  transaction('t3', '2026-10-28', '-3.00', 'Example Bakery'),
];
const bulkHandlers = (extra = {}) => ({
  list_transactions: (args) => ({ rows: bulkRows().slice(0, args?.limit ?? 200), count: 3, total: '-6.00' }),
  list_transaction_ids: ['t1', 't2', 't3'],
  ...extra,
});

test('test_csd032_ac_15_7_the_period_filter_of_the_list_asks_for_days', async () => {
  const app = await transactionsPage(readsWith({ rows: bulkRows(), count: 3 }), bulkHandlers());
  await app.click(app.document.querySelector('#transaction-period-filter button'));
  const fields = [...app.document.querySelectorAll('#transaction-period-filter input[data-mask]')];
  assert.deepEqual(fields.map((input) => app.text(input.closest('label'))), ['From date', 'To date']);
  assert.deepEqual(fields.map((input) => input.getAttribute('placeholder')), ['dd/mm/yyyy', 'dd/mm/yyyy']);
  await app.type(fields[0], '29102026');
  assert.equal(fields[0].value, '29/10/2026', 'typed digits are laid out as a day');
  assert.deepEqual(app.callsOf('list_transactions').at(-1).args.filter, { from: '2026-10-29', to: null, ids: [], min_amount: null, max_amount: null, counterparty: null, description: null }, 'Rust gets the ISO day');
  await app.type(fields[1], '31022026');
  assert.deepEqual(app.callsOf('list_transactions').at(-1).args.filter.to, null, 'a day that is not on the calendar is not sent');
  app.close();
});

test('test_csd032_ac_15_8_the_amount_filter_sends_the_typed_range', async () => {
  const app = await transactionsPage(readsWith({ rows: bulkRows(), count: 3 }), bulkHandlers());
  await app.click(app.document.querySelector('#transaction-amount-filter button'));
  const panel = app.document.querySelector('#transaction-amount-filter .filter-panel');
  const fields = [...panel.querySelectorAll('input')];
  assert.deepEqual(fields.map((input) => app.text(input.closest('label'))), ['From amount', 'To amount']);
  await app.type(fields[0], '-50,5');
  assert.deepEqual(app.callsOf('list_transactions').at(-1).args.filter, { from: null, to: null, ids: [], min_amount: '-50,5', max_amount: null, counterparty: null, description: null }, 'the typed text goes to Rust');
  await app.type(fields[1], 'abc');
  assert.equal(app.callsOf('list_transactions').at(-1).args.filter.max_amount, null, 'text without a digit is not sent');
  assert.match(app.document.querySelector('#transaction-amount-filter button').getAttribute('aria-label'), /active/);
  app.close();
});

test('test_csd032_ac_15_9_the_text_filters_send_the_typed_text', async () => {
  const app = await transactionsPage(readsWith({ rows: bulkRows(), count: 3 }), bulkHandlers());
  await app.click(app.document.querySelector('#transaction-counterparty-filter button'));
  const panel = app.document.querySelector('#transaction-counterparty-filter .filter-panel');
  const field = panel.querySelector('input');
  assert.equal(app.text(field.closest('label')), 'Counterparty (* is any text)');
  await app.type(field, ' Albert ');
  assert.equal(app.callsOf('list_transactions').at(-1).args.filter.counterparty, 'Albert', 'the text is sent without surrounding spaces');
  assert.equal(app.callsOf('list_transactions').at(-1).args.filter.description, null);
  assert.match(app.document.querySelector('#transaction-counterparty-filter button').getAttribute('aria-label'), /active/);
  await app.click(app.document.querySelector('#transaction-description-filter button'));
  const second = app.document.querySelector('#transaction-description-filter .filter-panel input');
  assert.equal(app.text(second.closest('label')), 'Description (* is any text)');
  await app.type(second, 'rent');
  assert.deepEqual(
    [app.callsOf('list_transactions').at(-1).args.filter.counterparty, app.callsOf('list_transactions').at(-1).args.filter.description],
    ['Albert', 'rent'],
  );
  await app.click(app.document.querySelector('#transaction-description-filter .filter-reset'));
  assert.equal(app.callsOf('list_transactions').at(-1).args.filter.description, null, 'reset clears only that column');
  assert.equal(app.callsOf('list_transactions').at(-1).args.filter.counterparty, 'Albert');
  app.close();
});

test('test_csd032_ac_28_8_the_bulk_select_button_looks_pressed_while_bulk_select_is_on', async () => {
  const app = await transactionsPage(readsWith({ rows: bulkRows(), count: 3 }), bulkHandlers());
  const button = app.$('transaction-bulk-toggle');
  assert.equal(button.getAttribute('aria-pressed'), 'false');
  await app.click(button);
  assert.equal(button.getAttribute('aria-pressed'), 'true', 'pressed while on');
  await app.click(button);
  assert.equal(button.getAttribute('aria-pressed'), 'false', 'released when switched off');
  app.close();
  // jsdom has no layout: the pressed look is guarded as a rule of the stylesheet.
  const css = readFileSync(fileURLToPath(new URL('../styles/components.css', import.meta.url)), 'utf8');
  assert.match(css, /\.btn\[aria-pressed="true"\]\s*\{[^}]*background:\s*var\(--accent\)/);
});

// ---- REQ-29: suggest rules ----

const suggestion = (text, count, categoryId = null) => ({ text, count, category_id: categoryId });
const suggestHandlers = (extra = {}) => ({
  suggest_transaction_rules: (args) => ({
    suggestions: [suggestion('Albert Heijn*', 6, GROCERIES), suggestion('Example Cafe', 4)].filter((each) => each.count >= args.minCount),
    not_shown: 0,
  }),
  count_transaction_rule_matches: 3,
  ...extra,
});
const openSuggest = async (handlers = suggestHandlers()) => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }), handlers);
  await app.click(app.$('transactions-tab-rules'));
  await app.click(app.$('transaction-suggest'));
  return app;
};
const suggestRows = (app) => [...app.$('transaction-suggest-rows').querySelectorAll('tr')];
const waitForRecount = async (app) => {
  await new Promise((resolve) => setTimeout(resolve, 320));
  await app.settle();
};

test('test_csd032_ac_29_1_the_dialog_opens_with_5_and_follows_the_field', async () => {
  const app = await openSuggest();
  assert.equal(app.$('transaction-suggest-dialog').open, true);
  const field = app.$('transaction-suggest-form').elements.min_count;
  assert.equal(field.value, '5');
  assert.deepEqual(app.callsOf('suggest_transaction_rules').at(-1).args, { minCount: 5 });
  assert.equal(suggestRows(app).length, 1, 'only the suggestion with a count of at least 5');
  field.value = '';
  await app.type(field, '4');
  assert.deepEqual(app.callsOf('suggest_transaction_rules').at(-1).args, { minCount: 4 });
  assert.equal(suggestRows(app).length, 2);
  app.close();
});

test('test_csd032_ac_29_7_a_minimal_count_that_is_not_a_whole_number_changes_nothing', async () => {
  const app = await openSuggest();
  const field = app.$('transaction-suggest-form').elements.min_count;
  const calls = app.callsOf('suggest_transaction_rules').length;
  for (const text of ['', '0', '2.5', 'abc']) {
    // One change of the whole text, as a paste does: typing "2.5" would pass through the valid "2".
    field.value = text;
    field.dispatchEvent(new app.window.InputEvent('input', { bubbles: true }));
    await app.settle();
    assert.equal(app.isShown(app.$('transaction-suggest-min-error')), true, `"${text}" gets the message`);
    assert.equal(suggestRows(app).length, 1, 'the list is not changed');
  }
  assert.equal(app.callsOf('suggest_transaction_rules').length, calls, 'Rust is not asked');
  assert.equal(app.textOf('transaction-suggest-min-error'), 'Enter a whole number of 1 or more.');
  app.close();
});

test('test_csd032_ac_29_6_rows_show_text_count_and_the_suggested_category', async () => {
  const app = await openSuggest();
  const [row] = suggestRows(app);
  assert.equal(row.querySelector('input[type="text"]').value, 'Albert Heijn*');
  assert.equal(app.text(row.children[2]), '6');
  assert.equal(row.querySelector('select').value, GROCERIES);
  assert.equal(row.querySelector('input[type="checkbox"]').checked, false, 'nothing is ticked by default');
  assert.deepEqual([...row.querySelector('select').options].map((option) => app.text(option)), ['\u2014', 'Groceries', 'Shopping']);
  app.close();
});

test('test_csd032_ac_29_11_only_ticked_rows_count_and_nothing_is_ticked_by_default', async () => {
  const app = await openSuggest(suggestHandlers());
  const field = app.$('transaction-suggest-form').elements.min_count;
  field.value = '';
  await app.type(field, '4');
  const box = (n) => suggestRows(app)[n].querySelector('input[type="checkbox"]');
  assert.deepEqual([box(0).checked, box(1).checked], [false, false]);
  assert.equal(app.textOf('transaction-suggest-chosen'), '0 rules chosen');
  assert.equal(app.textOf('transaction-suggest-add'), 'Add 0 rules');
  assert.equal(app.$('transaction-suggest-add').disabled, true, 'a suggested category alone adds nothing');
  assert.equal(box(1).disabled, true, 'a row without a category cannot be ticked');
  await app.click(box(0));
  assert.equal(app.textOf('transaction-suggest-chosen'), '1 rule chosen');
  assert.equal(app.textOf('transaction-suggest-add'), 'Add 1 rule');
  assert.equal(app.$('transaction-suggest-add').disabled, false);
  await app.choose(suggestRows(app)[1].querySelector('select'), SHOPPING);
  assert.equal(box(1).disabled, false, 'a category makes the row tickable, but does not tick it');
  assert.equal(box(1).checked, false);
  await app.choose(suggestRows(app)[0].querySelector('select'), '');
  assert.equal(box(0).checked, false, 'without a category the tick goes');
  assert.equal(app.textOf('transaction-suggest-chosen'), '0 rules chosen');
  app.close();
});

test('test_csd032_ac_29_8_editing_a_text_recounts_it_and_keeps_the_row', async () => {
  const app = await openSuggest(suggestHandlers({ count_transaction_rule_matches: ({ text }) => (text === 'Albert*' ? 2 : 9) }));
  const input = suggestRows(app)[0].querySelector('input[type="text"]');
  input.value = 'Albert';
  await app.type(input, '*');
  assert.equal(app.text(suggestRows(app)[0].children[2]), '\u2026', 'recounting');
  await waitForRecount(app);
  assert.deepEqual(app.callsOf('count_transaction_rule_matches').at(-1).args, { text: 'Albert*' });
  assert.equal(app.text(suggestRows(app)[0].children[2]), '2', 'below the minimal count, and the row stays');
  assert.equal(suggestRows(app).length, 1);
  app.close();
});

test('test_csd032_ac_29_12_chosen_rows_stay_when_the_minimal_count_changes', async () => {
  const app = await openSuggest();
  const field = app.$('transaction-suggest-form').elements.min_count;
  field.value = '';
  await app.type(field, '4');
  await app.choose(suggestRows(app)[1].querySelector('select'), SHOPPING);
  await app.click(suggestRows(app)[1].querySelector('input[type="checkbox"]'));
  field.value = '';
  await app.type(field, '5');
  assert.deepEqual(suggestRows(app).map((row) => row.querySelector('input[type="text"]').value), ['Albert Heijn*', 'Example Cafe'], 'the ticked "Example Cafe" stays');
  assert.equal(suggestRows(app)[1].querySelector('select').value, SHOPPING);
  app.close();
});

test('test_csd032_ac_29_5_the_note_counts_the_suggestions_that_are_not_shown', async () => {
  const app = await openSuggest(suggestHandlers({ suggest_transaction_rules: { suggestions: [suggestion('Example Cafe', 4)], not_shown: 30 } }));
  assert.equal(app.isShown(app.$('transaction-suggest-more')), true);
  assert.equal(app.textOf('transaction-suggest-more'), '30 more not shown: raise the minimal count');
  app.close();
});

test('test_csd032_ac_29_9_adding_sends_the_chosen_rows_and_shows_the_result', async () => {
  const app = await openSuggest(suggestHandlers({ add_transaction_rules: { added: 1, changed: 12, rejected: null } }));
  await app.click(suggestRows(app)[0].querySelector('input[type="checkbox"]'));
  await app.click(app.$('transaction-suggest-add'));
  assert.deepEqual(app.callsOf('add_transaction_rules').at(-1).args, { rules: [{ text: 'Albert Heijn*', category_id: GROCERIES }] });
  assert.equal(app.$('transaction-suggest-dialog').open, false);
  assert.equal(app.textOf('toast'), '1 rule added, 12 transactions categorised');
  assert.ok(app.$('toast').classList.contains('toast-notice'));
  app.close();
});

test('test_csd032_ac_29_10_a_refused_row_is_marked_and_the_dialog_stays_open', async () => {
  const app = await openSuggest(
    suggestHandlers({
      add_transaction_rules: { added: 0, changed: 0, rejected: { index: 0, code: 'TRANSACTION_RULE_TEXT_TAKEN', message: 'A rule with this text already exists.' } },
    }),
  );
  await app.click(suggestRows(app)[0].querySelector('input[type="checkbox"]'));
  await app.click(app.$('transaction-suggest-add'));
  assert.equal(app.$('transaction-suggest-dialog').open, true);
  const [row] = suggestRows(app);
  assert.equal(row.querySelector('input[type="text"]').getAttribute('aria-invalid'), 'true');
  assert.equal(app.text(row.querySelector('.row-error')), 'A rule with this text already exists.');
  app.close();
});

test('test_csd032_ac_29_13_the_suggestion_list_scrolls_inside_the_dialog', async () => {
  // jsdom has no layout: this guards the rule that makes the list scroll. `.table-wrap.in-dialog` hides overflow, so the
  // rule for the list has to be at least as specific, or the rows below the window cannot be reached.
  const { readFileSync } = await import('node:fs');
  const css = readFileSync(new URL('../styles/components.css', import.meta.url), 'utf8');
  const rule = /\.table-wrap\.in-dialog\.suggest-wrap\s*\{([^}]*)\}/.exec(css);
  assert.ok(rule, 'a rule as specific as .table-wrap.in-dialog');
  assert.match(rule[1], /overflow:\s*auto/);
  assert.match(css, /dialog\.wide\[open\]\s*\{[^}]*height:\s*min\(\d+px,\s*calc\(100vh/, 'a default height that follows the window');
  assert.match(css, /dialog\.wide\[open\]\s*\{[^}]*max-height:\s*calc\(100vh/, 'the dialog stays inside the window');
  const app = await openSuggest();
  assert.ok(app.$('transaction-suggest-wrap').classList.contains('suggest-wrap'));
  assert.ok(app.$('transaction-suggest-wrap').classList.contains('in-dialog'));
  app.close();
});

test('test_csd032_ac_28_1_bulk_select_is_off_until_enabled', async () => {
  const app = await transactionsPage(readsWith({ rows: bulkRows(), count: 3 }), bulkHandlers());
  assert.equal(app.document.querySelectorAll('#transactions-rows input[type="checkbox"]').length, 0);
  assert.equal(app.isShown(app.$('transactions-bulk')), false);
  assert.equal(app.$('transactions-select-all'), null, 'no header checkbox while it is off');
  const dateHead = app.document.querySelector('#transactions-table thead th');
  assert.equal(app.text(dateHead).startsWith('Date'), true, 'the date column heads the table, so its padding matches the rows');
  await app.click(app.$('transaction-bulk-toggle'));
  assert.equal(app.document.querySelectorAll('#transactions-rows input[type="checkbox"]').length, 3);
  assert.equal(app.isShown(app.$('transactions-bulk')), true);
  assert.equal(app.isShown(app.$('transactions-select-all')), true);
  assert.equal(app.textOf('transactions-bulk-count'), '0 selected');
  assert.equal(app.$('transactions-bulk-set').disabled, true);
  await app.click(app.$('transaction-bulk-toggle'));
  assert.equal(app.document.querySelectorAll('#transactions-rows input[type="checkbox"]').length, 0);
  app.close();
});

test('test_csd032_ac_28_2_a_checkbox_selects_one_and_the_header_selects_all_in_the_filter', async () => {
  const app = await transactionsPage(readsWith({ rows: bulkRows(), count: 3 }), bulkHandlers());
  await app.click(app.$('transaction-bulk-toggle'));
  await app.click(app.document.querySelector('[aria-label="Select Example Shop"]'));
  assert.equal(app.textOf('transactions-bulk-count'), '1 selected');
  assert.equal(app.$('transactions-select-all').indeterminate, true);
  await app.click(app.$('transactions-select-all'));
  assert.equal(app.textOf('transactions-bulk-count'), '3 selected');
  assert.equal(app.$('transactions-select-all').checked, true);
  assert.equal(app.callsOf('list_transaction_ids').length, 1, 'Rust names every id of the filter');
  await app.click(app.$('transactions-select-all'));
  assert.equal(app.textOf('transactions-bulk-count'), '0 selected');
  app.close();
});

test('test_csd032_ac_28_3_select_all_covers_rows_that_are_not_loaded', async () => {
  const ids = Array.from({ length: 450 }, (_, n) => `t${n}`);
  const rows = ids.slice(0, 200).map((id, n) => transaction(id, '2026-10-01', '-1.00', `Example ${n}`));
  const app = await transactionsPage(readsWith({ rows, count: 450 }), {
    list_transactions: () => ({ rows, count: 450, total: '-450.00' }),
    list_transaction_ids: ids,
    set_transactions_category: (args) => args.transactionIds.length,
  });
  await app.click(app.$('transaction-bulk-toggle'));
  await app.click(app.$('transactions-select-all'));
  assert.equal(app.textOf('transactions-bulk-count'), '450 selected');
  assert.match(app.$('transactions-select-all').getAttribute('aria-label'), /450/);
  await app.choose(app.$('transactions-bulk-category'), SHOPPING);
  await app.click(app.$('transactions-bulk-set'));
  const call = app.callsOf('set_transactions_category').at(-1).args;
  assert.equal(call.transactionIds.length, 450);
  assert.equal(call.categoryId, SHOPPING);
  assert.equal(app.textOf('toast'), '450 changed');
  assert.equal(app.textOf('transactions-bulk-count'), '0 selected', 'the selection is cleared after the change');
  app.close();
});

test('test_csd032_ac_28_4_select_all_follows_the_filter', async () => {
  const app = await transactionsPage(readsWith({ rows: bulkRows(), count: 3 }), bulkHandlers());
  await app.click(app.$('transaction-bulk-toggle'));
  await app.click(app.$('transaction-manual-only'));
  await app.click(app.$('transactions-select-all'));
  assert.deepEqual(app.callsOf('list_transaction_ids').at(-1).args, { manualOnly: true });
  await app.click(app.$('transaction-manual-only'));
  assert.equal(app.textOf('transactions-bulk-count'), '0 selected', 'changing the filter clears the selection');
  app.close();
});

test('test_csd032_ac_28_5_deleting_the_selected_asks_first', async () => {
  const app = await transactionsPage(readsWith({ rows: bulkRows(), count: 3 }), bulkHandlers({ delete_transactions: 2 }));
  await app.click(app.$('transaction-bulk-toggle'));
  await app.click(app.document.querySelector('[aria-label="Select Example Shop"]'));
  await app.click(app.document.querySelector('[aria-label="Select Example Cafe"]'));
  await app.click(app.$('transactions-bulk-delete'));
  assert.equal(app.$('confirm-dialog').open, true);
  assert.match(app.textOf('confirm-message'), /2 selected transactions/);
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.callsOf('delete_transactions').length, 0);
  assert.equal(app.textOf('transactions-bulk-count'), '2 selected');
  await app.click(app.$('transactions-bulk-delete'));
  await app.click(app.$('confirm-ok'));
  assert.deepEqual(app.callsOf('delete_transactions').at(-1).args, { transactionIds: ['t1', 't2'] });
  assert.equal(app.textOf('transactions-bulk-count'), '0 selected');
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
  assert.deepEqual(app.callsOf('add_transaction_rule').at(-1).args, { text: 'Example Market', categoryId: GROCERIES, from: '', to: '' });
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
  assert.deepEqual(app.callsOf('add_transaction_rule').at(-1).args, { text: 'Example Gym', newCategory: 'Example Sport', from: '', to: '' });
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
