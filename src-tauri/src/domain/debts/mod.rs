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
mod opening;
mod overview;
mod settlement;
mod statement;
mod totals;

#[cfg(test)]
mod ac_tests_debts;
#[cfg(test)]
mod ac_tests_opening;
#[cfg(test)]
mod tests;

pub use amounts::{debt_note, DebtBalance};
pub use debt::{Debt, DebtCategory, DebtId, DebtInput};
pub use insights::DebtInsights;
pub use opening::{ClosingInput, StatementRow};
pub use overview::DebtSummary;
pub use settlement::{OpenDebt, Outcome, Settlement};
pub use statement::{parse_period, DebtStatement, DebtStatementPrefill};
pub use totals::DebtYearlyTotal;

use serde::{Deserialize, Serialize};

use crate::domain::common::{Note, Period};
use crate::domain::error::DomainError;
use crate::domain::series::{self, SeriesData, SeriesError};

/// All debts with their statements. Debts are kept in the order they were
/// created (CSD-011 section 5). The rules every series shares are those of
/// [`SeriesData`]; the settlement of a debt (CSD-026) is added in `settlement`.
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
    /// Returns the name errors of REQ-2, `DebtPeriodUnsupported` or the
    /// `DebtOpening*` errors of REQ-30 and REQ-31.
    pub fn create_debt(&mut self, input: &DebtInput) -> Result<Debt, DomainError> {
        let valid = series::validate_identity::<Self>(&input.name, input.period.as_deref())?;
        self.check_name_free(&valid.name, None)?;
        let (opening_date, opening_amount) =
            opening::parse_opening(&input.opening_date, &input.opening_amount)?;
        let debt = Debt::new(
            DebtId::new(),
            valid.name,
            valid.period.unwrap_or_default(),
            series::next_colour_slot(self.debts.iter().map(|debt| debt.colour_slot)),
            opening_date,
            opening_amount,
        );
        self.debts.push(debt.clone());
        Ok(debt)
    }

    /// Changes the name, statement period and opening of a debt. The statements
    /// and the colour slot stay; the period is locked once the debt has any
    /// statement, and the opening cannot leave a statement outside the window
    /// (REQ-4, REQ-5, REQ-30 to REQ-32).
    ///
    /// # Errors
    /// Returns a validation error, `DebtNotFound`, `DebtNameDuplicate`,
    /// `DebtSettledLocked`, `DebtPeriodLocked`, the `DebtOpening*` errors,
    /// `DebtClosingBeforeOpening` or `DebtDatesExcludeStatements`.
    pub fn update_debt(&mut self, id: DebtId, input: &DebtInput) -> Result<Debt, DomainError> {
        let valid = series::validate_identity::<Self>(&input.name, input.period.as_deref())?;
        let (opening_date, opening_amount) =
            opening::parse_opening(&input.opening_date, &input.opening_amount)?;
        let current = self.account(id)?.period;
        self.check_name_free(&valid.name, Some(id))?;
        let period = valid.period.unwrap_or(current);
        if period != current && self.account(id)?.settlement.is_some() {
            return Err(DomainError::DebtSettledLocked);
        }
        self.check_period_unlocked(id, current, period)?;
        self.set_opening(id, opening_date, opening_amount)?;
        let debt = self.account_mut(id)?;
        debt.name = valid.name;
        debt.period = period;
        Ok(debt.clone())
    }

    /// Removes a debt and all its statements (REQ-6).
    ///
    /// # Errors
    /// Returns `DebtNotFound`.
    pub fn delete_debt(&mut self, id: DebtId) -> Result<(), DomainError> {
        self.remove_account(id)
    }

    /// Debts alphabetically by name, ignoring case (REQ-18).
    #[must_use]
    pub fn debts(&self) -> Vec<&Debt> {
        self.sorted_accounts()
    }

    /// How many statements a debt has.
    #[must_use]
    pub fn statement_count(&self, id: DebtId) -> usize {
        self.record_count(id)
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
    /// Returns `DebtNotFound`, `DebtPeriodMismatch`, `DebtStatementDuplicate`
    /// or `DebtSettledLocked`.
    pub fn add_statement(
        &mut self,
        debt_id: DebtId,
        period: Period,
        balance: DebtBalance,
        note: Note,
    ) -> Result<DebtStatement, DomainError> {
        self.add_record(DebtStatement::regular(debt_id, period, balance, note))
    }

    /// Changes the balance and note of a statement and, when `new_period` is a
    /// different period, moves it there. The new period follows the rules of
    /// adding a statement: the kind of the debt, not taken by another
    /// statement of the debt. The edit is applied as a whole or not at all
    /// (REQ-12, REQ-13).
    ///
    /// # Errors
    /// Returns `DebtNotFound`, `DebtStatementNotFound`, `DebtPeriodMismatch`,
    /// `DebtStatementDuplicate` or `DebtSettledLocked`.
    pub fn update_statement(
        &mut self,
        debt_id: DebtId,
        period: Period,
        new_period: Period,
        balance: DebtBalance,
        note: Note,
    ) -> Result<DebtStatement, DomainError> {
        self.update_record(debt_id, period, new_period, |statement| {
            statement.balance = balance;
            statement.note = note;
        })
    }

    /// Removes one statement and nothing else (REQ-14). The caller asks for
    /// the confirmation.
    ///
    /// # Errors
    /// Returns `DebtNotFound`, `DebtStatementNotFound` or `DebtSettledLocked`.
    pub fn delete_statement(&mut self, debt_id: DebtId, period: Period) -> Result<(), DomainError> {
        self.delete_record(debt_id, period)
    }

    /// The statements of all debts, newest period first. A yearly statement
    /// counts as December of its year; within a period the order is that of
    /// the debts (REQ-18).
    #[must_use]
    pub fn all_statements(&self) -> Vec<(&Debt, &DebtStatement)> {
        self.all_records()
    }

    /// Checks that data read from a file follows the rules of CSD-011, so a
    /// hand-edited or damaged file cannot break them.
    ///
    /// # Errors
    /// Returns `DebtDataInvalid` for the first inconsistency found.
    pub fn validate(&self) -> Result<(), DomainError> {
        SeriesData::validate(self)
    }
}

