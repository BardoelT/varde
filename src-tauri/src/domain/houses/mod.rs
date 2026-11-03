//! Houses and their monthly or yearly valuations (CSD-015), and the yearly
//! totals and insights built on them (CSD-015 REQ-20, CSD-016).
//!
//! `Houses` holds the data and the rules for changing it; the other files add
//! the value types (`amounts`, `house`, `valuation`) and the figures derived
//! from it (`overview`, `totals`, `insights`). The period and note types are
//! those of the investments (CSD-005).

mod amounts;
mod dates;
mod house;
mod insights;
pub mod linking;
mod overview;
mod totals;
mod valuation;

#[cfg(test)]
mod linking_tests;
#[cfg(test)]
mod purchase_tests;
#[cfg(test)]
mod tests;

pub use amounts::{house_note, HouseValue};
pub use dates::HouseDate;
pub use house::{House, HouseId, HouseInput};
pub use insights::HouseInsights;
pub use overview::HouseSummary;
pub use totals::HouseYearlyValue;
pub use valuation::{parse_period, HouseValuation, HouseValuationPrefill};

use serde::{Deserialize, Serialize};

use crate::domain::common::{Note, Period};
use crate::domain::error::DomainError;
use crate::domain::series::{self, SeriesData, SeriesError};

/// All houses with their valuations. Houses are kept in the order they were
/// created (CSD-015 section 5). The rules every series shares are those of
/// [`SeriesData`]; a house adds its holding window (CSD-020).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Houses {
    houses: Vec<House>,
    valuations: Vec<HouseValuation>,
}

impl Houses {
    /// Creates a house, monthly unless the input says Yearly, with the colour
    /// slot of REQ-3 (REQ-1, REQ-2). Any purchase and sell date
    /// is accepted (CSD-020 REQ-3).
    ///
    /// # Errors
    /// Returns the name errors of REQ-2, `HousePeriodUnsupported` or the
    /// purchase and sale errors of CSD-020 REQ-1 to REQ-5.
    pub fn create_house(&mut self, input: &HouseInput) -> Result<House, DomainError> {
        let valid = input.validate()?;
        self.check_name_free(&valid.name, None)?;
        let house = House {
            id: HouseId::new(),
            name: valid.name,
            period: valid.period.unwrap_or_default(),
            colour_slot: series::next_colour_slot(
                self.houses.iter().map(|house| house.colour_slot),
            ),
            purchase_date: valid.purchase_date,
            purchase_amount: valid.purchase_amount,
            sale: valid.sale,
        };
        self.houses.push(house.clone());
        Ok(house)
    }

    /// Changes the name, valuation period, purchase and sale of a house. The
    /// valuations and the colour slot stay; the period is locked once the house
    /// has any valuation, and the dates may not leave a valuation outside the
    /// holding window (REQ-4, REQ-5, CSD-020 REQ-6, REQ-7).
    ///
    /// # Errors
    /// Returns a validation error, `HouseNotFound`, `HouseNameDuplicate`,
    /// `HousePeriodLocked` or `HouseDatesExcludeValuations`.
    pub fn update_house(&mut self, id: HouseId, input: &HouseInput) -> Result<House, DomainError> {
        let valid = input.validate()?;
        let current = self.account(id)?.period;
        self.check_name_free(&valid.name, Some(id))?;
        let period = valid.period.unwrap_or(current);
        self.check_period_unlocked(id, current, period)?;
        let mut updated = self.account(id)?.clone();
        updated.name = valid.name;
        updated.period = period;
        updated.purchase_date = valid.purchase_date;
        updated.purchase_amount = valid.purchase_amount;
        updated.sale = valid.sale;
        if self
            .valuations
            .iter()
            .any(|valuation| valuation.house_id == id && !updated.window_contains(valuation.period))
        {
            return Err(DomainError::HouseDatesExcludeValuations);
        }
        *self.account_mut(id)? = updated.clone();
        Ok(updated)
    }

    /// Removes a house and all its valuations (REQ-6).
    ///
    /// # Errors
    /// Returns `HouseNotFound`.
    pub fn delete_house(&mut self, id: HouseId) -> Result<(), DomainError> {
        self.remove_account(id)
    }

    /// Houses alphabetically by name, ignoring case (REQ-18).
    #[must_use]
    pub fn houses(&self) -> Vec<&House> {
        self.sorted_accounts()
    }

    /// How many valuations a house has.
    #[must_use]
    pub fn valuation_count(&self, id: HouseId) -> usize {
        self.record_count(id)
    }

    /// How many valuations all houses have together.
    #[must_use]
    pub fn total_valuation_count(&self) -> usize {
        self.valuations.len()
    }

