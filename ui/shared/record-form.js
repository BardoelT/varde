// The record dialog of the series areas (bank entries, investment statements, debt statements,
// house valuations): a parent (account, debt or house) chosen in a dropdown, a month or a year
// that follows it, some amount fields and a note. The page supplies the ids, the fields and the
// texts; this file holds the open, fill and period behaviour they all share.
import { $, h } from './dom.js';
import { currentYear, currentMonth } from './format.js';
import { t } from './i18n.js';

/**
 * Creates the record form helpers.
 *
 * Options (the ids are element ids):
 * - `form`, `dialog`, `title`, `error`, `periodLabel`, `copy`: the parts of the dialog.
 * - `parentField`: the field of the form and of a record that names the parent (`account_id`).
 * - `parents()`: the parents as the page last loaded them, in the order of the dropdown.
 * - `valueFields`: the fields copied from a record when editing (`['balance', 'note']`).
 * - `focusField`: the field focused when editing (a new record focuses the period).
 * - `titleText(record)`: the dialog title; `record` is `null` when adding.
 * - `beforeFill()` and `afterFill()` (optional): run before and after the fields are set, for
 *   what only one area has (the balance and growth calculation of an investment statement).
 *
 * Returns `{ open(record), apply({ resetPeriod }), chosenParent(), isYearly(), editing }`;
 * `editing` is the record being edited, or `null` when adding.
 */
export function createRecordForm(options) {
  const { form, dialog, title, error, periodLabel, copy, parentField, parents, valueFields, focusField, titleText } = options;
  const { beforeFill = () => {}, afterFill = () => {} } = options;
  let editing = null;
  /** Whether the form currently asks for a year, so the period resets only when it changes. */
  let formIsYearly = false;

  function fillChoices(selectedId) {
    // The list is alphabetical, so the first option is the first parent by name.
    const select = $(form).elements[parentField];
    select.replaceChildren(...parents().map((parent) => h('option', { value: parent.id }, parent.name)));
    select.value = selectedId ?? parents()[0]?.id ?? '';
  }

  function chosenParent() {
    return parents().find((parent) => parent.id === $(form).elements[parentField].value);
  }

  /** Asks for a month or a year, following the chosen parent. */
  function apply({ resetPeriod }) {
    const period = $(form).elements.period;
    const yearly = chosenParent()?.period === 'Yearly';
    $(periodLabel).textContent = yearly ? t('Year') : t('Month');
    period.placeholder = yearly ? String(new Date().getFullYear() - 1) : t('yyyy-mm');
    period.maxLength = yearly ? 4 : 7;
    $(copy).textContent = yearly ? t('Copy previous year') : t('Copy previous month');
    if (resetPeriod && yearly !== formIsYearly) {
      period.value = yearly ? currentYear() : currentMonth();
    }
    formIsYearly = yearly;
  }

  /** Opens the form to add a record, or to edit `record` when given. */
  function open(record = null) {
    editing = record;
    const formElement = $(form);
    formElement.reset();
    $(error).hidden = true;
    fillChoices(record?.[parentField]);
    formElement.elements[parentField].disabled = Boolean(record);
    $(copy).hidden = Boolean(record);
    beforeFill();
    $(title).textContent = titleText(record);
    if (record) {
      // The period is editable too; the parent is not.
      formIsYearly = record.period.length === 4;
      $(periodLabel).textContent = formIsYearly ? t('Year') : t('Month');
      formElement.elements.period.maxLength = record.period.length;
      formElement.elements.period.value = record.period;
      for (const field of valueFields) formElement.elements[field].value = record[field];
    } else {
      formIsYearly = null; // force the period to its default for the first parent
      formElement.elements.period.value = '';
      apply({ resetPeriod: true });
    }
    afterFill();
    $(dialog).showModal();
    formElement.elements[record ? focusField : 'period'].focus();
  }

  return {
    open,
    apply,
    chosenParent,
    isYearly: () => formIsYearly,
    get editing() {
      return editing;
    },
  };
}
