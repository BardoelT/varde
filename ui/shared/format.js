// Display formatting and "today" helpers. Display only: no figure is computed here.

/** Shows an ISO date such as "2024-03-01" as dd/mm/yyyy (CSD-001 REQ-22). Display only. */
export function formatDate(iso) {
  return /^\d{4}-\d{2}-\d{2}$/.test(iso) ? iso.split('-').reverse().join('/') : String(iso);
}

// Amounts, percentages and typed amounts follow the number format the user chose (CSD-029).
export { formatAmount, formatLabel, amountForInput } from './number-format.js';

/** The current calendar year as text, such as "2026". */
export function currentYear() {
  return String(new Date().getFullYear());
}

/** The current month as text, such as "2026-10". */
export function currentMonth() {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`;
}

/** The first line of a note, with an ellipsis when there is more (the notes column of every record table). */
export function noteSummary(note) {
  const [first, ...rest] = note.split('\n');
  return rest.length > 0 ? `${first} …` : first;
}
