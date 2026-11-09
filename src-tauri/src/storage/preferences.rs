use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::file::write_atomic_with;
use super::StorageError;
use crate::domain::{Language, NumberFormat, Theme};

const FILE_NAME: &str = "preferences.json";

/// The preference file is tiny; anything bigger is not ours.
const MAX_BYTES: u64 = 64 * 1024;

#[derive(Serialize, Default)]
struct Stored {
    theme: Theme,
    language: Language,
    number_format: NumberFormat,
    discrete_mode: bool,
}

impl Stored {
    /// Reads each preference on its own, so one that is missing or unusable
    /// does not reset the other (CSD-022 AC-8.3).
    fn from_json(text: &str) -> Self {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
            return Self::default();
        };
        let field = |name: &str| value.get(name).cloned();
        Self {
            theme: field("theme")
                .and_then(|theme| serde_json::from_value(theme).ok())
                .unwrap_or_default(),
            language: field("language")
                .and_then(|language| serde_json::from_value(language).ok())
                .unwrap_or_default(),
            number_format: field("number_format")
                .and_then(|format| serde_json::from_value(format).ok())
                .unwrap_or_default(),
            // Only a real boolean counts (CSD-030 REQ-13).
            discrete_mode: field("discrete_mode")
                .and_then(|mode| mode.as_bool())
                .unwrap_or_default(),
        }
    }
}

/// The application preferences (theme, CSD-008, and language, CSD-022), kept in
/// the app's own config folder next to the last used project (CSD-008 CON-1).
/// They hold preferences only, never project data.
#[derive(Debug, Clone)]
pub struct Preferences {
    file: Option<PathBuf>,
}

impl Preferences {
    /// The preference file inside `config_dir`.
    #[must_use]
    pub fn in_dir(config_dir: &Path) -> Self {
        Self {
            file: Some(config_dir.join(FILE_NAME)),
        }
    }

    /// Used when the app has no config folder: the defaults are used and
    /// nothing can be saved.
    #[must_use]
    pub fn unavailable() -> Self {
        Self { file: None }
    }

    /// The saved theme. A missing, unreadable, oversized or malformed file, or
    /// an unknown theme, counts as System (REQ-8).
    #[must_use]
    pub fn load_theme(&self) -> Theme {
        self.read().theme
    }

    /// The saved language. A missing, unreadable, oversized or malformed file,
    /// or an unknown language, counts as English (CSD-022 REQ-8).
    #[must_use]
    pub fn load_language(&self) -> Language {
        self.read().language
    }

    /// The saved number format. A missing, unreadable, oversized or malformed
    /// file, or an unknown format, counts as 1,234.50 (CSD-029 REQ-9).
    #[must_use]
    pub fn load_number_format(&self) -> NumberFormat {
        self.read().number_format
    }

    /// Whether Discrete mode is on. A missing, unreadable, oversized or
    /// malformed file, or a value that is not a boolean, counts as off
    /// (CSD-030 REQ-2, REQ-13).
    #[must_use]
    pub fn load_discrete_mode(&self) -> bool {
        self.read().discrete_mode
    }

    fn read(&self) -> Stored {
        self.read_text()
            .map_or_else(Stored::default, |text| Stored::from_json(&text))
    }

    fn read_text(&self) -> Option<String> {
        let file = fs::File::open(self.file.as_ref()?).ok()?;
        let metadata = file.metadata().ok()?;
        if !metadata.is_file() || metadata.len() > MAX_BYTES {
            return None;
        }
        let mut text = String::new();
        file.take(MAX_BYTES).read_to_string(&mut text).ok()?;
        Some(text)
    }

    /// Saves `theme`.
    ///
    /// # Errors
    /// Returns `PreferencesSaveFailed` if the preference file cannot be written.
    pub fn store_theme(&self, theme: Theme) -> Result<(), StorageError> {
        let stored = Stored {
            theme,
            ..self.read()
        };
        self.write(&stored)
            .map_err(StorageError::PreferencesSaveFailed)
    }

