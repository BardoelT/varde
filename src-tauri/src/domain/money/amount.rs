//! `Money`: one money amount with exactly two decimals (CSD-000 CON-8).
//!
//! Every amount of the application is a `Money`, directly or wrapped in an
//! area type (`Balance`, `DebtBalance`, ...). The scale, the parsing of typed
//! text and the rounding of computed results live here and nowhere else.

use std::fmt;
use std::str::FromStr;

use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

/// Longest accepted amount text, a bound on untrusted input.
const MAX_TEXT_LEN: usize = 32;

/// The number of decimals every amount has.
const SCALE: u32 = 2;

/// Why a decimal or a text is not a valid amount. The area types map these to
/// their own error codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
pub enum MoneyError {
    /// The text is not a plain number such as `12500.00` or `-250`.
    #[error("not a plain decimal number")]
    Invalid,
    /// The value has more than two significant decimals.
    #[error("more than two decimals")]
    Precision,
    /// The value is too large to be held with two decimals at all.
    #[error("value too large")]
    TooLarge,
}

/// An amount in the project currency with exactly two decimals, never `-0.00`.
///
/// Serialised as a decimal string with two decimals, such as `"3000.50"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct Money(Decimal);

impl Money {
    /// `0.00`.
    pub const ZERO: Self = Self(Decimal::from_parts(0, 0, 0, false, SCALE));

    /// Largest paycheck amount (1,000,000.00), a guard against typos.
    pub const MAX_PAYCHECK: Self = Self(Decimal::from_parts(100_000_000, 0, 0, false, SCALE));

    /// Largest balance, value or expense (1,000,000,000.00), a guard against
    /// typos.
    pub const MAX_BALANCE: Self = Self(Decimal::from_parts(1_215_752_192, 23, 0, false, SCALE));

    /// Builds an amount from a decimal, which must not carry more than two
    /// significant decimals: `3000.500` is accepted as `3000.50`, `3000.505`
    /// is rejected. Nothing is rounded.
    ///
    /// # Errors
    /// `Precision` for more than two decimals, `TooLarge` for a value that
    /// cannot be held with two decimals.
    pub fn from_decimal(value: Decimal) -> Result<Self, MoneyError> {
        if value.normalize().scale() > SCALE {
            return Err(MoneyError::Precision);
        }
        Self::at_scale_two(value).ok_or(MoneyError::TooLarge)
    }

    /// Parses an amount typed by the user: `[-]digits[.digits]`, with no
    /// exponent, separators or `+` sign, and the precision rule of
    /// [`Money::from_decimal`].
    ///
    /// # Errors
    /// `Invalid` for text that is not a plain number, otherwise the errors of
    /// [`Money::from_decimal`].
    #[allow(
        dead_code,
        reason = "part of the Money API; the area types parse in two steps to keep their error order"
    )]
    pub fn parse(text: &str) -> Result<Self, MoneyError> {
        Self::from_decimal(Self::parse_decimal(text)?)
    }

    /// The plain-number rule of [`Money::parse`] without the precision rule,
    /// for area types that check their own rules (negative, zero) between the
    /// two.
    ///
    /// # Errors
    /// `Invalid` for text that is not a plain number.
    pub fn parse_decimal(text: &str) -> Result<Decimal, MoneyError> {
        let text = text.trim();
        if text.len() > MAX_TEXT_LEN {
            return Err(MoneyError::Invalid);
        }
        let unsigned = text.strip_prefix('-').unwrap_or(text);
        let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
        let digits_only = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
        let well_formed = !whole.is_empty()
            && digits_only(whole)
            && digits_only(fraction)
            && (fraction.is_empty() != unsigned.contains('.'));
        if !well_formed {
            return Err(MoneyError::Invalid);
        }
        Decimal::from_str(text).map_err(|_| MoneyError::Invalid)
    }

    /// Rounds a computed result to two decimals, half-up: the midpoint goes
    /// away from zero, so `0.125` is `0.13` and `-0.125` is `-0.13`. A result
    /// that rounds to zero is `0.00`, never `-0.00` (CSD-000 CON-8). `None`
    /// when the result cannot be held with two decimals.
    #[must_use]
    pub fn round_half_up(value: Decimal) -> Option<Self> {
        let rounded = value.round_dp_with_strategy(SCALE, RoundingStrategy::MidpointAwayFromZero);
        Self::at_scale_two(rounded)
    }

    /// The amount as a decimal, with exactly two decimals.
    #[must_use]
    pub fn value(self) -> Decimal {
        self.0
    }

    /// The sum, or `None` when it overflows.
    #[must_use]
    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).and_then(Self::at_scale_two)
    }

    /// The difference, or `None` when it overflows.
    #[must_use]
    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).and_then(Self::at_scale_two)
    }

    /// The sum of all amounts, `0.00` for none, or `None` on overflow.
    #[must_use]
    #[allow(dead_code, reason = "part of the Money API, covered by the unit tests")]
    pub fn checked_sum(amounts: impl IntoIterator<Item = Self>) -> Option<Self> {
        amounts.into_iter().try_fold(Self::ZERO, Self::checked_add)
    }

    /// The absolute value.
    #[must_use]
    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }

    /// Whether the amount is below zero.
    #[must_use]
    #[allow(dead_code, reason = "part of the Money API, covered by the unit tests")]
    pub fn is_negative(self) -> bool {
        self.0 < Decimal::ZERO
    }

    /// Whether the amount is zero.
    #[must_use]
    pub fn is_zero(self) -> bool {
        self.0.is_zero()
    }

    /// Gives `value` exactly two decimals, and a zero its sign removed.
    fn at_scale_two(mut value: Decimal) -> Option<Self> {
        if value.is_zero() {
            return Some(Self::ZERO);
        }
        value.rescale(SCALE);
        (value.scale() == SCALE).then_some(Self(value))
    }
}

impl std::ops::Neg for Money {
    type Output = Self;

    fn neg(self) -> Self {
        // Negating a non-zero amount keeps its scale, and zero stays `0.00`.
        if self.is_zero() {
            Self::ZERO
        } else {
            Self(-self.0)
        }
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'de> Deserialize<'de> for Money {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = <Decimal as Deserialize>::deserialize(deserializer)?;
        Self::from_decimal(value).map_err(serde::de::Error::custom)
    }
}
