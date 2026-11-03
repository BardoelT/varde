// The cheap UI bugs of refactoring step 11: double submit, reload errors, one failing page,
// the Dutch coverage and the wording of the settle dialog (CSD-011 REQ-27/28, CSD-020 REQ-31).
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { JSDOM } from 'jsdom';
import { openApp, commandError } from './harness.js';
import { emptyReads, exampleReads } from './fixtures.js';

const uiDir = fileURLToPath(new URL('..', import.meta.url));
const tick = () => new Promise((resolve) => setImmediate(resolve));

/** A command that stays pending until `release()` is called; `calls` counts how often it ran. */
function slowCommand(answer = {}) {
  const gate = { calls: 0, release: () => {} };
  gate.handler = () => {
    gate.calls += 1;
    return new Promise((resolve) => {
      gate.release = () => resolve(answer);
    });
  };
  return gate;
}

async function fillJob(app) {
  await app.click(app.item('income'));
  await app.click(app.$('job-add-first'));
  const form = app.$('job-form');
  await app.type(form.elements.employer, 'Example Employer');
  await app.type(form.elements.start_date, '01032024');
  return form;
}

// ---- 1. Double submit ----

test('test_ui_two_quick_submits_of_a_form_call_the_command_once', async () => {
  const create = slowCommand({});
  const app = await openApp({ reads: emptyReads(), handlers: { create_job: create.handler } });
  const form = await fillJob(app);
  const submit = form.querySelector('[type="submit"]');
  form.requestSubmit();
  form.requestSubmit();
  submit.click();
  await tick();
  assert.equal(create.calls, 1, 'the second and third submit are ignored while the first is pending');
  assert.equal(submit.disabled, true, 'the button is disabled while the command is pending');
  create.release();
  await app.settle();
  assert.equal(app.callsOf('create_job').length, 1);
  assert.equal(submit.disabled, false, 'the button is enabled again afterwards');
  app.close();
});

test('test_ui_the_submit_button_is_enabled_again_after_an_error', async () => {
  let fail = true;
  const handlers = {
    create_job: () => {
      if (fail) throw commandError('JOB_EMPLOYER_REQUIRED', 'The employer is required.');
      return {};
    },
  };
  const app = await openApp({ reads: emptyReads(), handlers });
  const form = await fillJob(app);
  form.requestSubmit();
  await app.settle();
  assert.equal(app.textOf('job-error'), 'The employer is required.');
  assert.equal(form.querySelector('[type="submit"]').disabled, false);
  fail = false;
  form.requestSubmit();
  await app.settle();
  assert.equal(app.callsOf('create_job').length, 2, 'the user can submit again after the error');
  app.close();
});

test('test_ui_every_other_form_ignores_a_second_submit_too', async () => {
  const cases = [
    ['bank-account-form', 'create_bank_account', 'bank', 'bank-account-add-first', 'name', 'Example Bank'],
    ['expense-form', 'add_expense', 'expenses', 'expense-add', null, null],
  ];
  for (const [formId, command, section, addId, field, value] of cases) {
    const gate = slowCommand({});
    const app = await openApp({ reads: emptyReads(), handlers: { [command]: gate.handler } });
    await app.click(app.item(section));
    const add = app.$(addId) ?? app.document.querySelector(`#${section} .btn.primary`);
    await app.click(add);
    const form = app.$(formId);
    if (field) await app.type(form.elements[field], value);
    form.requestSubmit();
    form.requestSubmit();
    await tick();
    assert.equal(gate.calls, 1, `${formId}: one call for two submits`);
    gate.release();
    await app.settle();
    app.close();
  }
});

// ---- 2. Deleting a paycheck or statement asks nothing (CSD-001 REQ-8) ----

