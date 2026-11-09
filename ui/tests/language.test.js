// CSD-022: the language of the app's own texts. Rust keeps the preference (`get_preferences`,
// `set_language`); these tests check what the screens show.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp, commandError } from './harness.js';
import { exampleReads } from './fixtures.js';

/** The example project with a saved language. */
function readsIn(language) {
  const reads = exampleReads();
  reads.get_preferences = { theme: 'system', language };
  return reads;
}

const navTexts = (app) => [...app.document.querySelectorAll('.nav-item')].map((item) => app.text(item));

test('test_csd022_ac_1_1_the_language_choice_and_its_options', async () => {
  const app = await openApp();
  await app.click(app.item('settings'));
  const panel = [...app.document.querySelectorAll('#settings .panel')].find((candidate) => app.text(candidate.querySelector('.panel-title')) === 'Appearance');
  assert.ok(panel, 'the "Appearance" panel');
  const names = [...panel.querySelectorAll('.setting-name')].map((label) => app.text(label));
  assert.deepEqual(names.slice(0, 2), ['Theme', 'Language'], 'Language is below Theme');
  const select = app.$('setting-language');
  assert.ok(panel.contains(select));
  assert.equal(panel.querySelector('label[for="setting-language"]').textContent, 'Language');
  assert.deepEqual([...select.options].map((option) => [option.value, option.text]), [['en', 'English'], ['nl', 'Nederlands']]);
  app.close();
});

for (const [language, label] of [['en', 'English'], ['nl', 'Nederlands']]) {
  test(`test_csd022_ac_1_2_the_choice_shows_the_current_language_${language}`, async () => {
    const app = await openApp({ reads: readsIn(language) });
    await app.click(app.item('settings'));
    const select = app.$('setting-language');
    assert.equal(select.value, language);
    assert.equal(select.selectedOptions[0].text, label);
    app.close();
  });
}

test('test_csd022_ac_2_1_first_start_is_in_english', async () => {
  const app = await openApp();
  assert.equal(app.document.documentElement.lang, 'en');
  assert.equal(app.text(app.item('debts')), 'Debts');
  assert.equal(app.textOf('project-state-text'), 'All changes saved');
  assert.equal(app.$('setting-language').value, 'en');
  app.close();
});

test('test_csd022_ac_3_1_choosing_dutch_changes_every_page_at_once', async () => {
  const app = await openApp();
  await app.click(app.item('settings'));
  await app.choose(app.$('setting-language'), 'nl');
  assert.deepEqual(app.callsOf('set_language').map((call) => call.args), [{ language: 'nl' }]);
  assert.equal(app.document.documentElement.lang, 'nl');
  assert.equal(app.textOf('settings-title') || app.text(app.document.querySelector('#settings .section-title')), 'Instellingen');
  assert.equal(app.text(app.item('debts')), 'Schulden');
  assert.equal(app.text(app.document.querySelector('#income .section-title')), 'Inkomen');
  assert.equal(app.text(app.document.querySelector('#ratios .section-title')), 'Kengetallen');
  assert.equal(app.$('confirm-dialog').open, false, 'no confirmation is asked');
  app.close();
});

test('test_csd022_ac_3_2_the_choice_is_not_an_unsaved_project_change', async () => {
  const app = await openApp();
  await app.click(app.item('settings'));
  await app.choose(app.$('setting-language'), 'nl');
  assert.equal(app.$('project-state').classList.contains('is-unsaved'), false, 'the unsaved indicator is not shown');
  assert.equal(app.textOf('project-state-text'), 'Alle wijzigingen opgeslagen');
  assert.equal(app.callsOf('save_project').length, 0);
  app.close();
});

test('test_csd022_ac_3_3_data_the_user_typed_is_unchanged', async () => {
  const app = await openApp();
  const bankText = () => app.text(app.$('bank'));
  assert.ok(bankText().includes('Example Current Account'));
  await app.click(app.item('settings'));
  await app.choose(app.$('setting-language'), 'nl');
  assert.ok(bankText().includes('Example Current Account'), 'the account is still shown as typed');
  assert.ok(app.text(app.$('income')).includes('Example Employer'));
  app.close();
});

test('test_csd022_ac_3_4_choosing_the_current_language_changes_nothing', async () => {
  const app = await openApp({ reads: readsIn('nl') });
  await app.click(app.item('settings'));
  await app.choose(app.$('setting-language'), 'nl');
  assert.equal(app.callsOf('set_language').length, 0, 'nothing is saved again');
  assert.equal(app.$('setting-language').value, 'nl');
  assert.equal(app.text(app.item('debts')), 'Schulden');
  app.close();
});

test('test_csd022_ac_4_1_the_first_page_is_already_in_dutch', async () => {
  const app = await openApp({ reads: readsIn('nl') });
  assert.equal(app.calls[0].command, 'get_preferences', 'the preference is the first thing the app asks');
  assert.equal(app.document.documentElement.lang, 'nl');
  assert.equal(app.text(app.document.querySelector('#dashboard .section-title')), 'Dashboard');
  assert.equal(app.text(app.document.querySelector('.nav-section-label')), 'Overzicht');
  assert.equal(app.textOf('project-state-text'), 'Alle wijzigingen opgeslagen');
  app.close();
});

