//! Yearly totals over all accounts (CSD-005 REQ-26).

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::money::zero;

use crate::domain::series::SeriesData;

use super::Investments;

/// The totals of one calendar year over all accounts (CSD-005 REQ-26).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InvestmentYearlyTotal {
    pub year: i32,
    /// The sum, over the accounts with a statement this year, of the balance
    /// of their latest statement this year.
    pub closing_balance: Decimal,
    pub net_deposits: Decimal,
    pub capital_growth: Decimal,
    pub statement_count: usize,
}

impl Investments {
    /// Totals per calendar year over all accounts, newest year first, without
    /// extrapolating partial years or carrying balances forward (REQ-26). A
    /// statement counts in the year of its own period only.
    #[must_use]
    pub fn yearly_totals(&self) -> Vec<InvestmentYearlyTotal> {
        self.yearly_closings()
            .into_iter()
            .map(|(year, statement_count, closing_balance)| {
                let statements = self.records_in_year(year);
                InvestmentYearlyTotal {
                    year,
                    closing_balance,
                    net_deposits: statements
                        .iter()
                        .fold(zero(), |sum, s| sum + s.net_deposits.value()),
                    capital_growth: statements
                        .iter()
                        .fold(zero(), |sum, s| sum + self.capital_growth(s)),
                    statement_count,
                }
            })
            .collect()
    }
}
