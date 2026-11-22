//! Budgets per category and month (CSD-036).
//!
//! A budget of a category is a list of "from this month" amounts and a list of
//! single-month amounts. The month looks first for a single-month amount and
//! then for the latest "from" amount at or before it, so the three scopes of
//! REQ-6 only replace the entries they cover.

use std::collections::{HashMap, HashSet};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::common::YearMonth;
use crate::domain::error::DomainError;
use crate::domain::money::{percent_of, Money, MoneyError};

use super::category::CategoryId;
use super::Transactions;

/// The largest budget of one category in one month (CSD-036 REQ-9).
pub const MAX_BUDGET: Money = Money::MAX_BALANCE;

/// What a change of a budget covers (CSD-036 REQ-6, REQ-7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BudgetScope {
    /// Only the month shown.
    Month,
    /// The month shown and every later month.
    From,
    /// Every month, earlier and later.
    All,
}

impl BudgetScope {
    /// Reads the wire name: `month`, `from` or `all`.
    ///
    /// # Errors
    /// Returns `BudgetScopeInvalid` for any other text.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        match text {
            "month" => Ok(Self::Month),
            "from" => Ok(Self::From),
            "all" => Ok(Self::All),
            _ => Err(DomainError::BudgetScopeInvalid),
        }
    }
}

/// Reads a month as typed or stored, `YYYY-MM` (CSD-036 REQ-9).
///
/// # Errors
/// Returns `BudgetMonthInvalid` for any other text.
pub fn parse_month(text: &str) -> Result<YearMonth, DomainError> {
    YearMonth::parse(text).map_err(|_| DomainError::BudgetMonthInvalid)
}

/// Reads a typed budget amount: a plain number, rounded half-up to 2 decimals
/// (CSD-000 CON-8), of 0.00 up to [`MAX_BUDGET`] (CSD-036 REQ-9).
///
/// # Errors
/// Returns `BudgetAmountInvalid`.
pub fn parse_amount(text: &str) -> Result<Money, DomainError> {
    let invalid = |_: MoneyError| DomainError::BudgetAmountInvalid;
    let amount =
        Money::from_decimal(Money::parse_decimal(text).map_err(invalid)?).map_err(invalid)?;
    valid_amount(amount)?;
    Ok(amount)
}

fn valid_amount(amount: Money) -> Result<(), DomainError> {
    if amount.is_negative() || amount > MAX_BUDGET {
        return Err(DomainError::BudgetAmountInvalid);
    }
    Ok(())
}

/// An amount that holds from a month on, until a later one. A `month` of
/// `None` means since the first month; an `amount` of `None` means no budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FromEntry {
    /// The first month the entry holds for, or no lower bound.
    pub month: Option<YearMonth>,
    /// The budget, or `None` for no budget.
    pub amount: Option<Money>,
}

/// An amount for one month only. An `amount` of `None` means no budget in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OnlyEntry {
    /// The month.
    pub month: YearMonth,
    /// The budget, or `None` for no budget.
    pub amount: Option<Money>,
}

/// The budgets of one category (CSD-036 section 5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budget {
    /// The category the budgets belong to.
    pub category_id: CategoryId,
    /// Amounts that hold from a month on.
    pub from: Vec<FromEntry>,
    /// Amounts that hold for one month.
    pub only: Vec<OnlyEntry>,
}

impl Budget {
    fn new(category_id: CategoryId) -> Self {
        Self {
            category_id,
            from: Vec::new(),
            only: Vec::new(),
        }
    }

    /// The budget of the month: a single-month entry wins, else the latest
    /// "from" entry at or before the month (CSD-036 REQ-5).
    #[must_use]
    pub fn amount_in(&self, month: YearMonth) -> Option<Money> {
        if let Some(entry) = self.only.iter().find(|entry| entry.month == month) {
            return entry.amount;
        }
        self.from
            .iter()
            .filter(|entry| entry.month.is_none_or(|from| from <= month))
            .max_by_key(|entry| entry.month)
            .and_then(|entry| entry.amount)
    }

    /// Whether the budget of the month is a single-month amount (REQ-8).
    #[must_use]
    pub fn is_single_month(&self, month: YearMonth) -> bool {
        self.only
            .iter()
            .any(|entry| entry.month == month && entry.amount.is_some())
    }

