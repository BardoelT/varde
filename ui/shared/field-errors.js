// Which form field a command error is about, so a failed save can mark that field for assistive
// technology (`aria-invalid`) and put the cursor there. The mapping reads the stable error code;
// a code that names no single field (a duplicate name of another kind, a data error, a failed
// project file) maps to nothing, and the error box alone speaks.

/** Code pattern -> field name, the first match wins. Specific patterns come before general ones. */
const FIELD_OF_CODE = [
  [/NET_DEPOSITS/, 'net_deposits'],
  [/GROWTH/, 'growth'],
  [/PURCHASE_AMOUNT/, 'purchase_amount'],
  [/PURCHASE_DATE/, 'purchase_date'],
  [/SELL_DATE|SELL_BEFORE_PURCHASE/, 'sell_date'],
  [/GROSS/, 'gross_salary'],
  [/EMPLOYER/, 'employer'],
  [/START_REQUIRED/, 'start_date'],
  [/END_BEFORE_START/, 'end_date'],
  [/NAME/, 'name'],
  [/NOTE/, 'note'],
  [/BALANCE/, 'balance'],
  [/CATEGORY/, 'category'],
  [/_VALUE_/, 'value'],
  [/^EXPENSE_AMOUNT/, 'amount'],
  [/^EXPENSE_DATE/, 'date'],
  [/PERIOD|_MONTH_|DUPLICATE|YEAR|OUTSIDE_HOLDING/, 'period'],
];

/**
 * The field of `form` that the error code is about, or `null` when the code names no field of
 * this form or the field cannot take focus (disabled).
 */
export function fieldForError(form, code) {
  if (!form || typeof code !== 'string') return null;
  const match = FIELD_OF_CODE.find(([pattern]) => pattern.test(code));
  const field = match ? form.elements[match[1]] : null;
  return field && !field.disabled ? field : null;
}
