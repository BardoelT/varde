// Display ACs of CSD-001 (employment income): what the Income page, the job form and the entry form show.
// Rust computes everything; the fake bridge answers with the golden example project.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { exampleReads } from './fixtures.js';

async function incomePage(options) {
  const app = await openApp(options);
  await app.click(app.item('income'));
  return app;
}

const rows = (app) => [...app.document.querySelectorAll('#job-list .job-row')];
const byLabel = (app, label) => app.document.querySelector(`[aria-label="${label}"]`);

test('test_csd001_ac_1_7_the_job_form_offers_the_pay_period_monthly_first', async () => {
  const app = await incomePage();
  await app.click(app.$('job-add'));
  const form = app.$('job-form');
  assert.equal(app.text(form.elements.period.closest('label')).startsWith('Pay period'), true);
  assert.deepEqual([...form.elements.period.options].map((option) => app.text(option)), ['Monthly', 'Yearly']);
  assert.equal(form.elements.period.value, 'Monthly', 'Monthly is chosen');
  app.close();
});

test('test_csd001_ac_4_2_cancelled_deletion_changes_nothing', async () => {
  const app = await incomePage();
  const before = { jobs: rows(app).length, entries: app.document.querySelectorAll('#paycheck-rows tr').length };
  await app.click(byLabel(app, 'Delete Example Employer'));
  assert.equal(app.$('confirm-dialog').open, true);
  await app.click(app.$('confirm-cancel'));
  assert.equal(app.$('confirm-dialog').open, false);
  assert.equal(app.callsOf('delete_job').length, 0, 'nothing is deleted');
  assert.deepEqual({ jobs: rows(app).length, entries: app.document.querySelectorAll('#paycheck-rows tr').length }, before);
  assert.equal(before.jobs, 2);
  assert.equal(before.entries, 7);
  app.close();
});

test('test_csd001_ac_4_3_the_confirmation_is_the_same_without_entries', async () => {
  const reads = exampleReads();
  reads.list_jobs = [{ employer: 'Example Employer', end_date: null, entry_count: 0, id: 'id-9', period: 'Monthly', start_date: '2024-01-01', title: null }];
  reads.list_entries = [];
  reads.yearly_totals = [];
  const app = await incomePage({ reads });
  await app.click(byLabel(app, 'Delete Example Employer'));
  assert.equal(app.textOf('confirm-message'), 'This job and all its paychecks and statements will be deleted.');
  app.close();
});

test('test_csd001_ac_5_6_the_add_form_chooses_the_newest_job_first', async () => {
  const app = await incomePage();
  await app.click(app.$('paycheck-add'));
  const select = app.$('paycheck-form').elements.job_id;
  const options = [...select.options].map((option) => app.text(option));
  assert.deepEqual(options, ['Example Employer · from 01/01/2024', 'Example Consultancy · from 01/01/2023']);
  assert.equal(select.value, 'id-9', 'the job with the newest start date is shown');
  await app.choose(select, 'id-10');
  assert.equal(select.value, 'id-10', 'the older job can be chosen instead');
  app.close();
});

test('test_csd001_ac_9_3_the_copy_follows_the_chosen_job', async () => {
  const prefills = {
    'id-9': { gross_salary: '3000.00', holiday_pay: '0.00', bonus: '0.00' },
    'id-10': { gross_salary: '1000.00', holiday_pay: '0.00', bonus: '0.00' },
  };
  const app = await incomePage({ handlers: { copy_previous_entry: ({ jobId }) => prefills[jobId] } });
  await app.click(app.$('paycheck-add'));
  const form = app.$('paycheck-form');
  await app.choose(form.elements.job_id, 'id-10');
  await app.click(app.$('paycheck-copy'));
  assert.deepEqual(app.callsOf('copy_previous_entry').map((call) => call.args.jobId), ['id-10']);
  assert.equal(form.elements.gross_salary.value, '1000.00');
  app.close();
});

test('test_csd001_ac_23_1_the_sections_appear_in_order', async () => {
  const app = await incomePage();
  assert.equal(app.textOf('income-title'), 'Income');
  const panels = [...app.$('income-overview').querySelectorAll('.panel-title')].map((title) => app.text(title));
  assert.deepEqual(panels, ['Jobs', 'Yearly totals', 'Paychecks and statements'], 'no section describes one selected job');
  app.close();
});

test('test_csd001_ac_23_2_job_rows_carry_their_pay_period_and_actions', async () => {
  const app = await incomePage();
  const [monthly, yearly] = rows(app);
  assert.equal(app.text(monthly.querySelector('.job-name')), 'Example Employer');
  assert.equal(app.text(monthly.querySelector('.job-meta')), 'Engineer · 01/01/2024 – present · Monthly');
  assert.equal(app.text(yearly.querySelector('.job-name')), 'Example Consultancy');
  assert.equal(app.text(yearly.querySelector('.job-meta')), '01/01/2023 – 31/12/2025 · Yearly');
  for (const row of [monthly, yearly]) assert.doesNotMatch(app.text(row), /paycheck|statement|\b[34]\b/i, 'no count of entries');
  const name = app.text(monthly.querySelector('.job-name'));
  const [edit, remove] = monthly.querySelectorAll('.job-actions button');
  assert.equal(edit.getAttribute('aria-label'), `Edit ${name}`);
  assert.equal(edit.title, `Edit ${name}`, 'a tooltip too');
  assert.equal(remove.getAttribute('aria-label'), `Delete ${name}`);
  assert.equal(remove.title, `Delete ${name}`);
  assert.equal(monthly.lastElementChild.className, 'job-actions', 'the buttons sit at the right edge');
  app.close();
});

