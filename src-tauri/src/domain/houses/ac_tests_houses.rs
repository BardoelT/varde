//! Acceptance-criteria tests of CSD-015 that `tests.rs` proves inside a test of another AC (or not at all): one small test each.

use super::*;

fn input(name: &str, period: &str) -> HouseInput {
    HouseInput {
        name: name.to_owned(),
        period: Some(period.to_owned()),
        purchase_date: Some("01/01/1900".to_owned()),
        purchase_amount: Some("0.00".to_owned()),
        ..HouseInput::default()
    }
}

fn new_house(houses: &mut Houses, name: &str, period: &str) -> HouseId {
    houses.create_house(&input(name, period)).unwrap().id
}

fn add(houses: &mut Houses, id: HouseId, at: &str, amount: &str, note: &str) {
    houses
        .add_valuation(
            id,
            parse_period(at).unwrap(),
            HouseValue::parse(amount).unwrap(),
            house_note(note).unwrap(),
        )
        .unwrap();
}

#[test]
fn test_csd015_ac_1_4_a_house_always_has_a_purchase() {
    let mut houses = Houses::default();
    let error = houses
        .create_house(&HouseInput {
            name: "Example House".to_owned(),
            period: Some("Monthly".to_owned()),
            ..HouseInput::default()
        })
        .unwrap_err();
    assert_eq!(error.code(), "HOUSE_PURCHASE_DATE_REQUIRED");
    assert!(houses.houses().is_empty(), "no house exists");
}

#[test]
fn test_csd015_ac_9_2_a_yearly_valuation_for_a_monthly_house_is_rejected() {
    let mut houses = Houses::default();
    let monthly = new_house(&mut houses, "Example House", "Monthly");
    let error = houses
        .add_valuation(
            monthly,
            parse_period("2025").unwrap(),
            HouseValue::parse("1000.00").unwrap(),
            Note::default(),
        )
        .unwrap_err();
    assert_eq!(error.code(), "HOUSE_PERIOD_MISMATCH");
    assert_eq!(houses.valuation_count(monthly), 0);
}

#[test]
fn test_csd015_ac_11_2_line_breaks_in_a_note_are_kept() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2026-09", "1000.00", "line one\nline two");
    let note = houses.all_valuations()[0].1.note.as_str().to_owned();
    assert_eq!(note, "line one\nline two");
    assert_eq!(note.lines().count(), 2);
}

#[test]
fn test_csd015_ac_12_2_emptying_the_note_clears_it() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2026-09", "300000.00", "First");
    houses
        .update_valuation(
            id,
            parse_period("2026-09").unwrap(),
            parse_period("2026-09").unwrap(),
            HouseValue::parse("300000.00").unwrap(),
            house_note("").unwrap(),
        )
        .unwrap();
    let rows = houses.all_valuations();
    assert_eq!(rows[0].1.note.as_str(), "");
    assert_eq!(rows[0].1.value.value().to_string(), "300000.00");
}

#[test]
fn test_csd015_ac_17_6_an_older_valuation_does_not_change_the_latest_value() {
    let mut houses = Houses::default();
    let id = new_house(&mut houses, "Example House", "Monthly");
    add(&mut houses, id, "2026-09", "300000.00", "");
    add(&mut houses, id, "2026-01", "280000.00", "");
    let overview = houses.overview();
    assert_eq!(
        overview.houses[0].latest_period.unwrap().to_string(),
        "2026-09"
    );
    assert_eq!(
        overview.houses[0].latest_value.unwrap().to_string(),
        "300000.00"
    );
    assert_eq!(overview.total.unwrap().to_string(), "300000.00");
}

#[test]
fn test_csd015_ac_18_3_a_yearly_valuation_counts_as_december() {
    let mut houses = Houses::default();
    let monthly = new_house(&mut houses, "Example House", "Monthly");
    let yearly = new_house(&mut houses, "Example Lodge", "Yearly");
    add(&mut houses, monthly, "2025-11", "1.00", "");
    add(&mut houses, monthly, "2025-12", "1.00", "");
    add(&mut houses, yearly, "2025", "1.00", "");
    let order: Vec<String> = houses
        .all_valuations()
        .iter()
        .map(|(house, valuation)| format!("{} {}", house.name, valuation.period))
        .collect();
    assert_eq!(
        order,
        [
            "Example House 2025-12",
            "Example Lodge 2025",
            "Example House 2025-11"
        ]
    );
}
