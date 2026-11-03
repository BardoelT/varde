//! The yearly series behind the Bank accounts Insights chart (CSD-010).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::common::{series_years, YearMonth};
use crate::domain::money::zero;

use super::{BankAccountId, BankAccounts};

/// A bank in the legend of the chart (CSD-010 REQ-9).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InsightsBank {
    pub id: BankAccountId,
    pub name: String,
    pub colour_slot: u8,
}

/// The part of a year's bar that belongs to one bank (CSD-010 REQ-5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InsightsSegment {
    pub account_id: BankAccountId,
    pub balance: Decimal,
}

/// One year of the insights chart (CSD-010 REQ-2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BankInsightsYear {
    pub year: i32,
    pub closing_balance: Decimal,
    pub entry_count: usize,
    pub is_partial: bool,
    pub segments: Vec<InsightsSegment>,
}

/// The payload of `get_bank_insights` (CSD-010 section 5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BankInsights {
    pub banks: Vec<InsightsBank>,
    pub years: Vec<BankInsightsYear>,
}

impl BankAccounts {
    /// One figure set per calendar year from the first entry up to and
    /// including the later of the current year and the latest entry, at most 50
    /// years back from it, oldest first, and the banks that have a segment in
    /// them, in creation order (CSD-010 REQ-2). An older entry has no slot but
    /// still counts in the yearly totals. Years without entries are zero and
    /// have no segments (nothing is carried forward). The current year is
    /// partial; a year after it is not.
    #[must_use]
    pub fn insights(&self, today: YearMonth) -> BankInsights {
        let mut counts: BTreeMap<i32, usize> = BTreeMap::new();
        for entry in &self.entries {
            *counts.entry(entry.period.year()).or_default() += 1;
        }
        let years: Vec<BankInsightsYear> = series_years(counts.keys().copied(), today.year())
            .into_iter()
            .map(|year| {
                let segments: Vec<InsightsSegment> = self
                    .year_segments(year)
                    .into_iter()
                    .map(|(account, balance)| InsightsSegment {
                        account_id: account.id,
                        balance,
                    })
                    .collect();
                BankInsightsYear {
                    year,
                    closing_balance: segments
                        .iter()
                        .fold(zero(), |sum, segment| sum + segment.balance),
                    entry_count: counts.get(&year).copied().unwrap_or(0),
                    is_partial: year == today.year(),
                    segments,
                }
            })
            .collect();
        let banks = self
            .accounts
            .iter()
            .filter(|account| {
                years.iter().any(|year| {
                    year.segments
                        .iter()
                        .any(|segment| segment.account_id == account.id)
                })
            })
            .map(|account| InsightsBank {
                id: account.id,
                name: account.name.clone(),
                colour_slot: account.colour_slot,
            })
            .collect();
        BankInsights { banks, years }
    }
}
