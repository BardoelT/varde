//! The value and note of a valuation (CSD-015 REQ-10, REQ-11).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::error::DomainError;
use crate::domain::investments::Note;
use crate::domain::money::{bounded, plain_decimal, Money};

/// What the house is worth at the end of a period, in the project currency, from
/// 0.00 up, with exactly two decimals (CSD-015 REQ-10).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HouseValue(Money);

impl HouseValue {
    /// Builds a value from a decimal.
    ///
    /// # Errors
    /// Returns the first violated rule: negative, more than two decimals, or
    /// above 1,000,000,000.00. Zero is valid.
    pub fn new(value: Decimal) -> Result<Self, DomainError> {
        if value < Decimal::ZERO {
            return Err(DomainError::HouseValuationValueNegative);
        }
        bounded(
            value,
            Money::MAX_BALANCE,
            DomainError::HouseValuationValuePrecision,
            DomainError::HouseValuationValueTooLarge,
        )
        .map(Self)
    }

    /// Parses a value typed by the user.
    ///
    /// # Errors
    /// Returns `HouseValuationValueInvalid` for text that is not a plain
    /// decimal number, and otherwise the rules of [`HouseValue::new`].
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        plain_decimal(text, DomainError::HouseValuationValueInvalid).and_then(Self::new)
    }

    /// Parses a purchase or sell amount typed by the user. The rules are those
    /// of a value, with the error codes of CSD-020 REQ-4.
    ///
    /// # Errors
    /// Returns `HouseAmountInvalid`, `HouseAmountNegative`,
    /// `HouseAmountPrecision` or `HouseAmountTooLarge`.
    pub fn parse_amount(text: &str) -> Result<Self, DomainError> {
        Self::parse(text).map_err(|error| match error {
            DomainError::HouseValuationValueNegative => DomainError::HouseAmountNegative,
            DomainError::HouseValuationValuePrecision => DomainError::HouseAmountPrecision,
            DomainError::HouseValuationValueTooLarge => DomainError::HouseAmountTooLarge,
            _ => DomainError::HouseAmountInvalid,
        })
    }

    /// The value as a decimal.
    #[must_use]
    pub fn value(self) -> Decimal {
        self.0.value()
    }
}

/// Builds the note of a valuation, at most 2,000 characters (CSD-015 REQ-11).
///
/// # Errors
/// Returns `HouseValuationNoteTooLong`.
pub fn house_note(text: &str) -> Result<Note, DomainError> {
    Note::new(text).map_err(|_| DomainError::HouseValuationNoteTooLong)
}
