//! Stacking the figures of a year into pieces (CSD-025 section 5), so the
//! frontend places them and adds up nothing.

use super::{Piece, Rounding};
use crate::domain::money::Money;

/// Stacks signed values from the zero line: a positive value rises from the
/// running top, a negative value hangs from the running bottom, in the order
/// given. A value of 0.00 gives no piece. The last piece that rises has its
/// top rounded and the last piece that hangs has its bottom rounded.
#[must_use]
pub fn stack_from_zero(values: impl IntoIterator<Item = (String, Money)>) -> Vec<Piece> {
    let (mut top, mut bottom) = (Money::ZERO, Money::ZERO);
    let mut pieces: Vec<Piece> = Vec::new();
    for (series, value) in values {
        if value.is_zero() {
            continue;
        }
        let (low, high, hangs) = if value.is_negative() {
            let high = bottom;
            bottom = bottom.saturating_add(value);
            (bottom, high, true)
        } else {
            let low = top;
            top = top.saturating_add(value);
            (low, top, false)
        };
        pieces.push(Piece::new(series, low, high, hangs));
    }
    for hangs in [false, true] {
        if let Some(piece) = pieces.iter_mut().rev().find(|piece| piece.hangs == hangs) {
            piece.rounding = if hangs {
                Rounding::Bottom
            } else {
                Rounding::Top
            };
        }
    }
    pieces
}

/// Stacks values as a waterfall: each value moves a running total, a positive
/// one rising from it and a negative one hanging down from it. A value of 0.00
/// gives no piece. The pieces that rise come first and those that hang last,
/// so a later rise does not hide one that hangs. The piece reaching highest
/// (the last of them on a tie) has its top rounded.
#[must_use]
pub fn waterfall(values: impl IntoIterator<Item = (String, Money)>) -> Vec<Piece> {
    let mut level = Money::ZERO;
    let mut pieces: Vec<Piece> = Vec::new();
    for (series, value) in values {
        if value.is_zero() {
            continue;
        }
        let next = level.saturating_add(value);
        pieces.push(Piece::new(
            series,
            level.min(next),
            level.max(next),
            value.is_negative(),
        ));
        level = next;
    }
    // Stable, so the order within each group is the order given.
    pieces.sort_by_key(|piece| piece.hangs);
    let highest = pieces.iter().map(|piece| piece.high).max();
    if let Some(piece) = pieces
        .iter_mut()
        .rev()
        .find(|piece| Some(piece.high) == highest)
    {
        piece.rounding = Rounding::Top;
    }
    pieces
}
