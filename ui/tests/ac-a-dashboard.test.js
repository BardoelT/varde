// Display ACs of CSD-004 (the income dashboard): the block titles carry no currency, and the
// dashboard follows changes and a new project. Rust computes the figures; the fake bridge
// answers what it would.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';
import { exampleReads } from './fixtures.js';

const totals = (gross) => ({ bonus: '0.00', entry_count: 1, gross_salary: gross, holiday_pay: '0.00', total_gross: gross });
const lastMonthTotal = (app) => app.text(app.$('dashboard-month-cards').querySelector('.card.primary .card-value'));

test('test_csd004_ac_6_1_no_currency_in_the_titles', async () => {
  const reads = exampleReads();
  reads.get_settings = { currency: 'USD', entry_count: 2 };
  reads.get_dashboard = { has_jobs: true, last_month: '2026-09', last_month_totals: totals('3000.00'), year: 2026, year_to_date: totals('3000.00') };
  const app = await openApp({ reads });
  assert.equal(app.textOf('dashboard-year-title'), 'This year · 2026');
  assert.equal(app.textOf('dashboard-month-title'), 'Last month · 2026-09');
  assert.doesNotMatch(app.textOf('dashboard-content'), /USD|EUR|GBP|[$€£]/, 'the dashboard shows no currency');
  app.close();
});

test('test_csd004_ac_7_1_a_new_paycheck_shows_up', async () => {
  let dashboard = { has_jobs: true, last_month: '2026-09', last_month_totals: totals('4500.00'), year: 2026, year_to_date: totals('4500.00') };
  const handlers = {
    get_dashboard: () => dashboard,
    add_paycheck: () => {
      dashboard = { ...dashboard, last_month_totals: totals('5000.00'), year_to_date: totals('5000.00') };
      return {};
    },
  };
  const app = await openApp({ handlers });
  assert.equal(lastMonthTotal(app), '4,500.00', 'the dashboard shows Last month with total 4500.00');

  await app.click(app.item('income'));
  await app.click(app.$('paycheck-add'));
  const form = app.$('paycheck-form');
  await app.choose(form.elements.job_id, 'id-9');
  await app.type(form.elements.period, '202609');
  await app.type(form.elements.gross_salary, '500.00');
  form.requestSubmit();
  await app.settle();
  assert.equal(app.callsOf('add_paycheck').length, 1, 'the paycheck is saved');

  await app.click(app.item('dashboard'));
  assert.equal(lastMonthTotal(app), '5,000.00', 'Last month shows total 5000.00');
  app.close();
});

test('test_csd004_ac_7_2_a_new_project_refreshes_the_dashboard', async () => {
  let dashboard = exampleReads().get_dashboard;
  const handlers = {
    get_dashboard: () => dashboard,
    new_project: () => {
      dashboard = { has_jobs: false, last_month: null, last_month_totals: null, year: dashboard.year, year_to_date: null };
      return { has_path: false, unsaved_changes: false };
    },
  };
  const app = await openApp({ handlers });
  assert.equal(app.isShown(app.$('dashboard-content')), true, 'the dashboard shows figures');
  await app.emit('menu-action', 'project-new');
  assert.equal(app.callsOf('new_project').length, 1);
  assert.equal(app.isShown(app.$('dashboard-empty')), true, 'the prompt to add the first job');
  assert.equal(app.isShown(app.$('dashboard-content')), false);
  assert.equal(app.text(app.$('dashboard-empty').querySelector('button')), 'Add your first job');
  app.close();
});
