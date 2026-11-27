// The About dialog of Windows (CSD-038 REQ-14). Windows only has a plain message box for About, which cannot
// show the app icon, so Rust sends the id of its own About item and this dialog answers it. The texts come
// from Rust (the same as the native panel of CSD-031) and stay in English, like the rest of that panel.
import { $, h } from './dom.js';
import { invoke } from './ipc.js';

/** Shows the About dialog with the icon, name, version and the texts Rust returns. */
export async function showAbout() {
  const info = await invoke('get_about_info');
  let dialog = $('about-dialog');
  if (!dialog) {
    dialog = h('dialog', { id: 'about-dialog', class: 'about-dialog', 'aria-labelledby': 'about-name' });
    document.body.append(dialog);
  }
  if (dialog.open) return;
  dialog.replaceChildren(
    h(
      'form',
      { method: 'dialog' },
      h('img', { class: 'about-icon', src: 'assets/app-icon.png', alt: '', width: '96', height: '96' }),
      h('h2', { id: 'about-name' }, info.name),
      h('p', { class: 'about-version' }, `Version ${info.version}`),
      h('p', {}, info.description),
      h('p', { class: 'about-meta' }, info.copyright),
      h('p', { class: 'about-meta' }, `Licence: ${info.license}`),
      h('p', { class: 'about-meta' }, info.website),
      h('div', { class: 'dialog-actions' }, h('button', { type: 'submit', class: 'btn primary' }, 'Close')),
    ),
  );
  dialog.showModal();
}
