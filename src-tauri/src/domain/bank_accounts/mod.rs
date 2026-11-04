//! Bank accounts and their monthly or yearly entries (CSD-009), and the yearly
//! totals and insights built on them (CSD-009 REQ-19, CSD-010).
//!
//! `BankAccounts` holds the data and the rules for changing it; the other files
//! add the value types (`amounts`, `account`, `entry`) and the figures derived
//! from it (`overview`, `totals`, `insights`). The period and note types are
//! those of the investments (CSD-005).

mod account;
mod amounts;
mod entry;
mod insights;
mod overview;
mod totals;

#[cfg(test)]
mod ac_tests;
#[cfg(test)]
mod tests;

pub use account::{BankAccount, BankAccountId, BankAccountInput};
pub use amounts::{bank_note, BankBalance};
pub use entry::{parse_period, BankEntry, EntryPrefill};
pub use insights::BankInsights;
pub use overview::BankOverview;
pub use totals::BankYearlyTotal;

use serde::{Deserialize, Serialize};

use crate::domain::common::{Note, Period};
use crate::domain::error::DomainError;
use crate::domain::series::{self, SeriesData, SeriesError};

/// All bank accounts with their entries. Accounts are kept in the order they
/// were created (CSD-009 section 5). The rules every series shares are those
/// of [`SeriesData`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BankAccounts {
    accounts: Vec<BankAccount>,
    entries: Vec<BankEntry>,
}

impl BankAccounts {
    /// Creates an account, monthly unless the input says Yearly, with the
    /// colour slot of REQ-3 (REQ-1, REQ-2).
    ///
    /// # Errors
    /// Returns the name errors of REQ-2 or `BankAccountPeriodUnsupported`.
    pub fn create_account(&mut self, input: &BankAccountInput) -> Result<BankAccount, DomainError> {
        let valid = series::validate_identity::<Self>(&input.name, input.period.as_deref())?;
        self.check_name_free(&valid.name, None)?;
        let account = BankAccount {
            id: BankAccountId::new(),
            name: valid.name,
            period: valid.period.unwrap_or_default(),
            colour_slot: series::next_colour_slot(self.accounts.iter().map(|a| a.colour_slot)),
        };
        self.accounts.push(account.clone());
        Ok(account)
    }

    /// Changes the name and entry period of an account. The entries and the
    /// colour slot stay; the period is locked once the account has any entry
    /// (REQ-4, REQ-5).
    ///
    /// # Errors
    /// Returns a validation error, `BankAccountNotFound`,
    /// `BankAccountNameDuplicate` or `BankAccountPeriodLocked`.
    pub fn update_account(
        &mut self,
        id: BankAccountId,
        input: &BankAccountInput,
    ) -> Result<BankAccount, DomainError> {
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

    /// Removes an account and all its entries (REQ-6).
    ///
    /// # Errors
    /// Returns `BankAccountNotFound`.
    pub fn delete_account(&mut self, id: BankAccountId) -> Result<(), DomainError> {
        self.remove_account(id)
    }

    /// Accounts alphabetically by name, ignoring case (REQ-18).
    #[must_use]
    pub fn accounts(&self) -> Vec<&BankAccount> {
        self.sorted_accounts()
    }

    /// How many entries an account has.
    #[must_use]
    pub fn entry_count(&self, id: BankAccountId) -> usize {
        self.record_count(id)
    }

    /// How many entries all accounts have together.
    #[must_use]
    pub fn total_entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Adds the entry of a period. Any period is accepted, a later one
    /// included (REQ-7 to REQ-9).
    ///
    /// # Errors
    /// Returns `BankAccountNotFound`, `BankEntryPeriodMismatch`
    /// or `BankEntryDuplicate`.
    pub fn add_entry(
        &mut self,
        account_id: BankAccountId,
        period: Period,
        balance: BankBalance,
        note: Note,
    ) -> Result<BankEntry, DomainError> {
        self.add_record(BankEntry {
            account_id,
            period,
            balance,
            note,
        })
    }

    /// Changes the balance and note of an entry and, when `new_period` is a
    /// different period, moves it there. The new period follows the rules of
    /// adding an entry: the kind of the account, not taken by another entry of
    /// the account. The edit is applied as a whole or not at all (REQ-12,
    /// REQ-13).
    ///
    /// # Errors
    /// Returns `BankAccountNotFound`, `BankEntryNotFound`,
    /// `BankEntryPeriodMismatch` or `BankEntryDuplicate`.
    pub fn update_entry(
        &mut self,
        account_id: BankAccountId,
        period: Period,
        new_period: Period,
        balance: BankBalance,
        note: Note,
    ) -> Result<BankEntry, DomainError> {
        self.update_record(account_id, period, new_period, |entry| {
            entry.balance = balance;
            entry.note = note;
        })
    }

    /// Removes one entry and nothing else (REQ-14). The caller asks for the
    /// confirmation.
    ///
    /// # Errors
    /// Returns `BankAccountNotFound` or `BankEntryNotFound`.
    pub fn delete_entry(
        &mut self,
        account_id: BankAccountId,
        period: Period,
    ) -> Result<(), DomainError> {
        self.delete_record(account_id, period)
    }

    /// The entries of all accounts, newest period first. A yearly entry counts
    /// as December of its year; within a period the order is that of the
    /// accounts (REQ-18).
    #[must_use]
    pub fn all_entries(&self) -> Vec<(&BankAccount, &BankEntry)> {
        self.all_records()
    }

    /// Checks that data read from a file follows the rules of CSD-009, so a
    /// hand-edited or damaged file cannot break them.
    ///
    /// # Errors
    /// Returns `BankDataInvalid` for the first inconsistency found.
    pub fn validate(&self) -> Result<(), DomainError> {
        SeriesData::validate(self)
    }
}

impl SeriesData for BankAccounts {
    type Account = BankAccount;
    type Record = BankEntry;

    const PIECE_PREFIX: &'static str = "bank";

    fn error(kind: SeriesError) -> DomainError {
        match kind {
            SeriesError::AccountNotFound => DomainError::BankAccountNotFound,
            SeriesError::NameRequired => DomainError::BankAccountNameRequired,
            SeriesError::NameTooLong => DomainError::BankAccountNameTooLong,
            SeriesError::NameDuplicate => DomainError::BankAccountNameDuplicate,
            SeriesError::PeriodUnsupported => DomainError::BankAccountPeriodUnsupported,
            SeriesError::PeriodLocked => DomainError::BankAccountPeriodLocked,
            SeriesError::PeriodMismatch => DomainError::BankEntryPeriodMismatch,
            SeriesError::RecordNotFound => DomainError::BankEntryNotFound,
            SeriesError::RecordDuplicate => DomainError::BankEntryDuplicate,
            SeriesError::DataInvalid => DomainError::BankDataInvalid,
        }
    }

    fn accounts_list(&self) -> &[BankAccount] {
        &self.accounts
    }

    fn accounts_list_mut(&mut self) -> &mut Vec<BankAccount> {
        &mut self.accounts
    }

    fn records_list(&self) -> &[BankEntry] {
        &self.entries
    }

    fn records_list_mut(&mut self) -> &mut Vec<BankEntry> {
        &mut self.entries
    }
}