test('test_csd022_ac_4_2_same_language_for_another_project', async () => {
  const reads = readsIn('nl');
  const handlers = { open_project: { done: true, status: { has_path: true, unsaved_changes: false } } };
  const app = await openApp({ reads, handlers });
  await app.emit('menu-action', 'project-open');
  assert.equal(app.callsOf('open_project').length, 1);
  assert.equal(app.document.documentElement.lang, 'nl');
  assert.equal(app.text(app.item('debts')), 'Schulden');
  assert.equal(app.callsOf('get_preferences').length, 1, 'the language is not asked again, it is the same for every project');
  app.close();
});

test('test_csd022_ac_5_1_a_page_in_dutch', async () => {
  const app = await openApp({ reads: readsIn('nl') });
  await app.click(app.item('ratios'));
  assert.equal(app.text(app.document.querySelector('#ratios .section-title')), 'Kengetallen');
  assert.equal(app.text(app.document.querySelector('#ratios .panel-title')), 'Kengetallen per jaar');
  const labels = [...app.document.querySelectorAll('#ratios-rows th[scope="row"]')].map((cell) => app.text(cell));
  assert.ok(labels.includes('Groei'), 'sub-heading');
  assert.ok(labels.includes('Vrij inkomen'), 'sub-heading');
  assert.ok(labels.includes('Totaal inkomen (bruto)'), 'row label');
  assert.ok(labels.includes('Groei van totaal vermogen'), 'row label');
  assert.ok(!labels.includes('Total income (gross)'), 'no English row label');

  const info = app.document.querySelector('#ratios-rows .info-btn');
  await app.click(info);
  assert.equal(
    app.textOf('ratios-info'),
    'Het brutoinkomen van het jaar uit alle salarisstroken, vakantiegeld en bonussen. 0.00 als het jaar geen inkomen heeft.',
  );
  app.close();
});

test('test_csd022_ac_5_2_a_dialog_in_dutch', async () => {
  const app = await openApp({ reads: readsIn('nl') });
  await app.click(app.item('income'));
  const deleteJob = app.document.querySelector('#job-list [aria-label="Example Employer verwijderen"]');
  assert.ok(deleteJob, 'the delete button of the job, in Dutch');
  await app.click(deleteJob);
  assert.equal(app.$('confirm-dialog').open, true);
  assert.equal(app.textOf('confirm-title'), 'Example Employer verwijderen?');
  assert.equal(app.textOf('confirm-message'), 'Deze baan en alle bijbehorende salarisstroken en overzichten worden verwijderd.');
  assert.equal(app.textOf('confirm-ok'), 'Baan verwijderen');
  assert.equal(app.textOf('confirm-cancel'), 'Annuleren');
  app.close();
});

test('test_csd022_ac_6_1_a_missing_dutch_text_shows_in_english', async () => {
  const app = await openApp({ reads: readsIn('nl') });
  const { t } = await app.load('shared/i18n.js');
  const text = 'A sentence that no dictionary has';
  assert.equal(t(text), text, 'the English text, not a key and not an empty string');
  assert.equal(t('A sentence about {thing} that no dictionary has', { thing: 'rain' }), 'A sentence about rain that no dictionary has');
  assert.equal(t('Debts'), 'Schulden', 'a text that has a Dutch version is translated');
  app.close();
});

test('test_csd022_ac_7_1_a_known_error_in_dutch', async () => {
  const handlers = {
    delete_job: () => {
      throw commandError('JOB_NOT_FOUND', 'The job was not found.');
    },
  };
  const app = await openApp({ reads: readsIn('nl'), handlers });
  await app.click(app.item('income'));
  await app.click(app.document.querySelector('#job-list [aria-label="Example Employer verwijderen"]'));
  await app.click(app.$('confirm-ok'));
  assert.equal(app.$('toast').hidden, false);
  assert.equal(app.textOf('toast'), 'Deze baan bestaat niet.');
  app.close();
});

test('test_csd022_ac_7_2_an_unknown_code_shows_the_english_message_from_rust', async () => {
  const handlers = {
    delete_job: () => {
      throw commandError('SOME_NEW_CODE', 'Something new went wrong.');
    },
  };
  const app = await openApp({ reads: readsIn('nl'), handlers });
  await app.click(app.item('income'));
  await app.click(app.document.querySelector('#job-list [aria-label="Example Employer verwijderen"]'));
  await app.click(app.$('confirm-ok'));
  assert.equal(app.textOf('toast'), 'Something new went wrong.');
  app.close();
});

test('test_csd022_ac_9_1_a_failed_save_keeps_the_language_and_shows_a_message', async () => {
  const message = 'The preferences could not be saved.';
  const handlers = {
    set_language: () => {
      throw commandError('PREFERENCES_SAVE_FAILED', message);
    },
  };
  const app = await openApp({ handlers });
  await app.click(app.item('settings'));
  await app.choose(app.$('setting-language'), 'nl');
  assert.equal(app.document.documentElement.lang, 'en', 'the app is still in English');
  assert.equal(app.text(app.item('debts')), 'Debts');
  assert.equal(app.$('setting-language').value, 'en', 'the choice shows English again');
  assert.equal(app.$('toast').hidden, false);
  assert.equal(app.textOf('toast'), message);
  assert.ok(!app.textOf('toast').includes('/'), 'no file path in the message');
  app.close();
});
