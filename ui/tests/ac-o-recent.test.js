// Acceptance tests of CSD-002 REQ-33 and REQ-35: choosing an Open Recent entry or being handed a file
// asks the confirmation of File > Open, and sends Rust an index or nothing, never a path. Rust opens the
// file (proven by cargo test); these tests check what the screen asks and which commands it calls.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp, commandError } from './harness.js';
import { exampleReads } from './fixtures.js';

const savedStatus = { has_path: true, unsaved_changes: false };

const unsaved = () => {
  const reads = exampleReads();
  reads.project_status = { has_path: true, unsaved_changes: true };
  return reads;
};

const OPEN_MESSAGE = 'You have unsaved changes. If you open another file, they will be lost.';

test('test_csd002_ac_33_1_a_recent_file_opens_by_its_index', async () => {
  const app = await openApp({ handlers: { open_recent_project: { done: true, status: savedStatus } } });
  await app.emit('menu-action', 'recent-open-2');
  const calls = app.callsOf('open_recent_project');
  assert.equal(calls.length, 1);
  assert.deepEqual(calls[0].args, { index: 2 }, 'the index is all that is sent, never a path');
  assert.equal(app.$('confirm-dialog').open, false, 'no unsaved changes, no question');
  app.close();
});

test('test_csd002_ac_33_2_unsaved_changes_are_asked_about_first', async () => {
  const app = await openApp({
    reads: unsaved(),
    handlers: { open_recent_project: { done: true, status: savedStatus } },
  });
  await app.emit('menu-action', 'recent-open-0');
  assert.equal(app.$('confirm-dialog').open, true);
  assert.equal(app.textOf('confirm-message'), OPEN_MESSAGE);
  assert.equal(app.callsOf('open_recent_project').length, 0, 'nothing opens before the answer');

  await app.click(app.$('confirm-cancel'));
  assert.equal(app.callsOf('open_recent_project').length, 0, 'declining keeps the project');

  await app.emit('menu-action', 'recent-open-0');
  await app.click(app.$('confirm-ok'));
  assert.equal(app.callsOf('open_recent_project').length, 1, 'confirming opens the entry');
  app.close();
});

test('test_csd002_ac_33_3_a_file_that_cannot_be_opened_shows_the_error_without_a_path', async () => {
  const app = await openApp({
    handlers: {
      open_recent_project: () => {
        throw commandError('PROJECT_OPEN_FAILED', 'The file could not be opened.');
      },
    },
  });
  await app.emit('menu-action', 'recent-open-1');
  assert.match(app.document.body.textContent, /The file could not be opened\./);
  app.close();
});

test('test_csd002_ac_35_1_a_file_handed_to_the_app_opens_after_the_confirmation', async () => {
  const app = await openApp({
    reads: unsaved(),
    handlers: { open_pending_file: { done: true, status: savedStatus } },
  });
  await app.emit('open-file-requested');
  assert.equal(app.$('confirm-dialog').open, true);
  assert.equal(app.textOf('confirm-message'), OPEN_MESSAGE);
  assert.equal(app.callsOf('open_pending_file').length, 0);

  await app.click(app.$('confirm-ok'));
  const calls = app.callsOf('open_pending_file');
  assert.equal(calls.length, 1);
  assert.equal(calls[0].args, undefined, 'no path is sent');
  app.close();
});

test('test_csd002_ac_35_2_declining_keeps_the_project', async () => {
  const app = await openApp({
    reads: unsaved(),
    handlers: { open_pending_file: { done: true, status: savedStatus } },
  });
  await app.emit('open-file-requested');
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.callsOf('open_pending_file').length, 0);
  app.close();
});

test('test_csd002_ac_35_1_without_unsaved_changes_the_handed_file_opens_at_once', async () => {
  const app = await openApp({ handlers: { open_pending_file: { done: true, status: savedStatus } } });
  await app.emit('open-file-requested');
  assert.equal(app.callsOf('open_pending_file').length, 1);
  assert.equal(app.$('confirm-dialog').open, false);
  app.close();
});
