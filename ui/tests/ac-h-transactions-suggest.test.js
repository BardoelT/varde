// DOM tests of CSD-032 (the dialog "Suggest rules"). Rust computes every figure; the tests feed canned answers and check only what is displayed.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { GROCERIES, SHOPPING, transaction, suggestion, suggestHandlers, openSuggest, suggestRows, waitForRecount } from './transactions-helpers.js';

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

test('test_csd032_ac_29_16_the_minimal_count_is_a_number_field_with_up_and_down_buttons', async () => {
  const app = await openSuggest();
  const field = app.$('transaction-suggest-form').elements.min_count;
  assert.equal(field.type, 'number', 'the browser adds the up and down buttons');
  assert.equal(field.min, '1');
  assert.equal(field.step, '1');
  // A click on the buttons changes the value and fires `input`, as typing does.
  field.value = '4';
  field.dispatchEvent(new app.window.InputEvent('input', { bubbles: true }));
  await app.settle();
  assert.deepEqual(app.callsOf('suggest_transaction_rules').at(-1).args, { minCount: 4 });
  app.close();
});

test('test_csd032_ac_29_16_the_minimal_count_is_remembered_until_the_app_is_closed', async () => {
  const app = await openSuggest();
  const field = () => app.$('transaction-suggest-form').elements.min_count;
  field().value = '';
  await app.type(field(), '3');
  // An invalid value is not remembered.
  field().value = '0';
  field().dispatchEvent(new app.window.InputEvent('input', { bubbles: true }));
  await app.escape(app.$('transaction-suggest-dialog'));
  assert.equal(app.$('transaction-suggest-dialog').open, false);
  await app.click(app.$('transaction-suggest'));
  assert.equal(field().value, '3', 'the dialog opens with the last valid count');
  assert.deepEqual(app.callsOf('suggest_transaction_rules').at(-1).args, { minCount: 3 });
  app.close();
  const fresh = await openSuggest();
  assert.equal(fresh.$('transaction-suggest-form').elements.min_count.value, '5', 'a new start of the app begins with 5 again');
  fresh.close();
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
  assert.equal(box(1).disabled, false, 'a category makes the row tickable');
  assert.equal(box(1).checked, true, 'and choosing it ticks the row');
  assert.equal(app.textOf('transaction-suggest-chosen'), '2 rules chosen');
  await app.choose(suggestRows(app)[1].querySelector('select'), GROCERIES);
  assert.equal(box(1).checked, true, 'changing the category ticks it too');
  await app.click(box(1));
  assert.equal(box(1).checked, false, 'the tick can be removed again');
  await app.choose(suggestRows(app)[1].querySelector('select'), SHOPPING);
  assert.equal(box(1).checked, true, 'and a new choice ticks it again');
  await app.click(box(1));
  assert.equal(app.textOf('transaction-suggest-chosen'), '1 rule chosen');
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
  assert.equal(suggestRows(app)[1].querySelector('input[type="checkbox"]').checked, true, 'choosing the category ticked it');
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
