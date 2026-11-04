//! Acceptance-criteria tests of CSD-018 that `tests.rs` does not carry (rounding, the settled debts of a sold house, free income without a wealth accumulation).

use super::*;
use crate::domain::bank_accounts::{BankAccountInput as BankInput, BankBalance};
use crate::domain::common::Note;
use crate::domain::debts::{debt_note, parse_period, DebtBalance, DebtId, DebtInput};
use crate::domain::expenses::ExpenseInput;
use crate::domain::houses::linking::{self, DebtFormInput, DebtLinksInput, SettlementChoice};
use crate::domain::houses::{HouseId, HouseInput, HouseValue};
use crate::domain::income::JobInput;
use crate::domain::money::Amounts;
use crate::domain::ProjectData;

fn today() -> YearMonth {
    YearMonth::parse("2026-10").unwrap()
}

fn dec(text: &str) -> Money {
    Money::parse(text).unwrap()
}

fn row(data: &ProjectData, wanted: i32) -> RatiosYear {
    ratios(
        &RatioSources {
            income: &data.income,
            expenses: &data.expenses,
            houses: &data.houses,
            investments: &data.investments,
            bank_accounts: &data.bank_accounts,
            debts: &data.debts,
        },
        today(),
    )
    .years
    .into_iter()
    .find(|year| year.year == wanted)
    .unwrap()
}

fn add_income(data: &mut ProjectData, year: &str, gross: &str) {
    let job = data
        .income
        .create_job(&JobInput {
            employer: "Example Employer".to_owned(),
            title: None,
            start_date: Some("01/01/2020".to_owned()),
            end_date: None,
            period: Some("Yearly".to_owned()),
        })
        .unwrap()
        .id;
    data.income
        .add_statement(
            job,
            crate::domain::common::Year::parse(year).unwrap(),
            Amounts::parse(gross, "", "").unwrap(),
        )
        .unwrap();
}

fn add_expense(data: &mut ProjectData, on: &str, amount: &str) {
    data.expenses
        .add_expense(&ExpenseInput {
            name: "Example Bike".to_owned(),
            amount: amount.to_owned(),
            date: on.to_owned(),
            category: "Other".to_owned(),
            note: None,
        })
        .unwrap();
}