    /// Applies `amount` (`None` removes the budget) to `month` with `scope`:
    /// each scope replaces what it covers and leaves the rest (REQ-6, REQ-7).
    fn apply(&mut self, scope: BudgetScope, month: YearMonth, amount: Option<Money>) {
        match scope {
            BudgetScope::Month => {
                self.only.retain(|entry| entry.month != month);
                self.only.push(OnlyEntry { month, amount });
            }
            BudgetScope::From => {
                self.from
                    .retain(|entry| entry.month.is_none_or(|from| from < month));
                self.only.retain(|entry| entry.month < month);
                self.from.push(FromEntry {
                    month: Some(month),
                    amount,
                });
            }
            BudgetScope::All => {
                self.only.clear();
                self.from = vec![FromEntry {
                    month: None,
                    amount,
                }];
            }
        }
        self.from.sort_by_key(|entry| entry.month);
        self.only.sort_by_key(|entry| entry.month);
    }

    /// Whether no month has a budget any more.
    fn is_empty(&self) -> bool {
        self.from.iter().all(|entry| entry.amount.is_none())
            && self.only.iter().all(|entry| entry.amount.is_none())
    }

    /// The earliest month an entry names.
    fn earliest_month(&self) -> Option<YearMonth> {
        self.from
            .iter()
            .filter_map(|entry| entry.month)
            .chain(self.only.iter().map(|entry| entry.month))
            .min()
    }

    /// Whether the entries follow the rules of the file (CSD-036 REQ-12): no
    /// amount below 0.00 or above the limit, and one entry per month.
    fn is_consistent(&self) -> bool {
        let amounts_fit = self
            .from
            .iter()
            .map(|entry| entry.amount)
            .chain(self.only.iter().map(|entry| entry.amount))
            .flatten()
            .all(|amount| valid_amount(amount).is_ok());
        let mut from_months = HashSet::new();
        let mut only_months = HashSet::new();
        amounts_fit
            && self
                .from
                .iter()
                .all(|entry| from_months.insert(entry.month))
            && self
                .only
                .iter()
                .all(|entry| only_months.insert(entry.month))
    }
}

/// One category in the month shown (CSD-036 REQ-3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BudgetRow {
    /// The id of the category.
    pub category: String,
    /// The name of the category.
    pub name: String,
    /// The budget of the month, or `None` for no budget.
    pub budget: Option<Money>,
    /// Whether the budget is a single-month amount (REQ-8).
    pub single_month: bool,
    /// The debits of the category in the month.
    pub spent: Money,
    /// The budget minus what was spent; `None` without a budget.
    pub remaining: Option<Money>,
    /// How full the bar is, 0.00 to 100.00; `None` without a budget.
    pub bar_percent: Option<String>,
    /// Whether more was spent than the budget.
    pub over: bool,
}

/// The sums of the categories that have a budget (CSD-036 REQ-3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BudgetTotal {
    /// The sum of the budgets.
    pub budget: Money,
    /// The sum of what was spent in those categories.
    pub spent: Money,
    /// The sum of the budgets minus the sum spent.
    pub remaining: Money,
}

/// The payload of `get_budgets` (CSD-036 section 5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BudgetsView {
    /// The month shown, `YYYY-MM`.
    pub month: YearMonth,
    /// Whether the month is the current month, which is not over yet.
    pub is_partial: bool,
    /// The earliest month the user can go back to (REQ-2).
    pub earliest_month: YearMonth,
    /// A row per budgetable category, in the order of the category list.
    pub rows: Vec<BudgetRow>,
    /// The spending of Uncategorized, which cannot have a budget.
    pub not_budgeted: Money,
    /// The sums of the budgeted categories.
    pub total: BudgetTotal,
}

impl Transactions {
    /// The budgets, one entry per category that has any.
    #[must_use]
    pub fn budgets(&self) -> &[Budget] {
        &self.budgets
    }

    /// Replaces the budgets with those of a project file and checks them
    /// (CSD-036 REQ-12).
    ///
    /// # Errors
    /// Returns `TransactionDataInvalid` for a budget of an unknown category,
    /// an amount out of range, a month twice, or a category twice.
    pub fn with_budgets(mut self, budgets: Vec<Budget>) -> Result<Self, DomainError> {
        let mut seen = HashSet::new();
        for budget in &budgets {
            if !self.has_category(budget.category_id)
                || !seen.insert(budget.category_id)
                || !budget.is_consistent()
            {
                return Err(DomainError::TransactionDataInvalid);
            }
        }
        self.budgets = budgets;
        Ok(self)
    }

