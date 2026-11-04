//! Acceptance-criteria tests of CSD-011, CSD-012, CSD-019 and CSD-026 that need a sold house
//! with settled debts, and that the other test files do not carry under their AC id.
//! Each test is named for the one AC it proves.

use crate::domain::common::YearMonth;
use crate::domain::debts::{debt_note, parse_period, DebtBalance, DebtId, DebtInput};
use crate::domain::houses::linking::{self, DebtFormInput, DebtLinksInput, SettlementChoice};
use crate::domain::houses::{HouseId, HouseInput, HouseValue};
use crate::domain::money::Money;
use crate::domain::net_worth::net_worth_insights;
use crate::domain::ProjectData;

fn today() -> YearMonth {
    YearMonth::parse("2026-10").unwrap()
}

fn money(text: &str) -> Money {
    Money::parse(text).unwrap()
}

fn debt(data: &mut ProjectData, name: &str, kind: &str) -> DebtId {
    data.debts
        .create_debt(&DebtInput {
            name: name.to_owned(),
            period: Some(kind.to_owned()),
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

fn house_input(name: &str, bought: (&str, &str), sold: (&str, &str)) -> HouseInput {
    HouseInput {
        name: name.to_owned(),
        period: Some("Monthly".to_owned()),
        purchase_date: Some(bought.0.to_owned()),
        purchase_amount: Some(bought.1.to_owned()),
        sell_date: Some(sold.0.to_owned()),
        sell_amount: Some(sold.1.to_owned()),
    }
}

fn links(choices: &[(DebtId, &str)]) -> DebtLinksInput {
    DebtLinksInput {
        settlements: choices
            .iter()
            .map(|(id, outcome)| SettlementChoice {
                debt_id: id.to_string(),
                outcome: (*outcome).to_owned(),
            })
            .collect(),
        confirm_undo: false,
    }
}

/// Creates a held "Example House" and links the debts to it as Loans.
fn held_house(data: &mut ProjectData, linked: &[DebtId]) -> HouseId {
    let house = linking::create_house(
        &mut data.houses,
        &mut data.debts,
        &house_input("Example House", ("15/03/2024", "250000.00"), ("", "")),
        &links(&[]),
    )
    .unwrap()
    .id;
    for id in linked {
        let name = data.debts.get(*id).unwrap().name.clone();
        linking::update_debt(
            &data.houses,
            &mut data.debts,
            *id,
            &DebtFormInput {
                debt: DebtInput { name, period: None },
                category: Some("Loan".to_owned()),
                house_id: Some(house.to_string()),
                settlement: None,
            },
        )
        .unwrap();
    }
    house
}

/// Sells the house on `date` for 300000.00 and settles the debts with the outcomes given.
fn sell_house(data: &mut ProjectData, id: HouseId, day: &str, choices: &[(DebtId, &str)]) {
    linking::update_house(
        &mut data.houses,
        &mut data.debts,
        id,
        &house_input(
            "Example House",
            ("15/03/2024", "250000.00"),
            (day, "300000.00"),
        ),
        &links(choices),
    )
    .unwrap();
}

/// The repaid amount of the yearly totals of `year`, as text, or "none".
fn repaid(data: &ProjectData, year: i32) -> String {
    data.debts
        .yearly_totals()
        .into_iter()
        .find(|total| total.year == year)
        .and_then(|total| total.repaid)
        .map_or_else(|| "none".to_owned(), |value| value.to_string())
}

/// "Example Mortgage" with a statement 2025-12 of 200000.00 and 2026-03 of 190000.00, linked to
/// a house that is sold on 10/06/2026 with the outcome given (settled amount 190000.00).
fn mortgage_settled(outcome: &str) -> (ProjectData, DebtId) {
    let mut data = ProjectData::default();
    let mortgage = debt(&mut data, "Example Mortgage", "Monthly");
    statement(&mut data, mortgage, "2025-12", "200000.00");
    statement(&mut data, mortgage, "2026-03", "190000.00");
    let house = held_house(&mut data, &[mortgage]);
    sell_house(&mut data, house, "10/06/2026", &[(mortgage, outcome)]);
    (data, mortgage)
}

// ---- CSD-011 ----

#[test]
fn test_csd011_ac_8_3_a_settlement_statement_shares_its_period_with_a_regular_statement() {
    let mut data = ProjectData::default();
    let loan = debt(&mut data, "Example Loan", "Monthly");
    statement(&mut data, loan, "2026-06", "3500.00");
    let house = held_house(&mut data, &[loan]);
    sell_house(&mut data, house, "10/06/2026", &[(loan, "Repaid")]);

    let june = parse_period("2026-06").unwrap();
    let rows: Vec<(String, bool, bool)> = data
        .debts
        .all_statements()
        .into_iter()
        .filter(|(d, s)| d.id == loan && s.period == june)
        .map(|(_, s)| {
            (
                s.balance.value().to_string(),
                s.is_settlement(),
                data.debts.statement_locked(s),
            )
        })
        .collect();
    // Newest first: the settlement statement stands after the regular one in time.
    assert_eq!(
        rows,
        [
            ("0.00".to_owned(), true, true),
            ("3500.00".to_owned(), false, false),
        ]
    );

    let again = data.debts.add_statement(
        loan,
        june,
        DebtBalance::parse("3000.00").unwrap(),
        debt_note("").unwrap(),
    );
    assert_eq!(again.unwrap_err().code(), "DEBT_STATEMENT_DUPLICATE");
}

#[test]
fn test_csd011_ac_19_13_a_repaid_debt_counts_in_full() {
    let (data, _) = mortgage_settled("Repaid");
    assert_eq!(repaid(&data, 2026), "200000.00");
}

// ---- CSD-019 ----

#[test]
fn test_csd019_ac_1_3_an_edit_keeps_the_purchase_required() {
    let mut data = ProjectData::default();
    let id = linking::create_house(
        &mut data.houses,
        &mut data.debts,
        &house_input("Example House", ("15/03/2024", "250000.00"), ("", "")),
        &links(&[]),
    )
    .unwrap()
    .id;
    let result = linking::update_house(
        &mut data.houses,
        &mut data.debts,
        id,
        &house_input("Example Home", ("", "250000.00"), ("", "")),
        &links(&[]),
    );
    assert_eq!(result.unwrap_err().code(), "HOUSE_PURCHASE_DATE_REQUIRED");
    assert_eq!(data.houses.get(id).unwrap().name, "Example House");
}

#[test]
fn test_csd019_ac_11_2_the_sale_amount_does_not_replace_the_house() {
    let mut data = ProjectData::default();
    let id = linking::create_house(
        &mut data.houses,
        &mut data.debts,
        &house_input(
            "Example House",
            ("15/03/2024", "250000.00"),
            ("10/06/2026", "300000.00"),
        ),
        &links(&[]),
    )
    .unwrap()
    .id;
    data.houses
        .add_valuation(
            id,
            crate::domain::houses::parse_period("2025-12").unwrap(),
            HouseValue::parse("210000.00").unwrap(),
            crate::domain::common::Note::default(),
        )
        .unwrap();
    let insights = net_worth_insights(
        &data.houses,
        &data.investments,
        &data.bank_accounts,
        &data.debts,
        today(),
    );
    let year = |n: i32| insights.years.iter().find(|y| y.year == n).unwrap();
    assert_eq!(year(2025).net_worth, money("210000.00"));
    assert_eq!(year(2026).net_worth, money("0.00"));
    assert_ne!(year(2026).net_worth, money("300000.00"));
}

// ---- CSD-026 ----

#[test]
fn test_csd026_ac_11_2_no_opening_balance_means_no_repaid_amount() {
    let mut data = ProjectData::default();
    let loan = debt(&mut data, "Example Loan", "Monthly");
    statement(&mut data, loan, "2026-02", "5000.00");
    let house = held_house(&mut data, &[loan]);
    sell_house(&mut data, house, "10/06/2026", &[(loan, "Cancelled")]);
    assert_eq!(
        data.debts
            .get(loan)
            .unwrap()
            .settlement()
            .unwrap()
            .amount
            .value(),
        money("5000.00")
    );
    assert_eq!(repaid(&data, 2026), "none");
}
