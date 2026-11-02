#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]

use serde::Serialize;
use tauri::{Runtime, State, Window};

use crate::domain::{Language, Theme};
use crate::error::AppError;
use crate::storage::Preferences;

/// The application preferences as the frontend needs them (CSD-008).
#[derive(Serialize)]
pub struct PreferencesView {
    theme: Theme,
    language: Language,
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

/// The saved theme and language: System and English when there is none or it
/// cannot be read (CSD-008 REQ-2, REQ-8, CSD-022 REQ-2, REQ-8).
#[tauri::command]
pub fn get_preferences(preferences: State<'_, Preferences>) -> PreferencesView {
    PreferencesView {
        theme: preferences.load_theme(),
        language: preferences.load_language(),
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
