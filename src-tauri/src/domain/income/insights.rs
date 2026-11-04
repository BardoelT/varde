//! The yearly series behind the Income Insights chart (CSD-006).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::charts::{percent_label, stack_from_zero, ChartYear, Marker, Piece};
use crate::domain::money::{percent_of, zero};

use super::{Income, IncomeYearlyTotal};
use crate::domain::common::{series_years, YearMonth};

/// One year of the insights chart: sums over all jobs, whether the year is
/// still running, and its growth against the year before (CSD-006).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IncomeInsightsYear {
    /// The calendar year.
    pub year: i32,
    /// Gross salary.
    pub gross_salary: Decimal,
    /// Holiday pay.
    pub holiday_pay: Decimal,
    /// Bonus.
    pub bonus: Decimal,
    /// Gross salary, holiday pay and bonus together.
    pub total_gross: Decimal,
    /// Number of entries in the year.
    pub entry_count: usize,
    /// Whether the year is still running, so its figures cover only part of it.
    pub is_partial: bool,
    /// Percent with two decimals, `None` when there is nothing to compare.
    pub growth_percent: Option<Decimal>,
    /// The growth as the chart shows it: "+6.42%", "-2.50%", "0.00%" or a dash
    /// (CSD-006 REQ-8).
    pub growth_label: String,
    /// The stacked pieces of the bar, bottom to top (CSD-025 section 5).
    pub pieces: Vec<Piece>,
    /// Income has no marker; always `None`.
    pub marker: Option<Marker>,
}

impl ChartYear for IncomeInsightsYear {
    fn pieces(&self) -> &[Piece] {
        &self.pieces
    }

    fn marker(&self) -> Option<&Marker> {
        self.marker.as_ref()
    }
}

impl Income {
    /// One figure set per calendar year from the first entry up to and
    /// including the later of the current year and the latest entry, at most 50
    /// years back from it, oldest first (CSD-006 REQ-2). An older entry has no
    /// slot but still counts in the yearly totals. Years without entries are
    /// zero. The current year is partial and, like a year without entries or
    /// after one, has no growth; a year after the current year is not partial.
    #[must_use]
    pub fn insights(&self, today: YearMonth) -> Vec<IncomeInsightsYear> {
        let totals: BTreeMap<i32, IncomeYearlyTotal> = self
            .yearly_totals()
            .into_iter()
            .map(|total| (total.year, total))
            .collect();
        let mut series: Vec<IncomeInsightsYear> = Vec::new();
        for year in series_years(totals.keys().copied(), today.year()) {
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
            let growth_percent = match totals.get(&(year - 1)) {
                Some(previous) if !is_partial && entry_count > 0 => total_gross
                    .checked_sub(previous.total_gross)
                    .and_then(|change| percent_of(change, previous.total_gross)),
                _ => None,
            };
            let pieces = stack_from_zero([
                ("salary".to_owned(), gross_salary),
                ("holiday".to_owned(), holiday_pay),
                ("bonus".to_owned(), bonus),
            ]);
            series.push(IncomeInsightsYear {
                year,
                gross_salary,
                holiday_pay,
                bonus,
                total_gross,
                entry_count,
                is_partial,
                growth_percent,
                growth_label: percent_label(growth_percent),
                pieces,
                marker: None,
            });
        }
        series
    }
}