test('test_csd001_ac_23_4_clicking_a_job_changes_nothing', async () => {
  const app = await incomePage();
  const totals = app.$('totals-rows').innerHTML;
  const entries = app.$('paycheck-rows').innerHTML;
  const calls = app.calls.length;
  for (const row of rows(app)) await app.click(row);
  assert.equal(app.$('totals-rows').innerHTML, totals);
  assert.equal(app.$('paycheck-rows').innerHTML, entries);
  assert.equal(app.calls.length, calls, 'nothing is fetched for a click');
  app.close();
});

test('test_csd001_ac_23_5_add_job_sits_in_the_jobs_section_header', async () => {
  const app = await incomePage();
  const header = app.$('job-add').closest('.panel-header');
  assert.equal(app.text(header.querySelector('.panel-title')), 'Jobs');
  assert.equal(app.text(app.$('job-add')), 'Add job');
  assert.equal(header.lastElementChild, app.$('job-add'), 'at its right edge');
  assert.equal(app.$('income').querySelector('.section-header').querySelectorAll('button').length, 0, 'the title row holds no button');
  app.close();
});

test('test_csd001_ac_23_6_entry_actions_sit_at_the_right', async () => {
  const app = await incomePage();
  const row = app.$('paycheck-rows').querySelector('tr');
  const last = row.lastElementChild;
  assert.equal(last.className, 'row-actions', 'the last column');
  assert.deepEqual([...last.querySelectorAll('button')].map((button) => button.getAttribute('aria-label')), ['Edit Example Employer 2026-08', 'Delete Example Employer 2026-08']);
  assert.ok(last.querySelector('button.danger'), 'the delete button is the trash icon');
  app.close();
});

test('test_csd001_ac_28_1_a_monthly_job_asks_for_a_month', async () => {
  const app = await incomePage();
  await app.click(app.$('paycheck-add'));
  const form = app.$('paycheck-form');
  await app.choose(form.elements.job_id, 'id-9');
  assert.equal(app.textOf('paycheck-dialog-title'), 'Add paycheck');
  assert.equal(app.textOf('paycheck-period-label'), 'Month');
  assert.equal(form.elements.period.placeholder, 'yyyy-mm');
  assert.equal(app.textOf('paycheck-copy'), 'Copy previous month');
  app.close();
});

test('test_csd001_ac_28_2_a_yearly_job_asks_for_a_year', async () => {
  const app = await incomePage();
  await app.click(app.$('paycheck-add'));
  const form = app.$('paycheck-form');
  await app.choose(form.elements.job_id, 'id-10');
  assert.equal(app.textOf('paycheck-dialog-title'), 'Add annual statement');
  assert.equal(app.textOf('paycheck-period-label'), 'Year');
  assert.match(form.elements.period.placeholder, /^\d{4}$/, 'a year such as 2025');
  assert.equal(form.elements.period.maxLength, 4);
  assert.equal(app.textOf('paycheck-copy'), 'Copy previous year');
  app.close();
});

test('test_csd001_ac_28_3_switching_job_switches_the_form', async () => {
  const app = await incomePage();
  await app.click(app.$('paycheck-add'));
  const form = app.$('paycheck-form');
  await app.choose(form.elements.job_id, 'id-10');
  assert.equal(app.textOf('paycheck-period-label'), 'Year');
  await app.choose(form.elements.job_id, 'id-9');
  assert.equal(app.textOf('paycheck-period-label'), 'Month');
  assert.equal(app.textOf('paycheck-dialog-title'), 'Add paycheck');
  app.close();
});

test('test_csd001_ac_29_6_the_edit_form_offers_the_month_or_year', async () => {
  const app = await incomePage();
  await app.click(byLabel(app, 'Edit Example Employer 2026-08'));
  const form = app.$('paycheck-form');
  assert.equal(app.textOf('paycheck-period-label'), 'Month');
  assert.equal(form.elements.period.value, '2026-08');
  assert.equal(form.elements.job_id.value, 'id-9');
  assert.equal(form.elements.job_id.disabled, true, 'the job cannot be changed');
  await app.click(app.$('paycheck-dialog').querySelector('[data-cancel]'));

  await app.click(byLabel(app, 'Edit Example Consultancy 2025'));
  assert.equal(app.textOf('paycheck-period-label'), 'Year');
  assert.equal(form.elements.period.value, '2025');
  assert.equal(form.elements.job_id.value, 'id-10');
  assert.equal(form.elements.job_id.disabled, true);
  app.close();
});
