//! Tests of CSD-004 (the dashboard). Names are `test_csd004_ac_<n>_<m>_…`.

use time::macros::date;

use super::*;
use crate::domain::bank_accounts::{BankAccountInput as BankInput, BankBalance};
use crate::domain::common::{Note, Year};
use crate::domain::debts::{DebtBalance, DebtInput};
use crate::domain::expenses::ExpenseInput;
use crate::domain::houses::{HouseInput, HouseValue};
use crate::domain::income::JobInput;
use crate::domain::investments::{Balance, InvestmentAccountInput as InvestmentInput, NetDeposits};
use crate::domain::money::Amounts;

fn today() -> Date {
    date!(2026 - 11 - 09)
}

fn month(text: &str) -> YearMonth {
    YearMonth::parse(text).unwrap()
}

fn money(text: &str) -> Money {
    Money::parse(text).unwrap()
}

fn period(text: &str) -> Period {
    Period::parse(text).unwrap()
}

fn job(data: &mut ProjectData, yearly: bool) -> crate::domain::income::JobId {
    data.income
        .create_job(&JobInput {
            employer: "Example Employer".to_owned(),
            title: None,
            start_date: Some("01/01/2020".to_owned()),
            end_date: None,
            period: Some(if yearly { "Yearly" } else { "Monthly" }.to_owned()),
        })
        .unwrap()
        .id
}

fn paychecks(data: &mut ProjectData, rows: &[(&str, &str)]) {
    let id = job(data, false);
    for (at, gross) in rows {
        data.income
            .add_paycheck(id, month(at), Amounts::parse(gross, "", "").unwrap())
            .unwrap();
    }
}

fn statement(data: &mut ProjectData, year: &str, gross: &str) {
    let id = job(data, true);
    data.income
        .add_statement(
            id,
            Year::parse(year).unwrap(),
            Amounts::parse(gross, "", "").unwrap(),
        )
        .unwrap();
}

