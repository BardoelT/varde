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
const MAX_TEXT_LEN: usize = 64;

/// The number of decimals every amount has.
const SCALE: u32 = 2;

/// [`SCALE`] as a digit count.
const FRACTION_DIGITS: usize = 2;

/// Adds one to the last digit of a string of ASCII digits, carrying, growing
/// the string by a leading `1` when every digit was a nine.
fn increment(digits: &mut Vec<u8>) {
    for digit in digits.iter_mut().rev() {
        if *digit == b'9' {
            *digit = b'0';
        } else {
            *digit += 1;
            return;
        }
    }
    digits.insert(0, b'1');
}

/// Why a decimal or a text is not a valid amount. The area types map these to
/// their own error codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
pub enum MoneyError {
    /// The text is not a plain number such as `12500.00` or `-250`.
    #[error("not a plain decimal number")]
    Invalid,
    /// A decimal read from a project file has more than two significant
    /// decimals. Typed text never gives this: it is rounded (CSD-000 CON-8).
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

    /// The largest amount a `Money` can hold. Every amount of the application
    /// is far below it ([`Money::MAX_BALANCE`]); it only gives the sums of
    /// [`Money::saturating_add`] and [`Money::saturating_sum`] an end.
    pub const LARGEST: Self = Self(Decimal::from_parts(
        u32::MAX,
        u32::MAX,
        u32::MAX,
        false,
        SCALE,
    ));

    /// Builds an amount from a decimal read from a project file, which must
    /// not carry more than two significant decimals: `3000.500` is accepted as
    /// `3000.50`, `3000.505` is rejected. Nothing is rounded: only typed text
    /// is rounded ([`Money::parse`]), the file stays strict (CSD-002).
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

    /// Reads the text of an amount from a project file: a plain number with
    /// at most two significant decimals. Unlike [`Money::parse`] nothing is
    /// rounded, and a fraction that a `Decimal` could not hold exactly is
    /// refused too.
    fn from_file_text(text: &str) -> Result<Self, MoneyError> {
        let unsigned = text.strip_prefix('-').unwrap_or(text);
        let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
        let digits_only = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
        let well_formed = text.len() <= MAX_TEXT_LEN
            && !whole.is_empty()
            && digits_only(whole)
            && digits_only(fraction)
            && (fraction.is_empty() != unsigned.contains('.'));
        if !well_formed {
            return Err(MoneyError::Invalid);
        }
        if fraction.trim_end_matches('0').len() > FRACTION_DIGITS {
            return Err(MoneyError::Precision);
        }
        let value = Decimal::from_str(text).map_err(|_| MoneyError::TooLarge)?;
        Self::from_decimal(value)
    }

    /// Parses an amount typed by the user: `[-]digits[.digits]`, with no
    /// exponent, separators or `+` sign, rounded to two decimals half-up
    /// ([`Money::parse_decimal`]).
    ///
    /// # Errors
    /// `Invalid` for text that is not a plain number, `TooLarge` for a value
    /// that cannot be held.
    #[allow(
        dead_code,
        reason = "part of the Money API; the area types parse in two steps to keep their error order"
    )]
    pub fn parse(text: &str) -> Result<Self, MoneyError> {
        Self::from_decimal(Self::parse_decimal(text)?)
    }

    /// The plain-number rule of [`Money::parse`] for area types that check
    /// their own rules (negative, zero, too large) between parsing and
    /// building. The text is rounded half-up to two decimals on its digits: the
    /// third fraction digit decides, 5 or more rounds the magnitude up, however
    /// long the rest is, and a result of zero has no sign (CSD-000 CON-8).
    ///
    /// # Errors
    /// `Invalid` for text that is not a plain number, `TooLarge` for a value a
    /// `Decimal` cannot hold.
    pub fn parse_decimal(text: &str) -> Result<Decimal, MoneyError> {
        let text = text.trim();
        if text.len() > MAX_TEXT_LEN {
            return Err(MoneyError::Invalid);
        }
        let negative = text.starts_with('-');
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
        let mut fraction_bytes = fraction.bytes().chain(std::iter::repeat(b'0'));
        let mut digits: Vec<u8> = whole.bytes().collect();
        digits.extend(fraction_bytes.by_ref().take(FRACTION_DIGITS));
        let round_up = fraction_bytes.next().is_some_and(|digit| digit >= b'5');
        if round_up {
            increment(&mut digits);
        }
        let is_zero = digits.iter().all(|digit| *digit == b'0');
        let (whole_digits, fraction_digits) =
            digits.split_at(digits.len().saturating_sub(FRACTION_DIGITS));
        let mut rounded = String::with_capacity(digits.len() + 2);
        if negative && !is_zero {
            rounded.push('-');
        }
        rounded.extend(whole_digits.iter().map(|digit| char::from(*digit)));
        rounded.push('.');
        rounded.extend(fraction_digits.iter().map(|digit| char::from(*digit)));
        Decimal::from_str(&rounded).map_err(|_| MoneyError::TooLarge)
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
    ///
    /// For what is not a sum of amounts: percentages and the positions of a
    /// chart. Amounts stay `Money`.
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

    /// The sum, held at the largest or smallest amount instead of overflowing.
    ///
    /// Every amount is within [`Money::MAX_BALANCE`], so the sum of the amounts
    /// of any real project is about 10^17 times below [`Money::LARGEST`]: the
    /// bound exists to keep the totals total functions, never to be reached.
    #[must_use]
    pub fn saturating_add(self, other: Self) -> Self {
        self.checked_add(other).unwrap_or(if other.is_negative() {
            -Self::LARGEST
        } else {
            Self::LARGEST
        })
    }

    /// The difference, held at the largest or smallest amount (see
    /// [`Money::saturating_add`]).
    #[must_use]
    pub fn saturating_sub(self, other: Self) -> Self {
        self.saturating_add(-other)
    }

    /// The sum of all amounts, `0.00` for none, held like
    /// [`Money::saturating_add`].
    #[must_use]
    pub fn saturating_sum(amounts: impl IntoIterator<Item = Self>) -> Self {
        amounts.into_iter().fold(Self::ZERO, Self::saturating_add)
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

impl Default for Money {
    /// `0.00`.
    fn default() -> Self {
        Self::ZERO
    }
}

impl From<Money> for Decimal {
    /// The amount as a decimal, for the constructors that check a decimal
    /// against the rules of an area.
    fn from(money: Money) -> Self {
        money.0
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
    /// Reads an amount of a project file, strictly: a plain decimal string
    /// with at most two significant decimals, never rounded (CSD-002 REQ-19).
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct StrictAmount;

        impl serde::de::Visitor<'_> for StrictAmount {
            type Value = Money;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a decimal string with at most two decimals")
            }

            fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<Money, E> {
                Money::from_file_text(text).map_err(E::custom)
            }

            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Money, E> {
                Money::from_decimal(Decimal::from(value)).map_err(E::custom)
            }

            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Money, E> {
                Money::from_decimal(Decimal::from(value)).map_err(E::custom)
            }
        }

        deserializer.deserialize_any(StrictAmount)
    }
}
