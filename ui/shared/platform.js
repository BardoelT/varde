// The platform the app runs on, and the shortcut texts that follow it (CSD-038 REQ-6).
// Rust draws the native menu; this only decides how a shortcut is written in the app's own texts.

/** Whether the app runs on Windows, as its webview says. */
export const isWindows = () => /Windows/i.test(globalThis.navigator?.userAgent ?? '');

/**
 * A text written with macOS shortcuts (`⌘O`, `⇧⌘S`) in the notation of the platform: unchanged on
 * macOS, `Ctrl+O` and `Ctrl+Shift+S` on Windows.
 */
export function platformShortcuts(text) {
  if (!isWindows()) return text;
  return text.replace(/([⌘⇧]+)([A-Z0-9,.])/g, (_all, modifiers, key) => `${modifiers.includes('⌘') ? 'Ctrl+' : ''}${modifiers.includes('⇧') ? 'Shift+' : ''}${key}`);
}

// The menu items with a Ctrl shortcut. On Windows the webview has the focus and the native menu never
// sees the key, so the page turns the key into the same `menu-action` id (CSD-038 REQ-8).
const WINDOWS_SHORTCUTS = {
  n: 'project-new',
  o: 'project-open',
  s: 'project-save',
  'shift+s': 'project-save-as',
  ',': 'open-settings',
  'shift+d': 'toggle-discrete-mode',
};

/** The menu item id for a Ctrl shortcut pressed on Windows, or null. Undo and redo are not here: see pages/project.js. */
export function windowsShortcutId(event) {
  if (!isWindows() || !event.ctrlKey || event.altKey || event.metaKey) return null;
  const key = `${event.shiftKey ? 'shift+' : ''}${event.key.toLowerCase()}`;
  return WINDOWS_SHORTCUTS[key] ?? null;
}
