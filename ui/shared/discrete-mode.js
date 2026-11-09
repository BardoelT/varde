// Discrete mode (CSD-030): the preference that hides every amount. Display only: Rust keeps the
// preference and computes every figure; nothing here changes a value, only whether the text that
// shows it is written or replaced by a placeholder. Like the number format, a change lets the
// listeners redraw every page.

/** The text that stands for any hidden amount: one fixed width, so no digit count or sign leaks (CON-5). */
export const HIDDEN = '•••••';

let on = false;
const listeners = [];

/** Whether Discrete mode is on. */
export const isDiscrete = () => on;

/** Calls `listener(on)` after the mode has changed, to draw the pages again. */
export function onDiscreteModeChange(listener) {
  listeners.push(listener);
}

/** Switches the mode on or off, marks the page for the blur CSS and lets the listeners redraw. */
export async function setDiscreteMode(value) {
  on = value === true;
  document.documentElement.classList.toggle('discrete-mode', on);
  for (const listener of listeners) await listener(on);
}
