//! The wire names of enums that have one text per variant: serde (the
//! `rename` attributes, or the variant name) is the only place they are
//! written down.

use serde::de::value::{Error as ValueError, StrDeserializer};
use serde::de::{Deserialize, IntoDeserializer};
use serde::Serialize;

/// The wire name of a variant of a plain enum, as serde writes it.
pub(crate) fn wire_name<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(name)) => name,
        _ => String::new(),
    }
}

/// The variant a plain enum has for `text` (trimmed, compared exactly, so case
/// matters), `None` for any other text.
pub(crate) fn parse_wire_name<'de, T: Deserialize<'de>>(text: &'de str) -> Option<T> {
    let deserializer: StrDeserializer<'de, ValueError> = text.trim().into_deserializer();
    T::deserialize(deserializer).ok()
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    enum Example {
        First,
        #[serde(rename = "Second one")]
        Second,
    }

    #[test]
    fn a_name_is_written_and_read_by_serde() {
        assert_eq!(wire_name(&Example::First), "First");
        assert_eq!(wire_name(&Example::Second), "Second one");
        assert_eq!(parse_wire_name(" Second one "), Some(Example::Second));
    }

    #[test]
    fn any_other_text_is_no_variant() {
        assert_eq!(parse_wire_name::<Example>("first"), None);
        assert_eq!(parse_wire_name::<Example>(""), None);
        assert_eq!(parse_wire_name::<Example>("Second"), None);
    }
}
