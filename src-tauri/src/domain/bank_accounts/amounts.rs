//! The balance and note of an entry (CSD-009 REQ-10, REQ-11).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::common::Note;
use crate::domain::error::DomainError;
use crate::domain::money::{bounded, plain_decimal, Money};
use crate::domain::series;

/// The closing balance of an entry, in the project currency, signed, with
/// exactly two decimals (CSD-009 REQ-10). Negative when the account is
/// overdrawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BankBalance(Money);

impl BankBalance {
    /// Builds a balance from a decimal.
    ///
    /// # Errors
    /// Returns `BankEntryBalancePrecision` for more than two decimals and
    /// `BankEntryBalanceTooLarge` above 1,000,000,000.00 in absolute value.
    /// Zero and negative values are valid.
    pub fn new(value: Decimal) -> Result<Self, DomainError> {
        bounded(
            value,
            Money::MAX_BALANCE,
            DomainError::BankEntryBalancePrecision,
            DomainError::BankEntryBalanceTooLarge,
        )
        .map(Self)
    }

    /// Parses a balance typed by the user.
    ///
    /// # Errors
    /// Returns `BankEntryBalanceInvalid` for text that is not a plain decimal
    /// number, and otherwise the rules of [`BankBalance::new`].
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        plain_decimal(text, DomainError::BankEntryBalanceInvalid).and_then(Self::new)
    }

    /// The balance as a decimal.
    #[must_use]
    pub fn value(self) -> Decimal {
        self.0.value()
    }
}

/// Builds the note of an entry, at most 2,000 characters (CSD-009 REQ-11).
///
/// # Errors
/// Returns `BankEntryNoteTooLong`.
pub fn bank_note(text: &str) -> Result<Note, DomainError> {
    series::record_note(text, DomainError::BankEntryNoteTooLong)
}
