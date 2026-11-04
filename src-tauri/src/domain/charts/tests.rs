//! CSD-025 REQ-3, REQ-4 and REQ-17: the axis, the pieces, the marker and the
//! short labels, built in Rust.

use proptest::prelude::*;
use rust_decimal::Decimal;

use super::*;

fn dec(text: &str) -> Decimal {
    text.parse().unwrap()
}

fn money(text: &str) -> Money {
    Money::round_half_up(dec(text)).unwrap()
}

fn series(items: &[(&str, &str)]) -> Vec<(String, Money)> {
    items
        .iter()
        .map(|(name, value)| ((*name).to_owned(), money(value)))
        .collect()
}

fn ticks_of(axis: &Axis) -> Vec<Money> {
    axis.ticks.iter().map(|tick| tick.value).collect()
}

fn labels_of(axis: &Axis) -> Vec<&str> {
    axis.ticks.iter().map(|tick| tick.label.as_str()).collect()
}

/// A year with just the figures the axis looks at.
struct Year {
    pieces: Vec<Piece>,
    marker: Option<Marker>,
}

impl ChartYear for Year {
    fn pieces(&self) -> &[Piece] {
        &self.pieces
    }

    fn marker(&self) -> Option<&Marker> {
        self.marker.as_ref()
    }
}

/// The values of a year's pieces and the value of its marker.
type YearSpec<'a> = (&'a [(&'a str, &'a str)], Option<&'a str>);

