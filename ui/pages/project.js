// Project file: runs the File menu actions and shows the unsaved-changes indicator (CSD-002).
import { invoke, listen, errorText } from '../shared/ipc.js';
import { confirmAction, showToast } from '../shared/dom.js';
import { t } from '../shared/i18n.js';

let status = { unsaved_changes: false, has_path: false };

function render() {
  const box = document.getElementById('project-state');
  const text = document.getElementById('project-state-text');
  box.classList.toggle('is-unsaved', status.unsaved_changes);
  if (status.unsaved_changes) {
    text.textContent = t('Unsaved changes');
  } else if (status.has_path) {
    text.textContent = t('All changes saved');
  } else {
    text.textContent = t('New project');
  }
}

/** Shows the status again, for example in another language. */
export const refreshTexts = render;

export async function refreshProjectStatus() {
  status = await invoke('project_status');
  render();
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

/** The flows behind the File menu items (CSD-002 REQ-9). */
const menuActions = (onReplaced) => ({
  'project-new': async () => {
    if (!(await confirmDiscard(t('You have unsaved changes. If you start a new project, they will be lost.')))) return;
    status = await invoke('new_project');
    render();
    await onReplaced();
  },
  'project-open': async () => {
    if (!(await confirmDiscard(t('You have unsaved changes. If you open another file, they will be lost.')))) return;
    const result = await invoke('open_project');
    status = result.status;
    render();
    if (result.done) await onReplaced();
  },
  'project-save': async () => {
    const result = await invoke('save_project');
    status = result.status;
    render();
  },
  'project-save-as': async () => {
    const result = await invoke('save_project_as');
    status = result.status;
    render();
  },
});

export async function initProject({ onReplaced }) {
  // Rust sends the id of the File menu item the user chose.
  const actions = menuActions(onReplaced);
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

  await refreshProjectStatus();

  // The last project could not be reopened at start (CSD-002 REQ-11).
  const notice = await invoke('take_startup_notice');
  if (notice) showToast(t('Your last project could not be reopened. {reason}', { reason: errorText(notice) }));
}
