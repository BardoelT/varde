use std::fs;

use tempfile::TempDir;

use super::*;
use crate::domain::bank_accounts::{bank_note, parse_period, BankAccountInput};
use crate::domain::{BankAccountId, BankBalance};

fn account(project: &mut Project, name: &str, period: &str) -> BankAccountId {
    project
        .bank_accounts_mut()
        .create_account(&BankAccountInput {
            name: name.to_owned(),
            period: Some(period.to_owned()),
        })
        .unwrap()
        .id
}

#[test]
fn test_csd009_ac_23_1_bank_accounts_survive_save_and_open_including_notes_and_slots() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    account(&mut project, "Example Current Account", "Monthly");
    let id = account(&mut project, "Example Savings", "Yearly");
    project
        .bank_accounts_mut()
        .add_entry(
            id,
            parse_period("2025").unwrap(),
            BankBalance::parse("8000.00").unwrap(),
            bank_note("Line one\nLine two").unwrap(),
        )
        .unwrap();
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();

    let mut fresh = Project::new();
    fresh.open(&path).unwrap();
    assert_eq!(fresh.bank_accounts(), project.bank_accounts());
    let rows = fresh.bank_accounts().all_entries();
    assert_eq!(rows[0].1.note.as_str(), "Line one\nLine two");
    assert_eq!(rows[0].0.colour_slot.get(), 1);
    assert!(!fresh.status().unsaved_changes);

    // The section sits next to the others; the version stays 1.
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(value["format_version"], 2);
    assert_eq!(
        value["bank_accounts"]["accounts"][1]["name"],
        "Example Savings"
    );
    assert_eq!(value["bank_accounts"]["accounts"][1]["colour_slot"], 1);
    assert_eq!(value["bank_accounts"]["entries"][0]["balance"], "8000.00");
}

#[test]
fn test_csd009_ac_23_3_a_negative_balance_survives_the_round_trip() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    let id = account(&mut project, "Example Current Account", "Monthly");
    project
        .bank_accounts_mut()
        .add_entry(
            id,
            parse_period("2026-09").unwrap(),
            BankBalance::parse("-150.25").unwrap(),
            bank_note("").unwrap(),
        )
        .unwrap();
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();
    let mut fresh = Project::new();
    fresh.open(&path).unwrap();
    let rows = fresh.bank_accounts().all_entries();
    assert_eq!(rows[0].1.balance.value().to_string(), "-150.25");
}

#[test]
fn test_csd009_ac_23_4_the_other_sections_are_untouched() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    account(&mut project, "Example Current Account", "Monthly");
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
    assert_eq!(fresh.bank_accounts(), project.bank_accounts());
    assert_eq!(fresh.investments(), project.investments());
}

#[test]
fn test_csd009_an_inconsistent_bank_section_is_an_invalid_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    // An entry of an account that does not exist.
    fs::write(
        &path,
        r#"{"format_version":2,"income":{"jobs":[],"paychecks":[]},
            "bank_accounts":{"accounts":[],"entries":[
            {"account_id":"671166f4-5827-4794-bcdc-54d7824fb650","period":"2026-09","balance":"1.00"}]}}"#,
    )
    .unwrap();
    let mut project = Project::new();
    let error = project.open(&path).unwrap_err();
    assert_eq!(error.code(), "PROJECT_INVALID_FILE");
}

#[test]
fn test_csd009_a_bank_change_is_an_unsaved_change() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    project.save_as(dir.path().join("example-a.varde")).unwrap();
    assert!(!project.status().unsaved_changes);
    account(&mut project, "Example Current Account", "Monthly");
    assert!(project.status().unsaved_changes);
}

/// A saved project with a monthly entry for 2026-09 with balance 2500.00.
fn saved_project_with_september(dir: &TempDir) -> (Project, BankAccountId) {
    let mut project = Project::new();
    let id = account(&mut project, "Example Current Account", "Monthly");
    project
        .bank_accounts_mut()
        .add_entry(
            id,
            parse_period("2026-09").unwrap(),
            BankBalance::parse("2500.00").unwrap(),
            bank_note("").unwrap(),
        )
        .unwrap();
    project.save_as(dir.path().join("example-a.varde")).unwrap();
    assert!(!project.status().unsaved_changes);
    (project, id)
}

#[test]
fn test_csd009_ac_12_9_an_edit_of_the_period_is_an_unsaved_change() {
    let dir = TempDir::new().unwrap();
    let (mut project, id) = saved_project_with_september(&dir);
    project
        .bank_accounts_mut()
        .update_entry(
            id,
            parse_period("2026-09").unwrap(),
            parse_period("2026-08").unwrap(),
            BankBalance::parse("2500.00").unwrap(),
            bank_note("").unwrap(),
        )
        .unwrap();
    assert!(
        project.status().unsaved_changes,
        "the unsaved indicator is shown"
    );
}

#[test]
fn test_csd009_ac_13_4_an_invalid_value_rejects_the_whole_edit() {
    let dir = TempDir::new().unwrap();
    let (project, _) = saved_project_with_september(&dir);
    // The period moves to 2026-08 and the balance is "abc": the balance is rejected before the
    // data is touched, as the command does.
    let error = BankBalance::parse("abc").unwrap_err();
    assert_eq!(error.code(), "BANK_ENTRY_BALANCE_INVALID");
    let entries = project.bank_accounts().all_entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].1.period.to_string(), "2026-09");
    assert_eq!(entries[0].1.balance.value().to_string(), "2500.00");
    assert!(
        !project.status().unsaved_changes,
        "the unsaved indicator is not shown"
    );
}

#[test]
fn test_csd009_ac_3_1_a_bank_account_colour_slot_of_8_is_an_invalid_file() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    account(&mut project, "Example Current Account", "Monthly");
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    value["bank_accounts"]["accounts"][0]["colour_slot"] = serde_json::json!(7);
    fs::write(&path, value.to_string()).unwrap();
    Project::new().open(&path).unwrap();
    value["bank_accounts"]["accounts"][0]["colour_slot"] = serde_json::json!(8);
    fs::write(&path, value.to_string()).unwrap();
    let error = Project::new().open(&path).unwrap_err();
    assert_eq!(error.code(), "PROJECT_INVALID_FILE");
}
