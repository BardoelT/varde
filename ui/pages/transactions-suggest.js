// The dialog "Suggest rules" of the Rules tab (CSD-032 REQ-29). Rust finds the suggestions, recounts an
// edited text and adds the chosen rules; this file only shows them and sends what the user chose.

import { $, h, submitOnce, showToast } from '../shared/dom.js';
import { invoke, errorText } from '../shared/ipc.js';
import { t, tn } from '../shared/i18n.js';

/** How long after the last keystroke an edited text is recounted. */
const RECOUNT_DELAY_MS = 250;
/** The minimal count the dialog opens with (CSD-032 REQ-29). */
const DEFAULT_MIN_COUNT = '5';

let getCategories = () => [];
let onChanged = async () => {};
/**
 * The rows of the list: `{ text, count, categoryId, checked }`, `categoryId` empty for none, `checked` false until
 * the user ticks the row or chooses a category for it; `error` after a refused add.
 */
let rows = [];
let requestNumber = 0;

const minCountField = () => $('transaction-suggest-form').elements.min_count;

/** The minimal count as a number, or `null` when the field is not a whole number of 1 or more. */
function minCount() {
  const text = minCountField().value.trim();
  return /^\d+$/.test(text) && Number(text) >= 1 ? Number(text) : null;
}

/** The rows the user ticked: the only ones that are added. */
function chosenRows() {
  return rows.filter((row) => row.checked);
}

/** The count of the chosen rules and the button that adds them. */
function syncChosen() {
  const count = chosenRows().length;
  $('transaction-suggest-chosen').textContent = tn('{count} rule chosen', '{count} rules chosen', count);
  const add = $('transaction-suggest-add');
  add.textContent = tn('Add {count} rule', 'Add {count} rules', count);
  add.disabled = count === 0;
}

function categorySelect(row) {
  const select = h(
    'select',
    { class: 'table-select', 'aria-label': t('Category of {name}', { name: row.text }) },
    h('option', { value: '' }, '\u2014'),
    getCategories().map((category) => h('option', { value: category.id }, category.name)),
  );
  select.value = row.categoryId;
  select.addEventListener('change', () => {
    row.categoryId = select.value;
    // Choosing or changing a category ticks the row, as the user means to add it; a rule needs a category,
    // so clearing it unticks the row.
    row.checked = row.categoryId !== '';
    syncRow(row);
    syncChosen();
  });
  row.select = select;
  return select;
}

/** The tick of a row is only possible while it has a category. */
function syncRow(row) {
  row.box.disabled = row.categoryId === '';
  row.box.checked = row.checked;
}

/** Recounts an edited text after a pause; the row stays whatever the new count is. */
function scheduleRecount(row, countCell) {
  clearTimeout(row.timer);
  countCell.textContent = '\u2026';
  row.timer = setTimeout(async () => {
    const asked = row.text;
    try {
      const count = await invoke('count_transaction_rule_matches', { text: asked });
      if (row.text === asked) {
        row.count = count;
        countCell.textContent = String(count);
      }
    } catch {
      // An empty text has no count; adding it is refused with the usual message.
      if (row.text === asked) countCell.textContent = '\u2013';
    }
  }, RECOUNT_DELAY_MS);
}

function renderRow(row) {
  const countCell = h('td', { class: 'num' }, String(row.count));
  const input = h('input', { type: 'text', maxlength: '100', autocomplete: 'off', value: row.text, 'aria-label': t('Rule text') });
  const message = h('span', { class: 'row-error', role: 'alert', hidden: row.error ? null : true }, row.error ?? '');
  row.input = input;
  row.message = message;
  input.addEventListener('input', () => {
    row.text = input.value;
    row.error = null;
    message.hidden = true;
    input.removeAttribute('aria-invalid');
    scheduleRecount(row, countCell);
  });
  const box = h('input', { type: 'checkbox', 'aria-label': t('Add the rule {name}', { name: row.text }) });
  row.box = box;
  box.addEventListener('change', () => {
    row.checked = box.checked;
    syncChosen();
  });
  const category = categorySelect(row);
  syncRow(row);
  return h('tr', {}, h('td', { class: 'select-col' }, box), h('td', {}, input, message), countCell, h('td', {}, category));
}

function renderRows(notShown) {
  $('transaction-suggest-rows').replaceChildren(...rows.map(renderRow));
  $('transaction-suggest-empty').hidden = rows.length > 0;
  $('transaction-suggest-wrap').hidden = rows.length === 0;
  const more = $('transaction-suggest-more');
  more.hidden = notShown === 0;
  more.textContent = tn('{count} more not shown: raise the minimal count', '{count} more not shown: raise the minimal count', notShown);
  syncChosen();
}

/**
 * Asks Rust for the suggestions of the minimal count. Ticked rows stay, with what the user typed, even when
 * they are no longer suggested; the rest follows the new count.
 */
async function loadSuggestions() {
  const count = minCount();
  $('transaction-suggest-min-error').hidden = count !== null;
  if (count === null) return;
  const mine = ++requestNumber;
  try {
    const result = await invoke('suggest_transaction_rules', { minCount: count });
    if (mine !== requestNumber) return;
    const kept = chosenRows();
    const keptTexts = new Set(kept.map((row) => row.text.toLowerCase()));
    const fresh = result.suggestions
      .filter((suggestion) => !keptTexts.has(suggestion.text.toLowerCase()))
      .map((suggestion) => ({ text: suggestion.text, count: suggestion.count, categoryId: suggestion.category_id ?? '', checked: false }));
    rows = [...kept, ...fresh].sort((a, b) => b.count - a.count || a.text.localeCompare(b.text));
    renderRows(result.not_shown);
  } catch (error) {
    showToast(errorText(error));
  }
}

function openSuggestDialog() {
  rows = [];
  minCountField().value = DEFAULT_MIN_COUNT;
  $('transaction-suggest-error').hidden = true;
  $('transaction-suggest-min-error').hidden = true;
  $('transaction-suggest-dialog').showModal();
  minCountField().focus();
  loadSuggestions();
}

/** Adds the chosen rules; a refused row is marked and nothing is added (CSD-032 REQ-29). */
async function addChosen() {
  const chosen = chosenRows();
  $('transaction-suggest-error').hidden = true;
  try {
    const result = await invoke('add_transaction_rules', {
      rules: chosen.map((row) => ({ text: row.text, category_id: row.categoryId })),
    });
    if (result.rejected) {
      const row = chosen[result.rejected.index];
      const message = errorText({ code: result.rejected.code, message: result.rejected.message });
      row.error = message;
      row.message.textContent = message;
      row.message.hidden = false;
      row.input.setAttribute('aria-invalid', 'true');
      row.input.focus();
      return;
    }
    $('transaction-suggest-dialog').close();
    showToast(
      `${tn('{count} rule added', '{count} rules added', result.added)}, ${tn('{count} transaction categorised', '{count} transactions categorised', result.changed)}`,
      'notice',
    );
    await onChanged();
  } catch (error) {
    $('transaction-suggest-error').textContent = errorText(error);
    $('transaction-suggest-error').hidden = false;
  }
}

/** Wires the button and the dialog; `categories()` gives the categories as Rust listed them. */
export function initTransactionSuggest({ categories, changed }) {
  getCategories = categories;
  onChanged = changed;
  $('transaction-suggest').addEventListener('click', openSuggestDialog);
  minCountField().addEventListener('input', loadSuggestions);
  submitOnce($('transaction-suggest-form'), addChosen);
}
