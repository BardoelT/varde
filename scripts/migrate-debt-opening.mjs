#!/usr/bin/env node
// One-off migration (CSD-002 REQ-20): upgrades a project file to format_version 2.
// Gives every debt an `opening_date` and `opening_amount`, and every settlement an
// `origin` (Sale for a debt with a house link, Manual otherwise). Outside the app,
// no dependencies.
//
// Usage: node scripts/migrate-debt-opening.mjs <file.varde>
//
// Test hook: the env var VARDE_TODAY (YYYY-MM-DD) replaces today's date, used
// for debts without statements.
//
// Prints only counts and debt names, never amounts or other paths.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';

/** The format_version this script upgrades to (CSD-002 REQ-20). */
const TARGET_VERSION = 2;

class MigrationError extends Error {}

function fail(message) {
  throw new MigrationError(message);
}

function today() {
  const override = process.env.VARDE_TODAY;
  if (override !== undefined) {
    if (!/^\d{4}-\d{2}-\d{2}$/.test(override)) fail('VARDE_TODAY must be YYYY-MM-DD');
    return override;
  }
  const now = new Date();
  const pad = (n) => String(n).padStart(2, '0');
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

/** First day of a statement period: 'YYYY-MM' or 'YYYY'. */
function periodStart(period) {
  if (typeof period === 'string') {
    if (/^\d{4}-(0[1-9]|1[0-2])$/.test(period)) return `${period}-01`;
    if (/^\d{4}$/.test(period)) return `${period}-01-01`;
  }
  return fail('a statement has an unknown period');
}

function isObject(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

/** Earliest statement of a debt; a regular one wins over a settlement one in the same period. */
function firstStatement(statements) {
  let best = null;
  for (const statement of statements) {
    const start = periodStart(statement.period);
    if (typeof statement.balance !== 'string') fail('a statement has an unknown shape');
    const settlement = statement.settlement === true ? 1 : 0;
    if (
      best === null ||
      start < best.start ||
      (start === best.start && settlement < best.settlement)
    ) {
      best = { start, settlement, statement };
    }
  }
  return best;
}

function writeAtomic(file, text) {
  const temporary = path.join(
    path.dirname(file),
    `.${path.basename(file)}.${crypto.randomUUID().replaceAll('-', '')}.tmp`,
  );
  try {
    const fd = fs.openSync(temporary, 'wx', 0o600);
    try {
      fs.writeSync(fd, text);
      fs.fsyncSync(fd);
    } finally {
      fs.closeSync(fd);
    }
    fs.renameSync(temporary, file);
  } catch (error) {
    try {
      fs.rmSync(temporary, { force: true });
    } catch {
      // best effort: the original is untouched either way
    }
    throw error;
  }
}

function run(file) {
  let original;
  try {
    original = fs.readFileSync(file);
  } catch {
    return fail('cannot read the file');
  }
  let project;
  try {
    project = JSON.parse(original.toString('utf8'));
  } catch {
    return fail('the file is not valid JSON');
  }
  if (!isObject(project) || !isObject(project.debts)) fail('the file has no debts section');
  const { debts, statements } = project.debts;
  if (!Array.isArray(debts) || !Array.isArray(statements)) fail('the debts section has an unknown shape');
  if (!debts.every(isObject) || !statements.every(isObject)) fail('the debts section has an unknown shape');

  const version = project.format_version;
  if (!Number.isInteger(version) || version < 1) fail('the file has no valid format_version');
  if (version > TARGET_VERSION) fail('the file is from a newer version of the app');

  const todo = [];
  const needOrigin = [];
  for (const debt of debts) {
    const hasDate = Object.hasOwn(debt, 'opening_date');
    const hasAmount = Object.hasOwn(debt, 'opening_amount');
    if (hasDate !== hasAmount) fail('a debt has only one of opening_date and opening_amount');
    if (!hasDate) {
      if (typeof debt.id !== 'string' || typeof debt.name !== 'string') fail('a debt has an unknown shape');
      todo.push(debt);
    }
    if (debt.settlement !== undefined && debt.settlement !== null) {
      if (!isObject(debt.settlement)) fail('a debt has an unknown shape');
      if (!Object.hasOwn(debt.settlement, 'origin')) needOrigin.push(debt);
    }
  }
  if (todo.length === 0 && needOrigin.length === 0 && version === TARGET_VERSION) {
    return console.log('Nothing to upgrade');
  }

  const uncorrected = [];
  for (const debt of todo) {
    const first = firstStatement(statements.filter((s) => s.debt_id === debt.id));
    if (first === null) {
      debt.opening_date = today();
      debt.opening_amount = '0.00';
      uncorrected.push(debt.name);
    } else {
      debt.opening_date = first.start;
      debt.opening_amount = first.statement.balance;
    }
  }
  // A settlement made by a sale sits on a linked debt, a manual one does not
  // (CSD-026 v3.0). Keys in the order the app writes them.
  for (const debt of needOrigin) {
    const { outcome, date, amount, ...rest } = debt.settlement;
    const linked = debt.house_id !== undefined && debt.house_id !== null;
    debt.settlement = { outcome, origin: linked ? 'Sale' : 'Manual', date, amount, ...rest };
  }
  project.format_version = TARGET_VERSION;
  // Same layout as the app writes: 2-space indent, no trailing newline.
  const text = JSON.stringify(project, null, 2);

  try {
    fs.writeFileSync(`${file}.bak`, original, { flag: 'wx', mode: 0o600 });
  } catch {
    fail('cannot write the .bak copy (does it already exist?)');
  }
  writeAtomic(file, text);

  console.log(`Upgraded to format_version ${TARGET_VERSION}: ${todo.length} opening(s), ${needOrigin.length} settlement origin(s). Backup written next to the file with the extension .bak.`);
  if (uncorrected.length > 0) {
    console.log('Debts without statements (opening date today, amount 0.00), correct these in the app:');
    for (const name of uncorrected) console.log(`- ${name}`);
  }
}

const file = process.argv[2];
if (!file || process.argv.length > 3) {
  console.error('Usage: node scripts/migrate-debt-opening.mjs <file.varde>');
  process.exit(2);
}
try {
  run(file);
} catch (error) {
  console.error(`Error: ${error instanceof MigrationError ? error.message : 'the upgrade failed'}. Nothing was changed.`);
  process.exit(1);
}
