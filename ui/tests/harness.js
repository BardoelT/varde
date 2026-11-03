// DOM test harness for the vanilla-JS frontend. It builds the page of `ui/index.html` in jsdom,
// puts a fake Tauri bridge in front of it and starts the real `main.js` against it.
//
// jsdom does not run `<script type=module>`, and `shared/ipc.js` reads `window.__TAURI__` when it
// is imported, so the globals and the bridge exist BEFORE the page modules are imported. Every app
// gets a module registry of its own (hooks.js), so one test never sees the state of another.
// No network, no timers to wait for: `settle()` waits until the bridge has no call in flight.
import './hooks.js';
import { readFileSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { JSDOM } from 'jsdom';
import { exampleReads } from './fixtures.js';

const uiDir = fileURLToPath(new URL('..', import.meta.url));
const indexHtml = readFileSync(`${uiDir}index.html`, 'utf8');
/** Names jsdom provides that the pages use as globals. */
const GLOBALS = [
  'window', 'document', 'Node', 'NodeFilter', 'HTMLElement', 'Event', 'KeyboardEvent', 'MouseEvent',
  'InputEvent', 'CustomEvent', 'getComputedStyle', 'requestAnimationFrame', 'cancelAnimationFrame',
];
// The pages use Node's timers (the toast hides itself after 6 s). They must not keep a finished
// test process alive, so the timers a test starts are unref'ed.
const nodeSetTimeout = globalThis.setTimeout;
globalThis.setTimeout = (...args) => {
  const timer = nodeSetTimeout(...args);
  timer.unref?.();
  return timer;
};
let appCount = 0;

/** A command failure as Rust sends it: a stable code and a safe English message. */
export function commandError(code, message) {
  return { code, message };
}

/**
 * The fake `window.__TAURI__`. `table` maps a command to a canned answer, or to a function of its
 * arguments that returns the answer (or throws a `commandError`). A command that is not in the
 * table fails the test loudly instead of returning something invented.
 */
function createBridge(table) {
  const calls = [];
  const listeners = new Map();
  let pending = 0;
  return {
    calls,
    get pending() {
      return pending;
    },
    table,
    core: {
      async invoke(command, args) {
        calls.push({ command, args });
        pending += 1;
        try {
          if (!(command in table)) throw commandError('TEST_UNSTUBBED', `No canned answer for command ${command}`);
          const answer = table[command];
          const value = typeof answer === 'function' ? await answer(args) : answer;
          return structuredClone(value);
        } finally {
          pending -= 1;
        }
      },
    },
    event: {
      async listen(name, handler) {
        if (!listeners.has(name)) listeners.set(name, []);
        listeners.get(name).push(handler);
        return () => {};
      },
    },
    /** Delivers an event from "Rust", such as `menu-action` with the id of the chosen menu item. */
    emit(name, payload) {
      for (const handler of listeners.get(name) ?? []) handler({ event: name, payload });
    },
  };
}

/** Starts the app. Resolves when the first loading has settled. Call `app.close()` after the test. */
export async function openApp({ reads = exampleReads(), handlers = {} } = {}) {
  appCount += 1;
  const id = appCount;
  const dom = new JSDOM(indexHtml, { url: 'http://localhost/', pretendToBeVisual: true });
  const { window } = dom;
  const bridge = createBridge({ ...reads, ...handlers });
  window.__TAURI__ = bridge;
  addDialogSupport(window);

  const previous = Object.fromEntries(GLOBALS.map((name) => [name, Object.getOwnPropertyDescriptor(globalThis, name)]));
  for (const name of GLOBALS) {
    const value = name === 'window' ? window : window[name];
    Object.defineProperty(globalThis, name, {
      configurable: true,
      writable: true,
      value: typeof value === 'function' && !/^[A-Z]/.test(name) ? value.bind(window) : value,
    });
  }

  const document = window.document;
  const settle = async () => {
    let quiet = 0;
    while (quiet < 3) {
      await new Promise((resolve) => setImmediate(resolve));
      quiet = bridge.pending === 0 ? quiet + 1 : 0;
    }
  };
  const errors = [];
  window.addEventListener('error', (event) => errors.push(event.error ?? event.message));

  // The scripts of index.html in their own order: navigation.js (plain script) and main.js (module).
  await import(`${pathToFileURL(`${uiDir}navigation.js`)}?app=${id}`);
  const main = import(`${pathToFileURL(`${uiDir}main.js`)}?app=${id}`);
  await main;
  await settle();

  const $ = (id) => document.getElementById(id);
  const app = {
    window,
    document,
    bridge,
    calls: bridge.calls,
    errors,
    $,
    settle,
    /** The `section` elements that are shown: only the one with class `active`. */
    sections: () => [...document.querySelectorAll('.section')],
    /** The nav item of a section id, e.g. `item('debts')`. */
    item: (section) => document.querySelector(`.nav-item[data-section="${section}"]`),
    /** Is the element shown? No `hidden` attribute on it or its parents, and an active section. */
    isShown(element) {
      for (let node = element; node; node = node.parentElement) {
        if (node.hidden) return false;
        if (node.classList?.contains('section') && !node.classList.contains('active')) return false;
        if (node.tagName === 'DIALOG' && !node.open) return false;
      }
      return true;
    },
    /** The text of an element with its white space collapsed. */
    text: (element) => element.textContent.replace(/\s+/g, ' ').trim(),
    textOf: (id) => app.text($(id)),
    /** Clicks, then lets the loading it starts settle. */
    async click(element) {
      element.click();
      await settle();
    },
    /** Pressing Enter or Space on a focused button: the browser turns it into a click. */
    async press(element, key) {
      element.focus();
      const down = new window.KeyboardEvent('keydown', { key, bubbles: true, cancelable: true });
      element.dispatchEvent(down);
      if (!down.defaultPrevented && element.tagName === 'BUTTON' && (key === 'Enter' || key === ' ')) element.click();
      await settle();
    },
    /** Types text character by character, as the browser fires `input` events for each. */
    async type(input, text) {
      input.focus();
      for (const char of text) {
        input.value += char;
        input.setSelectionRange(input.value.length, input.value.length);
        input.dispatchEvent(new window.InputEvent('input', { inputType: 'insertText', data: char, bubbles: true }));
      }
      await settle();
    },
    /** Deletes the last character, as backspace does. */
    async backspace(input) {
      input.value = input.value.slice(0, -1);
      input.setSelectionRange(input.value.length, input.value.length);
      input.dispatchEvent(new window.InputEvent('input', { inputType: 'deleteContentBackward', bubbles: true }));
      await settle();
    },
    /** Chooses an option of a `select`. */
    async choose(select, value) {
      select.value = value;
      select.dispatchEvent(new window.Event('change', { bubbles: true }));
      await settle();
    },
    /** Escape on an open modal dialog: the browser fires `cancel`, and closes it unless prevented. */
    async escape(dialog) {
      const cancel = new window.Event('cancel', { cancelable: true });
      dialog.dispatchEvent(cancel);
      if (!cancel.defaultPrevented && dialog.open) dialog.close();
      await settle();
    },
    /** How often a command was called, optionally with a matching argument check. */
    callsOf: (command) => bridge.calls.filter((call) => call.command === command),
    emit: async (name, payload) => {
      bridge.emit(name, payload);
      await settle();
    },
    /** Imports a module of `ui/` into this app's own registry, e.g. `await app.load('shared/i18n.js')`. */
    load: (path) => import(`${pathToFileURL(`${uiDir}${path}`)}?app=${id}`),
    /** Frees the page and puts the globals back. */
    close() {
      window.close();
      for (const name of GLOBALS) {
        if (previous[name]) Object.defineProperty(globalThis, name, previous[name]);
        else delete globalThis[name];
      }
    },
  };
  return app;
}

/**
 * jsdom has no `<dialog>` behaviour yet: `showModal`, `show` and `close` are added here. `open`
 * is the attribute, `close` fires the `close` event, like a browser.
 */
function addDialogSupport(window) {
  const proto = window.HTMLDialogElement.prototype;
  if (typeof proto.showModal === 'function') return;
  proto.show = function show() {
    this.setAttribute('open', '');
  };
  proto.showModal = function showModal() {
    if (this.open) throw new window.DOMException('The dialog is already open.', 'InvalidStateError');
    this.setAttribute('open', '');
  };
  proto.close = function close(returnValue) {
    if (!this.open) return;
    this.removeAttribute('open');
    if (returnValue !== undefined) this.returnValue = returnValue;
    this.dispatchEvent(new window.Event('close'));
  };
}
