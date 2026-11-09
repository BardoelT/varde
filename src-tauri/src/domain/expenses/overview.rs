//! The list of expenses and the form values for "copy previous" (CSD-013 REQ-9 to REQ-11).

use std::cmp::Reverse;

use serde::Serialize;

use crate::domain::common::{apply_filter, wire_name, StatementFilter};
use crate::domain::money::Money;

use super::{Expense, ExpenseCategory, Expenses};

/// All expenses with their total (CSD-013 REQ-11).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExpenseList {
    /// The expenses of the overview.
    pub expenses: Vec<Expense>,
    /// The sum of the amounts of `expenses`.
    pub total: Money,
}

/// Form values for "Copy previous expense": the date and the note are never
/// copied (CSD-013 REQ-9).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExpensePrefill {
    /// Name of the expense.
    pub name: String,
    /// The amount of the expense.
    pub amount: Money,
    /// The category of the expense.
    pub category: ExpenseCategory,
}

impl Expenses {
    /// All expenses, newest date first, then by name ignoring case, then in the
    /// order they were added (REQ-10), with their total (REQ-11).
    #[cfg(test)]
    #[must_use]
    pub fn list(&self) -> ExpenseList {
        self.list_filtered(None)
    }

    /// Like [`Expenses::list`], but only the expenses that pass `filter`, and
    /// the total of those (CSD-028 REQ-3, REQ-4, REQ-9). The entity of an
    /// expense is its category.
    #[must_use]
    pub fn list_filtered(&self, filter: Option<&StatementFilter>) -> ExpenseList {
        let mut expenses = apply_filter(self.expenses.iter().cloned(), filter, |expense| {
            (
                Some(expense.date.year_month()),
                wire_name(&expense.category),
            )
        });
        expenses.sort_by_cached_key(|expense| (Reverse(expense.date), expense.name.to_lowercase()));
        let total = Money::saturating_sum(expenses.iter().map(|expense| expense.amount.value()));
        ExpenseList { expenses, total }
    }

    /// The name, amount and category of the previous expense: the one with the
    /// latest date, and among those the one added last (REQ-9).
    #[must_use]
    pub fn prefill_from_previous(&self) -> Option<ExpensePrefill> {
        self.expenses
            .iter()
            .max_by_key(|expense| expense.date)
            .map(|expense| ExpensePrefill {
                name: expense.name.clone(),
                amount: expense.amount.value(),
                category: expense.category,
            })
    }
}
