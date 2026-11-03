//! Amounts and currency: [`Money`], the currency of a project, the amounts of a paycheck and the one place where percentages are computed.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::error::DomainError;

mod amount;
#[cfg(test)]
mod tests;

pub use amount::{Money, MoneyError};

/// Currencies a project can use (CSD-003).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Currency {
    #[default]
    Eur,
    Usd,
    Gbp,
}

impl Currency {
    /// Parses a currency code such as `"EUR"` (case-insensitive).
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_uppercase().as_str() {
            "EUR" => Some(Self::Eur),
            "USD" => Some(Self::Usd),
            "GBP" => Some(Self::Gbp),
            _ => None,
        }
    }
}

/// The three validated amounts of one paycheck, all with exactly two decimals.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Amounts {
    gross_salary: Money,
    holiday_pay: Money,
    bonus: Money,
}

impl Amounts {
    /// Builds amounts from decimals, applying the rules of CSD-001 REQ-7.
    ///
    /// # Errors
    /// Returns the first violated rule: negative, more than two decimals,
    /// above 1,000,000.00, or gross salary not greater than zero.
    pub fn new(
        gross_salary: Decimal,
        holiday_pay: Decimal,
        bonus: Decimal,
    ) -> Result<Self, DomainError> {
        let all = [gross_salary, holiday_pay, bonus];
        if all.iter().any(|amount| *amount < Decimal::ZERO) {
            return Err(DomainError::PaycheckAmountNegative);
        }
        let money = |value| {
            bounded(
                value,
                Money::MAX_PAYCHECK,
                DomainError::PaycheckAmountPrecision,
                DomainError::PaycheckAmountTooLarge,
            )
        };
        // The precision of all three comes before the size of any.
        if all
            .iter()
            .any(|amount| Money::from_decimal(*amount) == Err(MoneyError::Precision))
        {
            return Err(DomainError::PaycheckAmountPrecision);
        }
        let (gross_salary, holiday_pay, bonus) =
            (money(gross_salary)?, money(holiday_pay)?, money(bonus)?);
        if gross_salary.is_zero() {
            return Err(DomainError::PaycheckGrossInvalid);
        }
        Ok(Self {
            gross_salary,
            holiday_pay,
            bonus,
        })
    }

    /// Parses amounts typed by the user. Empty holiday pay or bonus mean `0.00`.
    ///
    /// # Errors
    /// An empty gross salary gives `PaycheckGrossInvalid`, text that is not a
    /// plain decimal number gives `PaycheckAmountInvalid`, and otherwise the
    /// rules of [`Amounts::new`] apply.
    pub fn parse(gross_salary: &str, holiday_pay: &str, bonus: &str) -> Result<Self, DomainError> {
        if gross_salary.trim().is_empty() {
            return Err(DomainError::PaycheckGrossInvalid);
        }
        Self::new(
            parse_decimal(gross_salary)?,
            parse_optional_decimal(holiday_pay)?,
            parse_optional_decimal(bonus)?,
        )
    }

    /// Gross salary of the month.
    #[must_use]
    pub fn gross_salary(&self) -> Decimal {
        self.gross_salary.value()
    }

    /// Holiday pay paid out in the month.
    #[must_use]
    pub fn holiday_pay(&self) -> Decimal {
        self.holiday_pay.value()
    }

    /// Bonus paid out in the month.
    #[must_use]
    pub fn bonus(&self) -> Decimal {
        self.bonus.value()
    }

    /// Gross salary + holiday pay + bonus (CSD-001 REQ-10).
    #[must_use]
    pub fn total_gross(&self) -> Decimal {
        self.gross_salary.value() + self.holiday_pay.value() + self.bonus.value()
    }
}

/// A zero amount with two decimals.
#[must_use]
pub fn zero() -> Decimal {
    Money::ZERO.value()
}

/// Checks a decimal against the precision rule and an absolute limit, giving
/// the area's own error codes. The sign rules of an area come before this.
pub(crate) fn bounded(
    value: Decimal,
    limit: Money,
    precision: DomainError,
    too_large: DomainError,
) -> Result<Money, DomainError> {
    let money = Money::from_decimal(value).map_err(|error| match error {
        MoneyError::TooLarge => too_large.clone(),
        MoneyError::Invalid | MoneyError::Precision => precision,
    })?;
    if money.abs() > limit {
        return Err(too_large);
    }
    Ok(money)
}

/// The plain-number rule of [`Money::parse_decimal`] as an area error code.
pub(crate) fn plain_decimal(text: &str, invalid: DomainError) -> Result<Decimal, DomainError> {
    Money::parse_decimal(text).map_err(|_| invalid)
}

fn parse_optional_decimal(text: &str) -> Result<Decimal, DomainError> {
    if text.trim().is_empty() {
        Ok(zero())
    } else {
        parse_decimal(text)
    }
}

fn parse_decimal(text: &str) -> Result<Decimal, DomainError> {
    plain_decimal(text, DomainError::PaycheckAmountInvalid)
}

/// `part` in percent of `whole`: the one place where a percentage or ratio is
/// computed. Rounded to two decimals, half-up, and a result that rounds to
/// zero is `0.00` without a sign (CSD-000 CON-8; CSD-006 REQ-5, CSD-007
/// REQ-6). `None` when `whole` is zero or the result overflows.
#[must_use]
pub fn percent_of(part: Decimal, whole: Decimal) -> Option<Decimal> {
    let percent = part.checked_mul(Decimal::ONE_HUNDRED)?.checked_div(whole)?;
    Money::round_half_up(percent).map(Money::value)
}
