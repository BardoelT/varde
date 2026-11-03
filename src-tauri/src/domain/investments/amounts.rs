//! The amounts and text of a statement: balance, net deposits and note (CSD-005 REQ-9, REQ-10, REQ-22).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::error::DomainError;
use crate::domain::money::{bounded, plain_decimal, Money};

/// The closing balance of a statement, in the project currency, with exactly
/// two decimals (CSD-005 REQ-9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Balance(Money);

impl Balance {
    /// Builds a balance from a decimal.
    ///
    /// # Errors
    /// Returns the first violated rule: negative, more than two decimals, or
    /// above 1,000,000,000.00. Zero is valid.
    pub fn new(value: Decimal) -> Result<Self, DomainError> {
        if value < Decimal::ZERO {
            return Err(DomainError::InvestmentBalanceNegative);
        }
        bounded(
            value,
            Money::MAX_BALANCE,
            DomainError::InvestmentBalancePrecision,
            DomainError::InvestmentBalanceTooLarge,
        )
        .map(Self)
    }

    /// Parses a balance typed by the user.
    ///
    /// # Errors
    /// Returns `InvestmentBalanceInvalid` for text that is not a plain decimal
    /// number, and otherwise the rules of [`Balance::new`].
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        plain_decimal(text, DomainError::InvestmentBalanceInvalid).and_then(Self::new)
    }

    /// The balance as a decimal.
    #[must_use]
    pub fn value(self) -> Decimal {
        self.0.value()
    }
}

/// Checks a signed amount: at most two decimals and an absolute value up to
/// 1,000,000,000.00 (CSD-005 REQ-22, REQ-25).
fn signed_amount(
    value: Decimal,
    precision: DomainError,
    too_large: DomainError,
) -> Result<Money, DomainError> {
    bounded(value, Money::MAX_BALANCE, precision, too_large)
}

/// The net deposits of a statement: what the user put in minus what was taken
/// out during the period, signed, with exactly two decimals (CSD-005 REQ-22).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NetDeposits(Money);

impl Default for NetDeposits {
    fn default() -> Self {
        Self(Money::ZERO)
    }
}

impl NetDeposits {
    /// Builds net deposits from a decimal.
    ///
    /// # Errors
    /// Returns `InvestmentNetDepositsPrecision` for more than two decimals and
    /// `InvestmentNetDepositsTooLarge` above 1,000,000,000.00 in absolute value.
    pub fn new(value: Decimal) -> Result<Self, DomainError> {
        signed_amount(
            value,
            DomainError::InvestmentNetDepositsPrecision,
            DomainError::InvestmentNetDepositsTooLarge,
        )
        .map(Self)
    }

    /// Parses net deposits typed by the user. Empty text means `0.00`.
    ///
    /// # Errors
    /// Returns `InvestmentNetDepositsInvalid` for text that is not a plain
    /// decimal number, and otherwise the rules of [`NetDeposits::new`].
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        if text.trim().is_empty() {
            return Ok(Self::default());
        }
        plain_decimal(text, DomainError::InvestmentNetDepositsInvalid).and_then(Self::new)
    }

    /// The net deposits as a decimal.
    #[must_use]
    pub fn value(self) -> Decimal {
        self.0.value()
    }
}

/// Parses a capital growth typed by the user (CSD-005 REQ-25).
pub(super) fn parse_growth(text: &str) -> Result<Money, DomainError> {
    let value = plain_decimal(text, DomainError::InvestmentGrowthInvalid)?;
    signed_amount(
        value,
        DomainError::InvestmentGrowthPrecision,
        DomainError::InvestmentGrowthTooLarge,
    )
}
