//! The series core: what bank accounts, investments, debts and houses share
//! (and, in part, expenses). Each of those areas is a list of named accounts
//! with one record per account and period (an entry, a statement or a
//! valuation). The name is at most 100 characters, unique ignoring case and
//! listed alphabetically; the period is monthly or yearly and locked once the
//! account has a record; a record has a period of the account's kind, unique
//! per account.
//!
//! [`store`] holds the traits and the rules written once on them
//! ([`SeriesData`] with [`SeriesAccount`] and [`SeriesRecord`]). An area keeps
//! its own types, its own errors and its own figures, and supplies only what
//! differs through those traits. This file holds the small helpers the rules
//! and the areas use: the colour slot choice, the name check, the alphabetical
//! order, the shared account input and the text parsers of a period and note.

mod store;

pub(crate) use store::{SeriesAccount, SeriesData, SeriesError, SeriesRecord};

use std::fmt::Display;

use serde::Deserialize;

use super::common::{ColourSlot, Note, PayPeriod, Period, MAX_NAME_CHARS};
use super::error::DomainError;
use super::money::Money;

/// The lowest colour slot used by the fewest series.
pub(crate) fn next_colour_slot(used: impl IntoIterator<Item = ColourSlot>) -> ColourSlot {
    let used: Vec<ColourSlot> = used.into_iter().collect();
    ColourSlot::all()
        .min_by_key(|slot| {
            let count: usize = used.iter().map(|u| usize::from(u == slot)).sum();
            (count, *slot)
        })
        .unwrap_or_default()
}

/// Whether a series other than `except` already has `name`, ignoring case.
/// `existing` pairs each series' id with its name.
pub(crate) fn name_taken<'a, Id: Copy + PartialEq>(
    existing: impl IntoIterator<Item = (Id, &'a str)>,
    name: &str,
    except: Option<Id>,
) -> bool {
    let wanted = name.to_lowercase();
    existing
        .into_iter()
        .any(|(id, other)| except.is_none_or(|e| e != id) && other.to_lowercase() == wanted)
}

/// Alphabetical order ignoring case, with the id as a stable tie-break.
pub(crate) fn name_order(name: &str, id: &impl Display) -> (String, String) {
    (name.to_lowercase(), id.to_string())
}

/// Account fields as typed by the user, before validation. Bank accounts,
/// investment accounts and debts take exactly these two; a house adds its
/// purchase and sale.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct AccountInput {
    #[serde(default)]
    pub name: String,
    /// `None` means Monthly when creating and "unchanged" when editing.
    #[serde(default)]
    pub period: Option<String>,
}

/// A trimmed, non-empty name and the period the user chose, if any.
pub(crate) struct ValidIdentity {
    pub(crate) name: String,
    pub(crate) period: Option<PayPeriod>,
}

/// Checks the name (required, at most 100 characters) and the optional period
/// text of an account, with the errors of the area.
pub(crate) fn validate_identity<S: SeriesData>(
    name: &str,
    period: Option<&str>,
) -> Result<ValidIdentity, DomainError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(S::error(SeriesError::NameRequired));
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(S::error(SeriesError::NameTooLong));
    }
    let period = period
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(|text| PayPeriod::parse(text).ok_or_else(|| S::error(SeriesError::PeriodUnsupported)))
        .transpose()?;
    Ok(ValidIdentity {
        name: name.to_owned(),
        period,
    })
}

/// Parses `YYYY-MM` or `YYYY` typed by the user, giving `invalid` for any
/// other text.
pub(crate) fn parse_period(text: &str, invalid: DomainError) -> Result<Period, DomainError> {
    Period::parse(text).map_err(|_| invalid)
}

/// Builds the note of a record, giving `too_long` above 2,000 characters.
pub(crate) fn record_note(text: &str, too_long: DomainError) -> Result<Note, DomainError> {
    Note::new(text).map_err(|_| too_long)
}

/// The sum of the amounts, `None` when there are none.
pub(crate) fn total_of(amounts: impl IntoIterator<Item = Money>) -> Option<Money> {
    let mut amounts = amounts.into_iter();
    amounts
        .next()
        .map(|first| amounts.fold(first, Money::saturating_add))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_colour_slot_takes_the_lowest_slot_used_least() {
        let slots = |numbers: &[u8]| -> Vec<ColourSlot> {
            numbers
                .iter()
                .copied()
                .filter_map(ColourSlot::new)
                .collect()
        };
        let next = |numbers: &[u8]| next_colour_slot(slots(numbers)).get();
        assert_eq!(next(&[]), 0);
        assert_eq!(next(&[0, 1]), 2);
        assert_eq!(next(&[0, 1, 2, 3, 4, 5, 6, 7]), 0);
        assert_eq!(next(&[0, 1, 2, 3, 4, 5, 6, 7, 0, 2]), 1);
    }

    #[test]
    fn name_taken_ignores_case_and_the_excepted_series() {
        let existing = [(1, "Savings"), (2, "Joint")];
        assert!(name_taken(existing, "savings", None));
        assert!(!name_taken(existing, "savings", Some(1)));
        assert!(name_taken(existing, "SAVINGS", Some(2)));
        assert!(!name_taken(existing, "Other", None));
    }

    #[test]
    fn name_order_sorts_ignoring_case_then_by_id() {
        assert!(name_order("apple", &"b") < name_order("Banana", &"a"));
        assert!(name_order("same", &"a") < name_order("Same", &"b"));
    }
}
