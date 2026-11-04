//! Yearly totals over all accounts (CSD-009 REQ-19).

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::series::SeriesData;

use super::BankAccounts;

/// The totals of one calendar year over all accounts (CSD-009 REQ-19).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BankYearlyTotal {
    /// The calendar year.
    pub year: i32,
    /// The sum, over the accounts with an entry this year, of the balance of
    /// their latest entry this year.
    pub closing_balance: Decimal,
    /// Number of entries in the year.
    pub entry_count: usize,
}

impl BankAccounts {
    /// Totals per calendar year over all accounts, newest year first, without
    /// extrapolating partial years or carrying balances forward (REQ-19). An
    /// entry counts in the year of its own period only.
    #[must_use]
    pub fn yearly_totals(&self) -> Vec<BankYearlyTotal> {
        self.yearly_closings()
            .into_iter()
            .map(|(year, entry_count, closing_balance)| BankYearlyTotal {
                year,
                closing_balance,
                entry_count,
            })
            .collect()
    }

    /// The increase of the bank accounts in `year`: for each account with an
    /// entry in the year and an earlier one, the balance of its latest entry in
    /// the year minus that of its latest earlier entry. `None` when no account
    /// has both (CSD-018 section 3).
    #[must_use]
    pub fn increase(&self, year: i32) -> Option<Decimal> {
        self.year_change(year)
    }
}
