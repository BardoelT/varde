// CSD-029: the number format. Rust keeps the preference, reads every typed amount and sends
// canonical figures (`get_preferences`, `set_number_format`); these tests check what the screens
// show and what they send.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp, commandError } from './harness.js';
import { exampleReads } from './fixtures.js';

/** The example project with a saved number format. */
function readsIn(numberFormat, language = 'en') {
  const reads = exampleReads();
  reads.get_preferences = { theme: 'system', language, number_format: numberFormat };
  return reads;
}

const settingsPanel = (app) =>
  [...app.document.querySelectorAll('#settings .panel')].find((panel) => app.text(panel.querySelector('.panel-title')) === 'Appearance');

test('test_csd029_ac_1_1_the_choice_and_its_options', async () => {
  const app = await openApp();
  await app.click(app.item('settings'));
  const panel = settingsPanel(app);
  const names = [...panel.querySelectorAll('.setting-name')].map((label) => app.text(label));
  assert.deepEqual(names, ['Theme', 'Language', 'Number format'], 'Number format is below Language');
  const select = app.$('setting-number-format');
  assert.ok(panel.contains(select));
  assert.deepEqual(
    [...select.options].map((option) => [option.value, option.text]),
    [['point_comma', '1,234.50'], ['comma_point', '1.234,50'], ['apostrophe_comma', "1'234,50"]],
  );
  app.close();
});

for (const [format, label] of [['point_comma', '1,234.50'], ['comma_point', '1.234,50'], ['apostrophe_comma', "1'234,50"]]) {
  test(`test_csd029_ac_1_2_the_choice_shows_the_current_format_${format}`, async () => {
    const app = await openApp({ reads: readsIn(format) });
    await app.click(app.item('settings'));
    const select = app.$('setting-number-format');
    assert.equal(select.value, format);
    assert.equal(select.selectedOptions[0].text, label);
    app.close();
  });
}

test('test_csd029_ac_2_1_first_start_writes_amounts_as_1234_50', async () => {
  const app = await openApp();
  assert.equal(app.textOf('bank-account-total-value'), '4,100.50');
  assert.equal(app.$('setting-number-format').value, 'point_comma');
  app.close();
});

test('test_csd029_ac_2_2_the_language_is_not_followed', async () => {
  const app = await openApp({ reads: readsIn('point_comma', 'nl') });
  assert.equal(app.textOf('bank-account-total-value'), '4,100.50');
  app.close();
});

test('test_csd029_ac_3_1_choosing_a_format_changes_every_amount_at_once', async () => {
  const app = await openApp();
  await app.click(app.item('settings'));
  await app.choose(app.$('setting-number-format'), 'comma_point');
  assert.deepEqual(app.callsOf('set_number_format').map((call) => call.args), [{ numberFormat: 'comma_point' }]);
  assert.equal(app.textOf('bank-account-total-value'), '4.100,50');
  await app.choose(app.$('setting-number-format'), 'apostrophe_comma');
  assert.equal(app.textOf('bank-account-total-value'), "4'100,50");
  assert.equal(app.document.querySelector('.confirm-dialog[open]'), null, 'no confirmation is asked');
  app.close();
});

test('test_csd029_ac_3_2_the_choice_is_not_an_unsaved_change', async () => {
  const app = await openApp();
  await app.click(app.item('settings'));
  await app.choose(app.$('setting-number-format'), 'comma_point');
  assert.equal(app.callsOf('save_project').length, 0, 'nothing is written to the project');
  app.close();
});

test('test_csd029_ac_3_3_a_typed_amount_in_an_open_form_is_written_again', async () => {
  const app = await openApp();
  await app.click(app.item('bank'));
  await app.click(app.$('bank-entry-add'));
  const field = app.$('bank-entry-form').elements.balance;
  field.value = '1,234.50';
  await app.click(app.item('settings'));
  await app.choose(app.$('setting-number-format'), 'comma_point');
  assert.equal(field.value, '1.234,50');
  app.close();
});

test('test_csd029_ac_3_4_a_typed_amount_that_cannot_be_read_is_cleared', async () => {
  const app = await openApp();
  await app.click(app.item('bank'));
  await app.click(app.$('bank-entry-add'));
  const form = app.$('bank-entry-form');
  form.elements.balance.value = '12,34.50';
  form.elements.note.value = 'Example note';
  await app.click(app.item('settings'));
  await app.choose(app.$('setting-number-format'), 'comma_point');
  assert.equal(form.elements.balance.value, '');
  assert.equal(form.elements.note.value, 'Example note');
  app.close();
});

test('test_csd029_ac_3_5_choosing_the_current_format_changes_nothing', async () => {
  const app = await openApp({ reads: readsIn('comma_point') });
  await app.click(app.item('settings'));
  await app.choose(app.$('setting-number-format'), 'comma_point');
  assert.equal(app.callsOf('set_number_format').length, 0);
  app.close();
});

test('test_csd029_ac_4_1_the_first_page_is_already_in_the_saved_format', async () => {
  const app = await openApp({ reads: readsIn('comma_point') });
  assert.equal(app.textOf('bank-account-total-value'), '4.100,50');
  app.close();
});

for (const [format, balance] of [['point_comma', '1,234,567.50'], ['comma_point', '1.234.567,50'], ['apostrophe_comma', "1'234'567,50"]]) {
  test(`test_csd029_ac_5_1_an_amount_in_each_format_${format}`, async () => {
    const app = await openApp({ reads: readsIn(format) });
    const { formatAmount } = await app.load('shared/format.js');
    assert.equal(formatAmount('1234567.50'), balance);
    app.close();
  });
}

