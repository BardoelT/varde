// The Categories tab of Transactions: the list of categories with their transaction counts, and adding,
// renaming and deleting them (CSD-032 REQ-8, REQ-13, REQ-30). Rust holds and checks everything.
import { invoke } from '../shared/ipc.js';
import { $, h, iconButton, submitOnce } from '../shared/dom.js';
import { t, tn } from '../shared/i18n.js';
import { openDialog, saveDialog, deleteWithConfirm } from '../shared/crud.js';
import { state, changed } from './transactions-shared.js';

export function renderCategories() {
  $('transaction-categories').replaceChildren(
    h('li', { class: 'job-row' }, h('span', { class: 'job-name' }, t('Uncategorized')), h('span', { class: 'job-meta' }, t('Built in'))),
    ...state.categories.map((category) =>
      h(
        'li',
        { class: 'job-row' },
        h('span', { class: 'job-name' }, category.name),
        h('span', { class: 'job-meta rule-count' }, tn('{count} transaction', '{count} transactions', category.transaction_count)),
        h(
          'span',
          { class: 'job-actions' },
          iconButton('edit', t('Edit {name}', { name: category.name }), () => openCategoryDialog(category)),
          iconButton('delete', t('Delete {name}', { name: category.name }), () => deleteCategory(category)),
        ),
      ),
    ),
  );
}

let editingCategory = null;

function openCategoryDialog(category = null) {
  editingCategory = category;
  openDialog({
    dialog: 'transaction-category-dialog',
    form: 'transaction-category-form',
    error: 'transaction-category-error',
    title: 'transaction-category-dialog-title',
    titleText: category ? t('Edit category') : t('Add category'),
    focus: 'name',
    fill(form) {
      if (category) form.elements.name.value = category.name;
    },
  });
}

function submitCategory() {
  const name = $('transaction-category-form').elements.name.value;
  return saveDialog({
    dialog: 'transaction-category-dialog',
    error: 'transaction-category-error',
    changed,
    save: () =>
      editingCategory
        ? invoke('rename_transaction_category', { categoryId: editingCategory.id, name })
        : invoke('add_transaction_category', { name }),
  });
}

function deleteCategory(category) {
  return deleteWithConfirm({
    title: t('Delete {name}?', { name: category.name }),
    message: t('Its transactions become Uncategorized and the rules that name it are deleted.'),
    confirmLabel: t('Delete category'),
    command: 'delete_transaction_category',
    args: { categoryId: category.id },
    changed,
  });
}

export function initTransactionCategories() {
  $('transaction-category-add').addEventListener('click', () => openCategoryDialog(null));
  submitOnce($('transaction-category-form'), submitCategory);
}
