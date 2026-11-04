//! One valuation of a house, its change, and the form values for "copy previous" (CSD-015 REQ-7, REQ-15, REQ-19).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::common::{Note, Period};
use crate::domain::error::DomainError;

use crate::domain::series::{self, SeriesData, SeriesRecord};

use super::{house_note, HouseId, HouseValue, Houses};

/// The record of one house for one period (CSD-015 REQ-7).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HouseValuation {
    /// Identifier of the house the valuation belongs to.
    pub house_id: HouseId,
    /// The month or year the valuation is for.
    pub period: Period,
    /// Value at the end of the period.
    pub value: HouseValue,
    /// Free-text note, possibly empty.
    pub note: Note,
}

impl SeriesRecord for HouseValuation {
    type AccountId = HouseId;
    type Order = (i32, u8);

    fn account_id(&self) -> HouseId {
        self.house_id
    }

    fn period(&self) -> Period {
        self.period
    }

    fn set_period(&mut self, period: Period) {
        self.period = period;
    }

    fn order_key(&self) -> (i32, u8) {
        self.period.sort_key()
    }

    fn amount(&self) -> Decimal {
        self.value.value()
    }

    fn is_valid(&self) -> bool {
        HouseValue::new(self.value.value()).is_ok() && house_note(self.note.as_str()).is_ok()
    }
}

/// Parses `YYYY-MM` or `YYYY` typed by the user (CSD-015 REQ-7).
///
/// # Errors
/// Returns `HouseValuationPeriodInvalid` for any other text.
pub fn parse_period(text: &str) -> Result<Period, DomainError> {
    series::parse_period(text, DomainError::HouseValuationPeriodInvalid)
}

/// Form values for "copy previous month" or "copy previous year" (CSD-015
/// REQ-15): the note is never copied.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HouseValuationPrefill {
    /// Value to prefill, `None` when there is nothing to copy.
    pub value: Option<Decimal>,
}

impl Houses {
    /// Form values for "copy previous month" or "copy previous year": the
    /// value of the latest valuation and no note (REQ-15).
    ///
    /// # Errors
    /// Returns `HouseNotFound`.
    pub fn prefill_from_latest(&self, id: HouseId) -> Result<HouseValuationPrefill, DomainError> {
        Ok(HouseValuationPrefill {
            value: self.latest_amount(id)?,
        })
    }

    /// The change of a valuation: its value minus the value of the previous
    /// valuation of the house, or `None` for a first valuation (REQ-19). Derived
    /// each time, never stored.
    #[must_use]
    pub fn change(&self, valuation: &HouseValuation) -> Option<Decimal> {
        self.previous_record(valuation)
            .map(|previous| valuation.value.value() - previous.value.value())
    }
}
