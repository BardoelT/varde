// Display ACs of CSD-023 (in-app updates): the Updates section of the Settings page. Rust decides
// what a check or an install finds; these tests feed canned answers and check only what is shown.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp, commandError } from './harness.js';
import { exampleReads } from './fixtures.js';

/** An app on the Settings page. `handlers` answer the update commands. */
async function settingsApp(handlers = {}, { unsaved = false, language = 'en' } = {}) {
  const reads = exampleReads();
  reads.project_status = { has_path: true, unsaved_changes: unsaved };
  reads.get_preferences = { ...reads.get_preferences, language };
  const app = await openApp({ reads, handlers });
  await app.click(app.item('settings'));
  return app;
}

const check = (app) => app.$('update-check');
const install = (app) => app.$('update-install');
const status = (app) => app.textOf('update-status');

/** A command that answers only when the test says so, to look at the page while a run is active. */
function held() {
  let release;
  let fail;
  const promise = new Promise((resolve, reject) => {
    release = resolve;
    fail = reject;
  });
  return { promise, release, fail };
}

/** Lets the page run while a held command is still pending: `settle()` would wait for it forever. */
const ticks = async () => {
  for (let index = 0; index < 5; index += 1) await new Promise((resolve) => setImmediate(resolve));
};

test('test_csd023_ac_1_1_the_section_shows_the_version_and_the_button', async () => {
  const app = await settingsApp();
  const panel = [...app.$('settings').querySelectorAll('.panel')].find((node) => app.text(node.querySelector('.panel-title')) === 'Updates');
  assert.ok(panel, 'an Updates panel');
  assert.equal(app.textOf('update-current'), '0.1.0');
  assert.equal(app.text(check(app)), 'Check for updates');
  assert.equal(status(app), '', 'no result yet');
  assert.equal(install(app).hidden, true);
  assert.equal(app.isShown(app.$('update-progress')), false);
  app.close();
});

test('test_csd023_ac_1_2_dutch', async () => {
  const app = await settingsApp({}, { language: 'nl' });
  assert.equal(app.text(check(app)), 'Zoek naar updates');
  assert.equal(app.textOf('update-current'), '0.1.0');
  app.close();
});

test('test_csd023_ac_2_1_and_2_2_nothing_is_checked_until_the_user_asks', async () => {
  const app = await settingsApp();
  assert.equal(app.callsOf('check_for_update').length, 0, 'starting the app and opening Settings make no call');
  assert.equal(app.callsOf('install_update').length, 0);
  app.close();
});

test('test_csd023_ac_3_1_already_the_latest', async () => {
  const app = await settingsApp({ check_for_update: { status: 'up_to_date', current: '0.1.0' } });
  await app.click(check(app));
  assert.equal(status(app), 'Up to date (0.1.0)');
  assert.equal(install(app).hidden, true);
  app.close();
});

test('test_csd023_ac_3_2_a_newer_version_shows_its_number_and_install', async () => {
  const app = await settingsApp({ check_for_update: { status: 'available', current: '0.1.0', version: '0.2.0' } });
  await app.click(check(app));
  assert.equal(status(app), 'Version 0.2.0 is available');
  assert.equal(install(app).hidden, false);
  assert.equal(app.text(install(app)), 'Install');
  assert.doesNotMatch(app.textOf('settings'), /release notes|changelog/i, 'no release notes (NG-3)');
  app.close();
});

test('test_csd023_ac_3_3_an_unreachable_endpoint_shows_the_message_and_the_code', async () => {
  const app = await settingsApp({
    check_for_update: () => {
      throw commandError('UPDATE_CHECK_FAILED', 'Checking for updates failed. Check your connection and try again.');
    },
  });
  await app.click(check(app));
  assert.equal(status(app), 'Checking for updates failed. Check your connection and try again. Code: UPDATE_CHECK_FAILED');
  assert.equal(check(app).disabled, false, 'Check is enabled again');
  assert.equal(install(app).hidden, true);
  app.close();
});

test('test_csd023_ac_3_5_a_rejection_without_a_code_still_shows_a_message_and_frees_the_buttons', async () => {
  for (const rejection of ['plain text failure', null]) {
    const app = await settingsApp({
      check_for_update: () => {
        throw rejection;
      },
    });
    await app.click(check(app));
    assert.equal(status(app), 'Something went wrong. Please try again.', 'a generic message and no "Code: undefined"');
    assert.equal(check(app).disabled, false, 'Check is enabled again');
    app.close();
  }
});

test('test_csd023_ac_4_1_a_second_check_shows_the_new_answer', async () => {
  const answers = [{ status: 'available', current: '0.1.0', version: '0.2.0' }, { status: 'available', current: '0.1.0', version: '0.3.0' }];
  const app = await settingsApp({ check_for_update: () => answers.shift() });
  await app.click(check(app));
  assert.equal(status(app), 'Version 0.2.0 is available');
  await app.click(check(app));
  assert.equal(status(app), 'Version 0.3.0 is available');
  assert.equal(app.callsOf('check_for_update').length, 2, 'each check asks Rust again');
  app.close();
});

