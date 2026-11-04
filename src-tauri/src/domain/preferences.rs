//! The look and the language of the app (CSD-008, CSD-022).

use serde::{Deserialize, Serialize};

use super::error::DomainError;

/// The look of the app (CSD-008). System follows the macOS appearance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Follow the system setting.
    #[default]
    System,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
}

impl Theme {
    /// Parses the lowercase name `"system"`, `"light"` or `"dark"` (REQ-9).
    /// Anything else, including other capitalisation, is rejected.
    ///
    /// # Errors
    /// Returns `PreferencesThemeUnsupported` for any other text.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        match text {
            "system" => Ok(Self::System),
            "light" => Ok(Self::Light),
            "dark" => Ok(Self::Dark),
            _ => Err(DomainError::PreferencesThemeUnsupported),
        }
    }
}

/// The language of the app's own texts (CSD-022). English is the default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    /// English.
    #[default]
    #[serde(rename = "en")]
    English,
    /// Dutch.
    #[serde(rename = "nl")]
    Dutch,
}

impl Language {
    /// Parses the code `"en"` or `"nl"` (REQ-8). Anything else, including other
    /// capitalisation, is rejected.
    ///
    /// # Errors
    /// Returns `PreferencesLanguageUnsupported` for any other text.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        match text {
            "en" => Ok(Self::English),
            "nl" => Ok(Self::Dutch),
            _ => Err(DomainError::PreferencesLanguageUnsupported),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csd008_ac_2_1_the_default_theme_is_system() {
        assert_eq!(Theme::default(), Theme::System);
    }

    #[test]
    fn test_csd008_ac_5_1_themes_round_trip_as_lowercase_names() {
        for (theme, name) in [
            (Theme::System, "system"),
            (Theme::Light, "light"),
            (Theme::Dark, "dark"),
        ] {
            assert_eq!(Theme::parse(name), Ok(theme));
            assert_eq!(serde_json::to_value(theme).unwrap(), name);
            assert_eq!(serde_json::from_value::<Theme>(name.into()).unwrap(), theme);
        }
    }

    #[test]
    fn test_csd008_ac_9_1_unsupported_themes_are_rejected() {
        for text in ["sepia", "Dark", "DARK", " dark", "dark ", ""] {
            let error = Theme::parse(text).unwrap_err();
            assert_eq!(error.code(), "PREFERENCES_THEME_UNSUPPORTED", "{text:?}");
        }
    }

    #[test]
    fn test_csd022_ac_2_1_the_default_language_is_english() {
        assert_eq!(Language::default(), Language::English);
    }

    #[test]
    fn test_csd022_ac_4_1_languages_round_trip_as_codes() {
        for (language, code) in [(Language::English, "en"), (Language::Dutch, "nl")] {
            assert_eq!(Language::parse(code), Ok(language));
            assert_eq!(serde_json::to_value(language).unwrap(), code);
            assert_eq!(
                serde_json::from_value::<Language>(code.into()).unwrap(),
                language
            );
        }
    }

    #[test]
    fn test_csd022_ac_8_2_unsupported_languages_are_rejected() {
        for text in ["fr", "NL", "En", " nl", "nl ", "dutch", ""] {
            let error = Language::parse(text).unwrap_err();
            assert_eq!(error.code(), "PREFERENCES_LANGUAGE_UNSUPPORTED", "{text:?}");
        }
    }
}
