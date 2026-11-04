//! Yearly values over all houses (CSD-015 REQ-20, CSD-019 REQ-10).

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::series::SeriesData;

use super::Houses;

/// The totals of one calendar year over all houses (CSD-015 REQ-20).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HouseYearlyValue {
    /// The calendar year.
    pub year: i32,
    /// The sum, over the houses with a valuation this year, of the value of
    /// their latest valuation this year.
    pub closing_value: Decimal,
    /// Closing value minus opening value, over the houses that have both;
    /// `None` when no house has an opening value this year.
    pub change: Option<Decimal>,
    /// Number of valuations in the year.
    pub valuation_count: usize,
}

impl Houses {
    /// Totals per calendar year over all houses, newest year first, without
    /// extrapolating partial years or carrying values forward (REQ-20). A
    /// valuation counts in the year of its own period only, and only when its
    /// house is held at the end of that year (CSD-019 REQ-10).
    #[must_use]
    pub fn yearly_values(&self) -> Vec<HouseYearlyValue> {
        self.yearly_closings()
            .into_iter()
            .map(|(year, valuation_count, closing_value)| HouseYearlyValue {
                year,
                closing_value,
                change: self.year_change(year),
                valuation_count,
            })
            .collect()
    }
}
