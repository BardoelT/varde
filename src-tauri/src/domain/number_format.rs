//! How numbers are written and typed (CSD-029).

use serde::{Deserialize, Serialize};

use super::error::DomainError;

/// How the user writes numbers: the grouping and the decimal separator. The
/// project file and IPC always use the canonical form (no grouping, a point
/// as decimal separator), whatever the format is (CSD-029 CON-4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NumberFormat {
    /// 1,234.50
    #[default]
    PointComma,
    /// 1.234,50
    CommaPoint,
    /// 1'234,50
    ApostropheComma,
}

impl NumberFormat {
    /// Parses `"point_comma"`, `"comma_point"` or `"apostrophe_comma"`
    /// (REQ-9). Anything else, including other capitalisation, is rejected.
    ///
    /// # Errors
    /// Returns `PreferencesNumberFormatUnsupported` for any other text.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        match text {
            "point_comma" => Ok(Self::PointComma),
            "comma_point" => Ok(Self::CommaPoint),
            "apostrophe_comma" => Ok(Self::ApostropheComma),
            _ => Err(DomainError::PreferencesNumberFormatUnsupported),
        }
    }

    /// The character that starts the decimals.
    fn decimal(self) -> char {
        match self {
            Self::PointComma => '.',
            Self::CommaPoint | Self::ApostropheComma => ',',
        }
    }

    /// Whether `character` may sit between groups of three digits.
    fn is_grouping(self, character: char) -> bool {
        match self {
            Self::PointComma => character == ',',
            Self::CommaPoint => character == '.',
            Self::ApostropheComma => character == '\'' || character == '\u{2019}',
        }
    }

    /// Reads an amount typed in this format and gives its canonical form
    /// (`[-]digits[.digits]`), or `None` when the text does not fit the format
    /// (REQ-6, REQ-7). Only the writing is checked and nothing is rounded, so
    /// the area rules and the rounding of CSD-000 CON-8 still apply to the
    /// result. Empty text stays empty, so a missing amount is reported as
    /// missing and not as invalid.
    #[must_use]
    pub fn typed_to_canonical(self, text: &str) -> Option<String> {
        let text = text.trim();
        if text.is_empty() {
            return Some(String::new());
        }
        let (sign, body) = text
            .strip_prefix('-')
            .map_or(("", text), |rest| ("-", rest));
        let mut parts = body.split(self.decimal());
        let whole = parts.next()?;
        let fraction = parts.next();
        if parts.next().is_some() {
            return None;
        }
        let digits = self.ungroup(whole)?;
        match fraction {
            None => Some(format!("{sign}{digits}")),
            Some(fraction) if is_digits(fraction) => Some(format!("{sign}{digits}.{fraction}")),
            Some(_) => None,
        }
    }

    /// The digits of a whole part that is plain or grouped in threes.
    fn ungroup(self, whole: &str) -> Option<String> {
        let mut groups = whole.split(|character| self.is_grouping(character));
        let first = groups.next()?;
        if !is_digits(first) {
            return None;
        }
        let mut digits = first.to_owned();
        let grouped = whole.chars().any(|character| self.is_grouping(character));
        if grouped && first.len() > 3 {
            return None;
        }
        for group in groups {
            if group.len() != 3 || !is_digits(group) {
                return None;
            }
            digits.push_str(group);
        }
        Some(digits)
    }
}

fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csd029_ac_2_1_the_default_format_is_point_comma() {
        assert_eq!(NumberFormat::default(), NumberFormat::PointComma);
    }

    #[test]
    fn test_csd029_ac_4_1_formats_round_trip_as_names() {
        for (format, name) in [
            (NumberFormat::PointComma, "point_comma"),
            (NumberFormat::CommaPoint, "comma_point"),
            (NumberFormat::ApostropheComma, "apostrophe_comma"),
        ] {
            assert_eq!(NumberFormat::parse(name), Ok(format));
            assert_eq!(serde_json::to_value(format).unwrap(), name);
            assert_eq!(
                serde_json::from_value::<NumberFormat>(name.into()).unwrap(),
                format
            );
        }
    }

    #[test]
    fn test_csd029_ac_9_2_unsupported_formats_are_rejected() {
        for text in ["comma", "COMMA_POINT", " point_comma", "point_comma ", ""] {
            let error = NumberFormat::parse(text).unwrap_err();
            assert_eq!(
                error.code(),
                "PREFERENCES_NUMBER_FORMAT_UNSUPPORTED",
                "{text:?}"
            );
        }
    }

    #[test]
    fn test_csd029_ac_6_1_typed_amounts_are_read_in_the_chosen_format() {
        use NumberFormat::{ApostropheComma, CommaPoint, PointComma};
        for (format, typed, canonical) in [
            (PointComma, "1234.50", "1234.50"),
            (PointComma, "1,234.50", "1234.50"),
            (PointComma, "1,234,567", "1234567"),
            (CommaPoint, "1234,50", "1234.50"),
            (CommaPoint, "1.234,50", "1234.50"),
            (CommaPoint, "1.234", "1234"),
            (CommaPoint, "-1.234,5", "-1234.5"),
            (CommaPoint, "  12,5 ", "12.5"),
            (ApostropheComma, "1'234,50", "1234.50"),
            (ApostropheComma, "1\u{2019}234,50", "1234.50"),
            (CommaPoint, "3000,505", "3000.505"),
            (CommaPoint, "", ""),
        ] {
            assert_eq!(
                format.typed_to_canonical(typed).as_deref(),
                Some(canonical),
                "{format:?} {typed:?}"
            );
        }
    }

    #[test]
    fn test_csd029_ac_7_1_amounts_that_do_not_fit_the_format_are_rejected() {
        use NumberFormat::{ApostropheComma, CommaPoint, PointComma};
        for (format, typed) in [
            (CommaPoint, "1234.50"),
            (CommaPoint, "12.34,50"),
            (CommaPoint, "1,234.50"),
            (CommaPoint, "1,234,50"),
            (CommaPoint, "1234,5.0"),
            (CommaPoint, "10,9x"),
            (CommaPoint, "1234,"),
            (CommaPoint, ",50"),
            (CommaPoint, "- 5"),
            (CommaPoint, "--5"),
            (PointComma, "1.234,50"),
            (PointComma, "12,34.50"),
            (ApostropheComma, "1.234,50"),
            (ApostropheComma, "1''234,50"),
            (ApostropheComma, "'234,50"),
        ] {
            assert_eq!(
                format.typed_to_canonical(typed),
                None,
                "{format:?} {typed:?}"
            );
        }
    }

    #[test]
    fn test_csd029_ac_6_1_the_canonical_form_is_read_by_the_default_format() {
        for text in ["0.00", "-150.25", "1000000.00", "12", "3000.999"] {
            assert_eq!(
                NumberFormat::PointComma.typed_to_canonical(text).as_deref(),
                Some(text)
            );
        }
    }
}
