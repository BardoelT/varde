// Tests of scripts/migrate-debt-opening.mjs (CSD-002 REQ-20). Synthetic data only.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const script = fileURLToPath(new URL('../migrate-debt-opening.mjs', import.meta.url));

function tempDir() {
  return fs.mkdtempSync(path.join(os.tmpdir(), 'varde-migrate-'));
}

function project(debts, statements) {
  return {
    format_version: 1,
    income: { jobs: [] },
    debts: { debts, statements },
    settings: { currency: 'EUR' },
  };
}

function write(dir, content) {
  const file = path.join(dir, 'example.varde');
  fs.writeFileSync(file, typeof content === 'string' ? content : JSON.stringify(content, null, 2));
  return file;
}

function run(file, today = '2026-11-04') {
  return spawnSync(process.execPath, [script, file], {
    encoding: 'utf8',
    env: { ...process.env, VARDE_TODAY: today },
  });
}

const mortgage = { id: 'id-1', name: 'Example Mortgage', period: 'Monthly', category: 'Mortgage' };
const loan = { id: 'id-2', name: 'Example Student Loan', period: 'Yearly', category: 'Other' };
const card = { id: 'id-3', name: 'Example Card', period: 'Monthly', category: 'Other' };
const st = (debt_id, period, balance, extra = {}) => ({ debt_id, period, balance, note: '', ...extra });

test('test_csd002_ac_20_1_opening_from_first_statement', () => {
  const dir = tempDir();
  const original = project(
    [mortgage],
    [st('id-1', '2025-04', '189000.00'), st('id-1', '2025-03', '190000.00')],
  );
  const file = write(dir, original);
  const before = fs.readFileSync(file);
  const result = run(file);
  assert.equal(result.status, 0, result.stderr);
  const upgraded = JSON.parse(fs.readFileSync(file, 'utf8'));
  assert.equal(upgraded.debts.debts[0].opening_date, '2025-03-01');
  assert.equal(upgraded.debts.debts[0].opening_amount, '190000.00');
  assert.deepEqual(fs.readFileSync(`${file}.bak`), before);
  assert.ok(!result.stdout.includes('190000'), 'no amounts in the output');
  // Layout as the app writes it: 2-space indent, no trailing newline.
  const text = fs.readFileSync(file, 'utf8');
  assert.equal(text, JSON.stringify(upgraded, null, 2));
  assert.ok(!text.endsWith('\n'));
  assert.deepEqual(fs.readdirSync(dir).sort(), ['example.varde', 'example.varde.bak']);
});

test('test_csd002_ac_20_2_yearly_debt', () => {
  const dir = tempDir();
  const file = write(dir, project([loan], [st('id-2', '2025', '9000.00'), st('id-2', '2024', '8000.00')]));
  assert.equal(run(file).status, 0);
  const debt = JSON.parse(fs.readFileSync(file, 'utf8')).debts.debts[0];
  assert.equal(debt.opening_date, '2024-01-01');
  assert.equal(debt.opening_amount, '8000.00');
});

test('test_csd002_ac_20_3_debt_without_statements', () => {
  const dir = tempDir();
  const file = write(dir, project([card, mortgage], [st('id-1', '2025-03', '190000.00')]));
  const result = run(file, '2026-11-04');
  assert.equal(result.status, 0, result.stderr);
  const debts = JSON.parse(fs.readFileSync(file, 'utf8')).debts.debts;
  assert.equal(debts[0].opening_date, '2026-11-04');
  assert.equal(debts[0].opening_amount, '0.00');
  assert.ok(result.stdout.includes('Example Card'));
  assert.ok(!result.stdout.includes('Example Mortgage'));
});

test('test_csd002_ac_20_4_nothing_to_do', () => {
  const dir = tempDir();
  const done = { ...mortgage, opening_date: '2025-03-01', opening_amount: '190000.00' };
  const file = write(dir, project([done], []));
  const old = new Date('2020-01-01T00:00:00Z');
  fs.utimesSync(file, old, old);
  const before = fs.readFileSync(file);
  const mtime = fs.statSync(file).mtimeMs;
  const result = run(file);
  assert.equal(result.status, 0);
  assert.match(result.stdout, /Nothing to upgrade/);
  assert.deepEqual(fs.readFileSync(file), before);
  assert.equal(fs.statSync(file).mtimeMs, mtime);
  assert.ok(!fs.existsSync(`${file}.bak`));
});

test('test_csd002_ac_20_5_invalid_json_changes_nothing', () => {
  const dir = tempDir();
  const file = write(dir, '{ "debts": ');
  const before = fs.readFileSync(file);
  const result = run(file);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Error/);
  assert.deepEqual(fs.readFileSync(file), before);
  assert.ok(!fs.existsSync(`${file}.bak`));
  assert.deepEqual(fs.readdirSync(dir), ['example.varde']);
});

test('test_csd002_ac_20_5_unreadable_file_and_missing_debts_section_fail', () => {
  const dir = tempDir();
  assert.notEqual(run(path.join(dir, 'missing.varde')).status, 0);
  const file = write(dir, { format_version: 1 });
  const before = fs.readFileSync(file);
  assert.notEqual(run(file).status, 0);
  assert.deepEqual(fs.readFileSync(file), before);
  assert.ok(!fs.existsSync(`${file}.bak`));
});

test('test_csd002_ac_20_5_debt_with_only_one_key_fails', () => {
  const dir = tempDir();
  const file = write(dir, project([{ ...mortgage, opening_date: '2025-03-01' }, card], []));
  const before = fs.readFileSync(file);
  const result = run(file);
  assert.notEqual(result.status, 0);
  assert.deepEqual(fs.readFileSync(file), before);
  assert.ok(!fs.existsSync(`${file}.bak`));
});

test('test_csd002_ac_20_1_settlement_statement_counts_but_regular_wins_in_same_period', () => {
  const dir = tempDir();
  const file = write(
    dir,
    project(
      [mortgage, card],
      [
        st('id-1', '2025-03', '100.00', { settlement: true }),
        st('id-1', '2025-03', '190000.00'),
        st('id-3', '2025-05', '500.00', { settlement: true }),
      ],
    ),
  );
  assert.equal(run(file).status, 0);
  const debts = JSON.parse(fs.readFileSync(file, 'utf8')).debts.debts;
  assert.equal(debts[0].opening_amount, '190000.00');
  assert.equal(debts[1].opening_date, '2025-05-01');
  assert.equal(debts[1].opening_amount, '500.00');
});

test('test_csd002_ac_20_4_second_run_says_nothing_to_upgrade', () => {
  const dir = tempDir();
  const file = write(dir, project([mortgage], [st('id-1', '2025-03', '190000.00')]));
  assert.equal(run(file).status, 0);
  const after = fs.readFileSync(file);
  const second = run(file);
  assert.equal(second.status, 0);
  assert.match(second.stdout, /Nothing to upgrade/);
  assert.deepEqual(fs.readFileSync(file), after);
});
