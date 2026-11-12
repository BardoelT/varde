//! Yearly totals over all transactions (CSD-032 REQ-24).

use std::collections::BTreeMap;

use serde::Serialize;

use crate::domain::money::Money;

use super::Transactions;

/// The totals of one calendar year (CSD-032 REQ-24).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TransactionYearlyTotal {
    /// The calendar year.
    pub year: i32,
    /// The sum of the credits of the year.
    pub income: Money,
    /// The sum of the debits of the year, as a positive amount.
    pub spent: Money,
    /// `income` minus `spent`, negative when more was spent than came in.
    pub net: Money,
    /// How many transactions the year has.
    pub transaction_count: usize,
}

impl Transactions {
    /// Totals per calendar year with at least one transaction, newest year
    /// first, over every transaction whatever the filter of the list, without
    /// extrapolating partial years (REQ-24).
    #[must_use]
    pub fn yearly_totals(&self) -> Vec<TransactionYearlyTotal> {
        let mut years: BTreeMap<i32, TransactionYearlyTotal> = BTreeMap::new();
        for transaction in self.transactions() {
            let year = transaction.date.year();
            let row = years.entry(year).or_insert_with(|| TransactionYearlyTotal {
                year,
                income: Money::ZERO,
                spent: Money::ZERO,
                net: Money::ZERO,
                transaction_count: 0,
            });
            if transaction.amount.is_negative() {
                row.spent = row.spent.saturating_add(transaction.amount.abs());
            } else {
                row.income = row.income.saturating_add(transaction.amount);
            }
            row.net = row.income.saturating_sub(row.spent);
            row.transaction_count += 1;
        }
        years.into_values().rev().collect()
    }
}
