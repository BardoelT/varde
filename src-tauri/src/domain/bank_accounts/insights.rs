//! The yearly series behind the Bank accounts Insights chart (CSD-010).

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::charts::{Axis, ChartYear, Marker, Piece};
use crate::domain::common::{ColourSlot, YearMonth};
use crate::domain::series::SeriesData;

use super::{BankAccountId, BankAccounts};

/// A bank in the legend of the chart (CSD-010 REQ-9).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InsightsBank {
    pub id: BankAccountId,
    pub name: String,
    pub colour_slot: ColourSlot,
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
    /// The stacked pieces of the bar (CSD-025 section 5).
    pub pieces: Vec<Piece>,
    /// The closing balance, for a year with entries.
    pub marker: Option<Marker>,
}

impl ChartYear for BankInsightsYear {
    fn pieces(&self) -> &[Piece] {
        &self.pieces
    }

    fn marker(&self) -> Option<&Marker> {
        self.marker.as_ref()
    }
}

/// The payload of `get_bank_insights` (CSD-010 section 5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BankInsights {
    pub banks: Vec<InsightsBank>,
    /// The axis of the chart (CSD-025 REQ-3).
    pub axis: Axis,
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
        let figures = self.year_figures(today);
        let banks = self
            .legend(&figures)
            .into_iter()
            .map(|account| InsightsBank {
                id: account.id,
                name: account.name.clone(),
                colour_slot: account.colour_slot,
            })
            .collect();
        let years: Vec<BankInsightsYear> = figures
            .into_iter()
            .map(|figure| BankInsightsYear {
                year: figure.year,
                closing_balance: figure.closing,
                entry_count: figure.count,
                is_partial: figure.is_partial,
                segments: figure
                    .segments
                    .iter()
                    .map(|(account, balance)| InsightsSegment {
                        account_id: account.id,
                        balance: *balance,
                    })
                    .collect(),
                pieces: figure.pieces,
                marker: figure.marker,
            })
            .collect();
        let axis = Axis::of_years(&years);
        BankInsights { banks, axis, years }
    }
}
