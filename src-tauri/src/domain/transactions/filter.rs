//! The filter of the Transactions list (CSD-032 REQ-15): a range of days, the
//! categories to keep, a range of amounts and text in the counterparty and the
//! description. The other statement tables filter on months (CSD-028); only
//! this one goes down to the day and filters on the amount and the text.

use serde::Deserialize;

use crate::domain::money::Money;
use crate::domain::NumberFormat;

use super::transaction::TransactionDate;
use super::Transaction;

/// The filter as the frontend sends it: the amounts are the text the user
/// typed, in the number format of the app (CSD-029 REQ-6).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct TransactionFilterInput {
    #[serde(default)]
    from: Option<TransactionDate>,
    #[serde(default)]
    to: Option<TransactionDate>,
    #[serde(default)]
    ids: Vec<String>,
    #[serde(default)]
    min_amount: Option<String>,
    #[serde(default)]
    max_amount: Option<String>,
    #[serde(default)]
    counterparty: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

/// The text to look for, lower-cased; `None` for an empty (or blank) field.
fn typed_text(text: Option<&str>) -> Option<String> {
    let trimmed = text?.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_lowercase())
}

/// An amount typed in `format`, rounded as every typed amount is (CSD-000
/// CON-8); `None` for an empty field or text that is not an amount, which
/// leaves that end of the range open.
fn typed_bound(format: NumberFormat, text: Option<&str>) -> Option<Money> {
    let canonical = format.typed_to_canonical(text?.trim())?;
    Money::parse(&canonical).ok()
}

impl TransactionFilterInput {
    /// Reads the typed amounts in `format`.
    #[must_use]
    pub fn resolve(self, format: NumberFormat) -> TransactionFilter {
        TransactionFilter {
            from: self.from,
            to: self.to,
            ids: self.ids,
            min_amount: typed_bound(format, self.min_amount.as_deref()),
            max_amount: typed_bound(format, self.max_amount.as_deref()),
            counterparty: typed_text(self.counterparty.as_deref()),
            description: typed_text(self.description.as_deref()),
        }
    }
}

/// What the user filtered the transactions on. It only decides which rows are
/// shown; it never changes project data.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TransactionFilter {
    /// First day to show, itself included; no lower bound when `None`.
    from: Option<TransactionDate>,
    /// Last day to show, itself included; no upper bound when `None`.
    to: Option<TransactionDate>,
    /// The categories to keep, as the text of their id or `uncategorized`.
    /// Empty means every category (CSD-028 REQ-4).
    ids: Vec<String>,
    /// The lowest amount to show, itself included; a debit is negative.
    min_amount: Option<Money>,
    /// The highest amount to show, itself included.
    max_amount: Option<Money>,
    /// Text the counterparty name must contain, lower-cased; any when `None`.
    counterparty: Option<String>,
    /// Text the description must contain, lower-cased; any when `None`.
    description: Option<String>,
}

impl TransactionFilter {
    /// Whether `transaction`, in the category `entity`, passes. The text
    /// filters look for the typed text anywhere, ignoring case.
    #[must_use]
    pub fn matches(&self, transaction: &Transaction, entity: &str) -> bool {
        let contains = |wanted: &Option<String>, text: &str| {
            wanted
                .as_ref()
                .is_none_or(|wanted| text.to_lowercase().contains(wanted.as_str()))
        };
        self.from.is_none_or(|from| from <= transaction.date)
            && self.to.is_none_or(|to| transaction.date <= to)
            && (self.ids.is_empty() || self.ids.iter().any(|id| id == entity))
            && self.min_amount.is_none_or(|min| min <= transaction.amount)
            && self.max_amount.is_none_or(|max| transaction.amount <= max)
            && contains(&self.counterparty, &transaction.counterparty_name)
            && contains(&self.description, &transaction.description)
    }
}
