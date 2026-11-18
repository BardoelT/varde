//! The accounts the transactions belong to (CSD-034). An account is a name the
//! user chooses; every transaction belongs to exactly one.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::domain::common::{Id, IdKind};
use crate::domain::error::DomainError;

use super::Transactions;

/// Identifies an account.
pub type AccountId = Id<Account>;

impl IdKind for Account {
    fn not_found() -> DomainError {
        DomainError::TransactionAccountNotFound
    }
}

/// The longest account name, in characters.
pub const MAX_ACCOUNT_CHARS: usize = 50;

/// A named place the transactions of one bank are kept (CSD-034 REQ-2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Account {
    /// Identifier of the account.
    pub id: AccountId,
    /// The name, unique ignoring case.
    pub name: String,
}

/// Checks an account name typed by the user and returns it trimmed. `own` is
/// the account being renamed, which may keep its own name (ignoring case).
pub(super) fn valid_account_name(
    accounts: &[Account],
    own: Option<AccountId>,
    typed: &str,
) -> Result<String, DomainError> {
    let name = typed.trim();
    if name.is_empty() {
        return Err(DomainError::TransactionAccountNameRequired);
    }
    if name.chars().count() > MAX_ACCOUNT_CHARS {
        return Err(DomainError::TransactionAccountNameTooLong);
    }
    let lower = name.to_lowercase();
    if accounts
        .iter()
        .any(|account| Some(account.id) != own && account.name.to_lowercase() == lower)
    {
        return Err(DomainError::TransactionAccountNameTaken);
    }
    Ok(name.to_owned())
}

impl Transactions {
    /// The accounts, in the order they were added.
    #[must_use]
    pub fn accounts(&self) -> &[Account] {
        &self.accounts
    }

    pub(super) fn has_account(&self, id: AccountId) -> bool {
        self.accounts.iter().any(|account| account.id == id)
    }

    /// Adds an account at the end of the list (CSD-034 REQ-2).
    ///
    /// # Errors
    /// Returns the violated name rule.
    pub fn add_account(&mut self, name: &str) -> Result<Account, DomainError> {
        let name = valid_account_name(&self.accounts, None, name)?;
        let account = Account {
            id: AccountId::new(),
            name,
        };
        self.accounts.push(account.clone());
        Ok(account)
    }

    /// Renames an account (CSD-034 REQ-2).
    ///
    /// # Errors
    /// Returns `TransactionAccountNotFound` or the violated name rule.
    pub fn rename_account(&mut self, id: AccountId, name: &str) -> Result<Account, DomainError> {
        let name = valid_account_name(&self.accounts, Some(id), name)?;
        let account = self
            .accounts
            .iter_mut()
            .find(|account| account.id == id)
            .ok_or(DomainError::TransactionAccountNotFound)?;
        account.name = name;
        Ok(account.clone())
    }

    /// Deletes an account together with all its transactions and returns how
    /// many transactions went (CSD-034 REQ-2).
    ///
    /// # Errors
    /// Returns `TransactionAccountNotFound`.
    pub fn delete_account(&mut self, id: AccountId) -> Result<usize, DomainError> {
        let deleted = self.delete_account_transactions(id)?;
        self.accounts.retain(|account| account.id != id);
        Ok(deleted)
    }

    /// Deletes every transaction of an account and keeps the account; returns
    /// how many were deleted (CSD-034 REQ-4).
    ///
    /// # Errors
    /// Returns `TransactionAccountNotFound`.
    pub fn delete_account_transactions(&mut self, id: AccountId) -> Result<usize, DomainError> {
        if !self.has_account(id) {
            return Err(DomainError::TransactionAccountNotFound);
        }
        let before = self.list.len();
        if self
            .list
            .iter()
            .any(|transaction| transaction.account_id == id)
        {
            Arc::make_mut(&mut self.list).retain(|transaction| transaction.account_id != id);
        }
        Ok(before.saturating_sub(self.list.len()))
    }

    /// Every account, in order, with the number of transactions in it
    /// (CSD-034 REQ-1).
    #[must_use]
    pub fn account_rows(&self) -> Vec<(&Account, usize)> {
        self.accounts
            .iter()
            .map(|account| {
                let count = self
                    .list
                    .iter()
                    .filter(|transaction| transaction.account_id == account.id)
                    .count();
                (account, count)
            })
            .collect()
    }
}
