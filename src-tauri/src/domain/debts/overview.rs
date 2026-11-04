//! The debts with their latest balances and the total debt (CSD-011 REQ-17).

use crate::domain::money::Money;
use serde::Serialize;

use crate::domain::common::Period;
use crate::domain::series::{self, SeriesData};

use super::{Debt, Debts};

/// One debt row of the Debts view: its latest balance and period (CSD-011
/// REQ-17).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DebtSummary {
    /// The debt fields.
    #[serde(flatten)]
    pub debt: Debt,
    /// Number of statements of the debt.
    pub statement_count: usize,
    /// Period of the latest statement, `None` without statements.
    pub latest_period: Option<Period>,
    /// The latest balance, `None` without statements.
    pub latest_balance: Option<Money>,
    /// Whether the latest statement has the balance 0.00 (CSD-011 REQ-26).
    pub repaid: bool,
}

/// The debts with the total of their latest balances (CSD-011 REQ-17). The
/// total is `None` while no debt has a statement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DebtsOverview {
    pub debts: Vec<DebtSummary>,
    pub total: Option<Money>,
}

impl Debts {
    /// The debts with their latest balances and the total debt (REQ-17).
    /// Periods are not aligned: each debt counts with its own latest statement.
    #[must_use]
    pub fn overview(&self) -> DebtsOverview {
        let debts: Vec<DebtSummary> = self
            .debts()
            .into_iter()
            .map(|debt| {
                let latest = self.latest(debt.id);
                DebtSummary {
                    debt: debt.clone(),
                    statement_count: self.statement_count(debt.id),
                    latest_period: latest.map(|statement| statement.period),
                    latest_balance: latest.map(|statement| statement.balance.value()),
                    repaid: latest.is_some_and(|statement| statement.balance.value().is_zero()),
                }
            })
            .collect();
        let total = series::total_of(debts.iter().filter_map(|summary| summary.latest_balance));
        DebtsOverview { debts, total }
    }
}
