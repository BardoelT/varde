//! Bank transactions, their categories and the rules that categorise them
//! (CSD-032).
//!
//! `Transactions` holds the data and the rules for changing it. The list of
//! transactions sits behind an `Arc`, so the undo history shares it between
//! steps and a step pays only for a list it changes (CSD-032 CON-4).

mod category;
mod import;
mod list;
mod totals;
mod transaction;

#[cfg(test)]
mod tests;

use std::collections::HashSet;
use std::sync::Arc;

use serde::Deserialize;

use crate::domain::error::DomainError;
use crate::domain::money::Currency;

pub use category::{Category, CategoryId, Rule, RuleId, UNCATEGORIZED_ENTITY};
pub use list::TransactionList;
pub use totals::TransactionYearlyTotal;
pub use transaction::{Transaction, TransactionId};

use category::{categorise, defaults, valid_name, valid_rule_text};

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
    /// Only transactions without a category.
    Uncategorized,
    /// Every transaction whose category was not set by hand.
    AllNonManual,
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
            if valid_rule_text(&rule.text).is_err()
                || rule.text != rule.text.trim()
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
                category_id: categorise(&self.rules, &row.counterparty_name, &row.description),
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

    fn has_category(&self, id: CategoryId) -> bool {
        self.categories.iter().any(|category| category.id == id)
    }

    /// Adds a category (REQ-8).
    ///
    /// # Errors
    /// Returns the violated name rule.
    pub fn add_category(&mut self, name: &str) -> Result<Category, DomainError> {
        let name = valid_name(&self.categories, None, name)?;
        let category = Category {
            id: CategoryId::new(),
            name,
        };
        self.categories.push(category.clone());
        Ok(category)
    }

    /// Renames a category (REQ-8).
    ///
    /// # Errors
    /// Returns `TransactionCategoryNotFound` or the violated name rule.
    pub fn rename_category(&mut self, id: CategoryId, name: &str) -> Result<Category, DomainError> {
        if !self.has_category(id) {
            return Err(DomainError::TransactionCategoryNotFound);
        }
        let name = valid_name(&self.categories, Some(id), name)?;
        let category = self
            .categories
            .iter_mut()
            .find(|category| category.id == id)
            .ok_or(DomainError::TransactionCategoryNotFound)?;
        category.name = name;
        Ok(category.clone())
    }

    /// Deletes a category: its transactions become Uncategorized, keeping
    /// their manual flag, and the rules that name it are deleted (REQ-13).
    ///
    /// # Errors
    /// Returns `TransactionCategoryNotFound`.
    pub fn delete_category(&mut self, id: CategoryId) -> Result<(), DomainError> {
        if !self.has_category(id) {
            return Err(DomainError::TransactionCategoryNotFound);
        }
        self.categories.retain(|category| category.id != id);
        self.rules.retain(|rule| rule.category_id != id);
        if self
            .list
            .iter()
            .any(|transaction| transaction.category_id == Some(id))
        {
            for transaction in Arc::make_mut(&mut self.list).iter_mut() {
                if transaction.category_id == Some(id) {
                    transaction.category_id = None;
                }
            }
        }
        Ok(())
    }

    fn rule_category(&self, id: CategoryId) -> Result<CategoryId, DomainError> {
        if self.has_category(id) {
            Ok(id)
        } else {
            Err(DomainError::TransactionRuleCategoryInvalid)
        }
    }

    /// Adds a rule at the end of the list (REQ-9).
    ///
    /// # Errors
    /// Returns the violated text or category rule.
    pub fn add_rule(&mut self, text: &str, category: CategoryId) -> Result<Rule, DomainError> {
        let text = valid_rule_text(text)?;
        let category_id = self.rule_category(category)?;
        let rule = Rule {
            id: RuleId::new(),
            text,
            category_id,
        };
        self.rules.push(rule.clone());
        Ok(rule)
    }

    /// Adds a rule at the end of the list together with a new category it
    /// names, as one change: when the text or the name is invalid, neither is
    /// added (REQ-9).
    ///
    /// # Errors
    /// Returns the violated text rule, or the violated name rule.
    pub fn add_rule_with_new_category(
        &mut self,
        text: &str,
        name: &str,
    ) -> Result<(Category, Rule), DomainError> {
        let text = valid_rule_text(text)?;
        let category = self.add_category(name)?;
        let rule = Rule {
            id: RuleId::new(),
            text,
            category_id: category.id,
        };
        self.rules.push(rule.clone());
        Ok((category, rule))
    }

    /// Changes the text and category of a rule (REQ-9).
    ///
    /// # Errors
    /// Returns `TransactionRuleNotFound` or the violated text or category rule.
    pub fn edit_rule(
        &mut self,
        id: RuleId,
        text: &str,
        category: CategoryId,
    ) -> Result<Rule, DomainError> {
        let text = valid_rule_text(text)?;
        let category_id = self.rule_category(category)?;
        let rule = self
            .rules
            .iter_mut()
            .find(|rule| rule.id == id)
            .ok_or(DomainError::TransactionRuleNotFound)?;
        rule.text = text;
        rule.category_id = category_id;
        Ok(rule.clone())
    }

    /// Moves a rule to `position`, counted from 0 at the top; a position past
    /// the end puts it last (REQ-9).
    ///
    /// # Errors
    /// Returns `TransactionRuleNotFound`.
    pub fn move_rule(&mut self, id: RuleId, position: usize) -> Result<(), DomainError> {
        let from = self
            .rules
            .iter()
            .position(|rule| rule.id == id)
            .ok_or(DomainError::TransactionRuleNotFound)?;
        let rule = self.rules.remove(from);
        let to = position.min(self.rules.len());
        self.rules.insert(to, rule);
        Ok(())
    }

    /// Deletes a rule (REQ-9).
    ///
    /// # Errors
    /// Returns `TransactionRuleNotFound`.
    pub fn delete_rule(&mut self, id: RuleId) -> Result<(), DomainError> {
        let before = self.rules.len();
        self.rules.retain(|rule| rule.id != id);
        if self.rules.len() == before {
            return Err(DomainError::TransactionRuleNotFound);
        }
        Ok(())
    }

    /// Runs the rules again on the transactions of `scope` and returns how many
    /// changed category. Manual transactions are never touched (REQ-12).
    pub fn reapply_rules(&mut self, scope: ReapplyScope) -> usize {
        let changes: Vec<(usize, Option<CategoryId>)> = self
            .list
            .iter()
            .enumerate()
            .filter(|(_, transaction)| {
                !transaction.manual
                    && (scope == ReapplyScope::AllNonManual || transaction.category_id.is_none())
            })
            .filter_map(|(index, transaction)| {
                let found = categorise(
                    &self.rules,
                    &transaction.counterparty_name,
                    &transaction.description,
                );
                (found != transaction.category_id).then_some((index, found))
            })
            .collect();
        if changes.is_empty() {
            return 0;
        }
        let list = Arc::make_mut(&mut self.list);
        let count = changes.len();
        for (index, category) in changes {
            if let Some(transaction) = list.get_mut(index) {
                transaction.category_id = category;
            }
        }
        count
    }
}
