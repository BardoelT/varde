// Form dialogs (the job form as the example), the confirmation dialog and the project status (CSD-001, CSD-002).
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp, commandError } from './harness.js';
import { emptyReads, exampleReads } from './fixtures.js';

const unsaved = () => {
  const reads = exampleReads();
  reads.project_status = { has_path: true, unsaved_changes: true };
  return reads;
};

test('test_csd001_ac_22_1_dates_are_shown_as_dd_mm_yyyy', async () => {
  const app = await openApp();
  await app.click(app.item('income'));
  const rows = [...app.document.querySelectorAll('#job-list .job-row')].map((row) => app.text(row.querySelector('.job-meta')));
  assert.ok(rows.some((meta) => meta.includes('01/01/2024 – present')), 'an open job shows its start date and "present"');
  assert.ok(rows.some((meta) => meta.includes('01/01/2023 – 31/12/2025')), 'a closed job shows both dates');

  await app.click(app.document.querySelector('#job-list [aria-label="Edit Example Consultancy"]'));
  const form = app.$('job-form');
  assert.equal(form.elements.start_date.value, '01/01/2023', 'the job details show dd/mm/yyyy too');
  assert.equal(form.elements.end_date.value, '31/12/2025');
  app.close();
});

test('test_form_dialog_opens_empty_and_cancel_closes_it', async () => {
  const app = await openApp();
  await app.click(app.item('income'));
  const dialog = app.$('job-dialog');
  assert.equal(dialog.open, false);
  await app.click(app.$('job-add'));
  assert.equal(dialog.open, true);
  assert.equal(app.textOf('job-dialog-title'), 'Add job');
  assert.equal(app.$('job-form').elements.employer.value, '');
  assert.equal(app.isShown(app.$('job-error')), false);
  const cancel = dialog.querySelector('[data-cancel]');
  assert.equal(app.text(cancel), 'Cancel');
  await app.click(cancel);
  assert.equal(dialog.open, false);
  assert.equal(app.callsOf('create_job').length, 0, 'nothing is saved');
  app.close();
});

test('test_form_dialog_edit_fills_the_form_and_locks_the_pay_period_of_a_job_with_entries', async () => {
  const app = await openApp();
  await app.click(app.item('income'));
  await app.click(app.document.querySelector('#job-list [aria-label="Edit Example Employer"]'));
  const form = app.$('job-form');
  assert.equal(app.textOf('job-dialog-title'), 'Edit job');
  assert.equal(form.elements.employer.value, 'Example Employer');
  assert.equal(form.elements.title.value, 'Engineer');
  assert.equal(form.elements.period.value, 'Monthly');
  assert.equal(form.elements.period.disabled, true, 'the job has paychecks');
  assert.equal(app.isShown(app.$('job-period-hint')), true);

  await app.escape(app.$('job-dialog'));
  assert.equal(app.$('job-dialog').open, false, 'Escape closes the form');
  app.close();
});

test('test_form_dialog_shows_a_safe_error_and_stays_open', async () => {
  const handlers = {
    create_job: () => {
      throw commandError('JOB_EMPLOYER_REQUIRED', 'The employer is required.');
    },
  };
  const app = await openApp({ reads: emptyReads(), handlers });
  await app.click(app.item('income'));
  await app.click(app.$('job-add-first'));
  app.$('job-form').requestSubmit();
  await app.settle();
  assert.equal(app.$('job-dialog').open, true);
  assert.equal(app.isShown(app.$('job-error')), true);
  assert.equal(app.textOf('job-error'), 'The employer is required.');
  app.close();
});

test('test_csd002_ac_3_1_the_indicator_follows_edits', async () => {
  let status = { has_path: true, unsaved_changes: false };
  const handlers = {
    project_status: () => status,
    create_job: () => {
      status = { has_path: true, unsaved_changes: true };
      return {};
    },
    save_project: () => {
      status = { has_path: true, unsaved_changes: false };
      return { status, done: true };
    },
  };
  const app = await openApp({ reads: emptyReads(), handlers });

  await app.click(app.item('income'));
  await app.click(app.$('job-add-first'));
  const form = app.$('job-form');
  await app.type(form.elements.employer, 'Example Employer');
  await app.type(form.elements.start_date, '01032024');
  form.requestSubmit();
  await app.settle();
  assert.deepEqual(app.callsOf('create_job').map((call) => call.args.input.start_date), ['01/03/2024'], 'the typed date goes to Rust as typed');

  await app.emit('menu-action', 'project-save');
  assert.equal(app.callsOf('save_project').length, 1, 'File > Save calls the save command');
  app.close();
});

test('test_csd002_ac_4_1_closing_with_unsaved_changes_asks_first', async () => {
  const app = await openApp({ reads: unsaved(), handlers: { quit_app: null } });
  await app.emit('close-requested');
  assert.equal(app.$('confirm-dialog').open, true, 'a confirmation is asked');
  assert.equal(app.textOf('confirm-title'), 'Quit without saving?');
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.$('confirm-dialog').open, false);
  assert.equal(app.callsOf('quit_app').length, 0, 'cancel keeps the app open');

  await app.emit('close-requested');
  await app.click(app.$('confirm-ok'));
  assert.equal(app.callsOf('quit_app').length, 1, 'confirming quits');
  app.close();
});

test('test_csd002_ac_7_1_open_prompts_when_unsaved_and_cancel_keeps_the_data', async () => {
  const app = await openApp({ reads: unsaved(), handlers: { open_project: { done: true, status: { has_path: true, unsaved_changes: false } } } });
  await app.emit('menu-action', 'project-open');
  assert.equal(app.$('confirm-dialog').open, true, 'a confirmation is asked before the open dialog');
  assert.equal(app.textOf('confirm-message'), 'You have unsaved changes. If you open another file, they will be lost.');
  assert.equal(app.callsOf('open_project').length, 0);
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.callsOf('open_project').length, 0, 'the current data is kept');
  app.close();
});

test('test_csd002_ac_7_2_a_new_project_without_unsaved_changes_asks_nothing', async () => {
  const app = await openApp({ handlers: { new_project: { has_path: false, unsaved_changes: false } } });
  await app.emit('menu-action', 'project-new');
  assert.equal(app.$('confirm-dialog').open, false, 'no confirmation is asked');
  assert.equal(app.callsOf('new_project').length, 1);
  app.close();
});

test('test_csd002_ac_9_3_no_buttons_in_the_window', async () => {
  const app = await openApp({ reads: unsaved() });
  const labels = [...app.document.querySelectorAll('button')].map((button) => app.text(button));
  for (const name of ['New', 'Open', 'Open…', 'Save', 'Save As', 'Save As…']) assert.ok(!labels.includes(name), `no ${name} button`);
  app.close();
});

test('test_confirm_dialog_escape_answers_no_and_the_next_confirmation_still_works', async () => {
  const app = await openApp({ reads: unsaved(), handlers: { new_project: { has_path: false, unsaved_changes: false } } });
  await app.emit('menu-action', 'project-new');
  await app.escape(app.$('confirm-dialog'));
  assert.equal(app.$('confirm-dialog').open, false);
  assert.equal(app.callsOf('new_project').length, 0, 'Escape does not confirm');

  await app.emit('menu-action', 'project-new');
  assert.equal(app.$('confirm-dialog').open, true, 'the dialog can be asked again');
  await app.click(app.$('confirm-ok'));
  assert.equal(app.callsOf('new_project').length, 1, 'the earlier Escape did not answer this confirmation');
  app.close();
});
