// DOM tests of CSD-032 (the categories and the rules). Rust computes every figure; the tests feed canned answers and check only what is displayed.
import test from 'node:test';
import assert from 'node:assert/strict';
import { GROCERIES, SHOPPING, RULES, transaction, readsWith, transactionsPage, layOutRules, pointer, HOLIDAY_RULE } from './transactions-helpers.js';

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

test('test_csd032_ac_9_1_the_rules_are_listed_in_order_with_their_category_and_a_handle', async () => {
  const app = await transactionsPage(readsWith({ rows: [], count: 0 }), { move_transaction_rule: null });
  const rows = [...app.$('transaction-rules').querySelectorAll('li')];
  assert.deepEqual(rows.map((row) => app.text(row.querySelector('.job-name'))), ['Example Market', 'Example']);
  assert.equal(app.text(rows[0].querySelector('.job-meta')), '→ Groceries');
  assert.equal(rows[0].firstElementChild.classList.contains('drag-handle'), true, 'the handle is the first thing in the row');
  assert.equal(rows[0].querySelector('[aria-label^="Move "]'), null, 'no arrows any more');
  app.close();
});

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
