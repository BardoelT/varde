//! The filter of the Transactions list (CSD-032 REQ-15): a range of days and
//! the categories to keep. The other statement tables filter on months
//! (CSD-028); only this one goes down to the day.

use serde::Deserialize;

use super::transaction::TransactionDate;

/// What the user filtered the transactions on. It only decides which rows are
/// shown; it never changes project data.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct TransactionFilter {
    /// First day to show, itself included; no lower bound when `None`.
    #[serde(default)]
    from: Option<TransactionDate>,
    /// Last day to show, itself included; no upper bound when `None`.
    #[serde(default)]
    to: Option<TransactionDate>,
    /// The categories to keep, as the text of their id or `uncategorized`.
    /// Empty means every category (CSD-028 REQ-4).
    #[serde(default)]
    ids: Vec<String>,
}

impl TransactionFilter {
    /// Whether a transaction on `date` in the category `entity` passes.
    #[must_use]
    pub fn matches(&self, date: TransactionDate, entity: &str) -> bool {
        self.from.is_none_or(|from| from <= date)
            && self.to.is_none_or(|to| date <= to)
            && (self.ids.is_empty() || self.ids.iter().any(|id| id == entity))
    }
}
