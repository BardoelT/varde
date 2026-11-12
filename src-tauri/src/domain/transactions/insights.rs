//! The series behind the Transactions Insights chart (CSD-033): what was spent,
//! stacked per category, with what came in as a marker, per calendar year or
//! per month.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::domain::charts::{stack_from_zero, Axis, ChartYear, Marker, Piece};
use crate::domain::common::{series_years, ColourSlot, YearMonth};
use crate::domain::money::Money;

use super::category::UNCATEGORIZED_ENTITY;
use super::Transactions;

/// The most month slots the monthly series holds, counted back from its last
/// month (CSD-033 REQ-2).
pub const MAX_SERIES_MONTHS: i32 = 36;

/// The colour slot of Uncategorized, the last of the eight (CSD-033 REQ-6).
const UNCATEGORIZED_SLOT: u8 = 7;

/// One bar per calendar year or per month (CSD-033 REQ-1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Granularity {
    /// One bar per calendar year.
    #[default]
    Yearly,
    /// One bar per month.
    Monthly,
}

/// A category in the legend of the chart (CSD-033 REQ-7). `id` is the category
/// id, or `uncategorized`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InsightsCategory {
    pub id: String,
    pub name: String,
    pub colour_slot: ColourSlot,
}

/// The part of a bar that belongs to one category (CSD-033 REQ-4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InsightsSegment {
    pub category: String,
    pub amount: Money,
}

/// One bar of the chart: a calendar year, or a month when `month` is set. The
/// field is called `year` and the list `years` because the frame of the yearly
/// charts reads those names (CSD-025).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TransactionInsightsPeriod {
    pub year: i32,
    pub month: Option<u8>,
    /// The spending of the bar: the sum of its segments.
    pub total: Money,
    /// What came in during the bar: the sum of the credits.
    pub income: Money,
    /// How many transactions, debits and credits, the bar has.
    pub transaction_count: usize,
    pub is_partial: bool,
    pub segments: Vec<InsightsSegment>,
    /// The stacked pieces of the bar (CSD-025 section 5).
    pub pieces: Vec<Piece>,
    /// The income of the bar as a marker, or `None` when nothing came in.
    pub marker: Option<Marker>,
}

impl ChartYear for TransactionInsightsPeriod {
    fn pieces(&self) -> &[Piece] {
        &self.pieces
    }

    fn marker(&self) -> Option<&Marker> {
        self.marker.as_ref()
    }
}

/// The payload of `get_transaction_insights` (CSD-033 section 5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TransactionInsights {
    pub granularity: Granularity,
    /// The categories of the legend, Uncategorized last.
    pub categories: Vec<InsightsCategory>,
    pub axis: Axis,
    /// The bars, oldest first.
    pub years: Vec<TransactionInsightsPeriod>,
}

/// The number of a month on a line: 12 per year.
fn month_number(month: YearMonth) -> i32 {
    month.year().saturating_mul(12) + i32::from(month.month()) - 1
}

/// The months of the monthly series, oldest first, as month numbers: from the
/// earliest month with a record up to the later of `current` and the latest
/// month with a record, at most [`MAX_SERIES_MONTHS`] back from the last.
fn series_months(record_months: impl IntoIterator<Item = i32>, current: i32) -> Vec<i32> {
    let (mut earliest, mut latest) = (i32::MAX, i32::MIN);
    for month in record_months {
        earliest = earliest.min(month);
        latest = latest.max(month);
    }
    if earliest > latest {
        return Vec::new();
    }
    let last = latest.max(current);
    let first = earliest.max(last.saturating_sub(MAX_SERIES_MONTHS - 1));
    (first..=last).collect()
}

