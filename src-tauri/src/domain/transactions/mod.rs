//! Bank transactions, their categories and the rules that categorise them
//! (CSD-032).
//!
//! `Transactions` holds the data and the rules for changing it. The list of
//! transactions sits behind an `Arc`, so the undo history shares it between
//! steps and a step pays only for a list it changes (CSD-032 CON-4).

mod bulk;
mod categories;
mod category;
mod filter;
mod import;
mod insights;
mod list;
mod rules;
mod suggest;
mod summary;
mod totals;
mod transaction;

#[cfg(test)]
mod tests;

use std::collections::HashSet;
use std::sync::Arc;

use serde::Deserialize;

use crate::domain::error::DomainError;
use crate::domain::money::Currency;

pub use category::{Category, CategoryId, Rule, RuleId, RulePeriod, UNCATEGORIZED_ENTITY};
pub use filter::{TransactionFilter, TransactionFilterInput};
pub use insights::{Granularity, TransactionInsights};
pub use list::TransactionList;
pub use rules::RuleDraft;
pub use suggest::RuleSuggestions;
pub use summary::CategorisationSummary;
pub use totals::TransactionYearlyTotal;
pub use transaction::{Transaction, TransactionDate, TransactionId};

use category::{defaults, fields_of, rule_text, valid_name, CompiledRules};

/// What an import did (CSD-032 REQ-1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ImportSummary {
    /// How many transactions were added.
    pub added: usize,
    /// How many rows were skipped as duplicates.
    pub skipped: usize,
}

/// Which transactions a re-apply of the rules looks at (CSD-032 REQ-12).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReapplyScope {
    /// Every transaction whose category was not set by hand.
    AllNonManual,
    /// Every transaction, also those whose category was set by hand; a hand-set
    /// category the rules change loses its manual flag.
    All,
}

/// Rough size of one stored transaction, for the memory budget of the undo
/// history (CSD-032 REQ-19).
const BYTES_PER_TRANSACTION: usize = 500;

/// All transactions, the categories and the rules, as in the project file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transactions {
    list: Arc<Vec<Transaction>>,
    categories: Vec<Category>,
    rules: Vec<Rule>,
}

impl Default for Transactions {
    /// A new project: no transactions, the default categories and rules
    /// (CSD-032 REQ-8, REQ-9).
    fn default() -> Self {
        let (categories, rules) = defaults();
        Self {
            list: Arc::new(Vec::new()),
            categories,
            rules,
        }
    }
}

impl Transactions {
    /// Builds the data from the three sections of a project file and checks
    /// them (CSD-032 REQ-17).
    ///
    /// # Errors
    /// Returns `TransactionDataInvalid` for the first inconsistency.
    pub fn from_parts(
        list: Vec<Transaction>,
        categories: Vec<Category>,
        rules: Vec<Rule>,
    ) -> Result<Self, DomainError> {
        let data = Self {
            list: Arc::new(list),
            categories,
            rules,
        };
        data.validate()?;
        Ok(data)
    }

    fn validate(&self) -> Result<(), DomainError> {
        let invalid = DomainError::TransactionDataInvalid;
        let mut category_ids = HashSet::new();
        let mut names = HashSet::new();
        for category in &self.categories {
            let name = category.name.trim();
            if valid_name(&[], None, name).is_err()
                || name != category.name
                || !category_ids.insert(category.id)
                || !names.insert(name.to_lowercase())
            {
                return Err(invalid);
            }
        }
        let mut rule_ids = HashSet::new();
        for rule in &self.rules {
            if rule_text(&rule.text, rule.period().is_set()).is_err()
                || rule.text != rule.text.trim()
                || rule.from.zip(rule.to).is_some_and(|(from, to)| from > to)
                || !rule_ids.insert(rule.id)
                || !category_ids.contains(&rule.category_id)
            {
                return Err(invalid);
            }
        }
        let mut transaction_ids = HashSet::new();
        for transaction in self.list.iter() {
            if !transaction.is_consistent()
                || !transaction_ids.insert(transaction.id)
                || transaction
                    .category_id
                    .is_some_and(|id| !category_ids.contains(&id))
            {
                return Err(invalid);
            }
        }
        Ok(())
    }

    /// The transactions in the order they were added.
    #[must_use]
    pub fn transactions(&self) -> &[Transaction] {
        &self.list
    }

