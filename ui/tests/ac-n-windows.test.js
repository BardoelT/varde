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

// REQ-8: on Windows the webview keeps the Ctrl keys from the native menu, so the page runs the item itself.
const saved = { has_path: true, unsaved_changes: false, undo_label: null, redo_label: null };

function press(app, key, { shift = false, ctrl = true, target = app.document.body } = {}) {
  const event = new app.window.KeyboardEvent('keydown', { key, ctrlKey: ctrl, shiftKey: shift, bubbles: true, cancelable: true });
  target.dispatchEvent(event);
  return event;
}

for (const [key, shift, command] of [
  ['o', false, 'open_project'],
  ['n', false, 'new_project'],
  ['s', false, 'save_project'],
  ['S', true, 'save_project_as'],
]) {
  test(`test_csd038_ac_8_2_ctrl_${shift ? 'shift_' : ''}${key.toLowerCase()}_runs_${command}_on_windows`, async () => {
    const app = await openApp({
      platform: 'windows',
      handlers: { project_status: saved, [command]: command === 'new_project' ? saved : { status: saved, done: false } },
    });
    const event = press(app, key, { shift });
    await app.settle();
    assert.equal(app.callsOf(command).length, 1);
    assert.equal(event.defaultPrevented, true);
    app.close();
  });
}

test('test_csd038_ac_8_2_ctrl_comma_opens_settings_on_windows', async () => {
  const app = await openApp({ platform: 'windows' });
  press(app, ',');
  await app.settle();
  assert.equal(app.document.querySelector('[data-section="settings"]').classList.contains('active'), true);
  app.close();
});

test('test_csd038_ac_8_2_ctrl_z_undoes_a_project_step_on_windows', async () => {
  const withUndo = { ...saved, undo_label: 'Add Bank Account' };
  const app = await openApp({ platform: 'windows', handlers: { project_status: withUndo, undo: { step: 'Add Bank Account', status: saved } } });
  press(app, 'z');
  await app.settle();
  assert.equal(app.callsOf('undo').length, 1);
  app.close();
});

test('test_csd038_ac_8_2_ctrl_z_in_a_text_field_is_left_to_the_browser', async () => {
  const app = await openApp({ platform: 'windows', handlers: { project_status: { ...saved, undo_label: 'x' }, undo: { step: 'x', status: saved } } });
  const field = app.document.createElement('input');
  app.document.body.append(field);
  field.focus();
  const event = press(app, 'z', { target: field });
  await app.settle();
  assert.equal(app.callsOf('undo').length, 0);
  assert.equal(event.defaultPrevented, false);
  app.close();
});

test('test_csd038_ac_8_3_ctrl_o_does_nothing_in_the_page_on_macos', async () => {
  const app = await openApp({ platform: 'macos', handlers: { open_project: { status: saved, done: false } } });
  const event = press(app, 'o');
  await app.settle();
  assert.equal(app.callsOf('open_project').length, 0, 'the native menu handles it');
  assert.equal(event.defaultPrevented, false);
  app.close();
});

// REQ-14: the About dialog, with the app icon, on Windows.
test('test_csd038_ac_14_1_about_varde_opens_a_dialog_with_the_icon_and_the_texts_from_rust', async () => {
  const info = { name: 'Varde', version: '1.2.3', description: 'Personal wealth tracking.', copyright: '© 2026 Example', license: 'GPL-3.0-only', website: 'https://example.com/varde' };
  const app = await openApp({ platform: 'windows', handlers: { get_about_info: info } });
  await app.emit('menu-action', 'show-about');
  const dialog = app.document.getElementById('about-dialog');
  assert.equal(dialog.open, true);
  assert.equal(dialog.querySelector('img').getAttribute('src'), 'assets/app-icon.png');
  const text = dialog.textContent;
  for (const part of ['Varde', 'Version 1.2.3', 'Personal wealth tracking.', '© 2026 Example', 'GPL-3.0-only', 'https://example.com/varde']) assert.ok(text.includes(part), part);
  app.close();
});
