// The Rules tab of Transactions: the ordered list of rules (with the transactions each categorises), the rule
// dialog, ordering by dragging, and applying the rules (CSD-032 REQ-9, REQ-12, REQ-23, REQ-27, REQ-32).
// Rust matches, orders and counts; this file shows the rules and sends what the user chose.
import { invoke } from '../shared/ipc.js';
import { $, h, iconButton, submitOnce, confirmAction } from '../shared/dom.js';
import { formatDate } from '../shared/format.js';
import { t, tn } from '../shared/i18n.js';
import { makeSortable } from '../shared/sortable.js';
import { openDialog, saveDialog, deleteWithConfirm } from '../shared/crud.js';
import { state, categoryName, changed, run, showResult } from './transactions-shared.js';

/** The rule whose handle gets the focus back after the list is drawn again, so the keys can move it on. */
let focusRuleHandle = null;

/** What names a rule on screen: its text, or "Any transaction" for a rule that only has a period. */
const ruleName = (rule) => (rule.text === '' ? t('Any transaction') : rule.text);

/** The days a rule is limited to, such as "01/07/2026 – 15/07/2026 " (open ends: "from …", "until …"); empty without a period. */
function rulePeriodText(rule) {
  if (!rule.from && !rule.to) return '';
  if (rule.from && rule.to) return `${formatDate(rule.from)} – ${formatDate(rule.to)} `;
  return rule.from ? `${t('from {date}', { date: formatDate(rule.from) })} ` : `${t('until {date}', { date: formatDate(rule.to) })} `;
}

export function renderRules() {
  const { rules } = state;
  $('transaction-rules-empty').hidden = rules.length > 0;
  $('transaction-rules').replaceChildren(
    ...rules.map((rule) => {
      const handle = iconButton('grip', t('Reorder {name}', { name: ruleName(rule) }), () => {});
      handle.classList.add('drag-handle');
      handle.dataset.ruleId = rule.id;
      return h(
        'li',
        { class: 'job-row' },
        handle,
        h('span', { class: 'job-name' }, ruleName(rule)),
        h('span', { class: 'job-meta' }, `${rulePeriodText(rule)}→ ${categoryName(rule.category_id)}`),
        h('span', { class: 'job-meta rule-count' }, tn('{count} transaction', '{count} transactions', rule.transaction_count)),
        h(
          'span',
          { class: 'job-actions' },
          iconButton('edit', t('Edit {name}', { name: ruleName(rule) }), () => openRuleDialog(rule)),
          iconButton('delete', t('Delete {name}', { name: ruleName(rule) }), () => deleteRule(rule)),
        ),
      );
    }),
  );
  if (focusRuleHandle !== null) {
    $('transaction-rules').querySelector(`.drag-handle[data-rule-id="${focusRuleHandle}"]`)?.focus();
    focusRuleHandle = null;
  }
}

function moveRule(rule, position) {
  return run(async () => {
    await invoke('move_transaction_rule', { ruleId: rule.id, position });
    await changed();
  });
}

let editingRule = null;
/** The value of the last option of the category choice of a new rule: the rule creates its category. */
const NEW_CATEGORY = '__new__';

/** Shows the name field while "New category…" is chosen, and only then (CSD-032 REQ-9). */
function syncNewCategory() {
  const form = $('transaction-rule-form');
  const creating = form.elements.category.value === NEW_CATEGORY;
  $('transaction-rule-new-category').hidden = !creating;
  if (creating) form.elements.name.focus();
}

/**
 * The rule dialog for a new rule that starts from a transaction: its counterparty (its description
 * when it has none) as the text and its category as the category, both still editable (CSD-032 REQ-23).
 */
export function openRuleFromTransaction(transaction) {
  openRuleDialog(null, {
    text: transaction.counterparty_name || transaction.description,
    categoryId: transaction.category_id,
  });
}

function openRuleDialog(rule = null, start = null) {
  editingRule = rule;
  openDialog({
    dialog: 'transaction-rule-dialog',
    form: 'transaction-rule-form',
    error: 'transaction-rule-error',
    title: 'transaction-rule-dialog-title',
    titleText: rule ? t('Edit rule') : t('Add rule'),
    focus: 'text',
    fill(form) {
      form.elements.category.replaceChildren(
        ...state.categories.map((category) => h('option', { value: category.id }, category.name)),
        // A new rule may create its category; an edit chooses among the existing ones.
        rule ? null : h('option', { value: NEW_CATEGORY }, t('New category…')),
      );
      form.elements.name.value = '';
      form.elements.from.value = rule?.from ? formatDate(rule.from) : '';
      form.elements.to.value = rule?.to ? formatDate(rule.to) : '';
      if (rule) {
        form.elements.text.value = rule.text;
        form.elements.category.value = rule.category_id;
      } else if (start) {
        form.elements.text.value = start.text;
        if (start.categoryId) form.elements.category.value = start.categoryId;
      }
      syncNewCategory();
    },
  });
}

function submitRule() {
  const form = $('transaction-rule-form');
  const text = form.elements.text.value;
  const categoryId = form.elements.category.value;
  const period = { from: form.elements.from.value, to: form.elements.to.value };
  const creating = categoryId === NEW_CATEGORY;
  return saveDialog({
    dialog: 'transaction-rule-dialog',
    error: 'transaction-rule-error',
    changed,
    save: () =>
      editingRule
        ? invoke('edit_transaction_rule', { ruleId: editingRule.id, text, categoryId, ...period })
        : invoke('add_transaction_rule', creating ? { text, newCategory: form.elements.name.value, ...period } : { text, categoryId, ...period }),
  });
}

function deleteRule(rule) {
  return deleteWithConfirm({
    title: t('Delete {name}?', { name: ruleName(rule) }),
    message: t('Transactions that already have a category keep it.'),
    confirmLabel: t('Delete rule'),
    command: 'delete_transaction_rule',
    args: { ruleId: rule.id },
    changed,
  });
}

/** Runs the rules again on the non-manual transactions, or on all of them after a confirmation (CSD-032 REQ-12). */
async function reapply(scope) {
  if (scope === 'all') {
    const confirmed = await confirmAction({
      title: t('Apply rules including manually set?'),
      message: t('The rules will also change the categories you set by hand. You can undo this.'),
      confirmLabel: t('Apply rules'),
    });
    if (!confirmed) return;
  }
  await run(async () => {
    const { changed: count } = await invoke('reapply_transaction_rules', { scope });
    showResult(t('{count} changed', { count }));
    await changed();
  });
}

export function initTransactionRules() {
  makeSortable($('transaction-rules'), {
    handle: '.drag-handle',
    onMove: (from, to) => {
      const rule = state.rules[from];
      focusRuleHandle = rule.id;
      return moveRule(rule, to);
    },
    onCancel: renderRules,
  });
  $('transaction-rule-add').addEventListener('click', () => openRuleDialog(null));
  submitOnce($('transaction-rule-form'), submitRule);
  $('transaction-rule-form').elements.category.addEventListener('change', syncNewCategory);
  $('transaction-apply-rules').addEventListener('click', () => reapply('all_non_manual'));
  $('transaction-apply-rules-all').addEventListener('click', () => reapply('all'));
}
