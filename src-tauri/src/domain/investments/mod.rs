//! Investment accounts and their monthly or yearly statements (CSD-005), and
//! the yearly totals and insights built on them (CSD-005 REQ-26, CSD-007).
//!
//! `Investments` holds the data and the rules for changing it; the other files
//! add the value types (`period`, `amounts`, `account`, `statement`) and the
//! figures derived from it (`overview`, `growth`, `totals`, `insights`).

mod account;
mod amounts;
mod growth;
mod insights;
mod overview;
mod statement;
mod totals;

#[cfg(test)]
mod tests;

pub use account::{InvestmentAccount, InvestmentAccountId, InvestmentAccountInput};
pub use amounts::{Balance, NetDeposits};
pub use growth::{CalculationInput, StatementCalculation};
pub use insights::InvestmentInsightsYear;
pub use overview::InvestmentsOverview;
pub use statement::{InvestmentStatement, InvestmentStatementPrefill};
pub use totals::InvestmentYearlyTotal;

use crate::domain::money::Money;
use serde::{Deserialize, Serialize};

use crate::domain::common::{Note, Period};
use crate::domain::error::DomainError;
use crate::domain::series::{self, SeriesData, SeriesError};

/// All investment accounts with their statements. The rules every series
/// shares are those of [`SeriesData`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Investments {
    accounts: Vec<InvestmentAccount>,
    statements: Vec<InvestmentStatement>,
}

impl Investments {
    /// Creates an account, monthly unless the input says Yearly (REQ-1).
    ///
    /// # Errors
    /// Returns the name errors of REQ-2 or `InvestmentPeriodUnsupported`.
    pub fn create_account(
        &mut self,
        input: &InvestmentAccountInput,
    ) -> Result<InvestmentAccount, DomainError> {
        let valid = series::validate_identity::<Self>(&input.name, input.period.as_deref())?;
        self.check_name_free(&valid.name, None)?;
        let account = InvestmentAccount {
            id: InvestmentAccountId::new(),
            name: valid.name,
            period: valid.period.unwrap_or_default(),
        };
        self.accounts.push(account.clone());
        Ok(account)
    }

    /// Changes the name and statement period of an account. The statements
    /// stay; the period is locked once the account has any (REQ-3, REQ-4).
    ///
    /// # Errors
    /// Returns a validation error, `InvestmentAccountNotFound`,
    /// `InvestmentAccountNameDuplicate` or `InvestmentPeriodLocked`.
    pub fn update_account(
        &mut self,
        id: InvestmentAccountId,
        input: &InvestmentAccountInput,
    ) -> Result<InvestmentAccount, DomainError> {
        let valid = series::validate_identity::<Self>(&input.name, input.period.as_deref())?;
        let current = self.account(id)?.period;
        self.check_name_free(&valid.name, Some(id))?;
        let period = valid.period.unwrap_or(current);
        self.check_period_unlocked(id, current, period)?;
        let account = self.account_mut(id)?;
        account.name = valid.name;
        account.period = period;
        Ok(account.clone())
    }

    /// Removes an account and all its statements (REQ-5).
    ///
    /// # Errors
    /// Returns `InvestmentAccountNotFound`.
    pub fn delete_account(&mut self, id: InvestmentAccountId) -> Result<(), DomainError> {
        self.remove_account(id)
    }

    /// Accounts alphabetically by name, ignoring case (REQ-16).
    #[must_use]
    pub fn accounts(&self) -> Vec<&InvestmentAccount> {
        self.sorted_accounts()
    }

    /// How many statements an account has.
    #[must_use]
    pub fn statement_count(&self, id: InvestmentAccountId) -> usize {
        self.record_count(id)
    }

    /// How many statements all accounts have together.
    #[must_use]
    pub fn total_statement_count(&self) -> usize {
        self.statements.len()
    }

    /// Adds the statement of a period. Any period is accepted, a later one
    /// included (REQ-6).
    ///
    /// # Errors
    /// Returns `InvestmentAccountNotFound`, `InvestmentPeriodMismatch`
    /// or `InvestmentStatementDuplicate`.
    pub fn add_statement(
        &mut self,
        account_id: InvestmentAccountId,
        period: Period,
        balance: Balance,
        net_deposits: NetDeposits,
        note: Note,
    ) -> Result<InvestmentStatement, DomainError> {
        self.add_record(InvestmentStatement {
            account_id,
            period,
            balance,
            net_deposits,
            note,
        })
    }

