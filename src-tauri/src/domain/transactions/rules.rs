//! The rules that categorise the transactions (CSD-032 REQ-9, REQ-10, REQ-12,
//! REQ-27, REQ-32).

use std::sync::Arc;

use crate::domain::error::DomainError;

use super::category::{fields_of, rule_text, CompiledRules};
use super::{Category, CategoryId, ReapplyScope, Rule, RuleId, RulePeriod, Transactions};

/// What the user typed for a rule: its text and the days it is limited to. The
/// category comes with it as a separate argument, as it may be a new one
/// (CSD-032 REQ-9, REQ-32).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuleDraft {
    /// The text, empty only for a rule with a period.
    pub text: String,
    /// The days the rule applies to.
    pub period: RulePeriod,
}

impl RuleDraft {
    /// A draft with `text` and `period`.
    #[must_use]
    pub fn new(text: impl Into<String>, period: RulePeriod) -> Self {
        Self {
            text: text.into(),
            period,
        }
    }

    /// A draft that is not limited in time.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::new(text, RulePeriod::default())
    }
}

impl Transactions {
    fn rule_category(&self, id: CategoryId) -> Result<CategoryId, DomainError> {
        if self.has_category(id) {
            Ok(id)
        } else {
            Err(DomainError::TransactionRuleCategoryInvalid)
        }
    }

    /// Checks a draft and returns its text trimmed. Without a period the text
    /// is required; a rule that has the same text, ignoring case, and the same
    /// period as another one is the same rule, because matching ignores case
    /// (REQ-9, REQ-32). `own` is the rule being edited.
    fn checked_rule_text(
        &self,
        draft: &RuleDraft,
        own: Option<RuleId>,
    ) -> Result<String, DomainError> {
        let period = draft.period;
        let text = rule_text(&draft.text, period.is_set())?;
        if period
            .from
            .zip(period.to)
            .is_some_and(|(from, to)| from > to)
        {
            return Err(DomainError::TransactionRulePeriodInvalid);
        }
        let lower = text.to_lowercase();
        let taken = self.rules.iter().any(|rule| {
            Some(rule.id) != own && rule.text.to_lowercase() == lower && rule.period() == period
        });
        if taken {
            return Err(DomainError::TransactionRuleTextTaken);
        }
        Ok(text)
    }

    /// Adds a rule without a period at the end of the list (REQ-9).
    ///
    /// # Errors
    /// Returns the violated text or category rule, or `TransactionRuleTextTaken`.
    pub fn add_rule(&mut self, text: &str, category: CategoryId) -> Result<Rule, DomainError> {
        self.add_rule_with(&RuleDraft::text(text), category)
    }

    /// Adds the rule of a draft at the end of the list (REQ-9, REQ-32).
    ///
    /// # Errors
    /// Returns the violated text, period or category rule, or
    /// `TransactionRuleTextTaken`.
    pub fn add_rule_with(
        &mut self,
        draft: &RuleDraft,
        category: CategoryId,
    ) -> Result<Rule, DomainError> {
        let text = self.checked_rule_text(draft, None)?;
        let category_id = self.rule_category(category)?;
        Ok(self.push_rule(text, draft.period, category_id))
    }

    fn push_rule(&mut self, text: String, period: RulePeriod, category_id: CategoryId) -> Rule {
        let rule = Rule {
            id: RuleId::new(),
            text,
            category_id,
            from: period.from,
            to: period.to,
        };
        self.rules.push(rule.clone());
        rule
    }

    /// Adds `rules`, each a text and a category, at the end of the list in
    /// their order, as one change: when one is invalid, none is added and the
    /// position of the first invalid one comes with the error (REQ-29).
    ///
    /// # Errors
    /// Returns the position and the violated rule of the first invalid rule,
    /// a text another rule or an earlier one of `rules` has included.
    pub fn add_rules(
        &mut self,
        rules: &[(String, CategoryId)],
    ) -> Result<Vec<Rule>, (usize, DomainError)> {
        let mut working = self.clone();
        let mut added = Vec::with_capacity(rules.len());
        for (position, (text, category)) in rules.iter().enumerate() {
            let rule = working
                .add_rule(text, *category)
                .map_err(|error| (position, error))?;
            added.push(rule);
        }
        *self = working;
        Ok(added)
    }

    /// Adds the rule of a draft at the end of the list together with a new
    /// category it names, as one change: when the text or the name is invalid,
    /// neither is added (REQ-9).
    ///
    /// # Errors
    /// Returns the violated text rule, or the violated name rule.
    pub fn add_rule_with_new_category(
        &mut self,
        draft: &RuleDraft,
        name: &str,
    ) -> Result<(Category, Rule), DomainError> {
        let text = self.checked_rule_text(draft, None)?;
        let category = self.add_category(name)?;
        let rule = self.push_rule(text, draft.period, category.id);
        Ok((category, rule))
    }

    /// Changes the text, period and category of a rule (REQ-9, REQ-32).
    ///
    /// # Errors
    /// Returns `TransactionRuleNotFound`, `TransactionRuleTextTaken` or the violated text or category rule.
    pub fn edit_rule(
        &mut self,
        id: RuleId,
        draft: &RuleDraft,
        category: CategoryId,
    ) -> Result<Rule, DomainError> {
        let text = self.checked_rule_text(draft, Some(id))?;
        let category_id = self.rule_category(category)?;
        let rule = self
            .rules
            .iter_mut()
            .find(|rule| rule.id == id)
            .ok_or(DomainError::TransactionRuleNotFound)?;
        rule.text = text;
        rule.category_id = category_id;
        rule.from = draft.period.from;
        rule.to = draft.period.to;
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

    /// Every rule, in order, with the number of transactions it would
    /// categorise (REQ-27); what the rule list shows.
    #[must_use]
    pub fn rule_rows(&self) -> Vec<(&Rule, usize)> {
        self.rules.iter().zip(self.rule_counts()).collect()
    }

    /// For each rule, in order, how many transactions it would categorise: the
    /// transactions that are not manual and whose first matching rule it is
    /// (REQ-27).
    #[must_use]
    pub fn rule_counts(&self) -> Vec<usize> {
        let compiled = CompiledRules::new(&self.rules);
        let mut counts = vec![0; self.rules.len()];
        for transaction in self.list.iter().filter(|transaction| !transaction.manual) {
            let found = compiled.first_match(
                transaction.date,
                &fields_of(&transaction.counterparty_name, &transaction.description),
            );
            if let Some(count) = found.and_then(|index| counts.get_mut(index)) {
                *count += 1;
            }
        }
        counts
    }

    /// Runs the rules again on the transactions of `scope` and returns how many
    /// changed category. Manual transactions are only touched by
    /// [`ReapplyScope::All`] (REQ-12).
    pub fn reapply_rules(&mut self, scope: ReapplyScope) -> usize {
        let compiled = CompiledRules::new(&self.rules);
        let changes: Vec<(usize, Option<CategoryId>)> = self
            .list
            .iter()
            .enumerate()
            .filter(|(_, transaction)| scope == ReapplyScope::All || !transaction.manual)
            .filter_map(|(index, transaction)| {
                let found = compiled.category_of(
                    transaction.date,
                    &fields_of(&transaction.counterparty_name, &transaction.description),
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
                transaction.manual = false;
            }
        }
        count
    }
}