test('test_csd029_ac_5_2_small_and_negative_amounts', async () => {
  const app = await openApp({ reads: readsIn('comma_point') });
  const { formatAmount } = await app.load('shared/format.js');
  assert.equal(formatAmount('999.00'), '999,00');
  assert.equal(formatAmount('-1234.50'), '-1.234,50');
  assert.equal(formatAmount('0.00'), '0,00');
  app.close();
});

test('test_csd029_ac_5_3_percentages_and_ratios', async () => {
  const app = await openApp({ reads: readsIn('comma_point') });
  const { formatLabel } = await app.load('shared/format.js');
  assert.equal(formatLabel('12.50%'), '12,50%');
  assert.equal(formatLabel('+6.09%'), '+6,09%');
  assert.equal(formatLabel('0.75'), '0,75');
  assert.equal(formatLabel('–'), '–');
  app.close();
});

test('test_csd029_ac_5_4_chart_labels_and_tooltips', async () => {
  const app = await openApp({ reads: readsIn('comma_point') });
  const { formatLabel, formatAmount } = await app.load('shared/format.js');
  assert.equal(formatLabel('1.50k'), '1,50k');
  assert.equal(formatLabel('-2.25M'), '-2,25M');
  assert.equal(formatLabel('500'), '500');
  assert.equal(formatAmount('1234.50'), '1.234,50');
  app.close();
});

test('test_csd029_ac_6_2_what_is_typed_is_sent_as_typed_and_rust_reads_it', async () => {
  const reads = readsIn('comma_point');
  const app = await openApp({ reads, handlers: { add_bank_entry: () => ({}) } });
  await app.click(app.item('bank'));
  await app.click(app.$('bank-entry-add'));
  const form = app.$('bank-entry-form');
  form.elements.period.value = '2026-09';
  form.elements.balance.value = '1.234,50';
  form.requestSubmit?.() ?? form.dispatchEvent(new app.window.Event('submit', { cancelable: true }));
  await app.settle();
  const [call] = app.callsOf('add_bank_entry');
  assert.equal(call?.args.balance, '1.234,50', 'the frontend sends the typed text; Rust normalises it');
  app.close();
});

test('test_csd029_ac_7_2_the_message_shows_the_example_in_the_chosen_format', async () => {
  const app = await openApp({ reads: readsIn('comma_point') });
  const { errorText } = await app.load('shared/ipc.js');
  const message = errorText(commandError('BANK_ENTRY_BALANCE_INVALID', 'Enter the balance as a number, such as 2500.00 or -150.25.'));
  assert.ok(message.includes("2.500,00") && message.includes('-150,25'), message);
  assert.ok(!message.includes('2500.00'), message);
  app.close();
});

for (const [format, placeholder, name, example] of [
  ['point_comma', '0.00', 'a dot', '3000.00'],
  ['comma_point', '0,00', 'a comma', '3000,00'],
  ['apostrophe_comma', '0,00', 'a comma', '3000,00'],
]) {
  test(`test_csd029_ac_8_1_placeholder_and_hint_${format}`, async () => {
    const app = await openApp({ reads: readsIn(format) });
    assert.equal(app.$('job-dialog') === null, false);
    const form = app.$('paycheck-form');
    assert.equal(form.elements.gross_salary.placeholder, placeholder);
    assert.equal(app.textOf('paycheck-hint'), `Use ${name} for decimals, for example ${example}.`);
    app.close();
  });
}

test('test_csd029_ac_8_1_a_change_rewrites_the_hint_without_a_restart', async () => {
  const app = await openApp();
  await app.click(app.item('settings'));
  await app.choose(app.$('setting-number-format'), 'comma_point');
  assert.equal(app.textOf('paycheck-hint'), 'Use a comma for decimals, for example 3000,00.');
  assert.equal(app.$('paycheck-form').elements.gross_salary.placeholder, '0,00');
  app.close();
});

test('test_csd029_ac_8_2_the_hint_in_dutch', async () => {
  const app = await openApp({ reads: readsIn('comma_point', 'nl') });
  assert.equal(app.textOf('paycheck-hint'), 'Gebruik een komma voor decimalen, bijvoorbeeld 3000,00.');
  app.close();
});

test('test_csd029_ac_9_1_a_format_that_cannot_be_used_means_the_default', async () => {
  for (const stored of [undefined, '', 'comma']) {
    const app = await openApp({ reads: readsIn(stored) });
    assert.equal(app.textOf('bank-account-total-value'), '4,100.50', String(stored));
    app.close();
  }
});

test('test_csd029_ac_10_1_a_failed_save_keeps_the_format_and_says_so', async () => {
  const app = await openApp({
    handlers: { set_number_format: () => { throw commandError('PREFERENCES_SAVE_FAILED', 'The preference could not be saved.'); } },
  });
  await app.click(app.item('settings'));
  await app.choose(app.$('setting-number-format'), 'comma_point');
  assert.equal(app.$('setting-number-format').value, 'point_comma');
  assert.equal(app.textOf('bank-account-total-value'), '4,100.50');
  assert.ok(app.text(app.document.querySelector('.toast, #toast')).length > 0, 'a message appears');
  app.close();
});
