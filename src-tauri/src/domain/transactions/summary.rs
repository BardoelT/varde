//! The share of transactions with a category (CSD-032 REQ-31).

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::money::Money;

use super::Transactions;

/// How many transactions have a category, and what share that is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CategorisationSummary {
    /// All transactions.
    pub total: usize,
    /// The transactions with a category.
    pub categorized: usize,
    /// The share as a label such as `12.50%` (two decimals, half-up, CSD-000
    /// CON-8); `None` while there are no transactions.
    pub percent_label: Option<String>,
}

impl Transactions {
    /// The number and the share of the transactions that are categorized.
    #[must_use]
    pub fn categorisation_summary(&self) -> CategorisationSummary {
        let total = self.list.len();
        let categorized = self
            .list
            .iter()
            .filter(|transaction| transaction.category_id.is_some())
            .count();
        let percent_label = (total > 0)
            .then(|| {
                Decimal::from(categorized)
                    .checked_mul(Decimal::ONE_HUNDRED)?
                    .checked_div(Decimal::from(total))
            })
            .flatten()
            .and_then(Money::round_half_up)
            .map(|percent| format!("{:.2}%", percent.value()));
        CategorisationSummary {
            total,
            categorized,
            percent_label,
        }
    }
}
