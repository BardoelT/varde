// Project file: runs the File menu actions (CSD-002); the unsaved dot is in the window title, set by Rust (CSD-024),
// and runs Edit > Undo and Redo (CSD-027).
import { invoke, listen, errorText } from '../shared/ipc.js';
import { confirmAction, showToast } from '../shared/dom.js';
import { t } from '../shared/i18n.js';
import { isWindows } from '../shared/platform.js';

let status = { unsaved_changes: false, has_path: false, undo_label: null, redo_label: null };

export async function refreshProjectStatus() {
  status = await invoke('project_status');
}

async function attempt(action) {
  try {
    await action();
  } catch (error) {
    showToast(errorText(error));
  }
}

async function confirmDiscard(message) {
  await refreshProjectStatus();
  if (!status.unsaved_changes) return true;
  return confirmAction({
    title: t('Discard unsaved changes?'),
    message,
    confirmLabel: t('Discard changes'),
    danger: true,
  });
}

/** Whether the user is typing in a field, where ⌘Z and ⇧⌘Z undo the text (CSD-027 REQ-11). */
function isTextEntry(element) {
  return Boolean(element) && (element.tagName === 'TEXTAREA' || element.tagName === 'INPUT' || element.isContentEditable);
}

/**
 * Edit > Undo or Redo (CSD-027). In a text field the menu item only passes the key on to the text;
 * with a form or dialog open nothing happens; otherwise Rust takes the step and every page reloads.
 */
async function undoOrRedo(command, onReplaced) {
  if (isTextEntry(document.activeElement)) {
    document.execCommand?.(command);
    return;
  }
  if (document.querySelector('dialog[open]')) return;
  const result = await invoke(command);
  status = result.status;
  await onReplaced();
  showToast(t(command === 'undo' ? 'Undid: {step}' : 'Redid: {step}', { step: t(result.step) }), 'notice');
}

/** The flows behind the File menu items (CSD-002 REQ-9) and the Edit items Undo and Redo (CSD-027 REQ-10). */
const menuActions = (onReplaced, onSwitched) => ({
  'project-new': async () => {
    if (!(await confirmDiscard(t('You have unsaved changes. If you start a new project, they will be lost.')))) return;
    status = await invoke('new_project');
    // A new project starts without filters; undo and redo keep them (CSD-028 REQ-13, REQ-14).
    onSwitched();
    await onReplaced();
  },
  'project-open': async () => {
    if (!(await confirmDiscard(t('You have unsaved changes. If you open another file, they will be lost.')))) return;
    const result = await invoke('open_project');
    status = result.status;
    if (result.done) {
      onSwitched();
      await onReplaced();
    }
  },
  'project-save': async () => {
    const result = await invoke('save_project');
    status = result.status;
  },
  'project-save-as': async () => {
    const result = await invoke('save_project_as');
    status = result.status;
  },
  'project-undo': () => undoOrRedo('undo', onReplaced),
  'project-redo': () => undoOrRedo('redo', onReplaced),
});

export async function initProject({ onReplaced, onSwitched = () => {} }) {
  // Rust sends the id of the File menu item the user chose.
  const actions = menuActions(onReplaced, onSwitched);
  await listen('menu-action', (event) => {
    const action = actions[event.payload];
    if (action) attempt(action);
  });

  // Rust asks for a confirmation when the window or app is closed with unsaved changes.
  await listen('close-requested', () =>
    attempt(async () => {
      const quit = await confirmAction({
        title: t('Quit without saving?'),
        message: t('You have unsaved changes. They will be lost if you quit.'),
        confirmLabel: t('Quit without saving'),
        danger: true,
      });
      if (quit) await invoke('quit_app');
    }),
  );

  // Ctrl+Y redoes on Windows, next to the Ctrl+Shift+Z of the Redo item (CSD-038 REQ-9). Ctrl+Z and
  // Ctrl+Shift+Z need this too: the native menu does not see them (REQ-8). In a text field the browser
  // keeps them, and Ctrl+Y goes to the same redo.
  document.addEventListener('keydown', (event) => {
    if (!isWindows() || !event.ctrlKey || event.altKey || event.metaKey) return;
    const key = event.key.toLowerCase();
    const redo = (key === 'y' && !event.shiftKey) || (key === 'z' && event.shiftKey);
    const undo = key === 'z' && !event.shiftKey;
    if (!redo && !undo) return;
    const inText = isTextEntry(document.activeElement);
    if (key === 'z' && inText) return;
    event.preventDefault();
    // Outside a text field the item is disabled without a step, so there is nothing to do.
    if (inText || status[redo ? 'redo_label' : 'undo_label']) attempt(actions[redo ? 'project-redo' : 'project-undo']);
  });

  await refreshProjectStatus();

  // The last project could not be reopened at start (CSD-002 REQ-11).
  const notice = await invoke('take_startup_notice');
  if (notice) showToast(t('Your last project could not be reopened. {reason}', { reason: errorText(notice) }));
}
