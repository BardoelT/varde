//! The short labels of a chart (CSD-025 REQ-4, REQ-17): the text of a marker,
//! of an axis tick and of a growth percentage, all built here so the frontend
//! only displays them.

use rust_decimal::Decimal;

use crate::domain::money::Money;

/// What stands in place of a label that cannot be built, and for a growth
/// percentage that does not exist.
const DASH: &str = "–";

const THOUSAND: Decimal = Decimal::from_parts(1_000, 0, 0, false, 0);
const MILLION: Decimal = Decimal::from_parts(1_000_000, 0, 0, false, 0);

/// The short label of an amount, as a marker or a growth figure shows it
/// (CSD-025 REQ-4): rounded to two decimals half-up, then `k` for thousands or
/// `M` for millions with two decimals, and in full below 1,000.00. The unit
/// follows the rounded figures, so 999,996.00 is "1.00M" and never
/// "1000.00k". A minus sign is kept, except for an amount that rounds to 0.00.
/// Amounts too large to hold with two decimals give a dash; the area guards
/// keep every amount far below that.
#[must_use]
pub fn short_label(amount: Decimal) -> String {
    short_label_of(amount).unwrap_or_else(|| DASH.to_owned())
}

fn short_label_of(amount: Decimal) -> Option<String> {
    let rounded = Money::round_half_up(amount)?;
    let magnitude = rounded.abs().value();
    let sign = if rounded.is_negative() { "-" } else { "" };
    if magnitude < THOUSAND {
        return Some(format!("{sign}{magnitude}"));
    }
    let thousands = Money::round_half_up(magnitude.checked_div(THOUSAND)?)?;
    if thousands.value() < THOUSAND {
        return Some(format!("{sign}{thousands}k"));
    }
    let millions = Money::round_half_up(magnitude.checked_div(MILLION)?)?;
    Some(format!("{sign}{millions}M"))
}

/// The label of an axis tick (CSD-025 REQ-17): a whole number with no
/// decimals, in thousands with `k` when the axis `step` is at least 1,000.00
/// and below 1,000,000.00, in millions with `M` from 1,000,000.00, and in full
/// below that. The unit follows the step, so every tick of an axis has the
/// same one, and a step of 1, 2 or 5 times a power of ten keeps every tick
/// whole in it. The sign is "-", and zero is "0".
#[must_use]
pub fn axis_label(tick: Decimal, step: Decimal) -> String {
    let (divisor, unit) = if step >= MILLION {
        (MILLION, "M")
    } else if step >= THOUSAND {
        (THOUSAND, "k")
    } else {
        (Decimal::ONE, "")
    };
    let whole = tick
        .checked_div(divisor)
        .unwrap_or(Decimal::ZERO)
        .trunc()
        .normalize();
    if whole.is_zero() {
        return "0".to_owned();
    }
    format!("{whole}{unit}")
}

/// A growth percentage as a label (CSD-006 REQ-8, CSD-007 REQ-10): "+x.xx%",
/// "-x.xx%", "0.00%" for no change (also for a decrease that rounds to 0.00,
/// without a sign) and a dash for no percentage.
#[must_use]
pub fn percent_label(percent: Option<Decimal>) -> String {
    let Some(rounded) = percent.and_then(Money::round_half_up) else {
        return DASH.to_owned();
    };
    if rounded.is_zero() {
        return "0.00%".to_owned();
    }
    let sign = if rounded.is_negative() { "-" } else { "+" };
    format!("{sign}{}%", rounded.abs())
}