/// A yearly bank account with 1000.00 in 2024 and `closing` in 2025.
fn add_bank(data: &mut ProjectData, closing: &str) {
    let id = data
        .bank_accounts
        .create_account(&BankInput {
            name: "Example Bank".to_owned(),
            period: Some("Yearly".to_owned()),
        })
        .unwrap()
        .id;
    for (at, balance) in [("2024", "1000.00"), ("2025", closing)] {
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

// REQ-6: rounding half-up

#[test]
fn test_csd018_ac_6_4_rounding_half_up() {
    // accumulation, income, percent: the examples of the scenario outline.
    let examples = [
        ("1001.00", "800.00", "0.13"),
        ("999.00", "800.00", "-0.13"),
        ("999.00", "48000.00", "0.00"),
    ];
    for (closing, income, percent) in examples {
        let mut data = ProjectData::default();
        add_bank(&mut data, closing);
        add_income(&mut data, "2025", income);
        let year = row(&data, 2025);
        assert_eq!(
            year.wealth_percent.map(|value| value.to_string()),
            Some(percent.to_owned()),
            "{closing} on {income}"
        );
    }
}

// REQ-8: a decrease that rounds to zero

#[test]
fn test_csd018_ac_8_5_a_decrease_that_rounds_to_zero_shows_no_sign() {
    let mut data = ProjectData::default();
    let id = data
        .houses
        .create_house(&HouseInput {
            name: "Example House".to_owned(),
            period: Some("Yearly".to_owned()),
            purchase_date: Some("01/01/1900".to_owned()),
            purchase_amount: Some("0.00".to_owned()),
            ..HouseInput::default()
        })
        .unwrap()
        .id;
    for (at, value) in [("2025", "100000.00"), ("2026", "99999.00")] {
        data.houses
            .add_valuation(
                id,
                crate::domain::houses::parse_period(at).unwrap(),
                HouseValue::parse(value).unwrap(),
                Note::default(),
            )
            .unwrap();
    }
    let year = row(&data, 2026);
    assert_eq!(
        year.wealth_growth_percent.map(|value| value.to_string()),
        Some("0.00".to_owned())
    );
    assert_eq!(year.wealth_growth_percent_label, "0.00%");
}

// REQ-9: free income without a wealth accumulation

#[test]
fn test_csd018_ac_9_4_no_wealth_accumulation_gives_no_disposable_income() {
    let mut data = ProjectData::default();
    add_income(&mut data, "2024", "48000.00");
    add_expense(&mut data, "05/03/2024", "4000.00");
    let year = row(&data, 2024);
    assert_eq!(year.wealth_accumulation, None);
    assert_eq!(year.disposable_income, None);
    assert_eq!(year.disposable_percent, None);
}

// REQ-5: the debts of a sold house

fn debt(data: &mut ProjectData) -> DebtId {
    data.debts
        .create_debt(&DebtInput {
            name: "Example Mortgage".to_owned(),
            period: Some("Monthly".to_owned()),
            ..DebtInput::example()
        })
        .unwrap()
        .id
}

fn statement(data: &mut ProjectData, id: DebtId, at: &str, balance: &str) {
    data.debts
        .add_statement(
            id,
            parse_period(at).unwrap(),
            DebtBalance::parse(balance).unwrap(),
            debt_note("").unwrap(),
        )
        .unwrap();
}

fn house_input(sold_on: &str) -> HouseInput {
    HouseInput {
        name: "Example House".to_owned(),
        period: Some("Monthly".to_owned()),
        purchase_date: Some("15/03/2024".to_owned()),
        purchase_amount: Some("250000.00".to_owned()),
        sell_date: Some(sold_on.to_owned()),
        sell_amount: Some(if sold_on.is_empty() {
            String::new()
        } else {
            "300000.00".to_owned()
        }),
    }
}

/// The mortgage with the given statements, linked to a house that is sold on 10/06/2026 with `outcome` as the settlement.
fn sold_with(outcome: &str, statements: &[(&str, &str)]) -> ProjectData {
    let mut data = ProjectData::default();
    let mortgage = debt(&mut data);
    for (at, balance) in statements {
        statement(&mut data, mortgage, at, balance);
    }
    let house: HouseId = linking::create_house(
        &mut data.houses,
        &mut data.debts,
        &house_input(""),
        &DebtLinksInput {
            settlements: vec![],
            confirm_undo: false,
        },
    )
    .unwrap()
    .id;
    let name = data.debts.get(mortgage).unwrap().name.clone();
    linking::update_debt(
        &data.houses,
        &mut data.debts,
        mortgage,
        &DebtFormInput {
            debt: DebtInput {
                name,
                period: None,
                ..DebtInput::example()
            },
            category: Some("Mortgage".to_owned()),
            house_id: Some(house.to_string()),
            settlement: None,
            ..Default::default()
        },
    )
    .unwrap();
    linking::update_house(
        &mut data.houses,
        &mut data.debts,
        house,
        &house_input("10/06/2026"),
        &DebtLinksInput {
            settlements: vec![SettlementChoice {
                debt_id: mortgage.to_string(),
                outcome: outcome.to_owned(),
            }],
            confirm_undo: false,
        },
    )
    .unwrap();
    data
}

fn wealth_2026(data: &ProjectData) -> Option<Money> {
    row(data, 2026).wealth_accumulation
}

#[test]
fn test_csd018_ac_5_6_a_repaid_mortgage_counts_in_full() {
    let data = sold_with(
        "Repaid",
        &[("2025-12", "200000.00"), ("2026-03", "190000.00")],
    );
    assert_eq!(wealth_2026(&data), Some(dec("200000.00")));
}

#[test]
fn test_csd018_ac_5_7_a_cancelled_mortgage_does_not_count_as_cash_repaid() {
    let data = sold_with(
        "Cancelled",
        &[("2025-12", "200000.00"), ("2026-03", "190000.00")],
    );
    assert_eq!(wealth_2026(&data), Some(dec("10000.00")));
}

#[test]
fn test_csd018_ac_5_8_a_debt_cancelled_in_the_year_it_started_adds_nothing() {
    let data = sold_with("Cancelled", &[("2026-02", "5000.00")]);
    assert_eq!(wealth_2026(&data), None);
}
