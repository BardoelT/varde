//! The data behind the two donut charts of the Categories tab: what was spent
//! per category in one year or one month (CSD-032 REQ-35).

use std::collections::{BTreeSet, HashMap};

use rust_decimal::RoundingStrategy;
use serde::Serialize;

use crate::domain::common::{ColourSlot, YearMonth};
use crate::domain::error::DomainError;
use crate::domain::money::{percent_of, Money};

use super::category::{CategoryId, UNCATEGORIZED_ENTITY};
use super::Transactions;

/// One calendar year or one calendar month, as the frontend asks for it:
/// `2026` or `2026-11`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpendingPeriod {
    /// A calendar year.
    Year(i32),
    /// A calendar month.
    Month(YearMonth),
}

impl SpendingPeriod {
    /// Parses `YYYY` or `YYYY-MM`.
    ///
    /// # Errors
    /// `TransactionPeriodInvalid` for any other text.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        let text = text.trim();
        if text.len() == 4 && text.bytes().all(|byte| byte.is_ascii_digit()) {
            let year: i32 = text
                .parse()
                .map_err(|_| DomainError::TransactionPeriodInvalid)?;
            return (year >= 1)
                .then_some(Self::Year(year))
                .ok_or(DomainError::TransactionPeriodInvalid);
        }
        YearMonth::parse(text)
            .map(Self::Month)
            .map_err(|_| DomainError::TransactionPeriodInvalid)
    }

    fn contains(self, month: YearMonth) -> bool {
        match self {
            Self::Year(year) => month.year() == year,
            Self::Month(wanted) => month == wanted,
        }
    }
}

/// The years and months that have at least one debit, newest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SpendingPeriods {
    /// Calendar years.
    pub years: Vec<i32>,
    /// Calendar months, written `YYYY-MM`.
    pub months: Vec<YearMonth>,
}

/// One slice of a donut: the spending of one category in the period.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SpendingSlice {
    /// The category id, or `uncategorized`.
    pub id: String,
    pub name: String,
    pub colour_slot: ColourSlot,
    pub amount: Money,
    /// The share of the total as a label such as `66.67%` (two decimals,
    /// half-up, CSD-000 CON-8).
    pub percent_label: String,
    /// Where the slice starts and ends on the ring, as fractions of the full
    /// circle written with six decimals (`0.000000` to `1.000000`), so the
    /// frontend only draws the arc. The first slice starts at 0 and the last
    /// ends at 1; they follow the order of the slices.
    pub start: String,
    pub end: String,
}

/// The payload of `transaction_spending_shares`: the total spent in the period
/// and its slices.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SpendingShares {
    pub total: Money,
    /// In the order of the Categories tab, Uncategorized last; only categories
    /// with spending.
    pub slices: Vec<SpendingSlice>,
}

impl Transactions {
    /// The years and months with at least one debit, newest first (REQ-35).
    #[must_use]
    pub fn spending_periods(&self) -> SpendingPeriods {
        let months: BTreeSet<YearMonth> = self
            .transactions()
            .iter()
            .filter(|transaction| transaction.amount.is_negative())
            .map(|transaction| transaction.date.year_month())
            .collect();
        let years: BTreeSet<i32> = months.iter().map(|month| month.year()).collect();
        SpendingPeriods {
            years: years.into_iter().rev().collect(),
            months: months.into_iter().rev().collect(),
        }
    }

    /// What was spent (the debits) per category in `period`, one slice per
    /// category with spending, with its share of the total (REQ-35).
    #[must_use]
    pub fn spending_shares(&self, period: SpendingPeriod) -> SpendingShares {
        let categories = self.insights_categories();
        let slots: HashMap<CategoryId, usize> = self
            .categories()
            .iter()
            .enumerate()
            .map(|(slot, category)| (category.id, slot))
            .collect();
        let uncategorized = categories
            .iter()
            .position(|category| category.id == UNCATEGORIZED_ENTITY);
        let mut sums = vec![Money::ZERO; categories.len()];
        let mut has_spending = vec![false; categories.len()];
        for transaction in self.transactions() {
            if !transaction.amount.is_negative() || !period.contains(transaction.date.year_month())
            {
                continue;
            }
            let slot = match transaction.category_id {
                Some(id) => slots.get(&id).copied(),
                None => uncategorized,
            };
            if let Some((sum, seen)) =
                slot.and_then(|slot| sums.get_mut(slot).zip(has_spending.get_mut(slot)))
            {
                *sum = sum.saturating_add(transaction.amount.abs());
                *seen = true;
            }
        }
        let total = Money::saturating_sum(sums.iter().copied());
        let mut before = Money::ZERO;
        let last = has_spending.iter().rposition(|seen| *seen);
        let mut slices = Vec::new();
        for (index, ((category, amount), seen)) in categories
            .into_iter()
            .zip(sums)
            .zip(has_spending)
            .enumerate()
        {
            if !seen {
                continue;
            }
            let after = before.saturating_add(amount);
            slices.push(SpendingSlice {
                percent_label: percent_of(amount, total)
                    .map_or_else(String::new, |percent| format!("{percent:.2}%")),
                id: category.id,
                name: category.name,
                colour_slot: category.colour_slot,
                amount,
                start: fraction(before, total),
                end: if Some(index) == last {
                    fraction(total, total)
                } else {
                    fraction(after, total)
                },
            });
            before = after;
        }
        SpendingShares { total, slices }
    }
}

/// `part` of `whole` as a fraction with six decimals, half-up; `0.000000` when
/// there is nothing to divide.
fn fraction(part: Money, whole: Money) -> String {
    let value = part
        .value()
        .checked_div(whole.value())
        .unwrap_or_default()
        .round_dp_with_strategy(6, RoundingStrategy::MidpointAwayFromZero);
    format!("{value:.6}")
}
