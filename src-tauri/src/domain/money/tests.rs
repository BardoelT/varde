use std::str::FromStr;

use proptest::prelude::*;
use rust_decimal::Decimal;

use super::*;
use crate::domain::income::YearMonth;
use crate::domain::investments::Period;

fn dec(text: &str) -> Decimal {
    Decimal::from_str(text).unwrap()
}

fn money(text: &str) -> Money {
    Money::parse(text).unwrap()
}

fn half_up(text: &str) -> String {
    Money::round_half_up(dec(text)).unwrap().to_string()
}

#[test]
fn the_limits_have_two_decimals() {
    assert_eq!(Money::MAX_PAYCHECK.to_string(), "1000000.00");
    assert_eq!(Money::MAX_BALANCE.to_string(), "1000000000.00");
}

#[test]
fn parse_rejects_non_plain_numbers() {
    for text in [
        "abc", "1e3", "1_000", "3000,50", "--1", "1.", ".5", "+5", "1.2.3",
    ] {
        assert_eq!(
            Amounts::parse("3000.00", text, "").unwrap_err(),
            DomainError::PaycheckAmountInvalid,
            "holiday text {text:?}"
        );
    }
}

#[test]
fn blank_gross_is_gross_invalid_and_blank_optionals_are_zero() {
    assert_eq!(
        Amounts::parse("  ", "", "").unwrap_err(),
        DomainError::PaycheckGrossInvalid
    );
    let amounts = Amounts::parse("1.00", " ", " ").unwrap();
    assert_eq!(amounts.total_gross().to_string(), "1.00");
}

#[test]
fn parse_accepts_whole_numbers_and_normalises_to_two_decimals() {
    let amounts = Amounts::parse("3000", "", "250.5").unwrap();
    assert_eq!(amounts.gross_salary().to_string(), "3000.00");
    assert_eq!(amounts.holiday_pay().to_string(), "0.00");
    assert_eq!(amounts.bonus().to_string(), "250.50");
}

#[test]
fn trailing_zeros_beyond_two_decimals_are_accepted() {
    let amounts = Amounts::parse("3000.500", "", "").unwrap();
    assert_eq!(amounts.gross_salary().to_string(), "3000.50");
}

#[test]
fn negative_zero_is_not_negative() {
    let amounts = Amounts::parse("3000.00", "-0.00", "").unwrap();
    assert_eq!(amounts.holiday_pay().to_string(), "0.00");
}

// Money

#[test]
fn money_has_exactly_two_decimals() {
    for (text, shown) in [
        ("3000", "3000.00"),
        ("250.5", "250.50"),
        ("-7", "-7.00"),
        ("0", "0.00"),
    ] {
        assert_eq!(money(text).to_string(), shown);
        assert_eq!(
            serde_json::to_string(&money(text)).unwrap(),
            format!("\"{shown}\"")
        );
    }
    assert_eq!(Money::ZERO.to_string(), "0.00");
}

#[test]
fn money_never_shows_negative_zero() {
    assert_eq!(money("-0.00").to_string(), "0.00");
    assert_eq!(money("-0").to_string(), "0.00");
    assert_eq!(
        Money::from_decimal(dec("-0.000")).unwrap().to_string(),
        "0.00"
    );
    assert_eq!((-Money::ZERO).to_string(), "0.00");
    assert_eq!(
        money("5").checked_sub(money("5")).unwrap().to_string(),
        "0.00"
    );
    assert_eq!(half_up("-0.004"), "0.00");
    assert!(!Money::round_half_up(dec("-0.004")).unwrap().is_negative());
}

#[test]
fn money_rejects_more_than_two_decimals_but_accepts_trailing_zeros() {
    assert_eq!(Money::parse("3000.505"), Err(MoneyError::Precision));
    assert_eq!(
        Money::from_decimal(dec("0.001")),
        Err(MoneyError::Precision)
    );
    assert_eq!(money("3000.500").to_string(), "3000.50");
    assert_eq!(money("1.000").to_string(), "1.00");
}

#[test]
fn money_parse_follows_the_plain_number_rule() {
    for text in [
        "abc", "1e3", "1_000", "3000,50", "--1", "1.", ".5", "+5", "1.2.3", "", "-",
    ] {
        assert_eq!(
            Money::parse(text),
            Err(MoneyError::Invalid),
            "text {text:?}"
        );
    }
    assert_eq!(Money::parse(&"9".repeat(40)), Err(MoneyError::Invalid));
    assert_eq!(money(" 12.5 ").to_string(), "12.50");
}

#[test]
fn money_rounds_half_up_away_from_zero() {
    assert_eq!(half_up("0.125"), "0.13");
    assert_eq!(half_up("-0.125"), "-0.13");
    assert_eq!(half_up("0.115"), "0.12");
    assert_eq!(half_up("2.675"), "2.68");
    assert_eq!(half_up("0.124999"), "0.12");
    assert_eq!(half_up("12.5"), "12.50");
    assert_eq!(half_up("3"), "3.00");
}

#[test]
fn money_serde_is_a_two_decimal_string() {
    let amount: Money = serde_json::from_str("\"3000.5\"").unwrap();
    assert_eq!(serde_json::to_string(&amount).unwrap(), "\"3000.50\"");
    assert!(serde_json::from_str::<Money>("\"1.234\"").is_err());
}