test('test_csd001_ac_8_3_deleting_a_paycheck_asks_no_confirmation', async () => {
  const app = await openApp({ handlers: { delete_paycheck: {}, delete_statement: {} } });
  await app.click(app.item('income'));
  const button = app.document.querySelector('#paycheck-list .icon-btn.danger, #income .icon-btn.danger[aria-label^="Delete"][aria-label*="20"]');
  const target = button ?? [...app.document.querySelectorAll('.icon-btn.danger')].find((candidate) => /paycheck|statement|20\d\d/i.test(candidate.getAttribute('aria-label')));
  assert.ok(target, 'an entry has a Delete button');
  await app.click(target);
  assert.equal(app.$('confirm-dialog').open, false, 'no confirmation was asked');
  assert.equal(app.callsOf('delete_paycheck').length + app.callsOf('delete_statement').length, 1);
  app.close();
});

// ---- 3. A failing reload after a save is shown ----

test('test_ui_a_failing_reload_after_a_save_shows_a_toast_and_leaves_the_dialog_closed', async () => {
  let broken = false;
  const reads = emptyReads();
  const handlers = {
    create_job: () => {
      broken = true;
      return {};
    },
    list_jobs: () => {
      if (broken) throw commandError('TEST_RELOAD', 'The project could not be read.');
      return [];
    },
  };
  const app = await openApp({ reads, handlers });
  const form = await fillJob(app);
  form.requestSubmit();
  await app.settle();
  assert.equal(app.callsOf('create_job').length, 1, 'the save went through');
  assert.equal(app.$('job-dialog').open, false, 'the dialog closed after the save');
  assert.equal(app.isShown(app.$('job-error')), false, 'no error in the hidden box');
  assert.equal(app.isShown(app.$('toast')), true, 'the failure is shown in the toast');
  assert.equal(app.textOf('toast'), 'The project could not be read.');
  assert.equal(form.querySelector('[type="submit"]').disabled, false);
  app.close();
});

// ---- 4. One failing page does not stop the others ----

test('test_ui_one_failing_page_at_start_does_not_stop_the_others', async () => {
  const handlers = {
    list_investment_accounts: () => {
      throw commandError('TEST_PAGE', 'The investments could not be read.');
    },
  };
  const app = await openApp({ handlers });
  assert.ok(app.callsOf('list_bank_accounts').length >= 1, 'the pages after the failing one still load');
  assert.ok(app.callsOf('list_expenses').length >= 1);
  assert.ok(app.callsOf('get_dashboard').length >= 1);
  assert.equal(app.isShown(app.$('toast')), true);
  assert.match(app.textOf('toast'), /The investments could not be read\./);
  app.close();
});

test('test_ui_one_failing_page_after_a_new_project_does_not_stop_the_others', async () => {
  let replaced = false;
  const handlers = {
    new_project: () => {
      replaced = true;
      return { has_path: false, unsaved_changes: false };
    },
    list_bank_accounts: () => {
      if (replaced) throw commandError('TEST_PAGE', 'The bank accounts could not be read.');
      return exampleReads().list_bank_accounts;
    },
  };
  const app = await openApp({ handlers });
  const before = app.callsOf('list_expenses').length;
  const dashboardBefore = app.callsOf('get_dashboard').length;
  await app.emit('menu-action', 'project-new');
  assert.ok(app.callsOf('list_expenses').length > before, 'expenses reloaded after the failing page');
  assert.ok(app.callsOf('get_dashboard').length > dashboardBefore, 'the dashboard reloaded too');
  assert.match(app.textOf('toast'), /The bank accounts could not be read\./);
  app.close();
});

// ---- 5. Dutch coverage ----

const unescape = (literal) => literal.replace(/\\(.)/g, (_all, char) => ({ n: '\n' })[char] ?? char);
const normalise = (text) => text.replace(/\s+/g, ' ').trim();
const STRING = String.raw`'((?:\\.|[^'\\])*)'|"((?:\\.|[^"\\])*)"`;
const literal = (match, first) => unescape(match[first] ?? match[first + 1]);

/** The English texts a script passes to `t()` or `tn()`. */
function translatableCalls(source) {
  const found = [];
  for (const match of source.matchAll(new RegExp(String.raw`\bt\(\s*(?:${STRING})`, 'g'))) found.push(literal(match, 1));
  for (const match of source.matchAll(new RegExp(String.raw`\btn\(\s*(?:${STRING})\s*,\s*(?:${STRING})`, 'g'))) {
    found.push(literal(match, 1), literal(match, 3));
  }
  return found;
}

