// Small DOM helpers shared by every page: element builder, icon buttons, toast
// and the confirmation dialog.
import { errorText } from './ipc.js';
import { fieldForError } from './field-errors.js';

/** The element with the given id. */
export const $ = (id) => document.getElementById(id);

/** Builds an element. Strings become text nodes, so user data is never parsed as HTML. */
export function h(tag, attrs = {}, ...children) {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(attrs)) {
    if (value === undefined || value === null || value === false) continue;
    if (key.startsWith('on')) {
      node.addEventListener(key.slice(2).toLowerCase(), value);
    } else if (key === 'class') {
      node.className = value;
    } else {
      node.setAttribute(key, value === true ? '' : value);
    }
  }
  for (const child of children.flat()) {
    if (child === null || child === undefined || child === false) continue;
    node.append(child);
  }
  return node;
}

const SVG_NS = 'http://www.w3.org/2000/svg';
const ICON_PATHS = {
  edit: ['M12 20h9', 'M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z'],
  info: ['M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20Z', 'M12 16v-4', 'M12 8h.01'],
  filter: ['M3 4h18l-7 8v6l-4 2v-8Z'],
  'filter-active': ['M3 4h18l-7 8v6l-4 2v-8Z'],
  delete: ['M3 6h18', 'M8 6V4h8v2', 'M19 6l-1 14H6L5 6', 'M10 11v6', 'M14 11v6'],
};

/** A row action as an icon button; `label` names it for screen readers and the tooltip. */
export function iconButton(kind, label, onClick) {
  const svg = document.createElementNS(SVG_NS, 'svg');
  for (const [key, value] of Object.entries({
    viewBox: '0 0 24 24', width: '16', height: '16', fill: kind === 'filter-active' ? 'currentColor' : 'none', stroke: 'currentColor',
    'stroke-width': '2', 'stroke-linecap': 'round', 'stroke-linejoin': 'round', 'aria-hidden': 'true',
  })) {
    svg.setAttribute(key, value);
  }
  for (const d of ICON_PATHS[kind]) {
    const path = document.createElementNS(SVG_NS, 'path');
    path.setAttribute('d', d);
    svg.append(path);
  }
  const danger = kind === 'delete' ? ' danger' : '';
  return h('button', { type: 'button', class: `icon-btn${danger}`, 'aria-label': label, title: label, onClick }, svg);
}

let toastTimer = null;

export function showToast(message) {
  const toast = document.getElementById('toast');
  toast.textContent = message;
  toast.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    toast.hidden = true;
  }, 6000);
}

/**
 * Asks the user to confirm. Resolves true only if they choose the confirm button.
 *
 * The answer comes from the button clicks and Escape, not from the dialog's
 * `close` event: that event can arrive late and would then answer the next
 * confirmation by mistake.
 */
export function confirmAction({ title, message, confirmLabel, danger = false }) {
  const dialog = document.getElementById('confirm-dialog');
  if (dialog.open) return Promise.resolve(false);
  document.getElementById('confirm-title').textContent = title;
  document.getElementById('confirm-message').textContent = message;
  const ok = document.getElementById('confirm-ok');
  const cancel = document.getElementById('confirm-cancel');
  ok.textContent = confirmLabel;
  ok.classList.toggle('danger', danger);
  ok.classList.toggle('primary', !danger);
  return new Promise((resolve) => {
    const finish = (confirmed) => {
      ok.onclick = null;
      cancel.onclick = null;
      dialog.oncancel = null;
      if (dialog.open) dialog.close();
      resolve(confirmed);
    };
    ok.onclick = (event) => {
      event.preventDefault();
      finish(true);
    };
    cancel.onclick = (event) => {
      event.preventDefault();
      finish(false);
    };
    dialog.oncancel = () => finish(false); // Escape
    dialog.showModal();
  });
}

/** Removes the `aria-invalid` mark from every field of `form`. */
function clearInvalid(form) {
  for (const field of form.querySelectorAll('[aria-invalid]')) field.removeAttribute('aria-invalid');
}

/** The forms that already clear their invalid marks themselves. */
const watchedForms = new WeakSet();

/** A field that is edited, a form that is reset or a new submit clears the marks of the last failure. */
function watchInvalid(form) {
  if (watchedForms.has(form)) return;
  watchedForms.add(form);
  const clearField = (event) => event.target.removeAttribute?.('aria-invalid');
  form.addEventListener('input', clearField);
  form.addEventListener('change', clearField);
  form.addEventListener('reset', () => clearInvalid(form));
}

/**
 * Shows `error` as safe text in the error box with the given id (a live region, `role="alert"`).
 * When the error code is about one field of the form around the box, that field is marked
 * `aria-invalid` and focused, until the user edits it; otherwise the box itself is focused.
 */
export function showError(id, error) {
  const box = document.getElementById(id);
  box.textContent = errorText(error);
  box.hidden = false;
  const form = box.closest('form');
  if (!form) return;
  watchInvalid(form);
  clearInvalid(form);
  const field = fieldForError(form, error?.code);
  if (field) {
    field.setAttribute('aria-invalid', 'true');
    field.focus();
  } else {
    box.tabIndex = -1;
    box.focus();
  }
}

/**
 * Shows or hides a conditional hint. A hint names the fields it describes in `data-describes`
 * (field names, space separated); while it is shown it is part of their `aria-describedby`.
 * A hidden hint must not be read out, although `aria-describedby` still reaches hidden text.
 */
export function setHint(id, visible) {
  const hint = document.getElementById(id);
  hint.hidden = !visible;
  const form = hint.closest('form');
  for (const name of (hint.dataset.describes ?? '').split(' ').filter(Boolean)) {
    const field = form?.elements[name];
    if (!field) continue;
    const ids = new Set((field.getAttribute('aria-describedby') ?? '').split(' ').filter(Boolean));
    if (visible) ids.add(id);
    else ids.delete(id);
    if (ids.size) field.setAttribute('aria-describedby', [...ids].join(' '));
    else field.removeAttribute('aria-describedby');
  }
}

/**
 * Runs `handler(event)` when `form` is submitted, one submit at a time. While the handler is
 * pending the submit buttons are disabled and a second submit (a double Enter or click) is
 * ignored, so one action never creates two records. Both are undone when the handler ends,
 * also when it throws.
 */
export function submitOnce(form, handler) {
  let busy = false;
  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    if (busy) return;
    busy = true;
    clearInvalid(form);
    const buttons = [...form.querySelectorAll('[type="submit"]')];
    buttons.forEach((button) => {
      button.disabled = true;
    });
    try {
      await handler(event);
    } finally {
      busy = false;
      buttons.forEach((button) => {
        button.disabled = false;
      });
    }
  });
}

/**
 * Reloads what a saved change touched. The change itself is saved and its dialog is closed by
 * then, so a failing reload goes to the toast, where the user sees it, and not into the closed
 * dialog's error box.
 */
export async function reloadAfterSave(reload) {
  try {
    await reload();
  } catch (error) {
    showToast(errorText(error));
  }
}
