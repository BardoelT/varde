//! The yearly series behind the Income Insights chart (CSD-006).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::money::{percent_of, zero};

use super::{Income, IncomeYearlyTotal};
use crate::domain::common::YearMonth;

/// One year of the insights chart: sums over all jobs, whether the year is
/// still running, and its growth against the year before (CSD-006).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IncomeInsightsYear {
    pub year: i32,
    pub gross_salary: Decimal,
    pub holiday_pay: Decimal,
    pub bonus: Decimal,
    pub total_gross: Decimal,
    pub entry_count: usize,
    pub is_partial: bool,
    /// Percent with two decimals, `None` when there is nothing to compare.
    pub growth_percent: Option<Decimal>,
}

impl Income {
    /// One figure set per calendar year from the first entry up to and
    /// including the current year, oldest first (CSD-006). Years without
    /// entries are zero. The current year is partial and, like a year without
    /// entries or after one, has no growth. Entries after the current year,
    /// which only a project file can contain, are not shown.
    #[must_use]
    pub fn insights(&self, today: YearMonth) -> Vec<IncomeInsightsYear> {
        let totals: BTreeMap<i32, IncomeYearlyTotal> = self
            .yearly_totals()
            .into_iter()
            .map(|total| (total.year, total))
            .collect();
        let Some(&first) = totals.keys().next() else {
            return Vec::new();
        };
        let mut series: Vec<IncomeInsightsYear> = Vec::new();
        for year in first..=today.year() {
            let (gross_salary, holiday_pay, bonus, total_gross, entry_count) =
                totals.get(&year).map_or_else(
                    || (zero(), zero(), zero(), zero(), 0),
                    |t| {
                        (
                            t.gross_salary,
                            t.holiday_pay,
                            t.bonus,
                            t.total_gross,
                            t.entry_count,
                        )
                    },
                );
            let is_partial = year == today.year();
            let growth_percent = match series.last() {
                Some(previous) if !is_partial && entry_count > 0 => total_gross
                    .checked_sub(previous.total_gross)
                    .and_then(|change| percent_of(change, previous.total_gross)),
                _ => None,
            };
            series.push(IncomeInsightsYear {
                year,
                gross_salary,
                holiday_pay,
                bonus,
                total_gross,
                entry_count,
                is_partial,
                growth_percent,
            });
        }
        series
    }
}
