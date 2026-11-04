//! Acceptance-criteria tests of CSD-016 (house insights) that `tests.rs` does not carry under their own name.

use super::*;
use crate::domain::YearMonth;

fn today() -> YearMonth {
    YearMonth::parse("2026-10").unwrap()
}

fn input(name: &str, period: &str) -> HouseInput {
    HouseInput {
        name: name.to_owned(),
        period: Some(period.to_owned()),
        purchase_date: Some("15/03/2024".to_owned()),
        purchase_amount: Some("250000.00".to_owned()),
        ..HouseInput::default()
    }
}

fn add(houses: &mut Houses, id: HouseId, at: &str, amount: &str) {
    houses
        .add_valuation(
            id,
            parse_period(at).unwrap(),
            HouseValue::parse(amount).unwrap(),
            Note::default(),
        )
        .unwrap();
}

fn years_of(houses: &Houses) -> Vec<i32> {
    houses
        .insights(today())
        .years
        .iter()
        .map(|row| row.year)
        .collect()
}

#[test]
fn test_csd016_ac_2_2_a_single_year() {
    let mut houses = Houses::default();
    let id = houses
        .create_house(&input("Example House", "Monthly"))
        .unwrap()
        .id;
    add(&mut houses, id, "2026-09", "100000.00");
    assert_eq!(years_of(&houses), [2026]);
}

#[test]
fn test_csd016_ac_2_3_no_valuations_gives_an_empty_series() {
    let mut houses = Houses::default();
    houses
        .create_house(&input("Example House", "Monthly"))
        .unwrap();
    let insights = houses.insights(today());
    assert!(insights.years.is_empty());
    assert!(insights.houses.is_empty());
}

#[test]
fn test_csd016_ac_4_1_the_current_year_is_marked_partial_and_a_past_year_is_not() {
    let mut houses = Houses::default();
    let id = houses
        .create_house(&input("Example House", "Yearly"))
        .unwrap()
        .id;
    add(&mut houses, id, "2025", "100000.00");
    add(&mut houses, id, "2026", "110000.00");
    let years = houses.insights(today()).years;
    let partial: Vec<(i32, bool)> = years.iter().map(|row| (row.year, row.is_partial)).collect();
    assert_eq!(partial, [(2025, false), (2026, true)]);
}

#[test]
fn test_csd016_ac_5_6_a_sold_house_drops_out_in_its_sale_year() {
    let mut houses = Houses::default();
    let sold = houses
        .create_house(&HouseInput {
            sell_date: Some("10/06/2026".to_owned()),
            sell_amount: Some("320000.00".to_owned()),
            ..input("Example House", "Monthly")
        })
        .unwrap()
        .id;
    let lodge = houses
        .create_house(&input("Example Lodge", "Monthly"))
        .unwrap()
        .id;
    add(&mut houses, sold, "2025-12", "300000.00");
    add(&mut houses, sold, "2026-03", "310000.00");
    add(&mut houses, lodge, "2026-06", "50000.00");
    let years = houses.insights(today()).years;
    let segments_of = |year: i32| -> Vec<(HouseId, String)> {
        years
            .iter()
            .find(|row| row.year == year)
            .unwrap()
            .segments
            .iter()
            .map(|segment| (segment.house_id, segment.value.to_string()))
            .collect()
    };
    assert_eq!(segments_of(2025), [(sold, "300000.00".to_owned())]);
    assert_eq!(segments_of(2026), [(lodge, "50000.00".to_owned())]);
}
