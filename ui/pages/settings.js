// Settings page: the project currency (CSD-003), the app theme (CSD-008) and the language (CSD-022).
import { invoke, errorText } from '../shared/ipc.js';
import { showToast, confirmAction } from '../shared/dom.js';
import { setLanguage, t } from '../shared/i18n.js';

const select = () => document.getElementById('setting-currency');
const themeSelect = () => document.getElementById('setting-theme');
const languageSelect = () => document.getElementById('setting-language');

let settings = { currency: 'EUR', entry_count: 0 };
let savedTheme = 'system';
let savedLanguage = 'en';

/** Shows a theme: `data-theme` for light and dark, none for system so macOS decides (REQ-4). */
function applyTheme(theme) {
  const root = document.documentElement;
  if (theme === 'light' || theme === 'dark') {
    root.dataset.theme = theme;
  } else {
    delete root.dataset.theme;
  }
  savedTheme = theme;
  themeSelect().value = theme;
}

/** Shows a language everywhere and in the choice (CSD-022 REQ-3). */
async function applyLanguage(language) {
  savedLanguage = language;
  languageSelect().value = language;
  await setLanguage(language);
}

/** Loads the saved theme and language from Rust and applies them. Called first at start (CSD-008 REQ-6, CSD-022 REQ-4). */
export async function loadPreferences() {
  const preferences = await invoke('get_preferences');
  applyTheme(preferences.theme);
  await applyLanguage(preferences.language);
}

/** Applies a theme only once Rust has saved it, so a failed save changes nothing (REQ-10). */
async function onThemeChosen() {
  const requested = themeSelect().value;
  if (requested === savedTheme) return;
  try {
    const preferences = await invoke('set_theme', { theme: requested });
    applyTheme(preferences.theme);
  } catch (error) {
    themeSelect().value = savedTheme;
    showToast(errorText(error));
  }
}

/** Applies a language only once Rust has saved it, so a failed save changes nothing (CSD-022 REQ-9). */
async function onLanguageChosen() {
  const requested = languageSelect().value;
  if (requested === savedLanguage) return;
  try {
    const preferences = await invoke('set_language', { language: requested });
    await applyLanguage(preferences.language);
  } catch (error) {
    languageSelect().value = savedLanguage;
    showToast(errorText(error));
  }
}

/** Loads the settings from Rust and shows them on the page. */
export async function reloadSettings() {
  settings = await invoke('get_settings');
  select().value = settings.currency;
}

async function onCurrencyChosen(onChanged) {
  const requested = select().value;
  if (requested === settings.currency) return;

  // With paychecks, a changed currency silently relabels every amount (REQ-5).
  if (settings.entry_count > 0) {
    const confirmed = await confirmAction({
      title: t('Change the currency to {currency}?', { currency: requested }),
      message: t('Existing amounts keep their values and only the currency changes. Amounts are not converted.'),
      confirmLabel: t('Change to {currency}', { currency: requested }),
    });
    if (!confirmed) {
      select().value = settings.currency;
      return;
    }
  }

  try {
    settings = await invoke('set_currency', { currency: requested });
  } catch (error) {
    select().value = settings.currency;
    showToast(errorText(error));
    return;
  }
  await onChanged();
}

export function initSettings({ onChanged }) {
  select().addEventListener('change', () => onCurrencyChosen(onChanged));
  themeSelect().addEventListener('change', onThemeChosen);
  languageSelect().addEventListener('change', onLanguageChosen);
}
