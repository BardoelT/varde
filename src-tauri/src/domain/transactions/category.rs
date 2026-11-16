//! Categories and categorisation rules (CSD-032 REQ-8 to REQ-13).

use serde::{Deserialize, Serialize};

use crate::domain::common::{Id, IdKind};
use crate::domain::error::DomainError;

use super::transaction::TransactionDate;

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

/// The days a rule is limited to, both ends included; an end that is `None`
/// is open. With neither end the rule is not limited in time (CSD-032 REQ-32).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RulePeriod {
    /// First day, or no lower bound.
    pub from: Option<TransactionDate>,
    /// Last day, or no upper bound.
    pub to: Option<TransactionDate>,
}

impl RulePeriod {
    /// Whether the rule is limited in time at all.
    #[must_use]
    pub fn is_set(self) -> bool {
        self.from.is_some() || self.to.is_some()
    }

    /// Whether `date` is inside the period.
    #[must_use]
    pub fn contains(self, date: TransactionDate) -> bool {
        self.from.is_none_or(|from| from <= date) && self.to.is_none_or(|to| date <= to)
    }
}

/// A text a transaction is matched against, and the category it gets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    /// Identifier of the rule.
    pub id: RuleId,
    /// The text a counterparty or description has to fit as a whole, ignoring
    /// case; `*` stands for any characters (REQ-10).
    pub text: String,
    /// The category a match gets.
    pub category_id: CategoryId,
    /// First day the rule applies to, itself included (REQ-32).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<TransactionDate>,
    /// Last day the rule applies to, itself included (REQ-32).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<TransactionDate>,
}

impl Rule {
    /// The days the rule is limited to.
    #[must_use]
    pub fn period(&self) -> RulePeriod {
        RulePeriod {
            from: self.from,
            to: self.to,
        }
    }

    /// Whether the rule applies to a transaction on `date` with these lower-case
    /// fields: the date is in the period and the text, when there is one, fits
    /// the counterparty or the description. A rule without a text applies to
    /// every transaction in its period (REQ-10, REQ-32).
    fn fits(&self, date: TransactionDate, fields: &(String, String)) -> bool {
        let pattern = self.text.trim().to_lowercase();
        let has_text = !pattern.is_empty();
        (has_text || self.period().is_set())
            && self.period().contains(date)
            && (!has_text || fields_match(fields, &pattern))
    }
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
    ("Albert Heijn*", "Groceries"),
    ("Jumbo*", "Groceries"),
    ("Lidl*", "Groceries"),
    ("Aldi*", "Groceries"),
    ("Dirk van den Broek*", "Groceries"),
    ("HEMA*", "Shopping"),
    ("bol.com*", "Shopping"),
    ("Coolblue*", "Shopping"),
    ("Kruidvat*", "Health"),
    ("NS Reizigers*", "Transport"),
    ("Shell*", "Transport"),
    ("Thuisbezorgd*", "Eating out"),
    ("Netflix*", "Subscriptions"),
    ("Spotify*", "Subscriptions"),
    ("Zilveren Kruis*", "Insurance"),
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
                    from: None,
                    to: None,
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

/// Checks a rule text typed by the user and returns it trimmed. A rule with a
/// period may have no text (CSD-032 REQ-32).
pub(super) fn rule_text(typed: &str, has_period: bool) -> Result<String, DomainError> {
    if has_period && typed.trim().is_empty() {
        return Ok(String::new());
    }
    valid_rule_text(typed)
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

/// Whether the whole of `text` fits `pattern`, where each `*` in the pattern
/// stands for any run of characters, none included; both are already lower
/// case. A pattern without a star is the exact text, `ca*` is a text that
/// starts with `ca`, `*ca` one that ends with it and `*ca*` one that contains
/// it; parts between stars have to be found in that order (CSD-032 REQ-10).
pub(super) fn pattern_matches(text: &str, pattern: &str) -> bool {
    let mut parts = pattern.split('*');
    let Some(first) = parts.next() else {
        return false;
    };
    let rest: Vec<&str> = parts.collect();
    let Some((last, middle)) = rest.split_last() else {
        return text == first;
    };
    let Some(mut remaining) = text.strip_prefix(first) else {
        return false;
    };
    let Some(trimmed) = remaining.strip_suffix(last) else {
        return false;
    };
    remaining = trimmed;
    for part in middle.iter().filter(|part| !part.is_empty()) {
        match remaining.find(part) {
            Some(at) => remaining = remaining.get(at + part.len()..).unwrap_or_default(),
            None => return false,
        }
    }
    true
}

/// The lower-case, trimmed counterparty name and description of a
/// transaction: the two texts a rule is matched against, each on its own
/// (CSD-032 REQ-10).
pub(super) fn fields_of(counterparty: &str, description: &str) -> (String, String) {
    (
        counterparty.trim().to_lowercase(),
        description.trim().to_lowercase(),
    )
}

/// Whether the lower-case `pattern` fits the counterparty or the description
/// (from [`fields_of`]) as a whole (CSD-032 REQ-10).
pub(super) fn fields_match(fields: &(String, String), pattern: &str) -> bool {
    pattern_matches(&fields.0, pattern) || pattern_matches(&fields.1, pattern)
}

/// The position of the first rule, in order, that applies on `date`: its text
/// fits, ignoring case, the whole counterparty name or the whole description,
/// with `*` as a wildcard, and the date is in its period (CSD-032 REQ-10,
/// REQ-32).
pub(super) fn matching_rule(
    rules: &[Rule],
    date: TransactionDate,
    counterparty: &str,
    description: &str,
) -> Option<usize> {
    let fields = fields_of(counterparty, description);
    rules.iter().position(|rule| rule.fits(date, &fields))
}

/// The category of the first matching rule (CSD-032 REQ-10).
pub(super) fn categorise(
    rules: &[Rule],
    date: TransactionDate,
    counterparty: &str,
    description: &str,
) -> Option<CategoryId> {
    matching_rule(rules, date, counterparty, description)
        .and_then(|index| rules.get(index))
        .map(|rule| rule.category_id)
}
