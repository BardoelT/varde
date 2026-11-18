// Updates section of the Settings page (CSD-023): check for a newer version, install it. Rust does
// all of it (check, download, signature, install, restart); this page only shows what Rust answers.
import { invoke, listen, errorText } from '../shared/ipc.js';
import { confirmAction } from '../shared/dom.js';
import { onLanguageChange, t } from '../shared/i18n.js';

const el = (id) => document.getElementById(id);

let current = '';
/** The newer version a check found and an install may take, or null. */
let available = null;
/** What the status line shows: nothing, a running check or install, a result or an error. */
let message = { kind: 'none' };
/** Download progress `{ downloaded, total }` while an install runs. */
let progress = null;
let busy = false;
/** The page was left while a run was active: the result is dropped when the run ends (REQ-10). */
let clearWhenIdle = false;

/** The percentage of a download for the bar, or null when the size is unknown (REQ-6). */
function percent(value) {
  if (!value || !value.total) return null;
  return Math.min(100, Math.floor((value.downloaded * 100) / value.total));
}

function statusText() {
  switch (message.kind) {
    case 'checking':
      return t('Checking for updates…');
    case 'installing': {
      const shown = percent(progress);
      const text = t('Downloading and installing the update…');
      return shown === null ? text : `${text} ${shown}%`;
    }
    case 'up_to_date':
      return t('Up to date ({version})', { version: message.current });
    case 'available':
      return t('Version {version} is available', { version: message.version });
    case 'error': {
      // A rejection can be a plain string or null (a missing command): then there is no code to show.
      const code = message.error?.code;
      const text = errorText(message.error);
      return code ? `${text} ${t('Code: {code}', { code })}` : text;
    }
    case 'installed':
      return t('Version {version} is installed. Restart Varde to use it.', { version: message.version });
    default:
      return '';
  }
}

function render() {
  el('update-current').textContent = current;
  el('update-status').textContent = statusText();

  const bar = el('update-progress');
  bar.hidden = message.kind !== 'installing';
  const shown = percent(progress);
  if (shown === null) bar.removeAttribute('value'); // no size known: a bar without a percentage
  else bar.value = shown;

  const check = el('update-check');
  const install = el('update-install');
  check.disabled = busy;
  install.disabled = busy;
  install.hidden = available === null;
}

/** Forgets the result of a check and the progress (REQ-10). */
function reset() {
  available = null;
  message = { kind: 'none' };
  progress = null;
  render();
}

/** Marks a run as started or ended: the buttons follow `busy` (REQ-9). */
function setBusy(value) {
  busy = value;
  if (!busy && clearWhenIdle) {
    clearWhenIdle = false;
    reset();
    return;
  }
  render();
}

/** The answer of a check or of an install that found nothing newer. */
function showAnswer(answer) {
  if (answer.status === 'installed') {
    available = null;
    message = { kind: 'installed', version: answer.version };
  } else if (answer.status === 'available') {
    available = answer.version;
    message = { kind: 'available', version: answer.version };
  } else {
    available = null;
    message = { kind: 'up_to_date', current: answer.current };
  }
}

async function onCheck() {
  if (busy) return;
  available = null;
  message = { kind: 'checking' };
  setBusy(true);
  try {
    showAnswer(await invoke('check_for_update'));
  } catch (error) {
    available = null;
    message = { kind: 'error', error };
  } finally {
    setBusy(false);
  }
}

async function onInstall() {
  if (busy) return;
  setBusy(true);
  try {
    const status = await invoke('project_status');
    const unsavedConfirmed = status.unsaved_changes;
    if (unsavedConfirmed) {
      const confirmed = await confirmAction({
        title: t('Install the update without saving?'),
        message: t('You have unsaved changes. They will be lost when the app restarts to install the update.'),
        confirmLabel: t('Install and restart'),
        danger: true,
      });
      if (!confirmed) return; // cancelled: nothing changes (REQ-7)
    }
    progress = null;
    message = { kind: 'installing' };
    render();
    // On success Rust restarts the app and never answers. It answers when nothing was installed, or when
    // the update is installed but the restart waits because changes appeared after the question (REQ-14).
    showAnswer(await invoke('install_update', { unsavedConfirmed }));
  } catch (error) {
    message = { kind: 'error', error }; // the same version stays available for a retry (REQ-8)
  } finally {
    progress = null;
    setBusy(false);
  }
}

export async function initUpdates() {
  el('update-check').addEventListener('click', onCheck);
  el('update-install').addEventListener('click', onInstall);

  await listen('update-progress', ({ payload }) => {
    if (message.kind !== 'installing') return;
    progress = payload;
    render();
  });

  // Leaving the Settings page drops the result, so an old check is never acted on (REQ-10).
  const settings = el('settings');
  let wasShown = settings.classList.contains('active');
  new MutationObserver(() => {
    const shown = settings.classList.contains('active');
    if (wasShown && !shown) {
      if (busy) clearWhenIdle = true;
      else reset();
    }
    wasShown = shown;
  }).observe(settings, { attributes: true, attributeFilter: ['class'] });

  onLanguageChange(render);

  try {
    current = await invoke('app_version');
  } catch {
    current = '';
  }
  render();
}
