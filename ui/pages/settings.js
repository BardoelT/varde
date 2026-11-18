// Settings page: the project currency (CSD-003), the app theme (CSD-008) and the language (CSD-022) the number format (CSD-029), Discrete mode (CSD-030) and Autosave (CSD-002).
import { invoke, errorText } from '../shared/ipc.js';
import { showToast, confirmAction } from '../shared/dom.js';
import { setLanguage, t } from '../shared/i18n.js';
import { setNumberFormat } from '../shared/number-format.js';
import { setDiscreteMode } from '../shared/discrete-mode.js';

const select = () => document.getElementById('setting-currency');
const themeSelect = () => document.getElementById('setting-theme');
const languageSelect = () => document.getElementById('setting-language');
const numberFormatSelect = () => document.getElementById('setting-number-format');
const discreteSwitch = () => document.getElementById('setting-discrete-mode');
const discreteBadge = () => document.getElementById('discrete-badge');
const autosaveSwitch = () => document.getElementById('setting-autosave');

let settings = { currency: 'EUR', entry_count: 0 };
let savedTheme = 'system';
let savedLanguage = 'en';
let savedNumberFormat = 'point_comma';
let savedDiscreteMode = false;
let savedAutosave = true;

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

/** Shows a number format everywhere and in the choice (CSD-029 REQ-3). */
async function applyNumberFormat(numberFormat) {
  savedNumberFormat = numberFormat;
  numberFormatSelect().value = numberFormat;
  await setNumberFormat(numberFormat);
}

/** Shows Discrete mode everywhere, in the switch and in the badge (CSD-030 REQ-3, REQ-12). */
async function applyDiscreteMode(on) {
  savedDiscreteMode = on === true;
  discreteSwitch().checked = savedDiscreteMode;
  discreteBadge().hidden = !savedDiscreteMode;
  await setDiscreteMode(savedDiscreteMode);
}

/** Shows the Autosave choice in the switch (CSD-002 REQ-24). */
function applyAutosave(on) {
  savedAutosave = on !== false;
  autosaveSwitch().checked = savedAutosave;
}

/** Loads the saved theme and language from Rust and applies them. Called first at start (CSD-008 REQ-6, CSD-022 REQ-4). */
export async function loadPreferences() {
  const preferences = await invoke('get_preferences');
  applyTheme(preferences.theme);
  await applyLanguage(preferences.language);
  await applyNumberFormat(preferences.number_format);
  await applyDiscreteMode(preferences.discrete_mode);
  applyAutosave(preferences.autosave);
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

/** Applies a number format only once Rust has saved it, so a failed save changes nothing (CSD-029 REQ-10). */
async function onNumberFormatChosen() {
  const requested = numberFormatSelect().value;
  if (requested === savedNumberFormat) return;
  try {
    const preferences = await invoke('set_number_format', { numberFormat: requested });
    await applyNumberFormat(preferences.number_format);
  } catch (error) {
    numberFormatSelect().value = savedNumberFormat;
    showToast(errorText(error));
  }
}

/** Applies Discrete mode only once Rust has saved it, so a failed save changes nothing (CSD-030 REQ-14). */
async function onDiscreteModeChosen() {
  const requested = discreteSwitch().checked;
  if (requested === savedDiscreteMode) return;
  try {
    const preferences = await invoke('set_discrete_mode', { discreteMode: requested });
    await applyDiscreteMode(preferences.discrete_mode);
  } catch (error) {
    discreteSwitch().checked = savedDiscreteMode;
    showToast(errorText(error));
  }
}

/**
 * Applies Autosave only once Rust has saved it, so a failed save changes nothing (CSD-002 REQ-24).
 * Switching it on may write the project, so the menu and the window follow (REQ-25).
 */
async function onAutosaveChosen(onWritten) {
  const requested = autosaveSwitch().checked;
  if (requested === savedAutosave) return;
  try {
    const preferences = await invoke('set_autosave', { autosave: requested });
    applyAutosave(preferences.autosave);
    await onWritten();
  } catch (error) {
    autosaveSwitch().checked = savedAutosave;
    showToast(errorText(error));
  }
}

/** The menu item and ⌘⇧D flip the mode like the switch does (CSD-030 REQ-11). */
export async function toggleDiscreteMode() {
  discreteSwitch().checked = !savedDiscreteMode;
  await onDiscreteModeChosen();
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

export function initSettings({ onChanged, onWritten = async () => {} }) {
  select().addEventListener('change', () => onCurrencyChosen(onChanged));
  themeSelect().addEventListener('change', onThemeChosen);
  languageSelect().addEventListener('change', onLanguageChosen);
  numberFormatSelect().addEventListener('change', onNumberFormatChosen);
  discreteSwitch().addEventListener('change', onDiscreteModeChosen);
  autosaveSwitch().addEventListener('change', () => onAutosaveChosen(onWritten));
}
