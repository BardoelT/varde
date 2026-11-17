//! Changes to several transactions at once, all or nothing (CSD-032 REQ-28).

use std::collections::HashSet;
use std::sync::Arc;

use crate::domain::error::DomainError;

use super::{CategoryId, TransactionId, Transactions};

impl Transactions {
    /// The positions in the list of the transactions named by `ids`, which
    /// all have to exist.
    fn positions_of(&self, ids: &[TransactionId]) -> Result<HashSet<usize>, DomainError> {
        let wanted: HashSet<TransactionId> = ids.iter().copied().collect();
        let positions: HashSet<usize> = self
            .list
            .iter()
            .enumerate()
            .filter(|(_, transaction)| wanted.contains(&transaction.id))
            .map(|(position, _)| position)
            .collect();
        if positions.len() == wanted.len() {
            Ok(positions)
        } else {
            Err(DomainError::TransactionNotFound)
        }
    }

    /// Sets the category of every transaction in `ids` by hand, like
    /// [`Transactions::set_category`], as one change: when one id is unknown or
    /// the category does not exist, nothing changes. Returns how many were
    /// set (REQ-28).
    ///
    /// # Errors
    /// Returns `TransactionNotFound` or `TransactionCategoryNotFound`.
    pub fn set_category_of_many(
        &mut self,
        ids: &[TransactionId],
        category: Option<CategoryId>,
    ) -> Result<usize, DomainError> {
        if category.is_some_and(|category| !self.has_category(category)) {
            return Err(DomainError::TransactionCategoryNotFound);
        }
        let positions = self.positions_of(ids)?;
        if positions.is_empty() {
            return Ok(0);
        }
        let list = Arc::make_mut(&mut self.list);
        for position in &positions {
            if let Some(transaction) = list.get_mut(*position) {
                transaction.category_id = category;
                transaction.manual = true;
            }
        }
        Ok(positions.len())
    }

    /// Removes every transaction in `ids` and nothing else, as one change:
    /// when one id is unknown, nothing is removed. Returns how many were
    /// removed (REQ-28).
    ///
    /// # Errors
    /// Returns `TransactionNotFound`.
    pub fn delete_many(&mut self, ids: &[TransactionId]) -> Result<usize, DomainError> {
        let positions = self.positions_of(ids)?;
        if positions.is_empty() {
            return Ok(0);
        }
        let mut position = 0;
        Arc::make_mut(&mut self.list).retain(|_| {
            let keep = !positions.contains(&position);
            position += 1;
            keep
        });
        Ok(positions.len())
    }
}