    /// Adds the valuation of a period. Any period is accepted, a later one
    /// included (REQ-7 to REQ-9).
    ///
    /// # Errors
    /// Returns `HouseNotFound`, `HousePeriodMismatch`,
    /// `HouseValuationOutsideHolding` or `HouseValuationDuplicate`.
    pub fn add_valuation(
        &mut self,
        house_id: HouseId,
        period: Period,
        value: HouseValue,
        note: Note,
    ) -> Result<HouseValuation, DomainError> {
        self.add_record(HouseValuation {
            house_id,
            period,
            value,
            note,
        })
    }

    /// Changes the value and note of a valuation and, when `new_period` is a
    /// different period, moves it there. The new period follows the rules of
    /// adding a valuation: the kind of the house, inside the holding window,
    /// not taken by another valuation of the house. The edit is applied as a
    /// whole or not at all (REQ-12, REQ-13).
    ///
    /// # Errors
    /// Returns `HouseNotFound`, `HouseValuationNotFound`, `HousePeriodMismatch`,
    /// `HouseValuationOutsideHolding` or `HouseValuationDuplicate`.
    pub fn update_valuation(
        &mut self,
        house_id: HouseId,
        period: Period,
        new_period: Period,
        value: HouseValue,
        note: Note,
    ) -> Result<HouseValuation, DomainError> {
        self.update_record(house_id, period, new_period, |valuation| {
            valuation.value = value;
            valuation.note = note;
        })
    }

    /// Removes one valuation and nothing else (REQ-14). The caller asks for
    /// the confirmation.
    ///
    /// # Errors
    /// Returns `HouseNotFound` or `HouseValuationNotFound`.
    pub fn delete_valuation(
        &mut self,
        house_id: HouseId,
        period: Period,
    ) -> Result<(), DomainError> {
        self.delete_record(house_id, period)
    }

    /// The valuations of all houses, newest period first. A yearly valuation
    /// counts as December of its year; within a period the order is that of
    /// the houses (REQ-18).
    #[must_use]
    pub fn all_valuations(&self) -> Vec<(&House, &HouseValuation)> {
        self.all_records()
    }

    /// Checks that data read from a file follows the rules of CSD-015, so a
    /// hand-edited or damaged file cannot break them.
    ///
    /// # Errors
    /// Returns `HouseDataInvalid` for the first inconsistency found.
    pub fn validate(&self) -> Result<(), DomainError> {
        SeriesData::validate(self)
    }

    /// A house by id.
    #[must_use]
    pub fn get(&self, id: HouseId) -> Option<&House> {
        self.account(id).ok()
    }
}

impl SeriesData for Houses {
    type Account = House;
    type Record = HouseValuation;

    const PIECE_PREFIX: &'static str = "house";

    fn error(kind: SeriesError) -> DomainError {
        match kind {
            SeriesError::AccountNotFound => DomainError::HouseNotFound,
            SeriesError::NameRequired => DomainError::HouseNameRequired,
            SeriesError::NameTooLong => DomainError::HouseNameTooLong,
            SeriesError::NameDuplicate => DomainError::HouseNameDuplicate,
            SeriesError::PeriodUnsupported => DomainError::HousePeriodUnsupported,
            SeriesError::PeriodLocked => DomainError::HousePeriodLocked,
            SeriesError::PeriodMismatch => DomainError::HousePeriodMismatch,
            SeriesError::RecordNotFound => DomainError::HouseValuationNotFound,
            SeriesError::RecordDuplicate => DomainError::HouseValuationDuplicate,
            SeriesError::DataInvalid => DomainError::HouseDataInvalid,
        }
    }

    fn accounts_list(&self) -> &[House] {
        &self.houses
    }

    fn accounts_list_mut(&mut self) -> &mut Vec<House> {
        &mut self.houses
    }

    fn records_list(&self) -> &[HouseValuation] {
        &self.valuations
    }

    fn records_list_mut(&mut self) -> &mut Vec<HouseValuation> {
        &mut self.valuations
    }

    /// The sale is not before the purchase and the amounts are valid
    /// (CSD-020 REQ-5, REQ-13). That the sale has both a date and an amount is
    /// the type `Sale`.
    fn account_is_consistent(&self, house: &House) -> bool {
        let amounts_valid = [
            Some(house.purchase_amount),
            house.sale.map(|sale| sale.amount),
        ]
        .into_iter()
        .flatten()
        .all(|amount| HouseValue::new(amount.value()).is_ok());
        let sale_valid = house
            .sale
            .is_none_or(|sale| sale.date >= house.purchase_date);
        amounts_valid && sale_valid
    }

    fn period_fits(&self, house: &House, period: Period) -> bool {
        house.window_contains(period)
    }

    /// A valuation lies inside the holding window (CSD-020 REQ-6).
    fn check_period_allowed(&self, id: HouseId, period: Period) -> Result<(), DomainError> {
        if !self.account(id)?.window_contains(period) {
            return Err(DomainError::HouseValuationOutsideHolding);
        }
        Ok(())
    }

    /// Only the houses held at the end of a year count in it (CSD-020 REQ-10).
    fn counts_in_year(&self, house: &House, year: i32) -> bool {
        house.held_at_end_of(year)
    }
}
