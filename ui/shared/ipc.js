// The bridge to the Rust backend: command calls, events and error text.
// The frontend only displays what Rust returns; it computes no figures.

import { errorMessage } from './i18n.js';

const tauri = window.__TAURI__;

export function invoke(command, args) {
  return tauri.core.invoke(command, args);
}

// The handlers of this page by event name, so the page can also deliver an event to itself.
const localListeners = new Map();

export function listen(eventName, handler) {
  if (!localListeners.has(eventName)) localListeners.set(eventName, []);
  localListeners.get(eventName).push(handler);
  return tauri.event.listen(eventName, handler);
}

/** Delivers an event to this page's own listeners, as if Rust had sent it. Nothing leaves the page. */
export function emitLocal(eventName, payload) {
  for (const handler of localListeners.get(eventName) ?? []) handler({ event: eventName, payload });
}

/** Safe, user-facing text of an error returned by a command, in the chosen language (CSD-022 REQ-7). */
export function errorText(error) {
  return errorMessage(error);
}
