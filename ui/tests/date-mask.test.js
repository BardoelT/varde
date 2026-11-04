// The date and month mask (shared/date-mask.js). The CSDs do not specify the mask itself, only the
// formats dd/mm/yyyy and yyyy-mm that it helps the user type, so these tests carry no AC id.
import test from 'node:test';
import assert from 'node:assert/strict';
import { openApp } from './harness.js';

const DATE = { groups: [2, 2, 4], separator: '/' };
const MONTH = { groups: [4, 2], separator: '-' };

test('test_date_mask_lays_digits_out_in_groups', async () => {
  const app = await openApp();
  const { maskDigits } = await app.load('shared/date-mask.js');
  assert.equal(maskDigits('', DATE, true), '');
  assert.equal(maskDigits('1', DATE, true), '1');
  assert.equal(maskDigits('01', DATE, true), '01/', 'a full group asks for its separator while typing');
  assert.equal(maskDigits('01', DATE, false), '01', 'and not while deleting');
  assert.equal(maskDigits('01032024', DATE, true), '01/03/2024');
  assert.equal(maskDigits('010320241', DATE, true), '01/03/2024', 'digits past the last group are dropped');
  assert.equal(maskDigits('202608', MONTH, true), '2026-08');
  app.close();
});

test('test_date_mask_types_the_slashes_of_a_date_field', async () => {
  const app = await openApp({});
  await app.click(app.item('income'));
  await app.click(app.$('job-add'));
  const input = app.$('job-form').elements.start_date;
  await app.type(input, '01');
  assert.equal(input.value, '01/');
  await app.type(input, '03');
  assert.equal(input.value, '01/03/');
  await app.type(input, '2024');
  assert.equal(input.value, '01/03/2024');
  app.close();
});

test('test_date_mask_accepts_typed_separators_and_ignores_letters', async () => {
  const app = await openApp();
  await app.click(app.item('income'));
  await app.click(app.$('job-add'));
  const input = app.$('job-form').elements.start_date;
  await app.type(input, '0a1/0');
  assert.equal(input.value, '01/0');
  app.close();
});

test('test_date_mask_deleting_does_not_bring_the_separator_back', async () => {
  const app = await openApp();
  await app.click(app.item('income'));
  await app.click(app.$('job-add'));
  const input = app.$('job-form').elements.start_date;
  await app.type(input, '0103');
  assert.equal(input.value, '01/03/');
  await app.backspace(input);
  assert.equal(input.value, '01/03', 'the separator typed after the last full group goes with the backspace');
  await app.backspace(input);
  assert.equal(input.value, '01/0');
  app.close();
});

test('test_date_mask_month_field_types_the_dash', async () => {
  const app = await openApp();
  await app.click(app.item('income'));
  const add = app.document.querySelector('#paycheck-dialog');
  assert.ok(add, 'the paycheck form has a month field');
  const input = add.querySelector('input[name="period"]');
  await app.type(input, '202608');
  assert.equal(input.value, '2026-08');
  app.close();
});
