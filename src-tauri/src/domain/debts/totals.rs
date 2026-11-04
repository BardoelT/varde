//! Yearly totals over all debts (CSD-011 REQ-19).

use serde::Serialize;

use crate::domain::money::Money;

use crate::domain::series::SeriesData;

use super::Debts;

/// The totals of one calendar year over all debts (CSD-011 REQ-19).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DebtYearlyTotal {
    /// The calendar year.
    pub year: i32,
    /// The sum, over the debts with a statement this year, of the balance of
    /// their latest statement this year.
    pub closing_balance: Money,
    /// Opening balance minus closing balance, over the debts that have both;
    /// `None` when no debt has an opening balance this year.
    pub repaid: Option<Money>,
    /// Number of statements in the year.
    pub statement_count: usize,
}

impl Debts {
    /// Totals per calendar year over all debts, newest year first, without
    /// extrapolating partial years or carrying balances forward (REQ-19). A
    /// statement counts in the year of its own period only.
    #[must_use]
    pub fn yearly_totals(&self) -> Vec<DebtYearlyTotal> {
        self.yearly_closings()
            .into_iter()
            .map(|(year, statement_count, closing_balance)| DebtYearlyTotal {
                year,
                closing_balance,
                repaid: self.repaid(year),
                statement_count,
            })
            .collect()
    }

    /// The amount repaid in `year`: for each debt with a statement in the year
    /// and an earlier one, its opening balance minus its closing balance.
    /// `None` when no debt has an opening balance (CSD-011 section 3).
    pub(super) fn repaid(&self, year: i32) -> Option<Money> {
        self.year_change(year).map(|change| -change)
    }
}
