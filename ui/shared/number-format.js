// The number format the user chose (CSD-029): how amounts, percentages and ratios are written and
// how a typed amount is read back when a format changes. Display only: Rust holds the preference,
// computes every figure and reads every typed amount; the project file and IPC stay canonical
// ("1234.50"). Nothing here changes a value, only how it is written.

import { HIDDEN, isDiscrete } from './discrete-mode.js';

/** The three formats, their separators and the example the Settings page shows. */
const FORMATS = {
  point_comma: { group: ',', decimal: '.', example: '1,234.50' },
  comma_point: { group: '.', decimal: ',', example: '1.234,50' },
  apostrophe_comma: { group: "'", decimal: ',', example: "1'234,50" },
};

/** The format a missing or unknown code falls back to (CSD-029 REQ-2). */
const DEFAULT_FORMAT = 'point_comma';

let current = DEFAULT_FORMAT;
const listeners = [];

/** The code of the format that is shown: `point_comma`, `comma_point` or `apostrophe_comma`. */
export const currentNumberFormat = () => current;

/** The example of a format as the Settings page lists it, such as "1.234,50". */
export const numberFormatExample = (code) => (FORMATS[code] ?? FORMATS[DEFAULT_FORMAT]).example;

/** The decimal separator of the current format. */
export const decimalSeparator = () => FORMATS[current].decimal;

/** Calls `listener(code, previousCode)` after the format has changed, to draw the pages again. */
export function onNumberFormatChange(listener) {
  listeners.push(listener);
}

/** Switches to `code` and lets the listeners redraw. An unknown code means the default (REQ-9). */
export async function setNumberFormat(code) {
  const previous = current;
  current = code in FORMATS ? code : DEFAULT_FORMAT;
  for (const listener of listeners) await listener(current, previous);
}

/** Puts the grouping separator between the thousands of the digits of a whole number. */
const group = (digits, separator) => digits.replace(/\B(?=(\d{3})+(?!\d))/g, separator);

/** Writes a canonical decimal string such as "-1234567.50" in the current format: "-1.234.567,50". */
function writeNumber(text) {
  const { group: grouping, decimal } = FORMATS[current];
  const [whole, fraction] = String(text).split('.');
  const grouped = group(whole, grouping);
  return fraction === undefined ? grouped : `${grouped}${decimal}${fraction}`;
}

/**
 * Writes an amount in the current format, or the placeholder of a hidden amount while Discrete
 * mode is on, whatever the amount and its sign (CSD-030 REQ-5, CON-5).
 */
export function formatAmount(text) {
  return isDiscrete() ? HIDDEN : writeNumber(text);
}

/** The text of an amount for an input field: the decimal separator of the format, no grouping. */
export function amountForInput(text) {
  if (text === null || text === undefined || text === '') return '';
  return String(text).replace('.', FORMATS[current].decimal);
}

/**
 * Writes a label that Rust built ("1.50k", "+12.50%", "0.75", "-3") with the decimal separator of
 * the current format. Rust sends these labels in canonical form.
 */
export const formatLabel = (text) =>
  typeof text === 'string' ? text.replace(/(\d)\.(\d)/g, `$1${FORMATS[current].decimal}$2`) : text;

/**
 * Writes a label of an amount that Rust built ("1.50k", "500": the axis and marker labels of a
 * chart) like `formatLabel`, or the placeholder while Discrete mode is on (CSD-030 REQ-5).
 */
export const formatAmountLabel = (text) => (isDiscrete() ? HIDDEN : formatLabel(text));

/**
 * Writes the amounts inside a text ("such as 3000.00") in the current format. A hint shows an example
 * as it is typed, with the decimal separator only; a message groups it like any other amount, or
 * hides it while Discrete mode is on.
 */
export const localiseNumbers = (text, { grouped = true } = {}) =>
  text.replace(/(?<![\w.,'])-?\d+\.\d\d(?!\w|\.\d)/g, (amount, offset, whole) => {
    if (!grouped) return amountForInput(amount);
    // A percentage stays readable while the mode is on (CSD-030 REQ-6, REQ-8).
    return whole[offset + amount.length] === '%' ? writeNumber(amount) : formatAmount(amount);
  });

/**
 * Reads an amount typed in `code` as `{ negative, digits, fraction, grouped }`, or `null` when it
 * does not fit the format: groups of three after a first group of one to three digits, one decimal
 * separator, digits only. Rust reads the typed amounts for real (CSD-029 REQ-6, REQ-7); this only
 * lets an open form keep its amount when the format changes.
 */
function readTyped(text, code) {
  const { group: grouping, decimal } = FORMATS[code];
  const grouping_chars = grouping === "'" ? ["'", '’'] : [grouping];
  const trimmed = text.trim();
  const negative = trimmed.startsWith('-');
  const body = negative ? trimmed.slice(1) : trimmed;
  const parts = body.split(decimal);
  if (parts.length > 2) return null;
  const [whole, fraction] = parts;
  const groups = whole.split(new RegExp(`[${grouping_chars.join('')}]`));
  const grouped = groups.length > 1;
  const digitsOnly = (part) => /^\d+$/.test(part);
  if (!groups.every(digitsOnly)) return null;
  if (grouped && (groups[0].length > 3 || groups.slice(1).some((part) => part.length !== 3))) return null;
  if (fraction !== undefined && !digitsOnly(fraction)) return null;
  return { negative, digits: groups.join(''), fraction, grouped };
}

/**
 * The amount typed in `from` written again in `to`, keeping the grouping when it was typed, or
 * `null` when it cannot be read in `from` (CSD-029 REQ-3).
 */
export function convertTyped(text, from, to) {
  if (text.trim() === '') return '';
  const read = readTyped(text, from);
  if (read === null) return null;
  const target = FORMATS[to];
  const whole = read.grouped ? group(read.digits, target.group) : read.digits;
  const fraction = read.fraction === undefined ? '' : `${target.decimal}${read.fraction}`;
  return `${read.negative ? '-' : ''}${whole}${fraction}`;
}

/** Every field that takes a typed amount. */
const amountFields = () => document.querySelectorAll('input[inputmode="decimal"]');

/**
 * Writes the amounts that are already typed in the new format, or empties a field that cannot be
 * read in the old one (CSD-029 REQ-3), and moves the default of a field that starts with an amount
 * ("0.00") to the new format.
 */
export function rewriteTypedAmounts(to, from) {
  for (const field of amountFields()) {
    const canonicalDefault = field.dataset.canonicalDefault ?? field.getAttribute('value');
    if (canonicalDefault !== null) {
      field.dataset.canonicalDefault = canonicalDefault;
      const wasDefault = field.value === field.defaultValue;
      field.defaultValue = convertTyped(canonicalDefault, 'point_comma', to);
      if (wasDefault) {
        field.value = field.defaultValue;
        continue;
      }
    }
    const converted = convertTyped(field.value, from, to);
    field.value = converted ?? '';
  }
}

onNumberFormatChange(rewriteTypedAmounts);