test('test_csd023_ac_5_3_nothing_newer_any_more_shows_up_to_date', async () => {
  const app = await settingsApp({
    check_for_update: { status: 'available', current: '0.1.0', version: '0.2.0' },
    install_update: { status: 'up_to_date', current: '0.1.0' },
  });
  await app.click(check(app));
  await app.click(install(app));
  assert.equal(status(app), 'Up to date (0.1.0)');
  assert.equal(install(app).hidden, true);
  app.close();
});

test('test_csd023_ac_6_1_a_known_size_shows_a_percentage_bar', async () => {
  const run = held();
  const app = await settingsApp({ check_for_update: { status: 'available', current: '0.1.0', version: '0.2.0' }, install_update: () => run.promise });
  await app.click(check(app));
  install(app).click();
  await ticks();
  app.bridge.emit('update-progress', { downloaded: 20, total: 40 });
  await ticks();
  assert.equal(app.isShown(app.$('update-progress')), true);
  assert.equal(app.$('update-progress').value, 50);
  assert.match(status(app), /50%/);
  run.release({ status: 'up_to_date', current: '0.1.0' });
  await app.settle();
  assert.equal(app.isShown(app.$('update-progress')), false, 'the bar is gone when the run ends');
  app.close();
});

test('test_csd023_ac_6_2_an_unknown_size_shows_a_bar_without_a_percentage', async () => {
  const run = held();
  const app = await settingsApp({ check_for_update: { status: 'available', current: '0.1.0', version: '0.2.0' }, install_update: () => run.promise });
  await app.click(check(app));
  install(app).click();
  await ticks();
  app.bridge.emit('update-progress', { downloaded: 20, total: null });
  await ticks();
  assert.equal(app.isShown(app.$('update-progress')), true);
  assert.equal(app.$('update-progress').hasAttribute('value'), false, 'indeterminate');
  assert.doesNotMatch(status(app), /%/);
  run.release({ status: 'up_to_date', current: '0.1.0' });
  await app.settle();
  app.close();
});

test('test_csd023_ac_7_1_unsaved_changes_ask_first_and_cancel_changes_nothing', async () => {
  const app = await settingsApp({ check_for_update: { status: 'available', current: '0.1.0', version: '0.2.0' } }, { unsaved: true });
  await app.click(check(app));
  install(app).click();
  await app.settle();
  assert.equal(app.$('confirm-dialog').open, true, 'a confirmation is asked');
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.callsOf('install_update').length, 0, 'nothing is downloaded');
  assert.equal(status(app), 'Version 0.2.0 is available', 'the section is as it was');
  assert.equal(install(app).hidden, false);
  assert.equal(check(app).disabled, false);
  app.close();
});

test('test_csd023_ac_7_2_confirming_goes_on', async () => {
  const app = await settingsApp({
    check_for_update: { status: 'available', current: '0.1.0', version: '0.2.0' },
    install_update: { status: 'up_to_date', current: '0.1.0' },
  }, { unsaved: true });
  await app.click(check(app));
  install(app).click();
  await app.settle();
  await app.click(app.$('confirm-ok'));
  assert.equal(app.callsOf('install_update').length, 1);
  app.close();
});

test('test_csd023_ac_7_3_and_7_4_no_unsaved_changes_no_question', async () => {
  const app = await settingsApp({
    check_for_update: { status: 'available', current: '0.1.0', version: '0.2.0' },
    install_update: { status: 'up_to_date', current: '0.1.0' },
  });
  await app.click(check(app));
  await app.click(install(app));
  assert.equal(app.$('confirm-dialog').open, false, 'no confirmation is asked');
  assert.equal(app.callsOf('install_update').length, 1);
  app.close();
});

test('test_csd023_ac_14_the_install_says_whether_the_user_confirmed_unsaved_changes', async () => {
  for (const unsaved of [true, false]) {
    const app = await settingsApp({
      check_for_update: { status: 'available', current: '0.1.0', version: '0.2.0' },
      install_update: { status: 'up_to_date', current: '0.1.0' },
    }, { unsaved });
    await app.click(check(app));
    install(app).click();
    await app.settle();
    if (unsaved) await app.click(app.$('confirm-ok'));
    assert.deepEqual(app.callsOf('install_update').map((call) => call.args), [{ unsavedConfirmed: unsaved }]);
    app.close();
  }
});

