//! Debts and their monthly or yearly statements (CSD-011), and the yearly
//! totals and insights built on them (CSD-011 REQ-19, CSD-012).
//!
//! `Debts` holds the data and the rules for changing it; the other files add
//! the value types (`amounts`, `debt`, `statement`) and the figures derived
//! from it (`overview`, `totals`, `insights`). The period and note types are
//! those of the investments (CSD-005).

mod amounts;
mod debt;
mod insights;
mod overview;
mod settlement;
mod statement;
mod totals;

#[cfg(test)]
mod tests;

pub use amounts::{debt_note, DebtBalance};
pub use debt::{Debt, DebtCategory, DebtId, DebtInput};
pub use insights::DebtInsights;
pub use overview::DebtSummary;
pub use settlement::{OpenDebt, Outcome, Settlement};
pub use statement::{parse_period, DebtStatement, DebtStatementPrefill, StatementKind};
pub use totals::DebtYearlyTotal;

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::domain::common::{Note, Period};
use crate::domain::error::DomainError;

use crate::domain::common::{ColourSlot, MAX_NAME_CHARS};
use crate::domain::series;

/// All debts with their statements. Debts are kept in the order they were
/// created (CSD-011 section 5).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Debts {
    debts: Vec<Debt>,
    statements: Vec<DebtStatement>,
}

impl Debts {
    /// Creates a debt, monthly unless the input says Yearly, with the colour
    /// slot of REQ-3 (REQ-1, REQ-2).
    ///
    /// # Errors
    /// Returns the name errors of REQ-2 or `DebtPeriodUnsupported`.
    pub fn create_debt(&mut self, input: &DebtInput) -> Result<Debt, DomainError> {
        let valid = input.validate()?;
        if self.name_taken(&valid.name, None) {
            return Err(DomainError::DebtNameDuplicate);
        }
        let debt = Debt {
            id: DebtId::new(),
            name: valid.name,
            period: valid.period.unwrap_or_default(),
            colour_slot: self.next_colour_slot(),
            category: DebtCategory::default(),
            house_id: None,
            settlement: None,
        };
        self.debts.push(debt.clone());
        Ok(debt)
    }

    /// Changes the name and statement period of a debt. The statements and the
    /// colour slot stay; the period is locked once the debt has any statement
    /// (REQ-4, REQ-5).
    ///
    /// # Errors
    /// Returns a validation error, `DebtNotFound`, `DebtNameDuplicate` or
    /// `DebtPeriodLocked`.
    pub fn update_debt(&mut self, id: DebtId, input: &DebtInput) -> Result<Debt, DomainError> {
        let valid = input.validate()?;
        let current = self.debt(id)?.period;
        if self.name_taken(&valid.name, Some(id)) {
            return Err(DomainError::DebtNameDuplicate);
        }
        let period = valid.period.unwrap_or(current);
        if period != current && self.debt(id)?.settlement.is_some() {
            return Err(DomainError::DebtSettledLocked);
        }
        if period != current && self.statement_count(id) > 0 {
            return Err(DomainError::DebtPeriodLocked);
        }
        let debt = self.debt_mut(id)?;
        debt.name = valid.name;
        debt.period = period;
        Ok(debt.clone())
    }

    /// Removes a debt and all its statements (REQ-6).
    ///
    /// # Errors
    /// Returns `DebtNotFound`.
    pub fn delete_debt(&mut self, id: DebtId) -> Result<(), DomainError> {
        self.debt(id)?;
        self.debts.retain(|debt| debt.id != id);
        self.statements.retain(|statement| statement.debt_id != id);
        Ok(())
    }

    /// Debts alphabetically by name, ignoring case (REQ-18).
    #[must_use]
    pub fn debts(&self) -> Vec<&Debt> {
        let mut debts: Vec<&Debt> = self.debts.iter().collect();
        debts.sort_by_cached_key(|debt| debt_order(debt));
        debts
    }

    /// How many statements a debt has.
    #[must_use]
    pub fn statement_count(&self, id: DebtId) -> usize {
        self.statements
            .iter()
            .filter(|statement| statement.debt_id == id)
            .count()
    }

    /// How many statements all debts have together.
    #[must_use]
    pub fn total_statement_count(&self) -> usize {
        self.statements.len()
    }

    /// Adds the statement of a period. Any period is accepted, a later one
    /// included (REQ-7 to REQ-9).
    ///
    /// # Errors
    /// Returns `DebtNotFound`, `DebtPeriodMismatch` or
    /// `DebtStatementDuplicate`.
    pub fn add_statement(
        &mut self,
        debt_id: DebtId,
        period: Period,
        balance: DebtBalance,
        note: Note,
    ) -> Result<DebtStatement, DomainError> {
        self.check_new_period(debt_id, period)?;
        self.check_unsettled_at(debt_id, period)?;
        let statement = DebtStatement {
            debt_id,
            period,
            balance,
            note,
            kind: StatementKind::Regular,
        };
        self.statements.push(statement.clone());
        Ok(statement)
    }

