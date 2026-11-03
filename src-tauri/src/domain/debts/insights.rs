//! The yearly series behind the Debts Insights chart (CSD-012).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::charts::{stack_from_zero, Axis, ChartYear, Marker, Piece};
use crate::domain::common::{series_years, ColourSlot, YearMonth};
use crate::domain::money::zero;

use super::{DebtId, Debts};

/// A debt in the legend of the chart (CSD-012 REQ-9).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InsightsDebt {
    pub id: DebtId,
    pub name: String,
    pub colour_slot: ColourSlot,
}

/// The part of a year's bar that belongs to one debt (CSD-012 REQ-5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InsightsSegment {
    pub debt_id: DebtId,
    pub balance: Decimal,
}

/// One year of the insights chart (CSD-012 REQ-2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DebtInsightsYear {
    pub year: i32,
    pub closing_balance: Decimal,
    pub repaid: Option<Decimal>,
    pub statement_count: usize,
    pub is_partial: bool,
    pub segments: Vec<InsightsSegment>,
    /// The stacked pieces of the bar (CSD-025 section 5).
    pub pieces: Vec<Piece>,
    /// The total debt, for a year with statements.
    pub marker: Option<Marker>,
}

impl ChartYear for DebtInsightsYear {
    fn pieces(&self) -> &[Piece] {
        &self.pieces
    }

    fn marker(&self) -> Option<&Marker> {
        self.marker.as_ref()
    }
}

/// The payload of `get_debt_insights` (CSD-012 section 5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DebtInsights {
    pub debts: Vec<InsightsDebt>,
    /// The axis of the chart (CSD-025 REQ-3).
    pub axis: Axis,
    pub years: Vec<DebtInsightsYear>,
}

impl Debts {
    /// One figure set per calendar year from the first statement up to and
    /// including the later of the current year and the latest statement, at
    /// most 50 years back from it, oldest first, and the debts that have a
    /// segment in them, in creation order (CSD-012, with the series range of
    /// CSD-017 REQ-2 so the net worth can show the debts of a later year). Years
    /// without statements are zero and have no segments. The current year is
    /// partial; a year after it is not.
    #[must_use]
    pub fn insights(&self, today: YearMonth) -> DebtInsights {
        let mut counts: BTreeMap<i32, usize> = BTreeMap::new();
        for statement in &self.statements {
            *counts.entry(statement.period.year()).or_default() += 1;
        }
        let years: Vec<DebtInsightsYear> = series_years(counts.keys().copied(), today.year())
            .into_iter()
            .map(|year| {
                let segments: Vec<InsightsSegment> = self
                    .year_segments(year)
                    .into_iter()
                    .map(|(debt, balance)| InsightsSegment {
                        debt_id: debt.id,
                        balance,
                    })
                    .collect();
                let closing_balance = segments
                    .iter()
                    .fold(zero(), |sum, segment| sum + segment.balance);
                let statement_count = counts.get(&year).copied().unwrap_or(0);
                let pieces = stack_from_zero(
                    segments
                        .iter()
                        .map(|segment| (format!("debt-{}", segment.debt_id), segment.balance)),
                );
                DebtInsightsYear {
                    year,
                    closing_balance,
                    repaid: self.repaid(year),
                    statement_count,
                    is_partial: year == today.year(),
                    segments,
                    pieces,
                    marker: (statement_count > 0).then(|| Marker::at(closing_balance)),
                }
            })
            .collect();
        let debts = self
            .debts
            .iter()
            .filter(|debt| {
                years.iter().any(|year| {
                    year.segments
                        .iter()
                        .any(|segment| segment.debt_id == debt.id)
                })
            })
            .map(|debt| InsightsDebt {
                id: debt.id,
                name: debt.name.clone(),
                colour_slot: debt.colour_slot,
            })
            .collect();
        let axis = Axis::of_years(&years);
        DebtInsights { debts, axis, years }
    }
}
