//! The yearly series behind the Houses Insights chart (CSD-016).

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::charts::{stack_from_zero, Axis, ChartYear, Marker, Piece};
use crate::domain::common::{series_years, ColourSlot, YearMonth};
use crate::domain::money::zero;

use super::{HouseId, Houses};

/// A house in the legend of the chart (CSD-016 REQ-9).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InsightsHouse {
    pub id: HouseId,
    pub name: String,
    pub colour_slot: ColourSlot,
}

/// The part of a year's bar that belongs to one house (CSD-016 REQ-5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InsightsSegment {
    pub house_id: HouseId,
    pub value: Decimal,
}

/// One year of the insights chart (CSD-016 REQ-2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HouseInsightsYear {
    pub year: i32,
    pub closing_value: Decimal,
    pub change: Option<Decimal>,
    pub valuation_count: usize,
    pub is_partial: bool,
    pub segments: Vec<InsightsSegment>,
    /// The stacked pieces of the bar (CSD-025 section 5).
    pub pieces: Vec<Piece>,
    /// The total value, for a year with valuations.
    pub marker: Option<Marker>,
}

impl ChartYear for HouseInsightsYear {
    fn pieces(&self) -> &[Piece] {
        &self.pieces
    }

    fn marker(&self) -> Option<&Marker> {
        self.marker.as_ref()
    }
}

/// The payload of `get_house_insights` (CSD-016 section 5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HouseInsights {
    pub houses: Vec<InsightsHouse>,
    /// The axis of the chart (CSD-025 REQ-3).
    pub axis: Axis,
    pub years: Vec<HouseInsightsYear>,
}

impl Houses {
    /// One figure set per calendar year from the first valuation up to and
    /// including the later of the current year and the latest valuation, at
    /// most 50 years back from it, oldest first, and the houses that have a
    /// segment in them, in creation order (CSD-016 REQ-2). An older valuation
    /// has no slot but still counts in the yearly values. Years without
    /// valuations are zero and have no segments. The current year is partial; a
    /// year after it is not.
    #[must_use]
    pub fn insights(&self, today: YearMonth) -> HouseInsights {
        let counts = self.counted_valuations_per_year();
        let years: Vec<HouseInsightsYear> = series_years(counts.keys().copied(), today.year())
            .into_iter()
            .map(|year| {
                let segments: Vec<InsightsSegment> = self
                    .year_segments(year)
                    .into_iter()
                    .map(|(house, value)| InsightsSegment {
                        house_id: house.id,
                        value,
                    })
                    .collect();
                let closing_value = segments
                    .iter()
                    .fold(zero(), |sum, segment| sum + segment.value);
                let valuation_count = counts.get(&year).copied().unwrap_or(0);
                let pieces = stack_from_zero(
                    segments
                        .iter()
                        .map(|segment| (format!("house-{}", segment.house_id), segment.value)),
                );
                HouseInsightsYear {
                    year,
                    closing_value,
                    change: self.year_change(year),
                    valuation_count,
                    is_partial: year == today.year(),
                    segments,
                    pieces,
                    marker: (valuation_count > 0).then(|| Marker::at(closing_value)),
                }
            })
            .collect();
        let houses = self
            .houses
            .iter()
            .filter(|house| {
                years.iter().any(|year| {
                    year.segments
                        .iter()
                        .any(|segment| segment.house_id == house.id)
                })
            })
            .map(|house| InsightsHouse {
                id: house.id,
                name: house.name.clone(),
                colour_slot: house.colour_slot,
            })
            .collect();
        let axis = Axis::of_years(&years);
        HouseInsights {
            houses,
            axis,
            years,
        }
    }
}
