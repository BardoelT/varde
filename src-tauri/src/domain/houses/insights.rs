//! The yearly series behind the Houses Insights chart (CSD-016).

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::charts::{Axis, ChartYear, Marker, Piece};
use crate::domain::common::{ColourSlot, YearMonth};
use crate::domain::series::SeriesData;

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
        let figures = self.year_figures(today);
        let houses = self
            .legend(&figures)
            .into_iter()
            .map(|house| InsightsHouse {
                id: house.id,
                name: house.name.clone(),
                colour_slot: house.colour_slot,
            })
            .collect();
        let years: Vec<HouseInsightsYear> = figures
            .into_iter()
            .map(|figure| HouseInsightsYear {
                year: figure.year,
                closing_value: figure.closing,
                change: self.year_change(figure.year),
                valuation_count: figure.count,
                is_partial: figure.is_partial,
                segments: figure
                    .segments
                    .iter()
                    .map(|(house, value)| InsightsSegment {
                        house_id: house.id,
                        value: *value,
                    })
                    .collect(),
                pieces: figure.pieces,
                marker: figure.marker,
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