/** The static texts of index.html, as `collectStaticTexts` notes them. */
function staticTexts() {
  const { document, NodeFilter, Node } = new JSDOM(readFileSync(`${uiDir}index.html`, 'utf8')).window;
  const texts = [];
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT | NodeFilter.SHOW_ELEMENT);
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    if (node.nodeType === Node.TEXT_NODE) {
      if (!['SCRIPT', 'STYLE'].includes(node.parentElement.tagName)) texts.push(normalise(node.data));
    } else {
      for (const attribute of ['placeholder', 'aria-label', 'title', 'alt']) {
        if (node.getAttribute(attribute)) texts.push(normalise(node.getAttribute(attribute)));
      }
    }
  }
  return texts.filter(Boolean);
}

/** Texts with no word to translate: amounts, symbols, units, codes. */
const NOT_TEXT = /^[^\p{L}]*$|^(EUR|USD|GBP|dd\/mm\/yyyy|YYYY-MM|Varde|English|Nederlands)$/u;

test('test_csd022_every_translatable_text_has_a_dutch_version', async () => {
  const nl = (await import(`${uiDir}locales/nl.js`)).default;
  const missing = new Set();
  for (const text of staticTexts()) if (!NOT_TEXT.test(text) && !(text in nl)) missing.add(`index.html: ${text}`);
  for (const folder of ['pages', 'shared']) {
    for (const file of readdirSync(`${uiDir}${folder}`)) {
      for (const text of translatableCalls(readFileSync(`${uiDir}${folder}/${file}`, 'utf8'))) {
        if (!NOT_TEXT.test(text) && !(text in nl)) missing.add(`${folder}/${file}: ${text}`);
      }
    }
  }
  assert.deepEqual([...missing], [], 'texts without a Dutch entry');
});

// ---- 6. The settle dialog hint and the term "wealth accumulation from cash" ----

const openDebts = [{ debt_id: 'debt-1', debt_name: 'Example Mortgage', amount: '190000.00' }];

test('test_csd020_ac_31_1_the_settle_hint_says_repaid_counts_in_full_and_cancelled_nets_to_zero', async () => {
  const app = await openApp();
  const { askSettlements } = await app.load('shared/settle.js');
  const answer = askSettlements(openDebts);
  const hint = app.$('house-settle-dialog').querySelector('.field-hint');
  assert.equal(app.text(hint), 'Repaid counts in full in the wealth accumulation from cash, Cancelled nets to zero.');
  assert.equal(app.textOf('house-settle-title'), 'Settle linked debts');
  assert.equal(app.$('house-settle-save').disabled, true);
  await app.click(app.$('house-settle-back'));
  assert.equal(await answer, null);
  app.close();
});

test('test_csd020_ac_31_1_the_settle_hint_in_dutch', async () => {
  const reads = exampleReads();
  reads.get_preferences = { theme: 'system', language: 'nl' };
  const app = await openApp({ reads });
  const hint = app.$('house-settle-dialog').querySelector('.field-hint');
  assert.equal(
    app.text(hint),
    'Afgelost telt volledig mee in de vermogensopbouw uit contant geld, Kwijtgescholden telt voor nul.',
  );
  app.close();
});

test('test_ui_wealth_accumulation_from_cash_is_the_one_term_and_cancelled_does_not_is_gone', async () => {
  const files = [`${uiDir}index.html`];
  for (const folder of ['pages', 'shared', 'locales']) for (const file of readdirSync(`${uiDir}${folder}`)) files.push(`${uiDir}${folder}/${file}`);
  for (const file of files) {
    const source = readFileSync(file, 'utf8');
    assert.doesNotMatch(source, /Cancelled does not/, `${file}: old settle wording`);
    assert.doesNotMatch(source, /wealth growth/i, `${file}: "wealth growth" for the wealth accumulation from cash`);
    assert.doesNotMatch(source, /wealth accumulation(?! from cash)/i, `${file}: say "wealth accumulation from cash"`);
  }
});
