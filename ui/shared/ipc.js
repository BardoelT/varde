// The bridge to the Rust backend: command calls, events and error text.
// The frontend only displays what Rust returns; it computes no figures.

import { errorMessage } from './i18n.js';

const tauri = window.__TAURI__;

export function invoke(command, args) {
  return tauri.core.invoke(command, args);
}

export function listen(eventName, handler) {
  return tauri.event.listen(eventName, handler);
}

/** Safe, user-facing text of an error returned by a command, in the chosen language (CSD-022 REQ-7). */
export function errorText(error) {
  return errorMessage(error);
}
