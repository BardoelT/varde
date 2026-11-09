// Accessibility of the forms: a failed save marks the offending field (aria-invalid) and focuses
// it, and the hints are linked to their fields with aria-describedby instead of sitting inside
// the label text, where they would become part of the field's accessible name.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { JSDOM } from 'jsdom';
import { openApp, commandError } from './harness.js';
import { emptyReads, exampleReads } from './fixtures.js';

const indexHtml = readFileSync(fileURLToPath(new URL('../index.html', import.meta.url)), 'utf8');

/** Opens the add-job form of an app whose create_job fails with the given code. */
async function failingJobForm(code) {
  const handlers = {
    create_job: () => {
      throw commandError(code, 'The job could not be saved.');
    },
  };
  const app = await openApp({ reads: emptyReads(), handlers });
  await app.click(app.item('income'));
  await app.click(app.$('job-add-first'));
  const form = app.$('job-form');
  await app.type(form.elements.employer, 'Example Employer');
  await app.type(form.elements.start_date, '01032024');
  return { app, form };
}

test('test_ui_a_failed_save_marks_the_field_of_the_error_and_focuses_it', async () => {
  const { app, form } = await failingJobForm('JOB_EMPLOYER_REQUIRED');
  form.requestSubmit();
  await app.settle();
  assert.equal(form.elements.employer.getAttribute('aria-invalid'), 'true');
  assert.equal(app.document.activeElement, form.elements.employer);
  assert.equal(form.elements.start_date.hasAttribute('aria-invalid'), false, 'only the offending field');
  assert.equal(app.$('job-error').getAttribute('role'), 'alert');
  assert.equal(app.isShown(app.$('job-error')), true);
  app.close();
});

test('test_ui_the_invalid_mark_is_cleared_when_the_user_edits_the_field', async () => {
  const { app, form } = await failingJobForm('JOB_EMPLOYER_REQUIRED');
  form.requestSubmit();
  await app.settle();
  await app.type(form.elements.employer, 'x');
  assert.equal(form.elements.employer.hasAttribute('aria-invalid'), false);
  app.close();
});

test('test_ui_the_invalid_mark_is_cleared_by_a_new_submit_and_by_a_reset', async () => {
  const { app, form } = await failingJobForm('JOB_EMPLOYER_REQUIRED');
  form.requestSubmit();
  await app.settle();
  form.elements.start_date.setAttribute('aria-invalid', 'true');
  form.reset();
  assert.equal(form.querySelectorAll('[aria-invalid]').length, 0, 'a reset (a dialog opening again) clears every mark');
  form.requestSubmit();
  await app.settle();
  assert.equal(form.querySelectorAll('[aria-invalid]').length, 1, 'one mark again after the second failure, not two');
  app.close();
});

test('test_ui_an_error_that_names_no_field_focuses_the_error_box', async () => {
  const { app, form } = await failingJobForm('INTERNAL');
  form.requestSubmit();
  await app.settle();
  assert.equal(form.querySelectorAll('[aria-invalid]').length, 0);
  assert.equal(app.document.activeElement, app.$('job-error'));
  assert.equal(app.$('job-error').getAttribute('role'), 'alert');
  app.close();
});

test('test_ui_a_record_form_error_marks_its_field_too', async () => {
  const handlers = {
    add_paycheck: () => {
      throw commandError('PAYCHECK_GROSS_INVALID', 'The gross salary is not a valid amount.');
    },
  };
  const app = await openApp({ handlers });
  await app.click(app.item('income'));
  await app.click(app.$('paycheck-add'));
  const form = app.$('paycheck-form');
  await app.type(form.elements.period, '202403');
  await app.type(form.elements.gross_salary, '1x');
  form.requestSubmit();
  await app.settle();
  assert.equal(form.elements.gross_salary.getAttribute('aria-invalid'), 'true');
  assert.equal(app.document.activeElement, form.elements.gross_salary);
  app.close();
});

test('test_ui_no_hint_sits_inside_a_label', () => {
  const { document } = new JSDOM(indexHtml).window;
  assert.deepEqual([...document.querySelectorAll('label .field-hint')].map((hint) => hint.id || hint.textContent.slice(0, 30)), []);
});

test('test_ui_every_hint_has_an_id_that_a_field_points_to_or_that_names_its_fields', () => {
  const { document } = new JSDOM(indexHtml).window;
  const form = (hint) => hint.closest('form');
  for (const hint of document.querySelectorAll('.field-hint[id]')) {
    const pointed = [...document.querySelectorAll('[aria-describedby]')].some((field) => field.getAttribute('aria-describedby').split(' ').includes(hint.id));
    const names = (hint.dataset.describes ?? '').split(' ').filter(Boolean);
    assert.ok(pointed || (names.length > 0 && names.every((name) => form(hint)?.elements[name])), `hint ${hint.id} is linked to a field`);
  }
  for (const field of document.querySelectorAll('[aria-describedby]')) {
    for (const id of field.getAttribute('aria-describedby').split(' ')) assert.ok(document.getElementById(id), `${id} exists`);
  }
});

test('test_ui_a_conditional_hint_describes_its_field_only_while_it_is_shown', async () => {
  const app = await openApp();
  await app.click(app.item('income'));
  await app.click(app.$('job-add'));
  const period = app.$('job-form').elements.period;
  assert.equal(app.isShown(app.$('job-period-hint')), false);
  assert.equal(period.hasAttribute('aria-describedby'), false, 'a hidden hint is not read out');
  await app.escape(app.$('job-dialog'));

  await app.click(app.document.querySelector('#job-list [aria-label="Edit Example Employer"]'));
  assert.equal(app.isShown(app.$('job-period-hint')), true);
  assert.equal(period.getAttribute('aria-describedby'), 'job-period-hint');
  await app.escape(app.$('job-dialog'));

  await app.click(app.$('job-add'));
  assert.equal(period.hasAttribute('aria-describedby'), false, 'and not any more after the form opens for a new job');
  app.close();
});

test('test_ui_the_amount_fields_of_a_form_are_described_by_its_hint', async () => {
  const { document } = new JSDOM(indexHtml).window;
  const form = document.getElementById('bank-entry-form');
  assert.equal(form.elements.balance.getAttribute('aria-describedby'), 'bank-entry-hint');
  assert.equal(form.elements.note.getAttribute('aria-describedby'), 'bank-entry-hint');
  assert.ok(document.getElementById('bank-entry-hint').textContent.includes('Use {decimal_name} for decimals'));
});
