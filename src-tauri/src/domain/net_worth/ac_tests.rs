//! Acceptance-criteria tests of CSD-017 that `tests.rs` does not carry.

use super::*;
use crate::domain::bank_accounts::{BankAccountInput as BankInput, BankBalance};
use crate::domain::common::Note;
use crate::domain::debts::{DebtBalance, DebtInput};
use crate::domain::houses::{HouseInput, HouseValue};
use crate::domain::investments::{Balance, InvestmentAccountInput as InvestmentInput, NetDeposits};
use crate::domain::ProjectData;

fn today() -> YearMonth {
    YearMonth::parse("2026-10").unwrap()
}

fn insights(data: &ProjectData) -> NetWorthInsights {
    net_worth_insights(
        &data.houses,
        &data.investments,
        &data.bank_accounts,
        &data.debts,
        today(),
    )
}

fn year(data: &ProjectData, wanted: i32) -> NetWorthYear {
    insights(data)
        .years
        .into_iter()
        .find(|row| row.year == wanted)
        .unwrap()
}

/// One bank account with an entry per `(period, balance)`.
fn add_cash(data: &mut ProjectData, kind: &str, entries: &[(&str, &str)]) {
    let id = data
        .bank_accounts
        .create_account(&BankInput {
            name: "Example Bank".to_owned(),
            period: Some(kind.to_owned()),
        })
        .unwrap()
        .id;
    for (at, balance) in entries {
        data.bank_accounts
            .add_entry(
                id,
                crate::domain::bank_accounts::parse_period(at).unwrap(),
                BankBalance::parse(balance).unwrap(),
                Note::default(),
            )
            .unwrap();
    }
}

fn add_investment(data: &mut ProjectData, at: &str, balance: &str) {
    let id = data
        .investments
        .create_account(&InvestmentInput {
            name: "Example Brokerage".to_owned(),
            period: Some("Yearly".to_owned()),
        })
        .unwrap()
        .id;
    data.investments
        .add_statement(
            id,
            crate::domain::common::Period::parse(at).unwrap(),
            Balance::parse(balance).unwrap(),
            NetDeposits::parse(balance).unwrap(),
            Note::default(),
        )
        .unwrap();
}

fn add_debt(data: &mut ProjectData, at: &str, balance: &str) {
    let id = data
        .debts
        .create_debt(&DebtInput {
            name: "Example Loan".to_owned(),
            period: Some("Yearly".to_owned()),
        })
        .unwrap()
        .id;
    data.debts
        .add_statement(
            id,
            crate::domain::debts::parse_period(at).unwrap(),
            DebtBalance::parse(balance).unwrap(),
            Note::default(),
        )
        .unwrap();
}

fn add_house(data: &mut ProjectData, input: &HouseInput, at: &str, value: &str) {
    let id = data.houses.create_house(input).unwrap().id;
    data.houses
        .add_valuation(
            id,
            crate::domain::houses::parse_period(at).unwrap(),
            HouseValue::parse(value).unwrap(),
            Note::default(),
        )
        .unwrap();
}

fn held_house(period: &str) -> HouseInput {
    HouseInput {
        name: "Example House".to_owned(),
        period: Some(period.to_owned()),
        purchase_date: Some("01/01/1900".to_owned()),
        purchase_amount: Some("0.00".to_owned()),
        ..HouseInput::default()
    }
}

fn text(value: &str) -> String {
    value.to_owned()
}

#[test]
fn test_csd017_ac_4_3_a_gap_year_counts_as_zero_in_the_net_worth() {
    let mut data = ProjectData::default();
    add_investment(&mut data, "2024", "20000.00");
    add_cash(
        &mut data,
        "Yearly",
        &[("2024", "5000.00"), ("2025", "5000.00")],
    );
    assert_eq!(year(&data, 2024).net_worth.to_string(), "25000.00");
    assert_eq!(year(&data, 2025).investments.to_string(), "0.00");
    assert_eq!(year(&data, 2025).net_worth.to_string(), "5000.00");
}

#[test]
fn test_csd017_ac_6_1_the_segments_rise_in_the_order_houses_investments_cash_and_debts_hang() {
    let mut data = ProjectData::default();
    add_house(&mut data, &held_house("Yearly"), "2025", "300000.00");
    add_investment(&mut data, "2025", "20000.00");
    add_cash(&mut data, "Yearly", &[("2025", "5000.00")]);
    add_debt(&mut data, "2025", "9000.00");
    let pieces: Vec<_> = year(&data, 2025)
        .pieces
        .iter()
        .map(|p| {
            (
                p.series.clone(),
                p.low.to_string(),
                p.high.to_string(),
                p.hangs,
            )
        })
        .collect();
    assert_eq!(
        pieces,
        [
            (text("houses"), text("0.00"), text("300000.00"), false),
            (
                text("investments"),
                text("300000.00"),
                text("320000.00"),
                false
            ),
            (text("cash"), text("320000.00"), text("325000.00"), false),
            (text("debts"), text("-9000.00"), text("0.00"), true),
        ]
    );
}

#[test]
fn test_csd017_ac_6_4_a_negative_net_worth_reaches_the_axis() {
    let mut data = ProjectData::default();
    add_cash(&mut data, "Yearly", &[("2025", "1000.00")]);
    add_debt(&mut data, "2025", "9000.00");
    let all = insights(&data);
    let row = all.years.iter().find(|row| row.year == 2025).unwrap();
    assert_eq!(row.net_worth.to_string(), "-8000.00");
    assert_eq!(row.debts.to_string(), "9000.00");
    let lowest = all.axis.ticks.first().unwrap().value;
    assert!(
        lowest < Money::parse("-9000.00").unwrap(),
        "the axis starts at {lowest}"
    );
    assert!(all.axis.ticks.iter().any(|tick| tick.value == Money::ZERO));
}

#[test]
fn test_csd017_ac_14_1_the_net_worth_dips_in_the_sale_year_of_a_house() {
    let mut data = ProjectData::default();
    add_house(
        &mut data,
        &HouseInput {
            sell_date: Some("10/06/2026".to_owned()),
            sell_amount: Some("300000.00".to_owned()),
            purchase_date: Some("15/03/2024".to_owned()),
            purchase_amount: Some("250000.00".to_owned()),
            ..held_house("Monthly")
        },
        "2025-12",
        "210000.00",
    );
    add_cash(
        &mut data,
        "Yearly",
        &[("2025", "20000.00"), ("2026", "20000.00")],
    );
    let (before, sale) = (year(&data, 2025), year(&data, 2026));
    assert_eq!(before.houses.to_string(), "210000.00");
    assert_eq!(before.net_worth.to_string(), "230000.00");
    assert_eq!(sale.houses.to_string(), "0.00");
    assert_eq!(sale.net_worth.to_string(), "20000.00");
}

#[test]
fn test_csd017_ac_15_1_a_later_month_of_the_current_year_counts_in_the_current_year() {
    let mut data = ProjectData::default();
    add_cash(&mut data, "Monthly", &[("2026-12", "6000.00")]);
    assert_eq!(year(&data, 2026).cash.to_string(), "6000.00");
}
