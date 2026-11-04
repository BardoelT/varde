//! The opening and the closing of a debt in the project file (CSD-011 REQ-23,
//! CSD-020 REQ-33).

use std::fs;

use serde_json::Value;
use tempfile::TempDir;

use super::*;
use crate::domain::debts::{ClosingInput, DebtInput};
use crate::domain::houses::linking::{self, DebtFormInput};

fn debt_form(date: &str, amount: &str) -> DebtFormInput {
    DebtFormInput {
        debt: DebtInput {
            name: "Example Mortgage".to_owned(),
            period: None,
            opening_date: date.to_owned(),
            opening_amount: amount.to_owned(),
        },
        ..DebtFormInput::default()
    }
}

fn saved(form: &DebtFormInput) -> (TempDir, std::path::PathBuf, Project) {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    let (houses, debts) = project.houses_and_debts_mut();
    linking::create_debt(houses, debts, form).unwrap();
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();
    (dir, path, project)
}

fn open_error_without(keys: &[&str]) -> String {
    let (_dir, path, _) = saved(&debt_form("15/03/2020", "200000.00"));
    let mut value: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    for key in keys {
        value["debts"]["debts"][0]
            .as_object_mut()
            .unwrap()
            .remove(*key);
    }
    fs::write(&path, value.to_string()).unwrap();
    let mut fresh = Project::new();
    fresh.open(&path).unwrap_err().code().to_owned()
}

#[test]
fn test_csd011_ac_23_7_the_opening_survives_the_round_trip() {
    let (_dir, path, project) = saved(&debt_form("15/03/2020", "200000.00"));
    let mut fresh = Project::new();
    fresh.open(&path).unwrap();
    assert_eq!(fresh.debts(), project.debts());
    let value: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(value["format_version"], 1);
    assert_eq!(value["debts"]["debts"][0]["opening_date"], "2020-03-15");
    assert_eq!(value["debts"]["debts"][0]["opening_amount"], "200000.00");
}

#[test]
fn test_csd011_ac_23_6_a_debt_without_an_opening_is_an_invalid_file() {
    assert_eq!(
        open_error_without(&["opening_date", "opening_amount"]),
        "PROJECT_INVALID_FILE"
    );
}

#[test]
fn test_csd020_ac_33_5_a_debt_without_an_opening_date_is_an_invalid_file() {
    assert_eq!(
        open_error_without(&["opening_date"]),
        "PROJECT_INVALID_FILE"
    );
    assert_eq!(
        open_error_without(&["opening_amount"]),
        "PROJECT_INVALID_FILE"
    );
}

#[test]
fn test_csd020_ac_33_4_a_manual_closing_without_a_house_link_is_valid() {
    let mut form = debt_form("15/03/2020", "200000.00");
    form.closing = ClosingInput {
        date: "10/06/2026".to_owned(),
        amount: "0.00".to_owned(),
        outcome: "Repaid".to_owned(),
    };
    let (_dir, path, project) = saved(&form);
    let value: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        value["debts"]["debts"][0]["settlement"]["date"],
        "2026-06-10"
    );
    assert!(value["debts"]["debts"][0].get("house_id").is_none());
    let mut fresh = Project::new();
    fresh.open(&path).unwrap();
    assert_eq!(fresh.debts(), project.debts());
}

#[test]
fn test_csd011_ac_32_a_statement_before_the_opening_is_an_invalid_file() {
    let (_dir, path, _) = saved(&debt_form("15/03/2020", "200000.00"));
    let mut value: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let id = value["debts"]["debts"][0]["id"].clone();
    value["debts"]["statements"] = serde_json::json!([
        { "debt_id": id, "period": "2020-02", "balance": "1.00", "note": "" }
    ]);
    fs::write(&path, value.to_string()).unwrap();
    let mut fresh = Project::new();
    assert_eq!(
        fresh.open(&path).unwrap_err().code(),
        "PROJECT_INVALID_FILE"
    );
}

#[test]
fn test_csd011_ac_31_a_closing_before_the_opening_is_an_invalid_file() {
    let mut form = debt_form("15/03/2020", "200000.00");
    form.closing = ClosingInput {
        date: "10/06/2026".to_owned(),
        amount: "0.00".to_owned(),
        outcome: "Repaid".to_owned(),
    };
    let (_dir, path, _) = saved(&form);
    let mut value: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    value["debts"]["debts"][0]["opening_date"] = "2027-01-01".into();
    fs::write(&path, value.to_string()).unwrap();
    let mut fresh = Project::new();
    assert_eq!(
        fresh.open(&path).unwrap_err().code(),
        "PROJECT_INVALID_FILE"
    );
}

#[test]
fn test_csd020_ac_33_6_a_settlement_that_does_not_match_the_sale_is_an_invalid_file() {
    use crate::domain::houses::linking::{DebtLinksInput, SettlementChoice};
    use crate::domain::houses::HouseInput;
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    let house_input = HouseInput {
        name: "Example House".to_owned(),
        purchase_date: Some("15/03/2024".to_owned()),
        purchase_amount: Some("250000.00".to_owned()),
        ..HouseInput::default()
    };
    let (houses, debts) = project.houses_and_debts_mut();
    let house = linking::create_house(houses, debts, &house_input, &DebtLinksInput::default())
        .unwrap()
        .id;
    let mut loan = debt_form("15/03/2020", "10000.00");
    loan.category = Some("Loan".to_owned());
    loan.house_id = Some(house.to_string());
    let (houses, debts) = project.houses_and_debts_mut();
    let loan_id = linking::create_debt(houses, debts, &loan).unwrap().id;
    project
        .debts_mut()
        .add_statement(
            loan_id,
            crate::domain::debts::parse_period("2026-03").unwrap(),
            crate::domain::debts::DebtBalance::parse("3500.00").unwrap(),
            crate::domain::debts::debt_note("").unwrap(),
        )
        .unwrap();
    let sold = HouseInput {
        sell_date: Some("10/06/2026".to_owned()),
        sell_amount: Some("300000.00".to_owned()),
        ..house_input
    };
    let choices = DebtLinksInput {
        settlements: vec![SettlementChoice {
            debt_id: loan_id.to_string(),
            outcome: "Cancelled".to_owned(),
        }],
        ..DebtLinksInput::default()
    };
    let (houses, debts) = project.houses_and_debts_mut();
    linking::update_house(houses, debts, house, &sold, &choices).unwrap();
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();
    let mut value: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    value["debts"]["debts"][0]["settlement"]["date"] = "2026-06-11".into();
    fs::write(&path, value.to_string()).unwrap();
    let mut fresh = Project::new();
    assert_eq!(
        fresh.open(&path).unwrap_err().code(),
        "PROJECT_INVALID_FILE"
    );
}
