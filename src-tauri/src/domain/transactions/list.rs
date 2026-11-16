//! The filtered, ordered list of the Transactions page (CSD-032 REQ-15).

use serde::Serialize;

use crate::domain::money::Money;

use super::category::UNCATEGORIZED_ENTITY;
use super::{Transaction, TransactionFilter, TransactionId, Transactions};

/// The most rows one answer holds, a bound on what the frontend can ask for.
pub const MAX_ROWS: usize = 20_000;

/// The transactions that pass the filter, newest first, cut to the rows asked
/// for, with the count and total of all that pass.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TransactionList {
    /// The first rows of the matches.
    pub rows: Vec<Transaction>,
    /// How many transactions pass the filter, not only the rows returned.
    pub count: usize,
    /// The sum of the amounts of every transaction that passes the filter.
    pub total: Money,
}

/// Whether a transaction passes the filters of the list: the filter on its day
/// and its category (the entity is the category id, or
/// `uncategorized`), and, with `manual_only`, the manual mark (REQ-26).
fn passes(
    transaction: &Transaction,
    filter: Option<&TransactionFilter>,
    manual_only: bool,
) -> bool {
    (!manual_only || transaction.manual)
        && filter.is_none_or(|filter| {
            let entity = transaction
                .category_id
                .map_or_else(|| UNCATEGORIZED_ENTITY.to_owned(), |id| id.to_string());
            filter.matches(transaction.date, &entity)
        })
}

impl Transactions {
    /// The ids of every transaction that passes the same filters as
    /// [`Transactions::list_filtered`], not cut to a page: what "select all in
    /// the filter" selects (REQ-28).
    #[must_use]
    pub fn matching_ids(
        &self,
        filter: Option<&TransactionFilter>,
        manual_only: bool,
    ) -> Vec<TransactionId> {
        self.transactions()
            .iter()
            .filter(|transaction| passes(transaction, filter, manual_only))
            .map(|transaction| transaction.id)
            .collect()
    }

    /// The transactions that pass `filter`, newest date first and, among equal
    /// dates, the one added last first, cut to `limit` rows. `count` and
    /// `total` cover every match (REQ-15). The entity of a transaction is its
    /// category id, or `uncategorized` (CSD-028 REQ-4). With `manual_only`, only
    /// the transactions whose category was set by hand pass (REQ-26).
    #[must_use]
    pub fn list_filtered(
        &self,
        filter: Option<&TransactionFilter>,
        manual_only: bool,
        limit: usize,
    ) -> TransactionList {
        let mut matches: Vec<(usize, &Transaction)> = self
            .transactions()
            .iter()
            .enumerate()
            .filter(|(_, transaction)| passes(transaction, filter, manual_only))
            .collect();
        matches.sort_by(|(left_index, left), (right_index, right)| {
            right.date.cmp(&left.date).then(right_index.cmp(left_index))
        });
        let total =
            Money::saturating_sum(matches.iter().map(|(_, transaction)| transaction.amount));
        let count = matches.len();
        let rows = matches
            .into_iter()
            .take(limit.min(MAX_ROWS))
            .map(|(_, transaction)| transaction.clone())
            .collect();
        TransactionList { rows, count, total }
    }
}