    /// Sets the budget of a category for the month, with the scope of
    /// REQ-6.
    ///
    /// # Errors
    /// Returns `BudgetCategoryUnknown` or `BudgetAmountInvalid`.
    pub fn set_budget(
        &mut self,
        category: CategoryId,
        month: YearMonth,
        amount: Money,
        scope: BudgetScope,
    ) -> Result<(), DomainError> {
        valid_amount(amount)?;
        self.change_budget(category, month, Some(amount), scope)
    }

    /// Removes the budget of a category for the month, with the scope of
    /// REQ-7: the months covered then have no budget, which is not 0.00.
    ///
    /// # Errors
    /// Returns `BudgetCategoryUnknown`.
    pub fn remove_budget(
        &mut self,
        category: CategoryId,
        month: YearMonth,
        scope: BudgetScope,
    ) -> Result<(), DomainError> {
        self.change_budget(category, month, None, scope)
    }

    fn change_budget(
        &mut self,
        category: CategoryId,
        month: YearMonth,
        amount: Option<Money>,
        scope: BudgetScope,
    ) -> Result<(), DomainError> {
        if !self.has_category(category) {
            return Err(DomainError::BudgetCategoryUnknown);
        }
        let position = self
            .budgets
            .iter()
            .position(|budget| budget.category_id == category);
        let position = position.unwrap_or_else(|| {
            self.budgets.push(Budget::new(category));
            self.budgets.len() - 1
        });
        if let Some(budget) = self.budgets.get_mut(position) {
            budget.apply(scope, month, amount);
        }
        self.budgets.retain(|budget| !budget.is_empty());
        Ok(())
    }

    /// The month as the Budgets tab shows it (CSD-036 REQ-2 to REQ-4, REQ-8).
    #[must_use]
    pub fn budgets_view(&self, month: YearMonth, today: YearMonth) -> BudgetsView {
        let mut spent: HashMap<Option<CategoryId>, Money> = HashMap::new();
        for transaction in self.transactions() {
            if transaction.amount.is_negative() && transaction.date.year_month() == month {
                let sum = spent.entry(transaction.category_id).or_insert(Money::ZERO);
                *sum = sum.saturating_add(transaction.amount.abs());
            }
        }
        let spent_in = |id: Option<CategoryId>| spent.get(&id).copied().unwrap_or(Money::ZERO);
        let rows: Vec<BudgetRow> = self
            .categories()
            .iter()
            .map(|category| {
                let budget = self
                    .budgets
                    .iter()
                    .find(|budget| budget.category_id == category.id);
                let amount = budget.and_then(|budget| budget.amount_in(month));
                let spent = spent_in(Some(category.id));
                BudgetRow {
                    category: category.id.to_string(),
                    name: category.name.clone(),
                    budget: amount,
                    single_month: budget.is_some_and(|budget| budget.is_single_month(month)),
                    spent,
                    remaining: amount.map(|amount| amount.saturating_sub(spent)),
                    bar_percent: amount.map(|amount| bar_percent(spent, amount)),
                    over: amount.is_some_and(|amount| spent > amount),
                }
            })
            .collect();
        let budgeted = rows.iter().filter(|row| row.budget.is_some());
        let budget = Money::saturating_sum(budgeted.clone().filter_map(|row| row.budget));
        let spent_total = Money::saturating_sum(budgeted.map(|row| row.spent));
        let earliest = self
            .transactions()
            .iter()
            .map(|transaction| transaction.date.year_month())
            .chain(self.budgets.iter().filter_map(Budget::earliest_month))
            .min()
            .map_or(today, |earliest| earliest.min(today));
        BudgetsView {
            month,
            is_partial: month == today,
            earliest_month: earliest,
            rows,
            not_budgeted: spent_in(None),
            total: BudgetTotal {
                budget,
                spent: spent_total,
                remaining: budget.saturating_sub(spent_total),
            },
        }
    }
}

/// How full the bar is: spent as a share of the budget, at most 100.00. A
/// budget of 0.00 gives an empty bar without spending and a full one with it.
fn bar_percent(spent: Money, budget: Money) -> String {
    const FULL: &str = "100.00";
    if budget.is_zero() {
        return if spent.is_zero() { "0.00" } else { FULL }.to_owned();
    }
    match percent_of(spent, budget) {
        Some(share) if share < Decimal::ONE_HUNDRED => share.to_string(),
        _ => FULL.to_owned(),
    }
}
