//! Acceptance-criteria tests of CSD-007 that the other test files carry no AC id for.

use super::*;
use crate::domain::charts::Axis;
use crate::domain::common::YearMonth;

fn today() -> YearMonth {
    YearMonth::parse("2026-10").unwrap()
}

/// A monthly account "Example Brokerage" with the statements `(period, balance, net deposits)`.
fn brokerage(statements: &[(&str, &str, &str)]) -> Investments {
    let mut investments = Investments::default();
    let id = investments
        .create_account(&InvestmentAccountInput {
            name: "Example Brokerage".to_owned(),
            period: Some("Monthly".to_owned()),
        })
        .unwrap()
        .id;
    for (period, amount, net_deposits) in statements {
        investments
            .add_statement(
                id,
                Period::parse(period).unwrap(),
                Balance::parse(amount).unwrap(),
                NetDeposits::parse(net_deposits).unwrap(),
                Note::default(),
            )
            .unwrap();
    }
    investments
}

fn year_of(investments: &Investments, year: i32) -> InvestmentInsightsYear {
    investments
        .insights(today())
        .into_iter()
        .find(|row| row.year == year)
        .unwrap()
}

/// The pieces of a year as `(series, low, high, hangs)`.
fn pieces(row: &InvestmentInsightsYear) -> Vec<(String, String, String, bool)> {
    row.pieces
        .iter()
        .map(|p| {
            (
                p.series.clone(),
                p.low.to_string(),
                p.high.to_string(),
                p.hangs,
            )
        })
        .collect()
}

fn piece(series: &str, low: &str, high: &str, hangs: bool) -> (String, String, String, bool) {
    (series.to_owned(), low.to_owned(), high.to_owned(), hangs)
}

#[test]
fn test_csd007_ac_8_3_negative_segments_hang_down_from_the_running_total() {
    // 2026 opens with 12000.00, has net deposits -500.00 and capital growth -500.00.
    let investments = brokerage(&[
        ("2025-12", "12000.00", "12000.00"),
        ("2026-06", "11000.00", "-500.00"),
    ]);
    let row = year_of(&investments, 2026);
    assert_eq!(row.opening_balance.to_string(), "12000.00");
    assert_eq!(row.net_deposits.to_string(), "-500.00");
    assert_eq!(row.capital_growth.to_string(), "-500.00");
    assert_eq!(
        pieces(&row),
        [
            piece("opening", "0.00", "12000.00", false),
            piece("deposits", "11500.00", "12000.00", true),
            piece("growth-down", "11000.00", "11500.00", true),
        ]
    );
}

#[test]
fn test_csd007_ac_8_4_the_axis_starts_at_zero_and_covers_the_highest_level() {
    let investments = brokerage(&[
        ("2025-12", "13100.00", "13100.00"),
        ("2026-06", "12000.00", "0.00"),
    ]);
    let series = investments.insights(today());
    let highest = series
        .iter()
        .flat_map(|row| row.pieces.iter().map(|p| p.high))
        .max()
        .unwrap();
    assert_eq!(highest.to_string(), "13100.00");
    let axis = Axis::of_years(&series);
    let first = axis.ticks.first().unwrap();
    let last = axis.ticks.last().unwrap();
    assert_eq!(first.value.to_string(), "0.00", "the axis starts at 0");
    assert!(last.value >= highest, "the top is at least 13100.00");
}

#[test]
fn test_csd007_ac_9_2_the_marker_sits_at_the_top_of_a_gain_and_the_bottom_of_a_loss() {
    // 2025 has capital growth 300.00 (deposits 500.00), 2026 has capital growth -500.00.
    let investments = brokerage(&[
        ("2024-12", "10000.00", "10000.00"),
        ("2025-12", "10800.00", "500.00"),
        ("2026-06", "10300.00", "0.00"),
    ]);
    let gain = year_of(&investments, 2025);
    assert_eq!(gain.capital_growth.to_string(), "300.00");
    let top = gain.pieces.iter().map(|p| p.high).max().unwrap();
    assert_eq!(gain.marker.as_ref().unwrap().value, top, "top of the stack");
    assert_eq!(top.to_string(), "10800.00");

    let loss = year_of(&investments, 2026);
    assert_eq!(loss.capital_growth.to_string(), "-500.00");
    let segment = loss
        .pieces
        .iter()
        .find(|p| p.series == "growth-down")
        .unwrap();
    assert!(segment.hangs);
    assert_eq!(
        loss.marker.as_ref().unwrap().value,
        segment.low,
        "the lower end of the growth segment"
    );
    assert_eq!(segment.low.to_string(), "10300.00");
}