    /// The categories, in the order they were added. "Uncategorized" is not
    /// among them.
    #[must_use]
    pub fn categories(&self) -> &[Category] {
        &self.categories
    }

    /// The rules, in the order they are applied.
    #[must_use]
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// How many transactions there are.
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.list.len()
    }

    /// The identity of the shared list and a rough size of it, so the undo
    /// history counts a list that many steps share only once (CSD-032 REQ-19).
    #[must_use]
    pub fn shared_weight(&self) -> (usize, usize) {
        let identity = Arc::as_ptr(&self.list).addr();
        (
            identity,
            self.list.len().saturating_mul(BYTES_PER_TRANSACTION),
        )
    }

    /// Adds the transactions of a Knab export that are not duplicates, each
    /// with the category of the first matching rule. All or nothing: a bad row
    /// or a currency that is not `currency` adds nothing (REQ-1 to REQ-5).
    ///
    /// # Errors
    /// Returns the error of the first offending line (REQ-3, REQ-4).
    pub fn import(&mut self, text: &str, currency: Currency) -> Result<ImportSummary, DomainError> {
        let rows = import::parse(text, currency)?;
        let mut seen: HashSet<(String, String)> = self
            .list
            .iter()
            .map(|transaction| {
                (
                    transaction.own_account.clone(),
                    transaction.reference.clone(),
                )
            })
            .collect();
        let compiled = CompiledRules::new(&self.rules);
        let mut added = Vec::new();
        let mut skipped = 0;
        for row in rows {
            if !seen.insert((row.own_account.clone(), row.reference.clone())) {
                skipped += 1;
                continue;
            }
            added.push(Transaction {
                id: TransactionId::new(),
                date: row.date,
                amount: row.amount,
                category_id: compiled.category_of(
                    row.date,
                    &fields_of(&row.counterparty_name, &row.description),
                ),
                counterparty_name: row.counterparty_name,
                counterparty_iban: row.counterparty_iban,
                description: row.description,
                own_account: row.own_account,
                reference: row.reference,
                payment_method: row.payment_method,
                manual: false,
            });
        }
        let summary = ImportSummary {
            added: added.len(),
            skipped,
        };
        if !added.is_empty() {
            Arc::make_mut(&mut self.list).extend(added);
        }
        Ok(summary)
    }

    /// Sets the category of a transaction by hand, `None` for Uncategorized,
    /// and marks it manual (REQ-11).
    ///
    /// # Errors
    /// Returns `TransactionNotFound` or `TransactionCategoryNotFound`.
    pub fn set_category(
        &mut self,
        id: TransactionId,
        category: Option<CategoryId>,
    ) -> Result<(), DomainError> {
        if category.is_some_and(|category| !self.has_category(category)) {
            return Err(DomainError::TransactionCategoryNotFound);
        }
        let list = Arc::make_mut(&mut self.list);
        let transaction = list
            .iter_mut()
            .find(|transaction| transaction.id == id)
            .ok_or(DomainError::TransactionNotFound)?;
        transaction.category_id = category;
        transaction.manual = true;
        Ok(())
    }

    /// Takes the manual mark off a transaction and leaves its category as it
    /// is, so a later re-apply may change it (REQ-11).
    ///
    /// # Errors
    /// Returns `TransactionNotFound`.
    pub fn unset_manual(&mut self, id: TransactionId) -> Result<(), DomainError> {
        let transaction = Arc::make_mut(&mut self.list)
            .iter_mut()
            .find(|transaction| transaction.id == id)
            .ok_or(DomainError::TransactionNotFound)?;
        transaction.manual = false;
        Ok(())
    }

    /// Removes one transaction and nothing else (REQ-14).
    ///
    /// # Errors
    /// Returns `TransactionNotFound`.
    pub fn delete_transaction(&mut self, id: TransactionId) -> Result<(), DomainError> {
        let position = self
            .list
            .iter()
            .position(|transaction| transaction.id == id)
            .ok_or(DomainError::TransactionNotFound)?;
        Arc::make_mut(&mut self.list).remove(position);
        Ok(())
    }

    /// Removes every transaction and nothing else: the categories and the
    /// rules stay (REQ-25). Returns how many were removed.
    pub fn delete_all_transactions(&mut self) -> usize {
        let count = self.list.len();
        self.list = Arc::new(Vec::new());
        count
    }
}