impl SeriesData for Debts {
    type Account = Debt;
    type Record = DebtStatement;

    const PIECE_PREFIX: &'static str = "debt";

    fn error(kind: SeriesError) -> DomainError {
        match kind {
            SeriesError::AccountNotFound => DomainError::DebtNotFound,
            SeriesError::NameRequired => DomainError::DebtNameRequired,
            SeriesError::NameTooLong => DomainError::DebtNameTooLong,
            SeriesError::NameDuplicate => DomainError::DebtNameDuplicate,
            SeriesError::PeriodUnsupported => DomainError::DebtPeriodUnsupported,
            SeriesError::PeriodLocked => DomainError::DebtPeriodLocked,
            SeriesError::PeriodMismatch => DomainError::DebtPeriodMismatch,
            SeriesError::RecordNotFound => DomainError::DebtStatementNotFound,
            SeriesError::RecordDuplicate => DomainError::DebtStatementDuplicate,
            SeriesError::DataInvalid => DomainError::DebtDataInvalid,
        }
    }

    fn accounts_list(&self) -> &[Debt] {
        &self.debts
    }

    fn accounts_list_mut(&mut self) -> &mut Vec<Debt> {
        &mut self.debts
    }

    fn records_list(&self) -> &[DebtStatement] {
        &self.statements
    }

    fn records_list_mut(&mut self) -> &mut Vec<DebtStatement> {
        &mut self.statements
    }

    /// A statement lies in the window of the debt: not before the period of
    /// its opening (CSD-011 REQ-32), and a settled debt takes no regular
    /// statement after its settlement period (CSD-026 REQ-6).
    fn check_period_allowed(&self, id: DebtId, period: Period) -> Result<(), DomainError> {
        if Self::is_before_opening(self.account(id)?, period)? {
            return Err(DomainError::DebtStatementOutsideWindow);
        }
        self.check_unsettled_at(id, period)
    }

    /// An opening that is not a valid amount, or a closing before it, in data
    /// read from a file.
    fn account_is_consistent(&self, account: &Debt) -> bool {
        DebtBalance::new(account.opening_amount.value()).is_ok()
            && account
                .settlement
                .as_ref()
                .is_none_or(|settlement| settlement.date >= account.opening_date)
    }

    /// A statement of a file is not before the opening of its debt.
    fn period_fits(&self, account: &Debt, period: Period) -> bool {
        !Self::is_before_opening(account, period).unwrap_or(true)
    }

    /// The settlement statement and the statements after the settlement period
    /// of a settled debt cannot change.
    fn check_record_editable(&self, id: DebtId, period: Period) -> Result<(), DomainError> {
        self.check_target_editable(id, period)
    }
}
