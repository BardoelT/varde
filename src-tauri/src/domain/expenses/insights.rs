//! The yearly series behind the Expenses Insights chart (CSD-014).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::charts::{stack_from_zero, Axis, ChartYear, Marker, Piece};
use crate::domain::common::{series_years, wire_name, ColourSlot};
use crate::domain::money::zero;

use super::{ExpenseCategory, Expenses};

/// A category in the legend of the chart (CSD-014 REQ-7).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InsightsCategory {
    pub name: ExpenseCategory,
    pub colour_slot: ColourSlot,
}

/// The part of a year's bar that belongs to one category (CSD-014 REQ-4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InsightsSegment {
    pub category: ExpenseCategory,
    pub amount: Decimal,
}

/// One year of the insights chart (CSD-014 REQ-1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExpenseInsightsYear {
    pub year: i32,
    pub total: Decimal,
    pub expense_count: usize,
    pub is_partial: bool,
    pub segments: Vec<InsightsSegment>,
    /// The stacked pieces of the bar (CSD-025 section 5).
    pub pieces: Vec<Piece>,
    /// Expenses have no marker; always `None`.
    pub marker: Option<Marker>,
}

impl ChartYear for ExpenseInsightsYear {
    fn pieces(&self) -> &[Piece] {
        &self.pieces
    }

    fn marker(&self) -> Option<&Marker> {
        self.marker.as_ref()
    }
}

/// The payload of `get_expense_insights` (CSD-014 section 5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExpenseInsights {
    /// The categories shown in the chart.
    pub categories: Vec<InsightsCategory>,
    /// The axis of the chart (CSD-025 REQ-3).
    pub axis: Axis,
    /// The years of the chart, one entry per year.
    pub years: Vec<ExpenseInsightsYear>,
}

impl Expenses {
    /// One figure set per calendar year from the first expense up to and
    /// including the later of `current_year` and the latest expense, at most 50
    /// years back from it, oldest first. An older expense has no slot but still
    /// counts in the yearly totals. A year without expenses is zero and has no
    /// segments; the current year is partial and a later year is not
    /// (CSD-014 REQ-1 to REQ-4).
    #[must_use]
    pub fn insights(&self, current_year: i32) -> ExpenseInsights {
        let categories = ExpenseCategory::ALL
            .into_iter()
            .map(|name| InsightsCategory {
                name,
                colour_slot: name.colour_slot(),
            })
            .collect();
        let mut sums: BTreeMap<(i32, usize), (Decimal, usize)> = BTreeMap::new();
        for expense in &self.expenses {
            let Some(slot) = ExpenseCategory::ALL
                .iter()
                .position(|c| *c == expense.category)
            else {
                continue;
            };
            let entry = sums
                .entry((expense.date.year(), slot))
                .or_insert((zero(), 0));
            entry.0 += expense.amount.value();
            entry.1 += 1;
        }
        let years: Vec<ExpenseInsightsYear> =
            series_years(self.expenses.iter().map(|e| e.date.year()), current_year)
                .into_iter()
                .map(|year| {
                    let segments: Vec<InsightsSegment> = ExpenseCategory::ALL
                        .iter()
                        .enumerate()
                        .filter_map(|(slot, category)| {
                            sums.get(&(year, slot)).map(|(amount, _)| InsightsSegment {
                                category: *category,
                                amount: *amount,
                            })
                        })
                        .collect();
                    let expense_count = (0..ExpenseCategory::ALL.len())
                        .filter_map(|slot| sums.get(&(year, slot)))
                        .map(|(_, count)| *count)
                        .sum();
                    let pieces = stack_from_zero(segments.iter().map(|segment| {
                        (
                            format!("category-{}", wire_name(&segment.category)),
                            segment.amount,
                        )
                    }));
                    ExpenseInsightsYear {
                        year,
                        total: segments
                            .iter()
                            .fold(zero(), |sum, segment| sum + segment.amount),
                        expense_count,
                        is_partial: year == current_year,
                        segments,
                        pieces,
                        marker: None,
                    }
                })
                .collect();
        let axis = Axis::of_years(&years);
        ExpenseInsights {
            categories,
            axis,
            years,
        }
    }
}