fn axis_of(years: &[YearSpec<'_>]) -> Axis {
    let years: Vec<Year> = years
        .iter()
        .map(|(values, marker)| Year {
            pieces: stack_from_zero(series(values)),
            marker: marker.map(|value| Marker::at(money(value))),
        })
        .collect();
    Axis::of_years(&years)
}

/// The step of an axis: the distance between its first two ticks.
fn step_of(axis: &Axis) -> Money {
    axis.ticks[1].value.saturating_sub(axis.ticks[0].value)
}

// REQ-3: the axis

#[test]
fn test_csd025_ac_3_1_axis_covers_the_highest_and_the_lowest_value() {
    let axis = axis_of(&[
        (&[("a", "13100.00")], None),
        (&[("a", "5000.00"), ("b", "-1000.00")], None),
    ]);
    let ticks = ticks_of(&axis);
    assert!(ticks.first().unwrap() <= &money("-1000.00"));
    assert!(ticks.last().unwrap() >= &money("13100.00"));
    assert!(ticks.contains(&money("0.00")));
    let step = step_of(&axis);
    assert!(ticks
        .windows(2)
        .all(|pair| pair[1].saturating_sub(pair[0]) == step));
}

#[test]
fn test_csd025_ac_3_1_axis_covers_a_marker_above_the_bars() {
    let axis = axis_of(&[(&[("a", "1000.00")], Some("-4200.00"))]);
    assert!(*ticks_of(&axis).first().unwrap() <= money("-4200.00"));
    let axis = axis_of(&[(&[("a", "1000.00")], Some("9100.00"))]);
    assert!(*ticks_of(&axis).last().unwrap() >= money("9100.00"));
}

#[test]
fn test_csd025_ac_3_4_the_step_is_a_whole_round_amount() {
    let axis = axis_of(&[(&[("a", "13100.00")], None)]);
    let step = step_of(&axis);
    assert!(step == money("2000") || step == money("5000"));
    assert_eq!(step.value().fract(), Decimal::ZERO);
    assert_eq!(
        ticks_of(&axis),
        [
            money("0.00"),
            money("5000.00"),
            money("10000.00"),
            money("15000.00")
        ]
    );
}

#[test]
fn test_csd025_ac_3_1_a_series_without_figures_has_the_axis_zero_to_one() {
    let axis = axis_of(&[(&[], None)]);
    assert_eq!(ticks_of(&axis), [money("0.00"), money("1.00")]);
    assert_eq!(Axis::of_years::<Year>(&[]), axis);
}

#[test]
fn test_csd025_ac_3_1_ticks_have_two_decimals_and_zero_is_one_of_them() {
    let axis = axis_of(&[(&[("a", "2000.00"), ("b", "-1000.00")], None)]);
    let json = serde_json::to_value(&axis).unwrap();
    assert_eq!(json["ticks"][0]["value"], "-1000.00");
    assert_eq!(json["ticks"][1]["value"], "0.00");
    assert_eq!(json["ticks"][3]["value"], "2000.00");
    assert_eq!(labels_of(&axis), ["-1k", "0", "1k", "2k"]);
}

// REQ-3: pieces and marker

#[test]
fn test_csd025_ac_3_2_pieces_and_marker_are_ready_made() {
    let pieces = stack_from_zero(series(&[("bank-1", "2000.00"), ("bank-2", "8000.00")]));
    assert_eq!(
        serde_json::to_value(&pieces).unwrap(),
        serde_json::json!([
            { "series": "bank-1", "low": "0.00", "high": "2000.00", "hangs": false, "rounding": "none" },
            { "series": "bank-2", "low": "2000.00", "high": "10000.00", "hangs": false, "rounding": "top" },
        ])
    );
    let marker = Marker::at(money("10000.00"));
    assert_eq!(marker.value, money("10000.00"));
    assert_eq!(marker.label, "10.00k");
}

#[test]
fn test_csd025_ac_3_2_a_negative_value_hangs_from_the_zero_line() {
    let pieces = stack_from_zero(series(&[
        ("a", "5000.00"),
        ("b", "-1000.00"),
        ("c", "2000.00"),
        ("d", "-500.00"),
    ]));
    let spans: Vec<_> = pieces
        .iter()
        .map(|p| (p.low.to_string(), p.high.to_string(), p.hangs, p.rounding))
        .collect();
    assert_eq!(
        spans,
        [
            ("0.00".into(), "5000.00".into(), false, Rounding::None),
            ("-1000.00".into(), "0.00".into(), true, Rounding::None),
            ("5000.00".into(), "7000.00".into(), false, Rounding::Top),
            ("-1500.00".into(), "-1000.00".into(), true, Rounding::Bottom),
        ]
    );
}

#[test]
fn test_csd025_ac_3_2_a_piece_of_zero_is_not_delivered() {
    let pieces = stack_from_zero(series(&[("a", "0.00"), ("b", "10.00"), ("c", "0.00")]));
    assert_eq!(pieces.len(), 1);
    assert_eq!(pieces[0].series, "b");
    assert_eq!(pieces[0].rounding, Rounding::Top);
}

#[test]
fn test_csd025_ac_3_2_a_waterfall_hangs_from_the_running_total() {
    let pieces = waterfall(series(&[
        ("opening", "1000.00"),
        ("deposits", "500.00"),
        ("growth-down", "-200.00"),
    ]));
    let spans: Vec<_> = pieces
        .iter()
        .map(|p| {
            (
                p.series.as_str(),
                p.low.to_string(),
                p.high.to_string(),
                p.hangs,
            )
        })
        .collect();
    assert_eq!(
        spans,
        [
            ("opening", "0.00".into(), "1000.00".into(), false),
            ("deposits", "1000.00".into(), "1500.00".into(), false),
            ("growth-down", "1300.00".into(), "1500.00".into(), true),
        ]
    );
    // The piece reaching highest, the last on a tie, has its top rounded.
    let rounding: Vec<_> = pieces.iter().map(|p| p.rounding).collect();
    assert_eq!(rounding, [Rounding::None, Rounding::None, Rounding::Top]);
}

#[test]
fn test_csd025_ac_3_2_a_waterfall_draws_what_hangs_last() {
    let pieces = waterfall(series(&[
        ("opening", "1000.00"),
        ("deposits", "-300.00"),
        ("growth-up", "100.00"),
    ]));
    let order: Vec<_> = pieces.iter().map(|p| p.series.as_str()).collect();
    assert_eq!(order, ["opening", "growth-up", "deposits"]);
    assert_eq!(waterfall(series(&[("a", "0.00")])).len(), 0);
}

#[test]
fn test_csd025_ac_3_3_a_year_without_figures_has_no_pieces_and_no_marker() {
    let year = Year {
        pieces: stack_from_zero(series(&[("a", "0.00")])),
        marker: None,
    };
    assert_eq!(year.pieces().len(), 0);
    assert!(year.marker().is_none());
}

// REQ-4: short labels

#[test]
fn test_csd025_ac_4_1_short_labels() {
    for (amount, label) in [
        ("0.00", "0.00"),
        ("950.50", "950.50"),
        ("999.99", "999.99"),
        ("2549.99", "2.55k"),
        ("10000.00", "10.00k"),
        ("999994.00", "999.99k"),
        ("1500000.00", "1.50M"),
        ("-2549.99", "-2.55k"),
        ("-0.004", "0.00"),
    ] {
        assert_eq!(short_label(money(amount)), label, "{amount}");
    }
}

#[test]
fn test_csd025_ac_4_2_the_boundary_rounds_up_into_the_next_unit() {
    for (amount, label) in [
        ("999996.00", "1.00M"),
        ("999.995", "1.00k"),
        ("-999996.00", "-1.00M"),
    ] {
        assert_eq!(short_label(money(amount)), label, "{amount}");
    }
}

#[test]
fn test_csd025_ac_4_3_above_the_millions() {
    assert_eq!(short_label(money("1000000000.00")), "1000.00M");
}

#[test]
fn test_csd025_ac_4_1_a_midpoint_goes_away_from_zero() {
    assert_eq!(short_label(money("0.005")), "0.01");
    assert_eq!(short_label(money("-0.005")), "-0.01");
    assert_eq!(short_label(money("-0.0049")), "0.00");
}

// REQ-17: axis labels

#[test]
fn test_csd025_ac_17_1_axis_labels_have_no_decimals() {
    for (step, tick, label) in [
        ("500.00", "1500.00", "1500"),
        ("2000.00", "10000.00", "10k"),
        ("2000.00", "-2000.00", "-2k"),
        ("500000.00", "1500000.00", "1500k"),
        ("1000000.00", "3000000.00", "3M"),
        ("5000.00", "0.00", "0"),
    ] {
        assert_eq!(
            axis_label(money(tick), money(step)),
            label,
            "{tick} at {step}"
        );
    }
}

#[test]
fn test_csd025_ac_17_2_markers_keep_their_short_label() {
    let years = [Year {
        pieces: stack_from_zero(series(&[("a", "2549.99")])),
        marker: Some(Marker::at(money("2549.99"))),
    }];
    let axis = Axis::of_years(&years);
    assert_eq!(labels_of(&axis), ["0", "1k", "2k", "3k"]);
    assert_eq!(years[0].marker.as_ref().unwrap().label, "2.55k");
}

// Growth labels (CSD-006 REQ-8, CSD-007 REQ-10)

#[test]
fn test_csd006_ac_8_1_growth_label() {
    for (growth, label) in [
        (Some("6.42"), "+6.42%"),
        (Some("-2.50"), "-2.50%"),
        (Some("0.00"), "0.00%"),
        (Some("-0.004"), "0.00%"),
        (None, "–"),
    ] {
        assert_eq!(percent_label(growth.map(dec)), label, "{growth:?}");
    }
}

#[test]
fn test_csd007_ac_10_1_growth_percentage_label() {
    for (percent, label) in [
        (Some("6.09"), "+6.09%"),
        (Some("-2.50"), "-2.50%"),
        (Some("0.00"), "0.00%"),
        (None, "–"),
    ] {
        assert_eq!(percent_label(percent.map(dec)), label, "{percent:?}");
    }
}

// Properties of the axis

/// The levels of a chart: whole cents up to 100 billion.
fn levels() -> impl Strategy<Value = Vec<Money>> {
    prop::collection::vec(-10_000_000_000_000_i64..=10_000_000_000_000_i64, 0..12).prop_map(
        |cents| {
            cents
                .into_iter()
                .map(|c| Money::round_half_up(Decimal::new(c, 2)).unwrap())
                .collect()
        },
    )
}

/// Whether `step` is 1, 2 or 5 times a power of ten.
fn is_round_step(step: Decimal) -> bool {
    let mut power = Decimal::ONE;
    while power <= step {
        if [1, 2, 5].iter().any(|m| power * Decimal::from(*m) == step) {
            return true;
        }
        power *= Decimal::TEN;
    }
    false
}

proptest! {
    #[test]
    fn test_csd025_prop_the_axis_includes_zero_and_is_equally_spaced(levels in levels()) {
        let axis = Axis::covering(levels);
        let ticks = ticks_of(&axis);
        prop_assert!(ticks.len() >= 2);
        prop_assert!(ticks.contains(&Money::ZERO));
        prop_assert!(ticks.windows(2).all(|p| p[0] < p[1]));
        let step = step_of(&axis);
        prop_assert!(ticks.windows(2).all(|p| p[1].saturating_sub(p[0]) == step));
    }

    #[test]
    fn test_csd025_prop_the_axis_covers_every_level(levels in levels()) {
        let axis = Axis::covering(levels.clone());
        let ticks = ticks_of(&axis);
        let (first, last) = (ticks[0], ticks[ticks.len() - 1]);
        for level in levels {
            prop_assert!(first <= level && level <= last);
        }
    }

    #[test]
    fn test_csd025_prop_the_step_is_whole_and_one_two_or_five_times_a_power_of_ten(levels in levels()) {
        let step = step_of(&Axis::covering(levels));
        prop_assert!(step.value() >= Decimal::ONE);
        prop_assert_eq!(step.value().fract(), Decimal::ZERO);
        prop_assert!(is_round_step(step.value()), "{step}");
    }

    #[test]
    fn test_csd025_prop_axis_labels_are_whole_numbers_in_one_unit(levels in levels()) {
        let axis = Axis::covering(levels);
        let step = step_of(&axis);
        let unit = if step >= money("1000000") { "M" } else if step >= money("1000") { "k" } else { "" };
        for tick in &axis.ticks {
            prop_assert!(!tick.label.contains('.'), "{}", tick.label);
            prop_assert!(tick.label != "-0" && !tick.label.starts_with("-0"), "{}", tick.label);
            let digits = tick.label.trim_start_matches('-');
            if tick.value.is_zero() {
                prop_assert_eq!(&tick.label, "0");
            } else {
                prop_assert!(digits.ends_with(unit));
                prop_assert!(digits.trim_end_matches(unit).bytes().all(|b| b.is_ascii_digit()));
            }
        }
    }

    #[test]
    fn test_csd025_prop_short_labels_have_two_decimals_and_never_minus_zero(cents in -100_000_000_000_000_i64..=100_000_000_000_000_i64) {
        let label = short_label(Money::round_half_up(Decimal::new(cents, 2)).unwrap());
        prop_assert!(!label.starts_with("-0.00"), "{label}");
        prop_assert!(!label.contains("1000.00k"), "{label}");
        let number = label.trim_start_matches('-').trim_end_matches(['k', 'M']);
        let (_, decimals) = number.split_once('.').unwrap();
        prop_assert_eq!(decimals.len(), 2);
    }
}
