use std::fs;

use tempfile::TempDir;

use super::*;
use crate::domain::debts::{debt_note, parse_period, DebtBalance, DebtInput};
use crate::domain::DebtId;

fn debt(project: &mut Project, name: &str, period: &str) -> DebtId {
    project
        .debts_mut()
        .create_debt(&DebtInput {
            name: name.to_owned(),
            period: Some(period.to_owned()),
            ..DebtInput::example()
        })
        .unwrap()
        .id
}

#[test]
fn test_csd011_ac_23_1_debts_survive_save_and_open_including_notes_and_slots() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    debt(&mut project, "Example Mortgage", "Monthly");
    let id = debt(&mut project, "Example Student Loan", "Yearly");
    project
        .debts_mut()
        .add_statement(
            id,
            parse_period("2025").unwrap(),
            DebtBalance::parse("8000.00").unwrap(),
            debt_note("Line one\nLine two").unwrap(),
        )
        .unwrap();
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();

    let mut fresh = Project::new();
    fresh.open(&path).unwrap();
    assert_eq!(fresh.debts(), project.debts());
    let rows = fresh.debts().all_statements();
    assert_eq!(rows[0].1.note.as_str(), "Line one\nLine two");
    assert_eq!(rows[0].0.colour_slot.get(), 1);
    assert!(!fresh.status().unsaved_changes);

    // The section sits next to the others; the version stays 1.
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(value["format_version"], 1);
    assert_eq!(value["debts"]["debts"][1]["name"], "Example Student Loan");
    assert_eq!(value["debts"]["debts"][1]["colour_slot"], 1);
    assert_eq!(value["debts"]["statements"][0]["balance"], "8000.00");
}

#[test]
fn test_csd011_ac_23_3_a_zero_balance_survives_the_round_trip() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    let id = debt(&mut project, "Example Mortgage", "Monthly");
    project
        .debts_mut()
        .add_statement(
            id,
            parse_period("2026-09").unwrap(),
            DebtBalance::parse("0.00").unwrap(),
            debt_note("").unwrap(),
        )
        .unwrap();
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();
    let mut fresh = Project::new();
    fresh.open(&path).unwrap();
    let rows = fresh.debts().all_statements();
    assert_eq!(rows[0].1.balance.value().to_string(), "0.00");
}

#[test]
fn test_csd011_ac_23_4_the_other_sections_are_untouched() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    debt(&mut project, "Example Mortgage", "Monthly");
    project
        .bank_accounts_mut()
        .create_account(&crate::domain::bank_accounts::BankAccountInput {
            name: "Example Current Account".to_owned(),
            period: None,
        })
        .unwrap();
    project
        .investments_mut()
        .create_account(&crate::domain::InvestmentAccountInput {
            name: "Example Brokerage".to_owned(),
            period: None,
        })
        .unwrap();
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();
    let mut fresh = Project::new();
    fresh.open(&path).unwrap();
    assert_eq!(fresh.debts(), project.debts());
    assert_eq!(fresh.bank_accounts(), project.bank_accounts());
    assert_eq!(fresh.investments(), project.investments());
}

#[test]
fn test_csd011_ac_23_5_a_damaged_section_is_an_invalid_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    // A statement of a debt that does not exist.
    fs::write(
        &path,
        r#"{"format_version":1,"income":{"jobs":[],"paychecks":[]},
            "debts":{"debts":[],"statements":[
            {"debt_id":"671166f4-5827-4794-bcdc-54d7824fb650","period":"2026-09","balance":"1.00"}]}}"#,
    )
    .unwrap();
    let mut project = Project::new();
    let error = project.open(&path).unwrap_err();
    assert_eq!(error.code(), "PROJECT_INVALID_FILE");
}

#[test]
fn test_csd011_a_debt_change_is_an_unsaved_change() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    project.save_as(dir.path().join("example-a.varde")).unwrap();
    assert!(!project.status().unsaved_changes);
    debt(&mut project, "Example Mortgage", "Monthly");
    assert!(project.status().unsaved_changes);
}

#[test]
fn test_csd011_ac_3_1_a_debt_colour_slot_of_8_is_an_invalid_file() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    debt(&mut project, "Example Mortgage", "Monthly");
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    value["debts"]["debts"][0]["colour_slot"] = serde_json::json!(7);
    fs::write(&path, value.to_string()).unwrap();
    Project::new().open(&path).unwrap();
    value["debts"]["debts"][0]["colour_slot"] = serde_json::json!(8);
    fs::write(&path, value.to_string()).unwrap();
    let error = Project::new().open(&path).unwrap_err();
    assert_eq!(error.code(), "PROJECT_INVALID_FILE");
}
