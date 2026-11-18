// CSD-002 REQ-23 to REQ-26: autosave. Rust writes the file; these tests check the switch on the Settings page,
// what it sends, and the message of a failed write. The writes themselves are proven in `ipc_autosave_tests.rs`.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp, commandError } from './harness.js';
import { exampleReads } from './fixtures.js';

const readsWith = (autosave) => {
  const reads = exampleReads();
  reads.get_preferences = { ...reads.get_preferences, autosave };
  return reads;
};

test('test_csd002_ac_24_1_the_switch_is_on_by_default', async () => {
  const app = await openApp();
  await app.click(app.item('settings'));
  const control = app.$('setting-autosave');
  assert.equal(control.type, 'checkbox');
  assert.equal(control.getAttribute('role'), 'switch');
  assert.equal(control.checked, true);
  assert.equal(app.document.querySelector('label[for="setting-autosave"]').textContent.trim(), 'Autosave');
  app.close();
});

test('test_csd002_ac_24_2_the_saved_choice_is_shown', async () => {
  const app = await openApp({ reads: readsWith(false) });
  await app.click(app.item('settings'));
  assert.equal(app.$('setting-autosave').checked, false);
  app.close();
});

test('test_csd002_ac_24_2_switching_off_and_on_sends_the_choice', async () => {
  const app = await openApp();
  await app.click(app.item('settings'));
  await app.click(app.$('setting-autosave'));
  await app.click(app.$('setting-autosave'));
  assert.deepEqual(app.callsOf('set_autosave').map((call) => call.args), [{ autosave: false }, { autosave: true }]);
  assert.equal(app.$('setting-autosave').checked, true);
  app.close();
});

test('test_csd002_ac_25_1_switching_on_asks_for_the_project_status_again', async () => {
  const app = await openApp({ reads: readsWith(false) });
  await app.click(app.item('settings'));
  const before = app.callsOf('project_status').length;
  await app.click(app.$('setting-autosave'));
  assert.ok(app.callsOf('project_status').length > before, 'the menu and the window follow what was written');
  app.close();
});

test('test_csd002_ac_24_4_a_preference_that_cannot_be_saved_shows_the_switch_as_before', async () => {
  const reads = readsWith(true);
  reads.set_autosave = () => {
    throw commandError('PREFERENCES_SAVE_FAILED', 'The preferences could not be saved.');
  };
  const app = await openApp({ reads });
  await app.click(app.item('settings'));
  await app.click(app.$('setting-autosave'));
  assert.equal(app.$('setting-autosave').checked, true, 'the switch shows on again');
  assert.equal(app.isShown(app.$('toast')), true);
  assert.equal(app.textOf('toast'), 'The preferences could not be saved.');
  app.close();
});

test('test_csd002_ac_24_5_the_switch_is_in_dutch', async () => {
  const reads = readsWith(true);
  reads.get_preferences = { ...reads.get_preferences, language: 'nl' };
  const app = await openApp({ reads });
  await app.click(app.item('settings'));
  assert.equal(app.document.querySelector('label[for="setting-autosave"]').textContent.trim(), 'Automatisch opslaan');
  app.close();
});

test('test_csd002_ac_26_1_a_failed_autosave_shows_a_message_without_a_path', async () => {
  const app = await openApp();
  await app.emit('autosave-failed', { code: 'PROJECT_SAVE_FAILED', message: 'The project could not be saved.' });
  assert.equal(app.isShown(app.$('toast')), true, 'a non-blocking message');
  assert.equal(app.textOf('toast'), 'The project could not be saved.');
  assert.doesNotMatch(app.textOf('toast'), /\//, 'no file path');
  app.close();
});