#[test]
fn money_arithmetic_is_checked() {
    assert_eq!(Money::from_decimal(Decimal::MAX), Err(MoneyError::TooLarge));
    // Just below what two decimals can hold, adding overflows to None.
    let near_limit = Decimal::from_parts(u32::MAX, u32::MAX, u32::MAX, false, 2);
    let near = Money::from_decimal(near_limit).unwrap();
    assert_eq!(near.checked_add(near), None);
    assert_eq!(near.checked_sub(-near), None);
    assert_eq!(Money::checked_sum([near, near]), None);
    assert_eq!(Money::round_half_up(Decimal::MAX), None);
}

#[test]
fn money_compares_and_sums() {
    assert!(money("-1") < money("0"));
    assert!(money("-1").is_negative() && !Money::ZERO.is_negative());
    assert!(Money::ZERO.is_zero());
    assert_eq!(money("-2.5").abs(), money("2.5"));
    assert_eq!(
        Money::checked_sum([money("1.10"), money("2.20")])
            .unwrap()
            .to_string(),
        "3.30"
    );
    assert_eq!(Money::checked_sum([]).unwrap(), Money::ZERO);
}

// Percentages

#[test]
fn percent_of_rounds_to_two_decimals_half_up() {
    let percent =
        |part: &str, whole: &str| percent_of(dec(part), dec(whole)).map(|p| p.to_string());
    assert_eq!(percent("1", "8"), Some("12.50".to_owned()));
    assert_eq!(percent("1", "3"), Some("33.33".to_owned()));
    assert_eq!(percent("2", "3"), Some("66.67".to_owned()));
    assert_eq!(percent("-1", "8"), Some("-12.50".to_owned()));
    assert_eq!(percent("1", "16"), Some("6.25".to_owned()));
    // 0.0125 percent: the midpoint goes away from zero.
    assert_eq!(percent("1", "8000"), Some("0.01".to_owned()));
    assert_eq!(percent("-1", "8000"), Some("-0.01".to_owned()));
    assert_eq!(percent("0", "5"), Some("0.00".to_owned()));
    assert_eq!(percent("-1", "100000"), Some("0.00".to_owned()));
    assert_eq!(percent("1", "0"), None);
}

// Properties

fn any_money() -> impl Strategy<Value = Money> {
    (-100_000_000_000_i64..=100_000_000_000)
        .prop_map(|cents| Money::from_decimal(Decimal::new(cents, 2)).unwrap())
}

fn any_decimal() -> impl Strategy<Value = Decimal> {
    (any::<i64>(), 0_u32..=8).prop_map(|(mantissa, scale)| Decimal::new(mantissa, scale))
}

proptest! {
    #[test]
    fn parse_of_the_displayed_amount_is_the_amount(amount in any_money()) {
        prop_assert_eq!(Money::parse(&amount.to_string()), Ok(amount));
    }

    #[test]
    fn serde_round_trips(amount in any_money()) {
        let text = serde_json::to_string(&amount).unwrap();
        prop_assert_eq!(serde_json::from_str::<Money>(&text).unwrap(), amount);
    }

    #[test]
    fn rounding_has_scale_two_and_stays_within_half_a_cent(value in any_decimal()) {
        if let Some(rounded) = Money::round_half_up(value) {
            prop_assert_eq!(rounded.value().scale(), 2);
            prop_assert!((rounded.value() - value).abs() <= dec("0.005"));
            prop_assert_ne!(rounded.to_string(), "-0.00");
        }
    }

    #[test]
    fn rounding_leaves_an_amount_unchanged(amount in any_money()) {
        prop_assert_eq!(Money::round_half_up(amount.value()), Some(amount));
    }

    #[test]
    fn sums_are_exact_and_order_independent(amounts in proptest::collection::vec(any_money(), 0..20)) {
        let forward = Money::checked_sum(amounts.iter().copied()).unwrap();
        let backward = Money::checked_sum(amounts.iter().rev().copied()).unwrap();
        let exact: Decimal = amounts.iter().map(|amount| amount.value()).sum();
        prop_assert_eq!(forward, backward);
        prop_assert_eq!(forward.value(), exact);
        prop_assert_eq!(forward.value().scale(), 2);
    }

    #[test]
    fn subtraction_undoes_addition(a in any_money(), b in any_money()) {
        let sum = a.checked_add(b).unwrap();
        prop_assert_eq!(sum.checked_sub(b), Some(a));
        prop_assert_eq!(a.checked_add(-a), Some(Money::ZERO));
    }

    #[test]
    fn percent_of_has_scale_two_and_no_negative_zero(
        part in -100_000_000_000_i64..=100_000_000_000,
        whole in prop_oneof![-100_000_000_000_i64..=-1, 1_i64..=100_000_000_000],
    ) {
        let percent = percent_of(Decimal::new(part, 2), Decimal::new(whole, 2)).unwrap();
        prop_assert_eq!(percent.scale(), 2);
        prop_assert_ne!(percent.to_string(), "-0.00");
    }

    #[test]
    fn period_sort_key_follows_year_and_month(
        a in (1_i32..=9999, 1_u8..=12),
        b in (1_i32..=9999, 1_u8..=12),
    ) {
        let first = Period::Month(YearMonth::new(a.0, a.1).unwrap());
        let second = Period::Month(YearMonth::new(b.0, b.1).unwrap());
        prop_assert_eq!(first.sort_key(), a);
        prop_assert_eq!(first.sort_key().cmp(&second.sort_key()), a.cmp(&b));
        prop_assert_eq!(YearMonth::new(a.0, a.1).unwrap().cmp(&YearMonth::new(b.0, b.1).unwrap()), a.cmp(&b));
    }
}
