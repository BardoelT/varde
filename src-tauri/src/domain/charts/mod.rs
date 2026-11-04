//! What every yearly chart needs from Rust (CSD-025): the axis ticks, the
//! pieces of each bar, the marker and the short labels. The frontend only
//! draws what it receives (CSD-025 REQ-5, CON-2).
//!
//! An area builds its pieces with [`stack_from_zero`] or [`waterfall`], its
//! marker with [`Marker::at`] and its axis with [`Axis::of_years`].

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::money::Money;

mod label;
mod stack;
#[cfg(test)]
mod tests;

pub use label::{axis_label, percent_label, short_label};
pub use stack::{stack_from_zero, waterfall};

/// The most intervals between the first and the last tick of an axis.
const MAX_INTERVALS: i64 = 5;

/// How the outer corners of a piece are drawn (CSD-025 section 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Rounding {
    /// The top corners are rounded.
    Top,
    /// The bottom corners are rounded.
    Bottom,
    /// The corners are square.
    None,
}

/// One stacked part of a bar, with its start and end value (CSD-025 section 3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Piece {
    /// The legend key of the series the piece belongs to; the area decides it.
    pub series: String,
    /// The value where the piece starts.
    pub low: Money,
    /// The value where the piece ends; never below `low`.
    pub high: Money,
    /// Whether the piece hangs down instead of rising.
    pub hangs: bool,
    /// How its outer corners are drawn.
    pub rounding: Rounding,
}

impl Piece {
    /// A piece with square corners.
    fn new(series: String, low: Money, high: Money, hangs: bool) -> Self {
        Self {
            series,
            low,
            high,
            hangs,
            rounding: Rounding::None,
        }
    }
}

/// A short horizontal line across a bar at one value of the year, with its
/// short label (CSD-025 REQ-3, REQ-4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Marker {
    /// Where the line stands.
    pub value: Money,
    /// The short label shown above the line, such as "2.55k".
    pub label: String,
}

impl Marker {
    /// The marker at `value`, with its short label.
    #[must_use]
    pub fn at(value: Money) -> Self {
        Self {
            value,
            label: short_label(value),
        }
    }
}

/// One gridline of the axis with its label (CSD-025 REQ-17).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Tick {
    /// The value the gridline stands at.
    pub value: Money,
    /// The whole-number label of the tick, such as "10k".
    pub label: String,
}

/// The axis of a chart: its ticks, ascending. The first and the last tick are
/// the ends of the axis, and the tick at 0.00 is the zero line.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Axis {
    /// The gridlines, lowest first.
    pub ticks: Vec<Tick>,
}

/// A year of a chart, as far as the axis is concerned.
pub trait ChartYear {
    /// The pieces of the bar of the year, none for a year without figures.
    fn pieces(&self) -> &[Piece];
    /// The marker of the year, if it has one.
    fn marker(&self) -> Option<&Marker>;
}

impl Axis {
    /// The axis that reaches every piece and marker of `years`.
    #[must_use]
    pub fn of_years<Y: ChartYear>(years: &[Y]) -> Self {
        Self::covering(years.iter().flat_map(|year| {
            year.pieces()
                .iter()
                .flat_map(|piece| [piece.low, piece.high])
                .chain(year.marker().map(|marker| marker.value))
        }))
    }

    /// The axis that reaches every value of `levels` and 0.00: equally spaced
    /// ticks at a step of 1, 2 or 5 times a power of ten (at least 1.00), from a
    /// value at or below the lowest level to one at or above the highest, with
    /// at most five intervals (CSD-025 REQ-3). Without a level above or below
    /// zero it runs from 0.00 to 1.00. An amount too large to chart gives an
    /// axis without ticks; the area guards keep every amount far below that.
    #[must_use]
    pub fn covering(levels: impl IntoIterator<Item = Money>) -> Self {
        let (mut low, mut high) = (Decimal::ZERO, Decimal::ZERO);
        for level in levels.into_iter().map(Money::value) {
            low = low.min(level);
            high = high.max(level);
        }
        if high == low {
            high = Decimal::ONE;
        }
        Self::build(low, high).unwrap_or_default()
    }

    fn build(low: Decimal, high: Decimal) -> Option<Self> {
        let step = nice_step(low, high)?;
        let step_amount = Money::round_half_up(step)?;
        let first = low.checked_div(step)?.floor().checked_mul(step)?;
        let last = high.checked_div(step)?.ceil().checked_mul(step)?;
        let mut ticks = Vec::new();
        let mut value = first;
        while value <= last {
            // A tick is a multiple of a step of at least 1.00: always a valid amount.
            let tick = Money::round_half_up(value)?;
            ticks.push(Tick {
                value: tick,
                label: axis_label(tick, step_amount),
            });
            value = value.checked_add(step)?;
        }
        Some(Self { ticks })
    }
}

/// The smallest step of 1, 2 or 5 times a power of ten, at least 1, that
/// covers `low` to `high` in at most [`MAX_INTERVALS`] intervals.
fn nice_step(low: Decimal, high: Decimal) -> Option<Decimal> {
    let mut magnitude = Decimal::ONE;
    loop {
        for multiplier in [1_i64, 2, 5] {
            let step = magnitude.checked_mul(Decimal::from(multiplier))?;
            let intervals = high
                .checked_div(step)?
                .ceil()
                .checked_sub(low.checked_div(step)?.floor())?;
            if intervals <= Decimal::from(MAX_INTERVALS) {
                return Some(step);
            }
        }
        magnitude = magnitude.checked_mul(Decimal::TEN)?;
    }
}