    /// Changes the balance and note of a statement and, when `new_period` is a
    /// different period, moves it there. The new period follows the rules of
    /// adding a statement: the kind of the debt, not
    /// taken by another statement of the debt. The edit is applied as a whole
    /// or not at all (REQ-12, REQ-13).
    ///
    /// # Errors
    /// Returns `DebtNotFound`, `DebtStatementNotFound`, `DebtPeriodMismatch`
    /// or `DebtStatementDuplicate`.
    pub fn update_statement(
        &mut self,
        debt_id: DebtId,
        period: Period,
        new_period: Period,
        balance: DebtBalance,
        note: Note,
    ) -> Result<DebtStatement, DomainError> {
        self.debt(debt_id)?;
        self.check_target_editable(debt_id, period)?;
        if self.statement(debt_id, period).is_none() {
            return Err(DomainError::DebtStatementNotFound);
        }
        if new_period != period {
            self.check_new_period(debt_id, new_period)?;
            self.check_unsettled_at(debt_id, new_period)?;
        }
        let statement = self
            .statements
            .iter_mut()
            .find(|s| s.debt_id == debt_id && s.period == period && !s.is_settlement())
            .ok_or(DomainError::DebtStatementNotFound)?;
        statement.period = new_period;
        statement.balance = balance;
        statement.note = note;
        Ok(statement.clone())
    }

    /// Removes one statement and nothing else (REQ-14). The caller asks for
    /// the confirmation.
    ///
    /// # Errors
    /// Returns `DebtNotFound` or `DebtStatementNotFound`.
    pub fn delete_statement(&mut self, debt_id: DebtId, period: Period) -> Result<(), DomainError> {
        self.debt(debt_id)?;
        self.check_target_editable(debt_id, period)?;
        let before = self.statements.len();
        self.statements
            .retain(|s| !(s.debt_id == debt_id && s.period == period && !s.is_settlement()));
        if self.statements.len() == before {
            return Err(DomainError::DebtStatementNotFound);
        }
        Ok(())
    }

    /// The statements of all debts, newest period first. A yearly statement
    /// counts as December of its year; within a period the order is that of
    /// the debts (REQ-18).
    #[must_use]
    pub fn all_statements(&self) -> Vec<(&Debt, &DebtStatement)> {
        let mut rows: Vec<(&Debt, &DebtStatement)> = self
            .statements
            .iter()
            .filter_map(|statement| {
                self.debts
                    .iter()
                    .find(|debt| debt.id == statement.debt_id)
                    .map(|debt| (debt, statement))
            })
            .collect();
        rows.sort_by_cached_key(|(debt, statement)| {
            (std::cmp::Reverse(statement.order_key()), debt_order(debt))
        });
        rows
    }

    /// Checks that data read from a file follows the rules of CSD-011, so a
    /// hand-edited or damaged file cannot break them.
    ///
    /// # Errors
    /// Returns `DebtDataInvalid` for the first inconsistency found.
    pub fn validate(&self) -> Result<(), DomainError> {
        let invalid = DomainError::DebtDataInvalid;
        let mut ids = HashSet::new();
        let mut names = HashSet::new();
        for debt in &self.debts {
            let name = debt.name.trim();
            if name.is_empty()
                || name.chars().count() > MAX_NAME_CHARS
                || !ids.insert(debt.id)
                || !names.insert(name.to_lowercase())
            {
                return Err(invalid);
            }
        }
        let mut seen = HashSet::new();
        for statement in &self.statements {
            let matches_debt = self
                .debt(statement.debt_id)
                .is_ok_and(|debt| debt.period == statement.period.kind());
            if !matches_debt
                || !seen.insert((statement.debt_id, statement.period, statement.kind))
                || DebtBalance::new(statement.balance.value()).is_err()
                || debt_note(statement.note.as_str()).is_err()
            {
                return Err(invalid);
            }
        }
        Ok(())
    }

    fn debt(&self, id: DebtId) -> Result<&Debt, DomainError> {
        self.debts
            .iter()
            .find(|debt| debt.id == id)
            .ok_or(DomainError::DebtNotFound)
    }

    fn debt_mut(&mut self, id: DebtId) -> Result<&mut Debt, DomainError> {
        self.debts
            .iter_mut()
            .find(|debt| debt.id == id)
            .ok_or(DomainError::DebtNotFound)
    }

    /// The regular statement of a period; the settlement statement is not one
    /// (CSD-011 REQ-8).
    fn statement(&self, debt_id: DebtId, period: Period) -> Option<&DebtStatement> {
        self.statements
            .iter()
            .find(|s| s.debt_id == debt_id && s.period == period && !s.is_settlement())
    }

    /// The rules a period must follow for a debt to hold a statement for it:
    /// the kind of the debt, not taken (REQ-7 to REQ-9).
    fn check_new_period(&self, debt_id: DebtId, period: Period) -> Result<(), DomainError> {
        if self.debt(debt_id)?.period != period.kind() {
            return Err(DomainError::DebtPeriodMismatch);
        }
        if self.statement(debt_id, period).is_some() {
            return Err(DomainError::DebtStatementDuplicate);
        }
        Ok(())
    }

    /// The statement with the most recent period of a debt.
    fn latest(&self, id: DebtId) -> Option<&DebtStatement> {
        self.statements
            .iter()
            .filter(|statement| statement.debt_id == id)
            .max_by_key(|statement| statement.order_key())
    }

    /// The lowest colour slot used by the fewest debts (REQ-3).
    fn next_colour_slot(&self) -> ColourSlot {
        series::next_colour_slot(self.debts.iter().map(|debt| debt.colour_slot))
    }

    /// Whether another debt already has this name, ignoring case (REQ-2).
    fn name_taken(&self, name: &str, except: Option<DebtId>) -> bool {
        series::name_taken(
            self.debts.iter().map(|debt| (debt.id, debt.name.as_str())),
            name,
            except,
        )
    }
}

/// Alphabetical order ignoring case, with the id as a stable tie-break.
fn debt_order(debt: &Debt) -> (String, String) {
    series::name_order(&debt.name, &debt.id)
}
