//! Yearly totals over all expenses (CSD-013 REQ-12).

use std::collections::BTreeMap;

use serde::Serialize;
use time::Date;

use crate::domain::money::Money;

use super::Expenses;

/// The totals of one calendar year (CSD-013 REQ-12).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExpenseYearlyTotal {
    /// The calendar year.
    pub year: i32,
    /// Total spent in the year.
    pub total: Money,
    /// Number of expenses in the year.
    pub expense_count: usize,
    /// The highest amount of a single expense this year.
    pub largest: Money,
}

impl Expenses {
    /// The sum of the amounts of the expenses dated from `from` up to and
    /// including `to` (CSD-004 REQ-9, REQ-10).
    #[must_use]
    pub fn total_between(&self, from: Date, to: Date) -> Money {
        Money::saturating_sum(
            self.expenses
                .iter()
                .filter(|expense| (from..=to).contains(&expense.date.date()))
                .map(|expense| expense.amount.value()),
        )
    }

    /// Totals per calendar year with at least one expense, newest year first,
    /// without extrapolating partial years (REQ-12).
    #[must_use]
    pub fn yearly_totals(&self) -> Vec<ExpenseYearlyTotal> {
        let mut years: BTreeMap<i32, ExpenseYearlyTotal> = BTreeMap::new();
        for expense in &self.expenses {
            let amount = expense.amount.value();
            let row = years
                .entry(expense.date.year())
                .or_insert_with(|| ExpenseYearlyTotal {
                    year: expense.date.year(),
                    total: Money::ZERO,
                    expense_count: 0,
                    largest: amount,
                });
            row.total = row.total.saturating_add(amount);
            row.expense_count += 1;
            row.largest = row.largest.max(amount);
        }
        years.into_values().rev().collect()
    }
}
