//! Houses: their id, name, valuation period, colour slot (CSD-015 REQ-1 to
//! REQ-6) and purchase and sale (CSD-019 REQ-1 to REQ-5).

use crate::domain::money::Money;
use serde::{Deserialize, Serialize};

use crate::domain::common::Period;
use crate::domain::common::{ColourSlot, PayPeriod};
use crate::domain::common::{Id, IdKind};
use crate::domain::error::DomainError;
use crate::domain::series::{self, SeriesAccount};

use super::{HouseDate, HouseValue, Houses};

/// Identifies a House.
pub type HouseId = Id<House>;

impl IdKind for House {
    fn not_found() -> DomainError {
        DomainError::HouseNotFound
    }
}

/// The sale of a house: the date and the amount together (CSD-019 REQ-5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sale {
    pub date: HouseDate,
    pub amount: HouseValue,
}

/// One thing the user owns that has a value (CSD-015 REQ-1). On the wire the
/// sale is the two flat keys `sell_date` and `sell_amount`, both set or both
/// null.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "HouseRaw", into = "HouseRaw")]
pub struct House {
    /// Identifier of the house.
    pub id: HouseId,
    /// Name of the house.
    pub name: String,
    /// Whether the house is kept per month or per year.
    pub period: PayPeriod,
    /// 0 to 7, fixed when the house is created (CSD-015 REQ-3).
    pub colour_slot: ColourSlot,
    /// Every house has a purchase (CSD-019 REQ-1).
    pub purchase_date: HouseDate,
    /// Purchase price.
    pub purchase_amount: HouseValue,
    /// The sale, with its date and amount together or none (CSD-019 REQ-5).
    pub sale: Option<Sale>,
}

/// The flat shape of a house in the project file and IPC. `flatten` does not
/// combine with `deny_unknown_fields`, so the sale is split into its two keys
/// here and a file with only one of them is rejected.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HouseRaw {
    id: HouseId,
    name: String,
    period: PayPeriod,
    colour_slot: ColourSlot,
    purchase_date: HouseDate,
    purchase_amount: HouseValue,
    sell_date: Option<HouseDate>,
    sell_amount: Option<HouseValue>,
}

impl TryFrom<HouseRaw> for House {
    type Error = DomainError;

    fn try_from(raw: HouseRaw) -> Result<Self, DomainError> {
        let sale = match (raw.sell_date, raw.sell_amount) {
            (None, None) => None,
            (Some(date), Some(amount)) => Some(Sale { date, amount }),
            _ => return Err(DomainError::HouseSaleIncomplete),
        };
        Ok(Self {
            id: raw.id,
            name: raw.name,
            period: raw.period,
            colour_slot: raw.colour_slot,
            purchase_date: raw.purchase_date,
            purchase_amount: raw.purchase_amount,
            sale,
        })
    }
}

impl From<House> for HouseRaw {
    fn from(house: House) -> Self {
        Self {
            id: house.id,
            name: house.name,
            period: house.period,
            colour_slot: house.colour_slot,
            purchase_date: house.purchase_date,
            purchase_amount: house.purchase_amount,
            sell_date: house.sale.map(|sale| sale.date),
            sell_amount: house.sale.map(|sale| sale.amount),
        }
    }
}

impl SeriesAccount for House {
    type Id = HouseId;

    fn id(&self) -> HouseId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn period(&self) -> PayPeriod {
        self.period
    }
}

impl House {
    /// Whether the house has no sale (CSD-019 section 3).
    #[must_use]
    pub fn is_held(&self) -> bool {
        self.sale.is_none()
    }

    /// The sell amount minus the purchase amount of a sold house (CSD-019
    /// REQ-8). It is derived and never stored.
    #[must_use]
    pub fn result(&self) -> Option<Money> {
        Some(
            self.sale?
                .amount
                .value()
                .saturating_sub(self.purchase_amount.value()),
        )
    }

    /// Whether the house was purchased on or before 31/12 of `year` and not
    /// sold on or before it (CSD-019 REQ-10).
    pub(super) fn held_at_end_of(&self, year: i32) -> bool {
        self.purchase_date.year() <= year && self.sale.is_none_or(|sale| sale.date.year() > year)
    }

    /// Whether `period` lies in the holding window: not before the period of
    /// the purchase date and not after the period of the sell date, compared
    /// in the month or year of the period (CSD-019 REQ-6).
    pub(super) fn window_contains(&self, period: Period) -> bool {
        let key = |date: HouseDate| match period {
            Period::Month(_) => (date.year(), date.month()),
            Period::Year(_) => (date.year(), 0),
        };
        let own = match period {
            Period::Month(_) => period.sort_key(),
            Period::Year(_) => (period.year(), 0),
        };
        own >= key(self.purchase_date) && self.sale.is_none_or(|sale| own <= key(sale.date))
    }
}

/// House fields as typed by the user, before validation.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct HouseInput {
    /// Name of the house.
    #[serde(default)]
    pub name: String,
    /// `None` means Monthly when creating and "unchanged" when editing.
    #[serde(default)]
    pub period: Option<String>,
    /// `dd/mm/yyyy`; required (CSD-019 REQ-1).
    #[serde(default)]
    pub purchase_date: Option<String>,
    /// Purchase price.
    #[serde(default)]
    pub purchase_amount: Option<String>,
    /// Both or neither (CSD-019 REQ-5).
    #[serde(default)]
    pub sell_date: Option<String>,
    /// Sale price.
    #[serde(default)]
    pub sell_amount: Option<String>,
}

pub(super) struct ValidHouse {
    pub(super) name: String,
    pub(super) period: Option<PayPeriod>,
    pub(super) purchase_date: HouseDate,
    pub(super) purchase_amount: HouseValue,
    pub(super) sale: Option<Sale>,
}

/// The text of an optional field, trimmed, `None` when empty.
fn filled(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|text| !text.is_empty())
}

impl HouseInput {
    /// Validates the input (CSD-019 REQ-1 to REQ-5).
    pub(super) fn validate(&self) -> Result<ValidHouse, DomainError> {
        let identity = series::validate_identity::<Houses>(&self.name, self.period.as_deref())?;
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
                Some(Sale {
                    date,
                    amount: HouseValue::parse_amount(amount)?,
                })
            }
            _ => return Err(DomainError::HouseSaleIncomplete),
        };
        Ok(ValidHouse {
            name: identity.name,
            period: identity.period,
            purchase_date,
            purchase_amount,
            sale,
        })
    }
}
