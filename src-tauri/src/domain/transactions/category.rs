//! Categories and categorisation rules (CSD-032 REQ-8 to REQ-13).

use serde::{Deserialize, Serialize};

use crate::domain::common::{Id, IdKind};
use crate::domain::error::DomainError;

/// Identifies a category.
pub type CategoryId = Id<Category>;
/// Identifies a rule.
pub type RuleId = Id<Rule>;

impl IdKind for Category {
    fn not_found() -> DomainError {
        DomainError::TransactionCategoryNotFound
    }
}

impl IdKind for Rule {
    fn not_found() -> DomainError {
        DomainError::TransactionRuleNotFound
    }
}

/// The longest category name, in characters.
pub const MAX_CATEGORY_CHARS: usize = 50;
/// The longest rule text, in characters.
pub const MAX_RULE_CHARS: usize = 100;
/// The name of the built-in category that holds what no rule matched. It is
/// not stored: a transaction without a category is in it.
const UNCATEGORIZED: &str = "Uncategorized";
/// How a filter and the frontend name the built-in category.
pub const UNCATEGORIZED_ENTITY: &str = "uncategorized";

/// A user-defined category.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Category {
    /// Identifier of the category.
    pub id: CategoryId,
    /// The name, unique ignoring case.
    pub name: String,
}

/// A text a transaction is matched against, and the category it gets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    /// Identifier of the rule.
    pub id: RuleId,
    /// The text to look for, ignoring case.
    pub text: String,
    /// The category a match gets.
    pub category_id: CategoryId,
}

/// The categories a new project holds (CSD-032 REQ-8).
const DEFAULT_CATEGORIES: [&str; 14] = [
    "Salary",
    "Groceries",
    "Housing",
    "Utilities",
    "Insurance",
    "Transport",
    "Eating out",
    "Shopping",
    "Health",
    "Leisure",
    "Subscriptions",
    "Savings & transfers",
    "Taxes",
    "Other",
];

/// The rules a new project holds: well-known chains and their categories
/// (CSD-032 REQ-9).
const DEFAULT_RULES: [(&str, &str); 15] = [
    ("Albert Heijn", "Groceries"),
    ("Jumbo", "Groceries"),
    ("Lidl", "Groceries"),
    ("Aldi", "Groceries"),
    ("Dirk van den Broek", "Groceries"),
    ("HEMA", "Shopping"),
    ("bol.com", "Shopping"),
    ("Coolblue", "Shopping"),
    ("Kruidvat", "Health"),
    ("NS Reizigers", "Transport"),
    ("Shell", "Transport"),
    ("Thuisbezorgd", "Eating out"),
    ("Netflix", "Subscriptions"),
    ("Spotify", "Subscriptions"),
    ("Zilveren Kruis", "Insurance"),
];

/// The ids of the default categories and rules count up from these, so every
/// new project holds the same ones.
const DEFAULT_CATEGORY_IDS: u128 = 0x7661_7264_6500_0001_0000_0000_0000_0000;
const DEFAULT_RULE_IDS: u128 = 0x7661_7264_6500_0002_0000_0000_0000_0000;

/// The default categories and rules, with fixed ids.
pub(super) fn defaults() -> (Vec<Category>, Vec<Rule>) {
    let categories: Vec<Category> = (DEFAULT_CATEGORY_IDS..)
        .zip(DEFAULT_CATEGORIES)
        .map(|(number, name)| Category {
            id: CategoryId::from_number(number),
            name: name.to_owned(),
        })
        .collect();
    let rules = (DEFAULT_RULE_IDS..)
        .zip(DEFAULT_RULES)
        .filter_map(|(number, (text, category))| {
            categories
                .iter()
                .find(|candidate| candidate.name == category)
                .map(|found| Rule {
                    id: RuleId::from_number(number),
                    text: (*text).to_owned(),
                    category_id: found.id,
                })
        })
        .collect();
    (categories, rules)
}

/// Checks a category name typed by the user and returns it trimmed. `own` is
/// the category being renamed, which may keep its own name.
pub(super) fn valid_name(
    categories: &[Category],
    own: Option<CategoryId>,
    typed: &str,
) -> Result<String, DomainError> {
    let name = typed.trim();
    if name.is_empty() {
        return Err(DomainError::TransactionCategoryNameRequired);
    }
    if name.chars().count() > MAX_CATEGORY_CHARS {
        return Err(DomainError::TransactionCategoryNameTooLong);
    }
    let lower = name.to_lowercase();
    let taken = lower == UNCATEGORIZED.to_lowercase()
        || categories
            .iter()
            .any(|category| Some(category.id) != own && category.name.to_lowercase() == lower);
    if taken {
        return Err(DomainError::TransactionCategoryNameTaken);
    }
    Ok(name.to_owned())
}

/// Checks a rule text typed by the user and returns it trimmed.
pub(super) fn valid_rule_text(typed: &str) -> Result<String, DomainError> {
    let text = typed.trim();
    if text.is_empty() {
        return Err(DomainError::TransactionRuleTextRequired);
    }
    if text.chars().count() > MAX_RULE_CHARS {
        return Err(DomainError::TransactionRuleTextTooLong);
    }
    Ok(text.to_owned())
}

/// The category of the first rule, in order, whose text is contained, ignoring
/// case, in the counterparty name followed by a space and the description
/// (CSD-032 REQ-10).
pub(super) fn categorise(
    rules: &[Rule],
    counterparty: &str,
    description: &str,
) -> Option<CategoryId> {
    let haystack = format!("{counterparty} {description}").to_lowercase();
    rules
        .iter()
        .find(|rule| haystack.contains(&rule.text.to_lowercase()))
        .map(|rule| rule.category_id)
}
