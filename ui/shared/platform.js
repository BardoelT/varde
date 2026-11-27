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
