// CSD-027: undo and redo. Rust keeps the history (`undo`, `redo`, and the labels in `project_status`);
// these tests check what the screen does and shows. There is no selected record or detail view in the
// app, so AC-12.2 holds by itself: every page is a list that is loaded again after an undo.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { exampleReads } from './fixtures.js';

const SAVED = { has_path: true, unsaved_changes: false, undo_label: null, redo_label: null };

/** An app whose `undo` and `redo` answer with the given step and status. */
function openWith({ language = 'en', step = 'Delete Bank Account', status = SAVED } = {}) {
  const reads = exampleReads();
  reads.get_preferences = { theme: 'system', language };
  return openApp({
    reads,
    handlers: {
      undo: { step, status },
      redo: { step, status: { ...status, unsaved_changes: true } },
    },
  });
}

const toast = (app) => app.$('toast');

test('test_csd027_ac_12_1_pages_show_the_restored_data', async () => {
  const app = await openWith();
  const before = {
    banks: app.callsOf('list_bank_accounts').length,
    dashboard: app.callsOf('get_dashboard').length,
    houses: app.callsOf('list_houses').length,
  };

  await app.emit('menu-action', 'project-undo');

  assert.equal(app.callsOf('undo').length, 1);
  assert.equal(app.callsOf('list_bank_accounts').length, before.banks + 1, 'the Bank accounts page is loaded again');
  assert.equal(app.callsOf('get_dashboard').length, before.dashboard + 1, 'so is the Dashboard');
  assert.ok(app.callsOf('list_houses').length > before.houses, 'and every other page');
  app.close();
});

test('test_csd027_ac_12_1_the_page_stays_where_it_is', async () => {
  const app = await openWith();
  await app.click(app.item('expenses'));

  await app.emit('menu-action', 'project-undo');

  assert.ok(app.item('expenses').classList.contains('current') || app.item('expenses').getAttribute('aria-current') === 'page');
  assert.ok(app.$('expenses').classList.contains('active'), 'the Expenses page is still shown');
  app.close();
});

test('test_csd027_ac_13_1_a_message_names_the_undone_step', async () => {
  const app = await openWith();
  await app.emit('menu-action', 'project-undo');
  assert.equal(toast(app).hidden, false);
  assert.equal(app.text(toast(app)), 'Undid: Delete Bank Account');
  assert.equal(toast(app).querySelector('button'), null, 'a message, not a button');
  app.close();
});

test('test_csd027_ac_13_1_a_message_names_the_redone_step', async () => {
  const app = await openWith();
  await app.emit('menu-action', 'project-redo');
  assert.equal(app.callsOf('redo').length, 1);
  assert.equal(app.text(toast(app)), 'Redid: Delete Bank Account');
  app.close();
});

test('test_csd027_ac_13_2_the_message_is_in_dutch', async () => {
  const app = await openWith({ language: 'nl' });
  await app.emit('menu-action', 'project-undo');
  assert.equal(app.text(toast(app)), 'Ongedaan gemaakt: Bankrekening verwijderen');
  app.close();
});

test('test_csd027_ac_11_1_command_z_in_a_text_field_undoes_typing', async () => {
  const app = await openWith();
  const calls = [];
  app.document.execCommand = (name) => calls.push(name);
  const dialog = app.$('job-dialog');
  dialog.showModal();
  const field = dialog.querySelector('input');
  field.focus();

  await app.emit('menu-action', 'project-undo');
  await app.emit('menu-action', 'project-redo');

  assert.deepEqual(calls, ['undo', 'redo'], 'the text field gets the key');
  assert.equal(app.callsOf('undo').length, 0, 'no project step is undone');
  assert.equal(app.callsOf('redo').length, 0);
  app.close();
});

test('test_csd027_ac_11_2_an_open_dialog_blocks_project_undo', async () => {
  const app = await openWith();
  const dialog = app.$('job-dialog');
  dialog.showModal();
  app.document.activeElement.blur();

  await app.emit('menu-action', 'project-undo');

  assert.equal(app.callsOf('undo').length, 0, 'nothing is undone');
  assert.equal(dialog.open, true, 'the form stays open');
  app.close();
});

test('test_csd002_ac_3_2_undo_and_redo_go_through_rust_which_keeps_the_window_title', async () => {
  // The unsaved dot is in the native window title, set by Rust (CSD-024 AC-9.4, proven by cargo test).
  const app = await openWith();

  await app.emit('menu-action', 'project-undo');
  await app.emit('menu-action', 'project-redo');
  assert.equal(app.callsOf('undo').length, 1);
  assert.equal(app.callsOf('redo').length, 1);
  assert.equal(app.$('project-state'), null, 'no project status in the left pane');
  app.close();
});
