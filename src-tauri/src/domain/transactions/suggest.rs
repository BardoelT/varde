//! Rule suggestions from the counterparties of the existing transactions
//! (CSD-032 REQ-29).

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::Serialize;

use crate::domain::error::DomainError;

use super::category::{fields_match, fields_of, valid_rule_text, CategoryId};
use super::Transactions;

/// The most suggestions one answer lists (REQ-29).
pub const MAX_SUGGESTIONS: usize = 100;

/// Legal forms dropped from the end of a counterparty name, lower case.
const LEGAL_FORMS: [&str; 6] = ["b.v.", "bv", "n.v.", "nv", "v.o.f.", "vof"];

/// A rule worth adding: its text, how many transactions it would match and the
/// category most of its categorised matches have (REQ-29).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RuleSuggestion {
    pub text: String,
    pub count: usize,
    pub category_id: Option<CategoryId>,
}

/// The answer to "suggest rules".
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RuleSuggestions {
    /// At most [`MAX_SUGGESTIONS`], highest count first, then by text.
    pub suggestions: Vec<RuleSuggestion>,
    /// How many more suggestions there are than are listed.
    pub not_shown: usize,
}

/// The counterparty name without trailing words that contain a digit or are a
/// legal form, and without trailing `.`, `,` and `-`; and whether a word was
/// dropped. `None` when nothing is left (REQ-29).
fn normalise(name: &str) -> Option<(String, bool)> {
    let mut words: Vec<&str> = name.split_whitespace().collect();
    let before = words.len();
    while let Some(last) = words.last() {
        let legal = LEGAL_FORMS.contains(&last.to_lowercase().as_str());
        if legal || last.chars().any(char::is_numeric) {
            words.pop();
        } else {
            break;
        }
    }
    let joined = words.join(" ");
    let trimmed =
        joined.trim_end_matches(|c: char| matches!(c, '.' | ',' | '-') || c.is_whitespace());
    (!trimmed.is_empty()).then(|| (trimmed.to_owned(), words.len() != before))
}

/// The names that normalise to the same text, ignoring case.
#[derive(Default)]
struct Group {
    /// How often each normalised spelling occurs.
    spellings: BTreeMap<String, usize>,
    /// Whether the name of a member had something dropped.
    dropped: bool,
}

impl Group {
    /// The most frequent spelling, the first in alphabetical order on a tie.
    fn text(&self) -> String {
        let best = self
            .spellings
            .iter()
            .fold(
                None::<(&String, usize)>,
                |best, (spelling, count)| match best {
                    Some((_, most)) if most >= *count => best,
                    _ => Some((spelling, *count)),
                },
            )
            .map(|(spelling, _)| spelling.clone())
            .unwrap_or_default();
        if self.dropped {
            format!("{best}*")
        } else {
            best
        }
    }
}

impl Transactions {
    /// The lower-case counterparty and description of every transaction, the
    /// texts a rule is matched against, with its category.
    fn haystacks(&self) -> Vec<((String, String), Option<CategoryId>)> {
        self.transactions()
            .iter()
            .map(|transaction| {
                (
                    fields_of(&transaction.counterparty_name, &transaction.description),
                    transaction.category_id,
                )
            })
            .collect()
    }

    /// How many of `haystacks` the rule text matches and the category most of
    /// the categorised ones have (a tie goes to the category listed first).
    fn match_summary(
        &self,
        haystacks: &[((String, String), Option<CategoryId>)],
        text: &str,
    ) -> (usize, Option<CategoryId>) {
        let pattern = text.trim().to_lowercase();
        let mut count = 0;
        let mut per_category: HashMap<CategoryId, usize> = HashMap::new();
        for (haystack, category) in haystacks {
            if fields_match(haystack, &pattern) {
                count += 1;
                if let Some(category) = category {
                    *per_category.entry(*category).or_insert(0) += 1;
                }
            }
        }
        let best = self
            .categories()
            .iter()
            .filter_map(|category| per_category.get(&category.id).map(|n| (category.id, *n)))
            .fold(None::<(CategoryId, usize)>, |best, (id, n)| match best {
                Some((_, most)) if most >= n => best,
                _ => Some((id, n)),
            })
            .map(|(id, _)| id);
        (count, best)
    }

    /// The rules worth adding, from every transaction that has a counterparty:
    /// the counterparties are grouped by their normalised name, each group is
    /// a rule text, and the ones with at least `min_count` matches are listed,
    /// except those a rule with the same text already covers (REQ-29).
    ///
    /// # Errors
    /// Returns `TransactionSuggestMinCount` when `min_count` is 0.
    pub fn suggest_rules(&self, min_count: usize) -> Result<RuleSuggestions, DomainError> {
        if min_count == 0 {
            return Err(DomainError::TransactionSuggestMinCount);
        }
        let mut groups: BTreeMap<String, Group> = BTreeMap::new();
        for transaction in self.transactions() {
            if let Some((name, dropped)) = normalise(&transaction.counterparty_name) {
                let group = groups.entry(name.to_lowercase()).or_default();
                *group.spellings.entry(name).or_insert(0) += 1;
                group.dropped |= dropped;
            }
        }
        let existing: HashSet<String> = self
            .rules()
            .iter()
            .map(|rule| rule.text.to_lowercase())
            .collect();
        let haystacks = self.haystacks();
        let mut found: Vec<RuleSuggestion> = groups
            .values()
            .map(Group::text)
            .filter(|text| {
                valid_rule_text(text).is_ok() && !existing.contains(&text.to_lowercase())
            })
            .filter_map(|text| {
                let (count, category_id) = self.match_summary(&haystacks, &text);
                (count >= min_count).then_some(RuleSuggestion {
                    text,
                    count,
                    category_id,
                })
            })
            .collect();
        found.sort_by(|left, right| {
            right
                .count
                .cmp(&left.count)
                .then_with(|| left.text.cmp(&right.text))
        });
        let not_shown = found.len().saturating_sub(MAX_SUGGESTIONS);
        found.truncate(MAX_SUGGESTIONS);
        Ok(RuleSuggestions {
            suggestions: found,
            not_shown,
        })
    }

    /// How many transactions a rule with `text` would match, for the recount of
    /// an edited suggestion (REQ-29).
    ///
    /// # Errors
    /// Returns the violated rule text rule.
    pub fn count_rule_matches(&self, text: &str) -> Result<usize, DomainError> {
        let text = valid_rule_text(text)?;
        Ok(self.match_summary(&self.haystacks(), &text).0)
    }
}