    /// Changes the balance, net deposits and note of a statement and, when
    /// `new_period` is a different period, moves it there. The new period
    /// follows the rules of adding a statement: the kind of the account, not
    /// taken by another statement of the account. The edit is applied as a
    /// whole or not at all (REQ-11, REQ-28, REQ-29).
    ///
    /// # Errors
    /// Returns `InvestmentAccountNotFound`, `InvestmentStatementNotFound`,
    /// `InvestmentPeriodMismatch` or
    /// `InvestmentStatementDuplicate`.
    #[allow(
        clippy::too_many_arguments,
        reason = "add_statement's values, plus the statement's current period"
    )]
    pub fn update_statement(
        &mut self,
        account_id: InvestmentAccountId,
        period: Period,
        new_period: Period,
        balance: Balance,
        net_deposits: NetDeposits,
        note: Note,
    ) -> Result<InvestmentStatement, DomainError> {
        self.update_record(account_id, period, new_period, |statement| {
            statement.balance = balance;
            statement.net_deposits = net_deposits;
            statement.note = note;
        })
    }

    /// Removes one statement and nothing else (REQ-12). The caller asks for the
    /// confirmation.
    ///
    /// # Errors
    /// Returns `InvestmentAccountNotFound` or `InvestmentStatementNotFound`.
    pub fn delete_statement(
        &mut self,
        account_id: InvestmentAccountId,
        period: Period,
    ) -> Result<(), DomainError> {
        self.delete_record(account_id, period)
    }

    /// The statements of all accounts, newest period first. A yearly statement
    /// counts as December of its year; within a period the order is that of the
    /// accounts (REQ-16).
    #[must_use]
    pub fn all_statements(&self) -> Vec<(&InvestmentAccount, &InvestmentStatement)> {
        self.all_records()
    }

    /// Checks that data read from a file follows the rules of CSD-005, so a
    /// hand-edited or damaged file cannot break them.
    ///
    /// # Errors
    /// Returns `InvestmentDataInvalid` for the first inconsistency found.
    pub fn validate(&self) -> Result<(), DomainError> {
        SeriesData::validate(self)
    }

    /// The balance of the statement of the account with the latest period
    /// before `period`, also when periods in between have none. The statement
    /// of `ignore`, the one being edited, never counts (REQ-24).
    fn previous_balance(
        &self,
        id: InvestmentAccountId,
        period: Period,
        ignore: Option<Period>,
    ) -> Option<Money> {
        self.statements
            .iter()
            .filter(|s| {
                s.account_id == id
                    && s.period.sort_key() < period.sort_key()
                    && Some(s.period) != ignore
            })
            .max_by_key(|s| s.period.sort_key())
            .map(|s| s.balance.value())
    }
}

impl SeriesData for Investments {
    type Account = InvestmentAccount;
    type Record = InvestmentStatement;

    const PIECE_PREFIX: &'static str = "investment";

    fn error(kind: SeriesError) -> DomainError {
        match kind {
            SeriesError::AccountNotFound => DomainError::InvestmentAccountNotFound,
            SeriesError::NameRequired => DomainError::InvestmentAccountNameRequired,
            SeriesError::NameTooLong => DomainError::InvestmentAccountNameTooLong,
            SeriesError::NameDuplicate => DomainError::InvestmentAccountNameDuplicate,
            SeriesError::PeriodUnsupported => DomainError::InvestmentPeriodUnsupported,
            SeriesError::PeriodLocked => DomainError::InvestmentPeriodLocked,
            SeriesError::PeriodMismatch => DomainError::InvestmentPeriodMismatch,
            SeriesError::RecordNotFound => DomainError::InvestmentStatementNotFound,
            SeriesError::RecordDuplicate => DomainError::InvestmentStatementDuplicate,
            SeriesError::DataInvalid => DomainError::InvestmentDataInvalid,
        }
    }

    fn accounts_list(&self) -> &[InvestmentAccount] {
        &self.accounts
    }

    fn accounts_list_mut(&mut self) -> &mut Vec<InvestmentAccount> {
        &mut self.accounts
    }

    fn records_list(&self) -> &[InvestmentStatement] {
        &self.statements
    }

    fn records_list_mut(&mut self) -> &mut Vec<InvestmentStatement> {
        &mut self.statements
    }
}