fn expense(data: &mut ProjectData, on: &str, amount: &str) {
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

fn bank(data: &mut ProjectData, name: &str, yearly: bool, entries: &[(&str, &str)]) {
    let id = data
        .bank_accounts
        .create_account(&BankInput {
            name: name.to_owned(),
            period: Some(if yearly { "Yearly" } else { "Monthly" }.to_owned()),
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

fn investment(data: &mut ProjectData, entries: &[(&str, &str)]) {
    let id = data
        .investments
        .create_account(&InvestmentInput {
            name: "Example Brokerage".to_owned(),
            period: Some("Monthly".to_owned()),
        })
        .unwrap()
        .id;
    for (at, balance) in entries {
        data.investments
            .add_statement(
                id,
                period(at),
                Balance::parse(balance).unwrap(),
                NetDeposits::parse("0.00").unwrap(),
                Note::default(),
            )
            .unwrap();
    }
}

fn debt(data: &mut ProjectData, entries: &[(&str, &str)]) {
    let id = data
        .debts
        .create_debt(&DebtInput {
            name: "Example Loan".to_owned(),
            period: Some("Monthly".to_owned()),
            ..DebtInput::example()
        })
        .unwrap()
        .id;
    for (at, balance) in entries {
        data.debts
            .add_statement(
                id,
                crate::domain::debts::parse_period(at).unwrap(),
                DebtBalance::parse(balance).unwrap(),
                Note::default(),
            )
            .unwrap();
    }
}

fn house(data: &mut ProjectData, bought: &str, sold: Option<&str>, valuations: &[(&str, &str)]) {
    let id = data
        .houses
        .create_house(&HouseInput {
            name: format!("Example House {bought}"),
            period: Some("Monthly".to_owned()),
            purchase_date: Some(bought.to_owned()),
            purchase_amount: Some("100000.00".to_owned()),
            sell_date: sold.map(str::to_owned),
            sell_amount: sold.map(|_| "100000.00".to_owned()),
        })
        .unwrap()
        .id;
    for (at, value) in valuations {
        data.houses
            .add_valuation(
                id,
                crate::domain::houses::parse_period(at).unwrap(),
                HouseValue::parse(value).unwrap(),
                Note::default(),
            )
            .unwrap();
    }
}

fn now(data: &ProjectData) -> Dashboard {
    dashboard(data, today())
}

// REQ-8: tiles

#[test]
fn test_csd004_ac_8_1_every_area_has_a_tile_in_the_payload() {
    let json = serde_json::to_value(now(&ProjectData::default())).unwrap();
    let keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    for key in [
        "net_worth",
        "income",
        "expenses",
        "bank_accounts",
        "investments",
        "houses",
        "debts",
    ] {
        assert!(keys.contains(&key), "missing {key}");
    }
    // The seven tiles of the dashboard, and no Ratios tile (CSD-004 NG-8).
    assert_eq!(keys.len(), 7);
}

// REQ-9: headline figures

#[test]
fn test_csd004_ac_9_1_income_year_to_date() {
    let mut data = ProjectData::default();
    paychecks(
        &mut data,
        &[
            ("2026-09", "3000.00"),
            ("2026-10", "3000.00"),
            ("2026-12", "3000.00"),
            ("2025-10", "3000.00"),
        ],
    );
    assert_eq!(now(&data).income.value, Some(money("6000.00")));
}

#[test]
fn test_csd004_ac_9_2_an_annual_statement_counts_in_full() {
    let mut data = ProjectData::default();
    statement(&mut data, "2026", "40000.00");
    paychecks(&mut data, &[("2026-09", "3000.00")]);
    assert_eq!(now(&data).income.value, Some(money("43000.00")));
}

#[test]
fn test_csd004_ac_9_3_expenses_year_to_date() {
    let mut data = ProjectData::default();
    expense(&mut data, "10/09/2026", "900.00");
    expense(&mut data, "12/11/2026", "80.00");
    expense(&mut data, "03/10/2025", "1200.00");
    assert_eq!(now(&data).expenses.value, Some(money("900.00")));
    // The day of today itself counts.
    expense(&mut data, "09/11/2026", "10.00");
    assert_eq!(now(&data).expenses.value, Some(money("910.00")));
}

#[test]
fn test_csd004_ac_9_4_balances_show_the_value_now_of_the_area() {
    let mut data = ProjectData::default();
    bank(
        &mut data,
        "Example Bank A",
        false,
        &[("2026-09", "4000.00"), ("2026-10", "5000.00")],
    );
    bank(&mut data, "Example Bank B", true, &[("2025", "2000.00")]);
    debt(
        &mut data,
        &[("2026-09", "160000.00"), ("2026-10", "150000.00")],
    );
    let board = now(&data);
    assert_eq!(board.bank_accounts.value, Some(money("7000.00")));
    assert_eq!(board.debts.value, Some(money("150000.00")));
}

#[test]
fn test_csd004_ac_9_5_a_sold_house_is_not_in_the_value_of_the_houses() {
    let mut data = ProjectData::default();
    house(&mut data, "01/01/2020", None, &[("2026-10", "300000.00")]);
    house(
        &mut data,
        "01/01/2019",
        Some("01/06/2026"),
        &[("2026-05", "250000.00")],
    );
    assert_eq!(now(&data).houses.value, Some(money("300000.00")));
}

#[test]
fn test_csd004_ac_9_5_only_sold_houses_give_zero() {
    let mut data = ProjectData::default();
    house(
        &mut data,
        "01/01/2019",
        Some("01/06/2026"),
        &[("2026-05", "250000.00")],
    );
    let tile = now(&data).houses;
    assert!(tile.has_data);
    assert_eq!(tile.value, Some(money("0.00")));
}

#[test]
fn test_csd004_ac_9_6_net_worth_adds_up_the_areas() {
    let mut data = ProjectData::default();
    house(&mut data, "01/01/2020", None, &[("2026-10", "300000.00")]);
    investment(&mut data, &[("2026-10", "20000.00")]);
    bank(&mut data, "Example Bank", false, &[("2026-10", "7000.00")]);
    debt(&mut data, &[("2026-10", "150000.00")]);
    assert_eq!(now(&data).net_worth.value, Some(money("177000.00")));
}

// REQ-10: comparison

#[test]
fn test_csd004_ac_10_1_income_in_the_same_period_last_year() {
    let mut data = ProjectData::default();
    paychecks(
        &mut data,
        &[
            ("2025-09", "2800.00"),
            ("2025-10", "2800.00"),
            ("2025-11", "2800.00"),
            ("2025-12", "2800.00"),
        ],
    );
    assert_eq!(
        data.income.gross_in_year_up_to(2025, month("2025-11")),
        money("8400.00")
    );
}

#[test]
fn test_csd004_ac_10_2_expenses_up_to_the_comparison_point() {
    let mut data = ProjectData::default();
    expense(&mut data, "09/11/2025", "100.00");
    expense(&mut data, "10/11/2025", "50.00");
    assert_eq!(
        expenses_between(&data, 2025, date!(2025 - 11 - 09)),
        money("100.00")
    );
    // End to end: 100.00 last year against nothing this year is -100.00%.
    let tile = now(&data).expenses;
    assert_eq!(tile.change.unwrap().percent_label, "-100.00%");
}

#[test]
fn test_csd004_ac_10_3_a_balance_at_the_comparison_point() {
    let mut data = ProjectData::default();
    bank(
        &mut data,
        "Example Bank",
        false,
        &[
            ("2025-10", "4000.00"),
            ("2025-11", "4500.00"),
            ("2025-12", "6000.00"),
        ],
    );
    let then = snapshot(&data.bank_accounts, Some(month("2025-11")), |_| true).unwrap();
    assert_eq!(then.total, money("4500.00"));
}

#[test]
fn test_csd004_ac_10_4_a_yearly_record_counts_from_its_year_end() {
    let mut data = ProjectData::default();
    bank(
        &mut data,
        "Example Bank",
        true,
        &[("2024", "1800.00"), ("2025", "2000.00")],
    );
    let then = snapshot(&data.bank_accounts, Some(month("2025-11")), |_| true).unwrap();
    assert_eq!(then.total, money("1800.00"));
}

#[test]
fn test_csd004_ac_10_5_29_february_looks_back_to_28_february() {
    assert_eq!(
        comparison_date(date!(2028 - 02 - 29)),
        Some(date!(2027 - 02 - 28))
    );
    assert_eq!(
        comparison_date(date!(2026 - 11 - 09)),
        Some(date!(2025 - 11 - 09))
    );
    let mut data = ProjectData::default();
    expense(&mut data, "28/02/2027", "100.00");
    expense(&mut data, "01/03/2027", "50.00");
    assert_eq!(
        expenses_between(&data, 2027, date!(2027 - 02 - 28)),
        money("100.00")
    );
}

#[test]
fn test_csd004_ac_10_6_houses_held_at_the_comparison_point() {
    let mut data = ProjectData::default();
    house(
        &mut data,
        "01/03/2025",
        Some("15/12/2025"),
        &[("2025-10", "280000.00")],
    );
    house(&mut data, "01/12/2025", None, &[("2025-12", "90000.00")]);
    let point = date!(2025 - 11 - 09);
    let then = snapshot(&data.houses, Some(month("2025-11")), |h| held_on(h, point)).unwrap();
    assert_eq!(then.total, money("280000.00"));
}

#[test]
fn test_csd004_ac_10_7_net_worth_at_the_comparison_point() {
    let mut data = ProjectData::default();
    house(
        &mut data,
        "01/01/2020",
        None,
        &[("2025-11", "280000.00"), ("2026-10", "300000.00")],
    );
    investment(
        &mut data,
        &[("2025-11", "15000.00"), ("2026-10", "20000.00")],
    );
    bank(
        &mut data,
        "Example Bank",
        false,
        &[("2025-11", "4500.00"), ("2026-10", "7000.00")],
    );
    debt(
        &mut data,
        &[("2025-11", "160000.00"), ("2026-10", "150000.00")],
    );
    // 177000.00 now against 139500.00 then is +26.88%.
    let change = now(&data).net_worth.change.unwrap();
    assert_eq!(change.percent_label, "+26.88%");
}

// REQ-11: change indicator

fn changed(value: &str, comparison: &str, polarity: Polarity) -> Option<Change> {
    change(Some(money(value)), Some(money(comparison)), polarity)
}

#[test]
fn test_csd004_ac_11_1_up() {
    let change = changed("10500.00", "10000.00", Polarity::UpIsGood).unwrap();
    assert_eq!(change.percent_label, "+5.00%");
    assert_eq!(change.direction, Direction::Up);
}

#[test]
fn test_csd004_ac_11_2_down() {
    let change = changed("9750.00", "10000.00", Polarity::DownIsGood).unwrap();
    assert_eq!(change.percent_label, "-2.50%");
    assert_eq!(change.direction, Direction::Down);
}

#[test]
fn test_csd004_ac_11_3_stable_when_the_change_rounds_to_zero() {
    let change = changed("10000.04", "10000.00", Polarity::UpIsGood).unwrap();
    assert_eq!(change.percent_label, "0.00%");
    assert_eq!(change.direction, Direction::Stable);
    assert_eq!(change.sentiment, Sentiment::Neutral);
}

#[test]
fn test_csd004_ac_11_4_a_negative_comparison_uses_its_absolute_value() {
    let change = changed("500.00", "-1000.00", Polarity::UpIsGood).unwrap();
    assert_eq!(change.percent_label, "+150.00%");
    assert_eq!(change.direction, Direction::Up);
}

#[test]
fn test_csd004_ac_11_5_a_comparison_of_zero_gives_no_change() {
    assert_eq!(changed("3000.00", "0.00", Polarity::UpIsGood), None);
}

#[test]
fn test_csd004_ac_11_6_no_data_last_year_keeps_the_headline() {
    let mut data = ProjectData::default();
    debt(&mut data, &[("2026-10", "150000.00")]);
    let tile = now(&data).debts;
    assert_eq!(tile.value, Some(money("150000.00")));
    assert_eq!(tile.change, None);
}

// REQ-12: colour

#[test]
fn test_csd004_ac_12_1_more_income_is_good() {
    let change = changed("10500.00", "10000.00", Polarity::UpIsGood).unwrap();
    assert_eq!(change.sentiment, Sentiment::Good);
}

#[test]
fn test_csd004_ac_12_2_more_expenses_are_bad() {
    let change = changed("10500.00", "10000.00", Polarity::DownIsGood).unwrap();
    assert_eq!(change.sentiment, Sentiment::Bad);
}

#[test]
fn test_csd004_ac_12_3_less_debt_is_good() {
    let change = changed("9000.00", "10000.00", Polarity::DownIsGood).unwrap();
    assert_eq!(change.sentiment, Sentiment::Good);
    let worse = changed("9000.00", "10000.00", Polarity::UpIsGood).unwrap();
    assert_eq!(worse.sentiment, Sentiment::Bad);
}

#[test]
fn test_csd004_ac_12_4_stable_is_neutral() {
    let change = changed("10000.00", "10000.00", Polarity::DownIsGood).unwrap();
    assert_eq!(change.sentiment, Sentiment::Neutral);
}

// REQ-13: as-of period

#[test]
fn test_csd004_ac_13_1_the_as_of_period_of_a_balance_tile() {
    let mut data = ProjectData::default();
    bank(&mut data, "Example Bank A", false, &[("2026-10", "100.00")]);
    bank(&mut data, "Example Bank B", true, &[("2025", "100.00")]);
    let tile = now(&data).bank_accounts;
    assert_eq!(
        tile.as_of.map(|p| p.to_string()),
        Some("2026-10".to_owned())
    );
    // Net worth takes the latest of its areas.
    assert_eq!(
        now(&data).net_worth.as_of.map(|p| p.to_string()),
        Some("2026-10".to_owned())
    );
}

#[test]
fn test_csd004_ac_13_2_flow_tiles_have_no_as_of_period() {
    let mut data = ProjectData::default();
    paychecks(&mut data, &[("2026-10", "100.00")]);
    expense(&mut data, "10/09/2026", "10.00");
    assert_eq!(now(&data).income.as_of, None);
    assert_eq!(now(&data).expenses.as_of, None);
}

// REQ-14: no data yet

#[test]
fn test_csd004_ac_14_1_an_empty_project_has_no_data_on_every_tile() {
    let board = now(&ProjectData::default());
    for tile in [
        &board.net_worth,
        &board.income,
        &board.expenses,
        &board.bank_accounts,
        &board.investments,
        &board.houses,
        &board.debts,
    ] {
        assert!(!tile.has_data);
        assert_eq!(tile.value, None);
        assert_eq!(tile.change, None);
    }
}

#[test]
fn test_csd004_ac_14_2_a_job_without_paychecks_has_no_data() {
    let mut data = ProjectData::default();
    job(&mut data, false);
    assert!(!now(&data).income.has_data);
}

#[test]
fn test_csd004_ac_14_3_records_only_last_year_give_zero_and_minus_100_percent() {
    let mut data = ProjectData::default();
    paychecks(&mut data, &[("2025-09", "5600.00")]);
    let tile = now(&data).income;
    assert_eq!(tile.value, Some(money("0.00")));
    let change = tile.change.unwrap();
    assert_eq!(change.percent_label, "-100.00%");
    assert_eq!(change.direction, Direction::Down);
}

#[test]
fn test_csd004_ac_14_1_the_tile_wire_shape() {
    let mut data = ProjectData::default();
    paychecks(&mut data, &[("2026-10", "6000.00")]);
    let json = serde_json::to_value(now(&data).income).unwrap();
    assert_eq!(json["has_data"], true);
    assert_eq!(json["value"], "6000.00");
    assert!(json["as_of"].is_null());
    assert!(json["change"].is_null());
}