    /// Saves `language`, keeping the saved theme.
    ///
    /// # Errors
    /// Returns `PreferencesSaveFailed` if the preference file cannot be written.
    pub fn store_language(&self, language: Language) -> Result<(), StorageError> {
        let stored = Stored {
            language,
            ..self.read()
        };
        self.write(&stored)
            .map_err(StorageError::PreferencesSaveFailed)
    }

    /// Saves `number_format`, keeping the saved theme and language.
    ///
    /// # Errors
    /// Returns `PreferencesSaveFailed` if the preference file cannot be written.
    pub fn store_number_format(&self, number_format: NumberFormat) -> Result<(), StorageError> {
        let stored = Stored {
            number_format,
            ..self.read()
        };
        self.write(&stored)
            .map_err(StorageError::PreferencesSaveFailed)
    }

    /// Saves `discrete_mode`, keeping the other preferences.
    ///
    /// # Errors
    /// Returns `PreferencesSaveFailed` if the preference file cannot be written.
    pub fn store_discrete_mode(&self, discrete_mode: bool) -> Result<(), StorageError> {
        let stored = Stored {
            discrete_mode,
            ..self.read()
        };
        self.write(&stored)
            .map_err(StorageError::PreferencesSaveFailed)
    }

    fn write(&self, stored: &Stored) -> io::Result<()> {
        let path = self
            .file
            .as_ref()
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        let json = serde_json::to_string_pretty(stored).map_err(io::Error::other)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        write_atomic_with(path, |file| file.write_all(json.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_csd008_ac_2_1_without_a_file_the_theme_is_system() {
        let dir = TempDir::new().unwrap();
        assert_eq!(Preferences::in_dir(dir.path()).load_theme(), Theme::System);
        assert_eq!(Preferences::unavailable().load_theme(), Theme::System);
    }

    #[test]
    fn test_csd008_ac_5_1_the_theme_survives_a_new_session() {
        let dir = TempDir::new().unwrap();
        // The config folder does not exist yet: it is created on the first save.
        let config = dir.path().join("config");
        Preferences::in_dir(&config)
            .store_theme(Theme::Dark)
            .unwrap();
        assert_eq!(Preferences::in_dir(&config).load_theme(), Theme::Dark);
        Preferences::in_dir(&config)
            .store_theme(Theme::Light)
            .unwrap();
        assert_eq!(Preferences::in_dir(&config).load_theme(), Theme::Light);
    }

    #[test]
    fn test_csd008_ac_5_2_the_file_holds_preferences_only_and_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        Preferences::in_dir(dir.path())
            .store_theme(Theme::Dark)
            .unwrap();

        let file = dir.path().join(FILE_NAME);
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "theme": "dark", "language": "en", "number_format": "point_comma",
                "discrete_mode": false
            })
        );
        let mode = fs::metadata(&file).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "mode {mode:o}");
    }

    #[test]
    fn test_csd008_ac_5_3_the_project_file_holds_no_theme() {
        let json = super::super::file::to_json(&crate::domain::ProjectData::default()).unwrap();
        assert!(!json.contains("theme"), "{json}");
    }

    #[test]
    fn test_csd008_ac_8_1_a_missing_or_invalid_file_means_system() {
        let dir = TempDir::new().unwrap();
        let preferences = Preferences::in_dir(dir.path());
        let file = dir.path().join(FILE_NAME);
        let oversized = format!(r#"{{"theme": "dark"}}{}"#, " ".repeat(70 * 1024));
        for content in [
            "",
            r#""dark""#,
            "dark",
            "not json",
            r#"{"theme": "sepia"}"#,
            r#"{"theme": "Dark"}"#,
            r#"{"theme": 7}"#,
            r#"{"theme": null}"#,
            "{}",
            oversized.as_str(),
        ] {
            fs::write(&file, content).unwrap();
            assert_eq!(preferences.load_theme(), Theme::System, "{content:.40?}");
        }
    }

    #[test]
    fn test_csd008_ac_8_1_a_directory_or_unreadable_file_means_system() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let preferences = Preferences::in_dir(dir.path());
        let file = dir.path().join(FILE_NAME);

        fs::create_dir(&file).unwrap();
        assert_eq!(preferences.load_theme(), Theme::System);
        fs::remove_dir(&file).unwrap();

        fs::write(&file, r#"{"theme": "dark"}"#).unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o000)).unwrap();
        assert_eq!(preferences.load_theme(), Theme::System);
    }

    #[test]
    fn test_csd008_ac_8_1_unknown_fields_are_ignored() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(FILE_NAME),
            r#"{"theme": "light", "future": true}"#,
        )
        .unwrap();
        assert_eq!(Preferences::in_dir(dir.path()).load_theme(), Theme::Light);
    }

    #[test]
    fn test_csd008_ac_10_1_a_failed_save_reports_a_code_and_no_path() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let preferences = Preferences::in_dir(dir.path());
        preferences.store_theme(Theme::Light).unwrap();

        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o500)).unwrap();
        let error = preferences.store_theme(Theme::Dark).unwrap_err();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();

        assert_eq!(error.code(), "PREFERENCES_SAVE_FAILED");
        assert!(!error.to_string().contains(dir.path().to_str().unwrap()));
        assert_eq!(preferences.load_theme(), Theme::Light);
    }

    #[test]
    fn test_csd008_ac_10_1_without_a_config_folder_saving_fails() {
        let error = Preferences::unavailable()
            .store_theme(Theme::Dark)
            .unwrap_err();
        assert_eq!(error.code(), "PREFERENCES_SAVE_FAILED");
    }

    #[test]
    fn test_csd022_ac_2_1_without_a_file_the_language_is_english() {
        let dir = TempDir::new().unwrap();
        assert_eq!(
            Preferences::in_dir(dir.path()).load_language(),
            Language::English
        );
        assert_eq!(
            Preferences::unavailable().load_language(),
            Language::English
        );
    }

    #[test]
    fn test_csd022_ac_4_4_language_and_theme_are_kept_together() {
        let dir = TempDir::new().unwrap();
        Preferences::in_dir(dir.path())
            .store_theme(Theme::Dark)
            .unwrap();
        Preferences::in_dir(dir.path())
            .store_language(Language::Dutch)
            .unwrap();
        let restarted = Preferences::in_dir(dir.path());
        assert_eq!(restarted.load_theme(), Theme::Dark);
        assert_eq!(restarted.load_language(), Language::Dutch);

        restarted.store_theme(Theme::Light).unwrap();
        assert_eq!(restarted.load_language(), Language::Dutch);
    }

    #[test]
    fn test_csd022_ac_8_1_a_language_that_cannot_be_used_means_english() {
        let dir = TempDir::new().unwrap();
        let preferences = Preferences::in_dir(dir.path());
        let file = dir.path().join(FILE_NAME);
        for content in [
            r#"{"language": ""}"#,
            r#"{"language": "fr"}"#,
            r#"{"language": "NL"}"#,
            r#"{"language": 7}"#,
            r#"{"language": null}"#,
            "not json",
            "{}",
        ] {
            fs::write(&file, content).unwrap();
            assert_eq!(preferences.load_language(), Language::English, "{content}");
        }
    }

    #[test]
    fn test_csd022_ac_8_3_an_older_file_without_a_language_keeps_the_theme() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(FILE_NAME), r#"{"theme": "dark"}"#).unwrap();
        let preferences = Preferences::in_dir(dir.path());
        assert_eq!(preferences.load_language(), Language::English);
        assert_eq!(preferences.load_theme(), Theme::Dark);
    }

    #[test]
    fn test_csd022_ac_8_3_a_bad_theme_does_not_reset_the_language() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(FILE_NAME),
            r#"{"theme": "sepia", "language": "nl"}"#,
        )
        .unwrap();
        let preferences = Preferences::in_dir(dir.path());
        assert_eq!(preferences.load_theme(), Theme::System);
        assert_eq!(preferences.load_language(), Language::Dutch);
    }

    #[test]
    fn test_csd022_ac_9_1_a_failed_language_save_keeps_the_language_and_shows_no_path() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let preferences = Preferences::in_dir(dir.path());
        preferences.store_language(Language::English).unwrap();

        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o500)).unwrap();
        let error = preferences.store_language(Language::Dutch).unwrap_err();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();

        assert_eq!(error.code(), "PREFERENCES_SAVE_FAILED");
        assert!(!error.to_string().contains(dir.path().to_str().unwrap()));
        assert_eq!(preferences.load_language(), Language::English);
    }

    #[test]
    fn test_csd022_ac_4_3_the_project_file_holds_no_language() {
        let json = super::super::file::to_json(&crate::domain::ProjectData::default()).unwrap();
        assert!(!json.contains("language"), "{json}");
    }

    #[test]
    fn test_csd029_ac_2_1_without_a_file_the_number_format_is_point_comma() {
        let dir = TempDir::new().unwrap();
        assert_eq!(
            Preferences::in_dir(dir.path()).load_number_format(),
            NumberFormat::PointComma
        );
        assert_eq!(
            Preferences::unavailable().load_number_format(),
            NumberFormat::PointComma
        );
    }

    #[test]
    fn test_csd029_ac_4_4_theme_language_and_number_format_are_kept_together() {
        let dir = TempDir::new().unwrap();
        let preferences = Preferences::in_dir(dir.path());
        preferences.store_theme(Theme::Dark).unwrap();
        preferences.store_language(Language::Dutch).unwrap();
        preferences
            .store_number_format(NumberFormat::ApostropheComma)
            .unwrap();
        let restarted = Preferences::in_dir(dir.path());
        assert_eq!(restarted.load_theme(), Theme::Dark);
        assert_eq!(restarted.load_language(), Language::Dutch);
        assert_eq!(
            restarted.load_number_format(),
            NumberFormat::ApostropheComma
        );

        restarted.store_theme(Theme::Light).unwrap();
        restarted.store_language(Language::English).unwrap();
        assert_eq!(
            restarted.load_number_format(),
            NumberFormat::ApostropheComma
        );
    }

    #[test]
    fn test_csd029_ac_9_1_a_number_format_that_cannot_be_used_means_point_comma() {
        let dir = TempDir::new().unwrap();
        let preferences = Preferences::in_dir(dir.path());
        let file = dir.path().join(FILE_NAME);
        for content in [
            r#"{"number_format": ""}"#,
            r#"{"number_format": "comma"}"#,
            r#"{"number_format": "COMMA_POINT"}"#,
            r#"{"number_format": 7}"#,
            r#"{"number_format": null}"#,
            "not json",
            "{}",
        ] {
            fs::write(&file, content).unwrap();
            assert_eq!(
                preferences.load_number_format(),
                NumberFormat::PointComma,
                "{content}"
            );
        }
    }

    #[test]
    fn test_csd029_ac_9_3_an_older_file_keeps_theme_and_language() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(FILE_NAME),
            r#"{"theme": "dark", "language": "nl"}"#,
        )
        .unwrap();
        let preferences = Preferences::in_dir(dir.path());
        assert_eq!(preferences.load_number_format(), NumberFormat::PointComma);
        assert_eq!(preferences.load_theme(), Theme::Dark);
        assert_eq!(preferences.load_language(), Language::Dutch);
    }

    #[test]
    fn test_csd029_ac_10_1_a_failed_save_keeps_the_format_and_shows_no_path() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let preferences = Preferences::in_dir(dir.path());
        preferences
            .store_number_format(NumberFormat::PointComma)
            .unwrap();

        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o500)).unwrap();
        let error = preferences
            .store_number_format(NumberFormat::CommaPoint)
            .unwrap_err();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();

        assert_eq!(error.code(), "PREFERENCES_SAVE_FAILED");
        assert!(!error.to_string().contains(dir.path().to_str().unwrap()));
        assert_eq!(preferences.load_number_format(), NumberFormat::PointComma);
    }

    #[test]
    fn test_csd029_ac_4_3_the_project_file_holds_no_number_format() {
        let json = super::super::file::to_json(&crate::domain::ProjectData::default()).unwrap();
        assert!(!json.contains("number_format"), "{json}");
    }

    #[test]
    fn test_csd030_ac_2_1_without_a_file_discrete_mode_is_off() {
        let dir = TempDir::new().unwrap();
        assert!(!Preferences::in_dir(dir.path()).load_discrete_mode());
        assert!(!Preferences::unavailable().load_discrete_mode());
    }

    #[test]
    fn test_csd030_ac_4_1_the_mode_survives_a_new_session() {
        let dir = TempDir::new().unwrap();
        Preferences::in_dir(dir.path())
            .store_discrete_mode(true)
            .unwrap();
        assert!(Preferences::in_dir(dir.path()).load_discrete_mode());
        Preferences::in_dir(dir.path())
            .store_discrete_mode(false)
            .unwrap();
        assert!(!Preferences::in_dir(dir.path()).load_discrete_mode());
    }

    #[test]
    fn test_csd030_ac_4_4_the_preferences_are_kept_together() {
        let dir = TempDir::new().unwrap();
        let preferences = Preferences::in_dir(dir.path());
        preferences.store_theme(Theme::Dark).unwrap();
        preferences.store_language(Language::Dutch).unwrap();
        preferences
            .store_number_format(NumberFormat::CommaPoint)
            .unwrap();
        preferences.store_discrete_mode(true).unwrap();
        let restarted = Preferences::in_dir(dir.path());
        assert_eq!(restarted.load_theme(), Theme::Dark);
        assert_eq!(restarted.load_language(), Language::Dutch);
        assert_eq!(restarted.load_number_format(), NumberFormat::CommaPoint);
        assert!(restarted.load_discrete_mode());

        restarted.store_theme(Theme::Light).unwrap();
        assert!(restarted.load_discrete_mode());
    }

    #[test]
    fn test_csd030_ac_13_1_a_value_that_cannot_be_used_means_off() {
        let dir = TempDir::new().unwrap();
        let preferences = Preferences::in_dir(dir.path());
        let file = dir.path().join(FILE_NAME);
        for content in [
            r#"{"discrete_mode": "yes"}"#,
            r#"{"discrete_mode": 1}"#,
            r#"{"discrete_mode": null}"#,
            r#"{"discrete_mode": "true"}"#,
            "not json",
            "{}",
        ] {
            fs::write(&file, content).unwrap();
            assert!(!preferences.load_discrete_mode(), "{content}");
        }
    }

    #[test]
    fn test_csd030_ac_13_1_a_bad_value_does_not_reset_the_other_preferences() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(FILE_NAME),
            r#"{"theme": "dark", "discrete_mode": "yes"}"#,
        )
        .unwrap();
        assert_eq!(Preferences::in_dir(dir.path()).load_theme(), Theme::Dark);
    }

    #[test]
    fn test_csd030_ac_13_2_an_older_file_means_off_and_keeps_the_rest() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join(FILE_NAME),
            r#"{ "theme": "dark", "language": "nl", "number_format": "comma_point" }"#,
        )
        .unwrap();
        let preferences = Preferences::in_dir(dir.path());
        assert!(!preferences.load_discrete_mode());
        assert_eq!(preferences.load_theme(), Theme::Dark);
        assert_eq!(preferences.load_language(), Language::Dutch);
        assert_eq!(preferences.load_number_format(), NumberFormat::CommaPoint);
    }

    #[test]
    fn test_csd030_ac_14_1_a_failed_save_keeps_the_mode_and_shows_no_path() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let preferences = Preferences::in_dir(dir.path());
        preferences.store_discrete_mode(false).unwrap();

        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o500)).unwrap();
        let error = preferences.store_discrete_mode(true).unwrap_err();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();

        assert_eq!(error.code(), "PREFERENCES_SAVE_FAILED");
        assert!(!error.to_string().contains(dir.path().to_str().unwrap()));
        assert!(!preferences.load_discrete_mode());
    }

    #[test]
    fn test_csd030_ac_4_3_the_project_file_holds_no_discrete_mode() {
        let json = super::super::file::to_json(&crate::domain::ProjectData::default()).unwrap();
        assert!(!json.contains("discrete"), "{json}");
    }
}
