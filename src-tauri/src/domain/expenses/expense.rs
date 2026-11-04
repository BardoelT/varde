//! One expense and the fields the user types for it (CSD-013 REQ-1 to REQ-7).

use serde::{Deserialize, Serialize};

use crate::domain::common::Note;
use crate::domain::common::MAX_NAME_CHARS;
use crate::domain::common::{Id, IdKind};
use crate::domain::error::DomainError;

use super::{expense_note, ExpenseAmount, ExpenseCategory, ExpenseDate};

/// Identifies a Expense.
pub type ExpenseId = Id<Expense>;

impl IdKind for Expense {
    fn not_found() -> DomainError {
        DomainError::ExpenseNotFound
    }
}

/// One purchase or cost (CSD-013 section 3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expense {
    /// Identifier of the expense.
    pub id: ExpenseId,
    /// Name of the expense.
    pub name: String,
    /// The amount spent.
    pub amount: ExpenseAmount,
    /// The date of the expense.
    pub date: ExpenseDate,
    /// The category of the expense.
    pub category: ExpenseCategory,
    /// Free-text note, possibly empty.
    pub note: Note,
}

/// Expense fields as typed by the user, before validation.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ExpenseInput {
    /// Name of the expense.
    #[serde(default)]
    pub name: String,
    /// Amount as typed, before validation.
    #[serde(default)]
    pub amount: String,
    /// `dd/mm/yyyy`.
    #[serde(default)]
    pub date: String,
    /// Category name as typed, before validation.
    #[serde(default)]
    pub category: String,
    /// Note as typed, before validation.
    #[serde(default)]
    pub note: Option<String>,
}

/// The values of an input that follow every rule, ready to be stored.
pub(super) struct ValidExpense {
    pub(super) name: String,
    pub(super) amount: ExpenseAmount,
    pub(super) date: ExpenseDate,
    pub(super) category: ExpenseCategory,
    pub(super) note: Note,
}

impl ExpenseInput {
    /// Checks every field, in the order name, amount, date, category and note,
    /// and returns the first violated rule.
    pub(super) fn validate(&self) -> Result<ValidExpense, DomainError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(DomainError::ExpenseNameRequired);
        }
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(DomainError::ExpenseNameTooLong);
        }
        Ok(ValidExpense {
            name: name.to_owned(),
            amount: ExpenseAmount::parse(&self.amount)?,
            date: ExpenseDate::parse(&self.date)?,
            category: ExpenseCategory::parse(&self.category)?,
            note: expense_note(self.note.as_deref().unwrap_or_default())?,
        })
    }
}
