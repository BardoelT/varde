//! The categories of the transactions (CSD-032 REQ-8, REQ-13, REQ-30).

use std::sync::Arc;

use crate::domain::error::DomainError;

use super::category::valid_name;
use super::{Category, CategoryId, Transactions};

impl Transactions {
    pub(super) fn has_category(&self, id: CategoryId) -> bool {
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

    /// Every category, in order, with the number of transactions in it
    /// (REQ-30); what the category list shows.
    #[must_use]
    pub fn category_rows(&self) -> Vec<(&Category, usize)> {
        self.categories.iter().zip(self.category_counts()).collect()
    }

    /// For every category, in the order of [`Transactions::categories`], how
    /// many transactions are in it, whoever set the category (REQ-30).
    #[must_use]
    pub fn category_counts(&self) -> Vec<usize> {
        self.categories
            .iter()
            .map(|category| {
                self.list
                    .iter()
                    .filter(|transaction| transaction.category_id == Some(category.id))
                    .count()
            })
            .collect()
    }
}
