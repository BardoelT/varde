//! Houses: their id, name, valuation period, colour slot (CSD-015 REQ-1 to
//! REQ-6) and purchase and sale (CSD-020 REQ-1 to REQ-5).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::common::PayPeriod;
use crate::domain::common::Period;
use crate::domain::common::MAX_NAME_CHARS;
use crate::domain::common::{Id, IdKind};
use crate::domain::error::DomainError;

use super::{HouseDate, HouseValue};

/// Identifies a House.
pub type HouseId = Id<House>;

impl IdKind for House {
    fn not_found() -> DomainError {
        DomainError::HouseNotFound
    }
}

/// One thing the user owns that has a value (CSD-015 REQ-1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct House {
    pub id: HouseId,
    pub name: String,
    pub period: PayPeriod,
    /// 0 to 7, fixed when the house is created (CSD-015 REQ-3).
    pub colour_slot: u8,
    /// Every house has a purchase (CSD-020 REQ-1).
    pub purchase_date: HouseDate,
    pub purchase_amount: HouseValue,
    /// The sale: both fields or neither (CSD-020 REQ-5).
    pub sell_date: Option<HouseDate>,
    pub sell_amount: Option<HouseValue>,
}

impl House {
    /// Whether the house has no sale (CSD-020 section 3).
    #[must_use]
    pub fn is_held(&self) -> bool {
        self.sell_date.is_none()
    }

    /// The sell amount minus the purchase amount of a sold house (CSD-020
    /// REQ-8). It is derived and never stored.
    #[must_use]
    pub fn result(&self) -> Option<Decimal> {
        Some(self.sell_amount?.value() - self.purchase_amount.value())
    }

    /// Whether the house was purchased on or before 31/12 of `year` and not
    /// sold on or before it (CSD-020 REQ-10).
    pub(super) fn held_at_end_of(&self, year: i32) -> bool {
        self.purchase_date.year() <= year && self.sell_date.is_none_or(|date| date.year() > year)
    }

    /// Whether `period` lies in the holding window: not before the period of
    /// the purchase date and not after the period of the sell date, compared
    /// in the month or year of the period (CSD-020 REQ-6).
    pub(super) fn window_contains(&self, period: Period) -> bool {
        let key = |date: HouseDate| match period {
            Period::Month(_) => (date.year(), date.month()),
            Period::Year(_) => (date.year(), 0),
        };
        let own = match period {
            Period::Month(_) => period.sort_key(),
            Period::Year(_) => (period.year(), 0),
        };
        own >= key(self.purchase_date) && self.sell_date.is_none_or(|date| own <= key(date))
    }
}

/// House fields as typed by the user, before validation.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct HouseInput {
    #[serde(default)]
    pub name: String,
    /// `None` means Monthly when creating and "unchanged" when editing.
    #[serde(default)]
    pub period: Option<String>,
    /// `dd/mm/yyyy`; required (CSD-020 REQ-1).
    #[serde(default)]
    pub purchase_date: Option<String>,
    #[serde(default)]
    pub purchase_amount: Option<String>,
    /// Both or neither (CSD-020 REQ-5).
    #[serde(default)]
    pub sell_date: Option<String>,
    #[serde(default)]
    pub sell_amount: Option<String>,
}

pub(super) struct ValidHouse {
    pub(super) name: String,
    pub(super) period: Option<PayPeriod>,
    pub(super) purchase_date: HouseDate,
    pub(super) purchase_amount: HouseValue,
    pub(super) sale: Option<(HouseDate, HouseValue)>,
}

/// The text of an optional field, trimmed, `None` when empty.
fn filled(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|text| !text.is_empty())
}

impl HouseInput {
    /// Validates the input (CSD-020 REQ-1 to REQ-5).
    pub(super) fn validate(&self) -> Result<ValidHouse, DomainError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(DomainError::HouseNameRequired);
        }
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(DomainError::HouseNameTooLong);
        }
        let period = self
            .period
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| PayPeriod::parse(text).ok_or(DomainError::HousePeriodUnsupported))
            .transpose()?;
        let purchase_date = filled(self.purchase_date.as_deref())
            .ok_or(DomainError::HousePurchaseDateRequired)
            .and_then(HouseDate::parse)?;
        let purchase_amount = filled(self.purchase_amount.as_deref())
            .ok_or(DomainError::HousePurchaseAmountRequired)
            .and_then(HouseValue::parse_amount)?;
        let sale = match (
            filled(self.sell_date.as_deref()),
            filled(self.sell_amount.as_deref()),
        ) {
            (None, None) => None,
            (Some(date), Some(amount)) => {
                let date = HouseDate::parse(date)?;
                if date < purchase_date {
                    return Err(DomainError::HouseSellBeforePurchase);
                }
                Some((date, HouseValue::parse_amount(amount)?))
            }
            _ => return Err(DomainError::HouseSaleIncomplete),
        };
        Ok(ValidHouse {
            name: name.to_owned(),
            period,
            purchase_date,
            purchase_amount,
            sale,
        })
    }
}