test('test_csd023_ac_14_3_an_installed_update_that_waits_for_a_restart_says_so', async () => {
  const app = await settingsApp({
    check_for_update: { status: 'available', current: '0.1.0', version: '0.2.0' },
    install_update: { status: 'installed', version: '0.2.0' },
  });
  await app.click(check(app));
  await app.click(install(app));
  assert.equal(status(app), 'Version 0.2.0 is installed. Restart Varde to use it.');
  assert.equal(install(app).hidden, true, 'nothing left to install');
  assert.equal(check(app).disabled, false);
  app.close();
});

test('test_csd023_ac_8_1_a_failed_install_shows_its_code_and_can_be_retried', async () => {
  const codes = ['UPDATE_DOWNLOAD_FAILED', 'UPDATE_SIGNATURE_INVALID', 'UPDATE_INSTALL_FAILED'];
  const app = await settingsApp({
    check_for_update: { status: 'available', current: '0.1.0', version: '0.2.0' },
    install_update: () => {
      throw commandError(codes.shift(), 'A safe message.');
    },
  });
  await app.click(check(app));
  for (const code of ['UPDATE_DOWNLOAD_FAILED', 'UPDATE_SIGNATURE_INVALID', 'UPDATE_INSTALL_FAILED']) {
    await app.click(install(app));
    assert.match(status(app), new RegExp(`Code: ${code}$`));
    assert.equal(status(app).includes('http'), false);
    assert.equal(install(app).hidden, false, 'Install stays for a retry (AC-8.3)');
    assert.equal(install(app).disabled, false);
    assert.equal(check(app).disabled, false);
  }
  assert.equal(app.callsOf('install_update').length, 3, 'each try is a new run');
  app.close();
});

test('test_csd023_ac_8_1_the_dutch_message_comes_by_code', async () => {
  const app = await settingsApp({
    check_for_update: () => {
      throw commandError('UPDATE_CHECK_FAILED', 'English message');
    },
  }, { language: 'nl' });
  await app.click(check(app));
  assert.equal(status(app), 'Zoeken naar updates is mislukt. Controleer je verbinding en probeer het opnieuw. Code: UPDATE_CHECK_FAILED');
  app.close();
});

test('test_csd023_ac_9_1_buttons_are_disabled_during_a_check', async () => {
  const run = held();
  const app = await settingsApp({ check_for_update: () => run.promise });
  check(app).click();
  await ticks();
  assert.equal(check(app).disabled, true);
  assert.equal(install(app).disabled, true);
  assert.equal(status(app), 'Checking for updates…');
  run.release({ status: 'up_to_date', current: '0.1.0' });
  await app.settle();
  assert.equal(check(app).disabled, false);
  app.close();
});

test('test_csd023_ac_9_2_buttons_are_disabled_during_an_install_and_there_is_no_cancel', async () => {
  const run = held();
  const app = await settingsApp({ check_for_update: { status: 'available', current: '0.1.0', version: '0.2.0' }, install_update: () => run.promise });
  await app.click(check(app));
  install(app).click();
  await ticks();
  assert.equal(check(app).disabled, true);
  assert.equal(install(app).disabled, true);
  const buttons = [...app.$('settings').querySelectorAll('.panel')].find((node) => node.querySelector('#update-check')).querySelectorAll('button');
  assert.deepEqual([...buttons].map((button) => app.text(button)), ['Check for updates', 'Install'], 'no cancel button');
  run.fail(commandError('UPDATE_DOWNLOAD_FAILED', 'x'));
  await app.settle();
  assert.equal(check(app).disabled, false, 'AC-9.3: back after an error');
  app.close();
});

test('test_csd023_ac_10_1_leaving_settings_clears_the_result', async () => {
  const app = await settingsApp({ check_for_update: { status: 'available', current: '0.1.0', version: '0.2.0' } });
  await app.click(check(app));
  assert.equal(install(app).hidden, false);
  await app.click(app.item('income'));
  await app.click(app.item('settings'));
  assert.equal(status(app), '');
  assert.equal(install(app).hidden, true);
  app.close();
});

test('test_csd023_ac_13_1_every_text_of_the_section_has_a_dutch_version', async () => {
  const app = await settingsApp();
  const nl = (await app.load('locales/nl.js')).default;
  const texts = [
    'Updates', 'Current version', 'Check for updates', 'Install', 'Checking for updates…', 'Up to date ({version})',
    'Version {version} is available', 'Downloading and installing the update…', 'Code: {code}',
    'Install the update without saving?', 'Install and restart',
    'You have unsaved changes. They will be lost when the app restarts to install the update.',
    'Varde only looks for updates when you ask. Checking contacts GitHub; nothing about you or your project is sent.',
  ];
  for (const text of texts) assert.ok(text in nl, text);
  for (const code of ['UPDATE_CHECK_FAILED', 'UPDATE_DOWNLOAD_FAILED', 'UPDATE_SIGNATURE_INVALID', 'UPDATE_INSTALL_FAILED']) {
    assert.ok(`code:${code}` in nl, code);
  }
  app.close();
});
