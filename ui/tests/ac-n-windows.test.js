// CSD-038: Windows support. The native menu is Rust's (tested in menu.rs); these tests check what the
// screen shows and does on Windows: the shortcut texts (REQ-6) and Ctrl+Y (REQ-9).
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { emptyReads } from './fixtures.js';

const withRedo = { has_path: true, unsaved_changes: false, undo_label: null, redo_label: 'Delete Bank Account' };

function welcomeApp({ platform, language }) {
  const reads = emptyReads();
  reads.get_preferences = { ...reads.get_preferences, language };
  return openApp({ reads, platform });
}

for (const [platform, language, open, save] of [
  ['windows', 'en', 'Ctrl+O', 'Ctrl+S'],
  ['windows', 'nl', 'Ctrl+O', 'Ctrl+S'],
  ['macos', 'en', '⌘O', '⌘S'],
  ['macos', 'nl', '⌘O', '⌘S'],
]) {
  test(`test_csd038_ac_6_1_the_welcome_text_on_${platform}_in_${language}`, async () => {
    const app = await welcomeApp({ platform, language });
    const text = app.textOf('dashboard-welcome');
    assert.ok(text.includes(`(${open})`), text);
    assert.ok(text.includes(`(${save})`), text);
    app.close();
  });
}

test('test_csd038_ac_6_2_no_mac_symbol_on_windows', async () => {
  const app = await welcomeApp({ platform: 'windows', language: 'en' });
  assert.doesNotMatch(app.document.body.textContent, /[⌘⇧]/);
  app.close();
});

test('test_csd038_ac_9_1_ctrl_y_redoes_a_project_step_on_windows', async () => {
  const app = await openApp({
    platform: 'windows',
    handlers: { project_status: withRedo, redo: { step: 'Delete Bank Account', status: { ...withRedo, redo_label: null } } },
  });
  const event = new app.window.KeyboardEvent('keydown', { key: 'y', ctrlKey: true, bubbles: true, cancelable: true });
  app.document.body.dispatchEvent(event);
  await app.settle();
  assert.equal(app.callsOf('redo').length, 1);
  assert.equal(event.defaultPrevented, true);
  app.close();
});

test('test_csd038_ac_9_1_ctrl_y_without_a_step_does_nothing', async () => {
  const app = await openApp({ platform: 'windows', handlers: { redo: { step: 'x', status: withRedo } } });
  app.document.body.dispatchEvent(new app.window.KeyboardEvent('keydown', { key: 'y', ctrlKey: true, bubbles: true }));
  await app.settle();
  assert.equal(app.callsOf('redo').length, 0);
  app.close();
});

test('test_csd038_ac_9_2_ctrl_y_in_a_text_field_leaves_the_project_alone', async () => {
  const app = await openApp({ platform: 'windows', handlers: { project_status: withRedo, redo: { step: 'x', status: withRedo } } });
  const field = app.document.createElement('input');
  app.document.body.append(field);
  field.focus();
  field.dispatchEvent(new app.window.KeyboardEvent('keydown', { key: 'y', ctrlKey: true, bubbles: true }));
  await app.settle();
  assert.equal(app.callsOf('redo').length, 0, 'the text is redone, not the project');
  app.close();
});

test('test_csd038_ac_9_3_ctrl_y_does_nothing_on_macos', async () => {
  const app = await openApp({ platform: 'macos', handlers: { project_status: withRedo, redo: { step: 'x', status: withRedo } } });
  app.document.body.dispatchEvent(new app.window.KeyboardEvent('keydown', { key: 'y', ctrlKey: true, bubbles: true }));
  await app.settle();
  assert.equal(app.callsOf('redo').length, 0);
  app.close();
});
