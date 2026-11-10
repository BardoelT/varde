#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]

use serde::Serialize;
use tauri::{AppHandle, Runtime, State, Window};

use crate::domain::{Language, NumberFormat, Theme};
use crate::error::AppError;
use crate::menu;
use crate::storage::Preferences;

/// The application preferences as the frontend needs them (CSD-008).
#[derive(Serialize)]
pub struct PreferencesView {
    theme: Theme,
    language: Language,
    number_format: NumberFormat,
    discrete_mode: bool,
}

/// The language as `set_language` answers it (CSD-022).
#[derive(Serialize)]
pub struct LanguageView {
    language: Language,
}

/// The native window theme for `theme`: `None` leaves the window to follow
/// macOS (REQ-7).
#[must_use]
pub fn window_theme(theme: Theme) -> Option<tauri::Theme> {
    match theme {
        Theme::System => None,
        Theme::Light => Some(tauri::Theme::Light),
        Theme::Dark => Some(tauri::Theme::Dark),
    }
}

/// The saved theme, language, number format and discrete mode: System, English, 1,234.50 and off when there is none or it
/// cannot be read (CSD-008 REQ-2, REQ-8, CSD-022 REQ-2, REQ-8).
#[tauri::command]
pub fn get_preferences<R: Runtime>(
    app: AppHandle<R>,
    preferences: State<'_, Preferences>,
) -> PreferencesView {
    let discrete_mode = preferences.load_discrete_mode();
    // The first thing the frontend asks at start: the menu exists by now.
    menu::sync_discrete_mode(&app, discrete_mode);
    PreferencesView {
        theme: preferences.load_theme(),
        language: preferences.load_language(),
        number_format: preferences.load_number_format(),
        discrete_mode,
    }
}

/// Saves the theme and gives the native window the same theme. Saving comes
/// first, so a failed save changes nothing (REQ-10).
#[tauri::command]
pub fn set_theme<R: Runtime>(
    window: Window<R>,
    preferences: State<'_, Preferences>,
    theme: String,
) -> Result<PreferencesView, AppError> {
    let theme = Theme::parse(&theme)?;
    preferences.store_theme(theme)?;
    // The saved theme is what counts; a window that refuses the theme keeps
    // following macOS until the next start.
    let _ = window.set_theme(window_theme(theme));
    Ok(PreferencesView {
        theme,
        language: preferences.load_language(),
        number_format: preferences.load_number_format(),
        discrete_mode: preferences.load_discrete_mode(),
    })
}

/// Saves the language. Saving comes first, so a failed save changes nothing
/// (CSD-022 REQ-9).
#[tauri::command]
pub fn set_language(
    preferences: State<'_, Preferences>,
    language: String,
) -> Result<LanguageView, AppError> {
    let language = Language::parse(&language)?;
    preferences.store_language(language)?;
    Ok(LanguageView { language })
}

/// Saves the number format. Saving comes first, so a failed save changes
/// nothing (CSD-029 REQ-10).
#[tauri::command]
pub fn set_number_format(
    preferences: State<'_, Preferences>,
    number_format: String,
) -> Result<PreferencesView, AppError> {
    let number_format = NumberFormat::parse(&number_format)?;
    preferences.store_number_format(number_format)?;
    Ok(PreferencesView {
        theme: preferences.load_theme(),
        language: preferences.load_language(),
        number_format,
        discrete_mode: preferences.load_discrete_mode(),
    })
}

/// Saves Discrete mode and makes the menu item agree with the saved value.
/// Saving comes first, so a failed save changes nothing (CSD-030 REQ-14); the
/// check mark is set from what is saved either way, because the native item
/// flips itself when it is chosen (REQ-11).
#[tauri::command]
pub fn set_discrete_mode<R: Runtime>(
    app: AppHandle<R>,
    preferences: State<'_, Preferences>,
    discrete_mode: bool,
) -> Result<PreferencesView, AppError> {
    let stored = preferences.store_discrete_mode(discrete_mode);
    let saved = preferences.load_discrete_mode();
    menu::sync_discrete_mode(&app, saved);
    stored?;
    Ok(PreferencesView {
        theme: preferences.load_theme(),
        language: preferences.load_language(),
        number_format: preferences.load_number_format(),
        discrete_mode: saved,
    })
}