impl Transactions {
    /// The legend: the categories in their order with the colour slots 0, 1, 2
    /// and so on, repeating after eight, and Uncategorized last (REQ-6, REQ-7).
    fn insights_categories(&self) -> Vec<InsightsCategory> {
        let slots: Vec<ColourSlot> = ColourSlot::all().collect();
        self.categories()
            .iter()
            .zip(slots.iter().copied().cycle())
            .map(|(category, colour_slot)| InsightsCategory {
                id: category.id.to_string(),
                name: category.name.clone(),
                colour_slot,
            })
            .chain(
                ColourSlot::new(UNCATEGORIZED_SLOT).map(|colour_slot| InsightsCategory {
                    id: UNCATEGORIZED_ENTITY.to_owned(),
                    name: "Uncategorized".to_owned(),
                    colour_slot,
                }),
            )
            .collect()
    }

    /// What was spent (the debits), stacked per category, and what came in
    /// (the credits) as a marker, per year or per month, oldest first. A bar
    /// without a debit has no segments; the current year or month is partial
    /// (CSD-033 REQ-1 to REQ-5).
    #[must_use]
    pub fn insights(&self, granularity: Granularity, today: YearMonth) -> TransactionInsights {
        let categories = self.insights_categories();
        let slot_of = |id: &str| categories.iter().position(|category| category.id == id);
        let key_of = |month: YearMonth| match granularity {
            Granularity::Yearly => month.year(),
            Granularity::Monthly => month_number(month),
        };
        let mut sums: BTreeMap<(i32, usize), (Money, usize)> = BTreeMap::new();
        let mut income: BTreeMap<i32, (Money, usize)> = BTreeMap::new();
        for transaction in self.transactions() {
            if !transaction.amount.is_negative() {
                let entry = income
                    .entry(key_of(transaction.date.year_month()))
                    .or_insert((Money::ZERO, 0));
                entry.0 = entry.0.saturating_add(transaction.amount);
                entry.1 += 1;
                continue;
            }
            let id = transaction
                .category_id
                .map_or_else(|| UNCATEGORIZED_ENTITY.to_owned(), |id| id.to_string());
            let Some(slot) = slot_of(&id) else { continue };
            let key = key_of(transaction.date.year_month());
            let entry = sums.entry((key, slot)).or_insert((Money::ZERO, 0));
            entry.0 = entry.0.saturating_add(transaction.amount.abs());
            entry.1 += 1;
        }
        let record_keys: Vec<i32> = sums
            .keys()
            .map(|(key, _)| *key)
            .chain(income.keys().copied())
            .collect();
        let (keys, current) = match granularity {
            Granularity::Yearly => (series_years(record_keys, today.year()), today.year()),
            Granularity::Monthly => (
                series_months(record_keys, month_number(today)),
                month_number(today),
            ),
        };
        let years: Vec<TransactionInsightsPeriod> =
            keys.into_iter()
                .map(|key| {
                    let segments: Vec<InsightsSegment> = categories
                        .iter()
                        .enumerate()
                        .filter_map(|(slot, category)| {
                            sums.get(&(key, slot)).map(|(amount, _)| InsightsSegment {
                                category: category.id.clone(),
                                amount: *amount,
                            })
                        })
                        .collect();
                    let (earned, credits) = income.get(&key).copied().unwrap_or((Money::ZERO, 0));
                    let debits: usize = (0..categories.len())
                        .filter_map(|slot| sums.get(&(key, slot)))
                        .map(|(_, count)| *count)
                        .sum();
                    let pieces =
                        stack_from_zero(segments.iter().map(|segment| {
                            (format!("category-{}", segment.category), segment.amount)
                        }));
                    let (year, month) = match granularity {
                        Granularity::Yearly => (key, None),
                        Granularity::Monthly => (
                            key.div_euclid(12),
                            u8::try_from(key.rem_euclid(12) + 1).ok(),
                        ),
                    };
                    TransactionInsightsPeriod {
                        year,
                        month,
                        total: Money::saturating_sum(segments.iter().map(|segment| segment.amount)),
                        income: earned,
                        transaction_count: debits + credits,
                        is_partial: key == current,
                        segments,
                        pieces,
                        marker: (!earned.is_zero()).then(|| Marker::at(earned)),
                    }
                })
                .collect();
        let axis = Axis::of_years(&years);
        TransactionInsights {
            granularity,
            categories,
            axis,
            years,
        }
    }
}
