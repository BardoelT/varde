//! The yearly series behind the Debts Insights chart (CSD-012).

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::charts::{Axis, ChartYear, Marker, Piece};
use crate::domain::common::{ColourSlot, YearMonth};
use crate::domain::series::SeriesData;

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
        let figures = self.year_figures(today);
        let debts = self
            .legend(&figures)
            .into_iter()
            .map(|debt| InsightsDebt {
                id: debt.id,
                name: debt.name.clone(),
                colour_slot: debt.colour_slot,
            })
            .collect();
        let years: Vec<DebtInsightsYear> = figures
            .into_iter()
            .map(|figure| DebtInsightsYear {
                year: figure.year,
                closing_balance: figure.closing,
                repaid: self.repaid(figure.year),
                statement_count: figure.count,
                is_partial: figure.is_partial,
                segments: figure
                    .segments
                    .iter()
                    .map(|(debt, balance)| InsightsSegment {
                        debt_id: debt.id,
                        balance: *balance,
                    })
                    .collect(),
                pieces: figure.pieces,
                marker: figure.marker,
            })
            .collect();
        let axis = Axis::of_years(&years);
        DebtInsights { debts, axis, years }
    }
}
