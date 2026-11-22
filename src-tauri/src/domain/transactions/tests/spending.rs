use super::*;

use crate::domain::transactions::{SpendingPeriod, UNCATEGORIZED_ENTITY};

fn period(text: &str) -> SpendingPeriod {
    SpendingPeriod::parse(text).unwrap()
}

#[test]
fn test_csd032_ac_35_1_shares_for_a_year_are_the_debits_per_category() {
    let data = insights_fixture();
    let groceries = category(&data, "Groceries").to_string();
    let shares = data.spending_shares(period("2026"));
    assert_eq!(shares.total, money("22.50"), "the credit is not spending");
    let slices: Vec<(&str, Money, &str)> = shares
        .slices
        .iter()
        .map(|slice| {
            (
                slice.id.as_str(),
                slice.amount,
                slice.percent_label.as_str(),
            )
        })
        .collect();
    assert_eq!(
        slices,
        [
            (groceries.as_str(), money("15.00"), "66.67%"),
            (UNCATEGORIZED_ENTITY, money("7.50"), "33.33%"),
        ]
    );
    assert_eq!(shares.slices.last().unwrap().colour_slot.get(), 7);
}

#[test]
fn test_csd032_ac_35_2_shares_for_a_month_and_the_periods_offered() {
    let data = insights_fixture();
    let october = data.spending_shares(period("2026-10"));
    assert_eq!(october.total, money("22.50"));
    let december = data.spending_shares(period("2024-12"));
    assert_eq!(december.total, money("20.00"));
    assert_eq!(december.slices.len(), 1);
    assert_eq!(december.slices[0].percent_label, "100.00%");
    let periods = data.spending_periods();
    assert_eq!(periods.years, [2026, 2024]);
    let months: Vec<String> = periods.months.iter().map(ToString::to_string).collect();
    assert_eq!(months, ["2026-10", "2024-12"]);
}

#[test]
fn test_csd032_ac_35_3_a_year_with_only_credits_is_not_offered() {
    let mut data = with_rules(&[]);
    let rows = [
        simple(
            "01-03-2025",
            "Bijschrijvingen",
            "100,00",
            "Example Employer",
            "R1",
        ),
        simple("01-03-2026", "Afschrijvingen", "9,00", "Example Shop", "R2"),
    ];
    import_to_first(&mut data, &export(&rows), Currency::Eur).unwrap();
    assert_eq!(data.spending_periods().years, [2026]);
    let none = data.spending_shares(period("2025"));
    assert_eq!(none.slices.len(), 0);
    assert_eq!(none.total, money("0.00"));
}

#[test]
fn test_csd032_ac_35_8_percentages_are_rounded_on_their_own() {
    let mut data = with_rules(&[
        ("Example A", "Groceries"),
        ("Example B", "Leisure"),
        ("Example C", "Other"),
    ]);
    let rows = [
        simple("01-03-2026", "Afschrijvingen", "10,00", "Example A", "R1"),
        simple("02-03-2026", "Afschrijvingen", "10,00", "Example B", "R2"),
        simple("03-03-2026", "Afschrijvingen", "10,00", "Example C", "R3"),
    ];
    import_to_first(&mut data, &export(&rows), Currency::Eur).unwrap();
    let shares = data.spending_shares(period("2026-03"));
    let labels: Vec<&str> = shares
        .slices
        .iter()
        .map(|slice| slice.percent_label.as_str())
        .collect();
    assert_eq!(
        labels,
        ["33.33%", "33.33%", "33.33%"],
        "not adjusted to 100"
    );
}

#[test]
fn test_csd032_ac_35_period_text_is_validated() {
    for bad in ["", "26", "2026-13", "2026-1", "abcd", "2026-10-01", "0000"] {
        assert!(SpendingPeriod::parse(bad).is_err(), "{bad}");
    }
    assert_eq!(
        SpendingPeriod::parse(" 2026 ").unwrap(),
        SpendingPeriod::Year(2026)
    );
}

#[test]
fn test_csd032_ac_35_5_slices_carry_their_place_on_the_ring() {
    let data = insights_fixture();
    let shares = data.spending_shares(period("2026"));
    let ring: Vec<(&str, &str)> = shares
        .slices
        .iter()
        .map(|slice| (slice.start.as_str(), slice.end.as_str()))
        .collect();
    assert_eq!(ring, [("0.000000", "0.666667"), ("0.666667", "1.000000")]);
    let none = data.spending_shares(period("2023"));
    assert_eq!(none.slices.len(), 0);
}
