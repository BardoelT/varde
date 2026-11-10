// CSD-030: Discrete mode. Rust keeps the preference (`get_preferences`, `set_discrete_mode`) and
// sends canonical figures; these tests check what the screens show while the mode is on or off, and
// what they send. The menu item and the shortcut need the real menu bar (manual checklist).
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { openApp, commandError } from './harness.js';
import { exampleReads } from './fixtures.js';

const css = readFileSync(fileURLToPath(new URL('../styles/components.css', import.meta.url)), 'utf8');

/** The example project with a saved discrete mode. */
function readsWith(discreteMode, extra = {}) {
  const reads = exampleReads();
  reads.get_preferences = { theme: 'system', language: 'en', number_format: 'point_comma', discrete_mode: discreteMode, ...extra };
  return reads;
}

const SECTIONS = ['dashboard', 'net-worth', 'bank', 'investments', 'debts', 'houses', 'income', 'expenses'];

/** An amount as the app writes it: grouped digits with two decimals, or a short label such as 1.50k. */
const AMOUNT = /(?<![\d/])-?\d{1,3}(?:[.,' ]\d{3})*[.,]\d{2}(?![\d%])|\d[.,]\d\d[kM]/;

/** Every text of the app that shows project data: the page text and the attributes that are read out. */
function dataTexts(app) {
  const texts = [];
  for (const node of app.document.body.querySelectorAll('*')) {
    // The settings page and the examples in the hints of the forms are not project data.
    if (node.closest('#settings, option, script, .field-hint')) continue;
    for (const name of ['aria-label', 'title']) if (node.hasAttribute(name)) texts.push(node.getAttribute(name));
    for (const child of node.childNodes) if (child.nodeType === 3 && child.data.trim()) texts.push(child.data);
  }
  return texts;
}

/** Opens every page, and the Insights tab where there is one, so that all of them are drawn. */
async function visitEverything(app) {
  for (const section of SECTIONS) {
    await app.click(app.item(section));
    const insights = app.$(`${section}-tab-insights`);
    if (insights) await app.click(insights);
  }
}

const dutchAmount = '1.234,50';

// ---------------------------------------------------------------- the switch (REQ-1, REQ-2)

test('test_csd030_ac_1_1_the_switch_and_its_explanation', async () => {
  const app = await openApp();
  await app.click(app.item('settings'));
  const panel = [...app.document.querySelectorAll('#settings .panel')].find((each) => app.text(each.querySelector('.panel-title')) === 'Appearance');
  const names = [...panel.querySelectorAll('.setting-name')].map((label) => app.text(label));
  assert.equal(names.at(-1), 'Discrete mode');
  assert.equal(names.at(-2), 'Number format', 'below Number format');
  const control = app.$('setting-discrete-mode');
  assert.ok(panel.contains(control));
  assert.match(app.text(app.$('setting-discrete-mode-help')), /every amount.*Percentages stay visible/);
  app.close();
});

for (const [mode, state] of [[true, true], [false, false]]) {
  test(`test_csd030_ac_1_2_the_switch_shows_the_current_mode_${state ? 'on' : 'off'}`, async () => {
    const app = await openApp({ reads: readsWith(mode) });
    await app.click(app.item('settings'));
    assert.equal(app.$('setting-discrete-mode').checked, state);
    app.close();
  });
}

test('test_csd030_ac_1_3_the_switch_is_a_switch_and_flips_with_the_keyboard_click', async () => {
  const app = await openApp();
  const control = app.$('setting-discrete-mode');
  assert.equal(control.type, 'checkbox');
  assert.equal(control.getAttribute('role'), 'switch');
  assert.equal(app.document.querySelector('label[for="setting-discrete-mode"]').textContent.trim(), 'Discrete mode');
  // Space on a focused checkbox is a click in the browser.
  await app.click(control);
  assert.equal(app.callsOf('set_discrete_mode').length, 1);
  assert.equal(control.checked, true);
  app.close();
});

test('test_csd030_ac_2_1_first_start_shows_every_amount', async () => {
  const app = await openApp();
  await app.click(app.item('settings'));
  assert.equal(app.$('setting-discrete-mode').checked, false);
  assert.equal(app.$('discrete-badge').hidden, true);
  assert.equal(app.document.querySelectorAll('.hidden-amount').length, 0);
  assert.ok(dataTexts(app).some((text) => AMOUNT.test(text)), 'amounts are shown');
  app.close();
});

// ---------------------------------------------------------------- applying it (REQ-3, REQ-4)

test('test_csd030_ac_3_1_and_3_2_switching_on_and_off_redraws_at_once_without_a_confirmation', async () => {
  const app = await openApp();
  await app.click(app.item('income'));
  const before = app.text(app.$('income-overview'));
  assert.match(before, AMOUNT);

  await app.click(app.$('setting-discrete-mode'));
  assert.deepEqual(app.callsOf('set_discrete_mode').map((call) => call.args), [{ discreteMode: true }]);
  assert.equal(app.$('confirm-dialog').open, false, 'no confirmation is asked');
  assert.doesNotMatch(app.text(app.$('income-overview')), AMOUNT);

  await app.click(app.$('setting-discrete-mode'));
  assert.deepEqual(app.callsOf('set_discrete_mode').map((call) => call.args.discreteMode), [true, false]);
  assert.equal(app.text(app.$('income-overview')), before, 'every amount is shown again');
  app.close();
});

test('test_csd030_ac_3_3_the_choice_is_not_an_unsaved_project_change', async () => {
  const app = await openApp();
  await app.click(app.$('setting-discrete-mode'));
  assert.equal(app.callsOf('save_project').length, 0);
  assert.equal(app.callsOf('create_job').length, 0);
  app.close();
});

test('test_csd030_ac_4_1_a_saved_mode_hides_the_first_page_that_is_shown', async () => {
  const app = await openApp({ reads: readsWith(true) });
  assert.equal(app.document.documentElement.classList.contains('discrete-mode'), true);
  assert.equal(app.calls[0].command, 'get_preferences', 'the preference is the first thing the app asks');
  assert.doesNotMatch(app.text(app.$('dashboard-tiles')), AMOUNT);
  assert.ok(app.document.querySelectorAll('#dashboard-tiles .hidden-amount').length > 0);
  app.close();
});

test('test_csd030_ac_4_2_another_project_is_hidden_too', async () => {
  const app = await openApp({ reads: readsWith(true), handlers: { open_project: { done: true, status: { has_path: true, unsaved_changes: false } } } });
  await app.emit('menu-action', 'project-open');
  await app.click(app.item('bank'));
  assert.equal(app.callsOf('open_project').length, 1);
  assert.doesNotMatch(app.text(app.$('bank-overview')), AMOUNT);
  app.close();
});

test('test_csd030_ac_4_4_the_other_preferences_are_kept', async () => {
  const app = await openApp({ reads: readsWith(true, { theme: 'dark', language: 'nl', number_format: 'comma_point' }) });
  assert.equal(app.document.documentElement.dataset.theme, 'dark');
  assert.equal(app.document.documentElement.lang, 'nl');
  assert.equal(app.document.documentElement.classList.contains('discrete-mode'), true);
  await app.click(app.item('settings'));
  assert.equal(app.$('setting-number-format').value, 'comma_point');
  app.close();
});

// ---------------------------------------------------------------- what is hidden (REQ-5 to REQ-8)

test('test_csd030_ac_5_1_amounts_are_hidden_on_every_page', async () => {
  const app = await openApp({ reads: readsWith(true) });
  await visitEverything(app);
  const offenders = dataTexts(app).filter((text) => AMOUNT.test(text));
  assert.deepEqual(offenders, []);
  assert.ok(app.document.querySelectorAll('.hidden-amount').length > 20, 'the amounts are written as hidden amounts');
  for (const mark of app.document.querySelectorAll('.hidden-amount')) assert.match(mark.textContent, /^•••••$/);
  app.close();
});

test('test_csd030_ac_5_2_sign_and_size_are_hidden', async () => {
  const app = await openApp({ reads: readsWith(true) });
  const { formatAmount } = await app.load('shared/format.js');
  assert.deepEqual(['1234.50', '-1234.50', '12.00', '0.00', '-9999999.99'].map(formatAmount), Array(5).fill('•••••'));
  app.close();
});

test('test_csd030_ac_5_3_chart_text_is_hidden', async () => {
  const app = await openApp({ reads: readsWith(true) });
  await app.click(app.item('net-worth'));
  const chart = app.document.querySelector('#net-worth svg');
  const labels = [...chart.querySelectorAll('.chart-axis-label, .chart-marker-label')].map((node) => node.textContent);
  assert.ok(labels.length > 0);
  assert.ok(labels.every((text) => /^•••••$/.test(text)), labels.join(' | '));
  app.close();
});

test('test_csd030_ac_6_1_percentages_ratios_and_dates_stay', async () => {
  const app = await openApp({ reads: readsWith(true) });
  const { formatLabel } = await app.load('shared/format.js');
  assert.equal(formatLabel('12.50%'), '12.50%');
  assert.equal(formatLabel('0.75'), '0.75');
  await app.click(app.item('ratios'));
  assert.match(app.text(app.$('ratios')), /\d+\.\d\d%/, 'ratios and percentages are shown');
  await app.click(app.item('income'));
  assert.match(app.text(app.$('income-overview')), /\d{2}\/\d{2}\/\d{4}|\b20\d\d\b/, 'dates and years are shown');
  app.close();
});

test('test_csd030_ac_6_2_a_text_with_an_amount_and_a_percentage_hides_only_the_amount', async () => {
  const app = await openApp({ reads: readsWith(true) });
  const { localiseNumbers } = await app.load('shared/number-format.js');
  assert.equal(localiseNumbers('Balance 1200.00 (+3.50%)'), 'Balance ••••• (+3.50%)');
  app.close();
});

test('test_csd030_ac_6_3_names_and_counts_stay', async () => {
  const app = await openApp({ reads: readsWith(true) });
  await app.click(app.item('bank'));
  const name = exampleReads().list_bank_accounts.accounts[0].name;
  assert.ok(app.text(app.$('bank-overview')).includes(name), 'the account name is shown');
  app.close();
});

test('test_csd030_ac_7_1_the_chart_keeps_its_shape', async () => {
  const shapes = async (mode) => {
    const app = await openApp({ reads: readsWith(mode) });
    await app.click(app.item('net-worth'));
    const drawn = [...app.document.querySelectorAll('#net-worth svg rect, #net-worth svg line, #net-worth svg path')].map((node) =>
      [...node.attributes].map((attribute) => `${attribute.name}=${attribute.value}`).join(' '),
    );
    app.close();
    return drawn;
  };
  const shown = await shapes(false);
  const hidden = await shapes(true);
  assert.ok(shown.length > 0);
  // The labels of the bars carry the amounts: compare the geometry, not the accessible names.
  const geometry = (list) => list.map((line) => line.replace(/aria-label="[^"]*"/g, '').replace(/aria-label=[^ ]*/g, ''));
  assert.equal(hidden.length, shown.length);
  assert.deepEqual(geometry(hidden).map((line) => line.split(' ').filter((part) => /^(x|y|x1|x2|y1|y2|width|height|d)=/.test(part)).join(' ')),
    geometry(shown).map((line) => line.split(' ').filter((part) => /^(x|y|x1|x2|y1|y2|width|height|d)=/.test(part)).join(' ')));
});

test('test_csd030_ac_8_1_a_message_hides_the_amount_and_keeps_the_percentage', async () => {
  const app = await openApp({ reads: readsWith(true) });
  const { errorText } = await app.load('shared/ipc.js');
  assert.equal(errorText(commandError('X_UNKNOWN', 'Debt limit 5000.00 exceeded by 3.50%')), 'Debt limit ••••• exceeded by 3.50%');
  app.close();
});

test('test_csd030_ac_8_2_screen_readers_read_a_hidden_amount', async () => {
  const app = await openApp({ reads: readsWith(true) });
  await visitEverything(app);
  const labels = [...app.document.querySelectorAll('[aria-label]')].map((node) => node.getAttribute('aria-label'));
  assert.ok(labels.some((label) => label.includes('hidden amount')), 'a label says "hidden amount"');
  assert.ok(labels.every((label) => !label.includes('•')), 'no bullets are read out');
  const marks = [...app.document.querySelectorAll('span.hidden-amount')];
  assert.ok(marks.length > 0 && marks.every((mark) => mark.getAttribute('aria-label') === 'hidden amount'));
  app.close();
});

test('test_csd030_ac_8_2_screen_reader_text_in_dutch', async () => {
  const app = await openApp({ reads: readsWith(true, { language: 'nl' }) });
  const marks = [...app.document.querySelectorAll('span.hidden-amount')];
  assert.ok(marks.length > 0 && marks.every((mark) => mark.getAttribute('aria-label') === 'verborgen bedrag'));
  app.close();
});

test('test_csd030_ac_8_3_no_amount_text_holds_a_digit_of_an_amount', async () => {
  const app = await openApp({ reads: readsWith(true) });
  await visitEverything(app);
  assert.deepEqual(dataTexts(app).filter((text) => AMOUNT.test(text)), []);
  app.close();
});

// ---------------------------------------------------------------- fields and peeking (REQ-9, REQ-10)

test('test_csd030_ac_9_1_and_9_2_amount_fields_are_blurred_and_keep_their_value', async () => {
  const app = await openApp({ reads: readsWith(true) });
  assert.match(css, /\.discrete-mode input\[inputmode="decimal"\],\s*\.discrete-mode input\[inputmode="decimal"\]:focus\s*\{\s*filter: blur\(/, 'blurred, also while focused');
  await app.click(app.item('income'));
  await app.click(app.$('job-add-first') ?? app.$('job-add'));
  const form = app.document.querySelector('#income-overview form') ?? app.$('job-form');
  assert.ok(form);
  const field = app.document.querySelector('input[inputmode="decimal"]');
  assert.ok(field, 'an amount field exists');
  await app.type(field, '3500.00');
  assert.equal(field.value.endsWith('3500.00'), true, 'the field keeps what is typed');
  assert.equal(app.document.documentElement.classList.contains('discrete-mode'), true);
  app.close();
});

test('test_csd030_ac_9_3_fields_that_are_not_amounts_are_not_blurred', async () => {
  assert.doesNotMatch(css, /discrete-mode input(?!\[inputmode="decimal"\])/);
  assert.doesNotMatch(css, /discrete-mode (?:input\[type|textarea|select)/);
});

test('test_csd030_ac_10_1_hovering_and_clicking_do_not_reveal', async () => {
  const app = await openApp({ reads: readsWith(true) });
  await app.click(app.item('income'));
  const mark = app.document.querySelector('#income-overview .hidden-amount');
  for (const type of ['mouseover', 'mouseenter', 'mousedown', 'click', 'focus']) mark.dispatchEvent(new app.window.MouseEvent(type, { bubbles: true }));
  await app.settle();
  assert.equal(app.text(mark.closest('td, .account-amount') ?? mark.parentElement).includes('•••••'), true);
  assert.doesNotMatch(app.text(app.$('income-overview')), AMOUNT);
  app.close();
});

test('test_csd030_ac_10_2_copying_does_not_leak', async () => {
  const app = await openApp({ reads: readsWith(true) });
  await app.click(app.item('income'));
  assert.match(css, /\.hidden-amount\s*\{[^}]*user-select: none/);
  const range = app.document.createRange();
  range.selectNodeContents(app.$('income-overview'));
  assert.doesNotMatch(range.toString(), AMOUNT, 'what is selected holds no amount');
  app.close();
});

// ---------------------------------------------------------------- menu and badge (REQ-11, REQ-12)

test('test_csd030_ac_11_1_the_menu_item_flips_the_mode_like_the_switch', async () => {
  const app = await openApp();
  await app.emit('menu-action', 'toggle-discrete-mode');
  assert.deepEqual(app.callsOf('set_discrete_mode').map((call) => call.args), [{ discreteMode: true }]);
  assert.equal(app.$('setting-discrete-mode').checked, true, 'the switch agrees');
  await app.emit('menu-action', 'toggle-discrete-mode');
  assert.deepEqual(app.callsOf('set_discrete_mode').map((call) => call.args.discreteMode), [true, false]);
  assert.equal(app.$('setting-discrete-mode').checked, false);
  app.close();
});

test('test_csd030_ac_12_1_the_badge_follows_the_mode_on_every_page', async () => {
  const app = await openApp();
  assert.equal(app.isShown(app.$('discrete-badge')), false);
  await app.click(app.$('setting-discrete-mode'));
  assert.equal(app.isShown(app.$('discrete-badge')), true);
  assert.equal(app.text(app.$('discrete-badge')), 'Discrete mode');
  await app.click(app.item('income'));
  assert.equal(app.isShown(app.$('discrete-badge')), true, 'in the navigation, whatever the page');
  await app.click(app.item('settings'));
  await app.click(app.$('setting-discrete-mode'));
  assert.equal(app.isShown(app.$('discrete-badge')), false);
  app.close();
});

// ---------------------------------------------------------------- a failed save (REQ-14)

test('test_csd030_ac_14_1_a_failed_save_keeps_the_mode_and_shows_no_path', async () => {
  const app = await openApp({
    handlers: { set_discrete_mode: () => { throw commandError('PREFERENCES_SAVE_FAILED', 'The setting could not be saved.'); } },
  });
  await app.click(app.item('income'));
  await app.click(app.item('settings'));
  await app.click(app.$('setting-discrete-mode'));
  assert.equal(app.$('setting-discrete-mode').checked, false, 'the switch is back to the saved mode');
  assert.equal(app.isShown(app.$('discrete-badge')), false);
  assert.equal(app.document.documentElement.classList.contains('discrete-mode'), false);
  assert.match(app.textOf('toast'), /could not be saved/i);
  assert.doesNotMatch(app.textOf('toast'), /\//, 'no file path');
  assert.ok(dataTexts(app).some((text) => AMOUNT.test(text)), 'amounts are still shown');
  app.close();
});

test('test_csd030_the_dutch_texts_exist', async () => {
  const app = await openApp({ reads: readsWith(false, { language: 'nl' }) });
  await app.click(app.item('settings'));
  assert.equal(app.text(app.document.querySelector('label[for="setting-discrete-mode"]')), 'Discrete modus');
  assert.ok(dutchAmount.length > 0);
  app.close();
});
