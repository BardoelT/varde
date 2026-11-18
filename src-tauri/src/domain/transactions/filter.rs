//! The filter of the Transactions list (CSD-032 REQ-15): a range of days, the
//! categories to keep, a range of amounts and text in the counterparty and the
//! description. The other statement tables filter on months (CSD-028); only
//! this one goes down to the day and filters on the amount and the text.

use std::collections::HashSet;

use serde::Deserialize;

use crate::domain::money::Money;
use crate::domain::NumberFormat;

use super::category::{CategoryId, Pattern, UNCATEGORIZED_ENTITY};
use super::transaction::TransactionDate;
use super::AccountId;
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
    account_ids: Vec<String>,
    #[serde(default)]
    min_amount: Option<String>,
    #[serde(default)]
    max_amount: Option<String>,
    #[serde(default)]
    counterparty: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

/// The most characters of a filter text that count, as long as a rule text;
/// more is cut, a bound on untrusted input (CSD-032 CON-9).
const MAX_FILTER_TEXT_CHARS: usize = super::category::MAX_RULE_CHARS;
/// The most category ids of a filter that count; more are left out.
const MAX_FILTER_IDS: usize = 1000;

/// The pattern to look for, cut to [`MAX_FILTER_TEXT_CHARS`] characters;
/// `None` for an empty (or blank) field.
fn typed_text(text: Option<&str>) -> Option<Pattern> {
    let trimmed = text?.trim();
    (!trimmed.is_empty()).then(|| {
        Pattern::new(
            &trimmed
                .chars()
                .take(MAX_FILTER_TEXT_CHARS)
                .collect::<String>(),
        )
    })
}

/// What a category choice of the filter names: the built-in Uncategorized or
/// one category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Entity {
    Uncategorized,
    Category(CategoryId),
}

impl Entity {
    /// Reads the text the frontend sends; `None` for one that names nothing.
    fn parse(text: &str) -> Option<Self> {
        if text == UNCATEGORIZED_ENTITY {
            Some(Self::Uncategorized)
        } else {
            CategoryId::parse(text).ok().map(Self::Category)
        }
    }
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
            // An empty list keeps every category; a list that names nothing known keeps none.
            entities: (!self.ids.is_empty()).then(|| {
                self.ids
                    .iter()
                    .take(MAX_FILTER_IDS)
                    .filter_map(|id| Entity::parse(id))
                    .collect()
            }),
            min_amount: typed_bound(format, self.min_amount.as_deref()),
            max_amount: typed_bound(format, self.max_amount.as_deref()),
            // The same for the accounts (CSD-034 REQ-6).
            accounts: (!self.account_ids.is_empty()).then(|| {
                self.account_ids
                    .iter()
                    .take(MAX_FILTER_IDS)
                    .filter_map(|id| AccountId::parse(id).ok())
                    .collect()
            }),
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
    /// The categories to keep; `None` means every category (CSD-028 REQ-4).
    entities: Option<HashSet<Entity>>,
    /// The accounts to keep; `None` means every account (CSD-034 REQ-6).
    accounts: Option<HashSet<AccountId>>,
    /// The lowest amount to show, itself included; a debit is negative.
    min_amount: Option<Money>,
    /// The highest amount to show, itself included.
    max_amount: Option<Money>,
    /// The pattern the whole counterparty name must fit; any when `None`. `*`
    /// stands for any characters, so without one it is exact.
    counterparty: Option<Pattern>,
    /// The pattern the whole description must fit; any when `None`.
    description: Option<Pattern>,
}

impl TransactionFilter {
    /// Whether `transaction` passes. The text
    /// filters match the whole counterparty or description, ignoring case:
    /// `ca` is exactly "ca", `ca*` starts with it, `*ca` ends with it and
    /// `*ca*` contains it.
    #[must_use]
    pub fn matches(&self, transaction: &Transaction) -> bool {
        let contains = |wanted: &Option<Pattern>, text: &str| {
            wanted
                .as_ref()
                .is_none_or(|pattern| pattern.matches(text.trim().to_lowercase().as_str()))
        };
        let entity = transaction
            .category_id
            .map_or(Entity::Uncategorized, Entity::Category);
        self.from.is_none_or(|from| from <= transaction.date)
            && self.to.is_none_or(|to| transaction.date <= to)
            && self
                .entities
                .as_ref()
                .is_none_or(|kept| kept.contains(&entity))
            && self
                .accounts
                .as_ref()
                .is_none_or(|kept| kept.contains(&transaction.account_id))
            && self.min_amount.is_none_or(|min| min <= transaction.amount)
            && self.max_amount.is_none_or(|max| transaction.amount <= max)
            && contains(&self.counterparty, &transaction.counterparty_name)
            && contains(&self.description, &transaction.description)
    }
}
