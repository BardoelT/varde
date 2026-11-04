//! Acceptance-criteria tests of CSD-011 and CSD-020 that go through the commands and the project
//! file. Each test is named for the one AC it proves. Data is synthetic.

use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};
use tauri::Manager;
use tempfile::TempDir;

use crate::commands::AppState;
use crate::ipc_tests::{harness, Harness};
use crate::storage::Project;

fn debt_input(name: &str, period: &str) -> Value {
    json!({ "name": name, "period": period })
}

/// A saved project with "Example Mortgage" (monthly) and its statement for 2026-09 of
/// 150000.00. Returns the harness, the debt id, the folder and the saved path.
fn saved_project() -> (Harness, Value, TempDir, PathBuf) {
    let app = harness();
    let debt = app.ok(
        "create_debt",
        json!({ "input": debt_input("Example Mortgage", "Monthly") }),
    );
    app.ok(
        "add_debt_statement",
        json!({ "debtId": debt["id"], "period": "2026-09", "balance": "150000.00" }),
    );
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    app.window
        .state::<AppState>()
        .project()
        .unwrap()
        .save_as(path.clone())
        .unwrap();
    assert_eq!(
        app.ok("project_status", json!({}))["unsaved_changes"],
        false
    );
    (app, debt, dir, path)
}

#[test]
fn test_csd011_ac_12_9_an_edit_of_a_statement_is_an_unsaved_change() {
    let (app, debt, _dir, _path) = saved_project();
    app.ok(
        "update_debt_statement",
        json!({
            "debtId": debt["id"], "period": "2026-09", "newPeriod": "2026-08",
            "balance": "150000.00", "note": ""
        }),
    );
    assert_eq!(app.ok("project_status", json!({}))["unsaved_changes"], true);
}

#[test]
fn test_csd011_ac_13_4_an_invalid_value_rejects_the_whole_edit() {
    let (app, debt, _dir, _path) = saved_project();
    let error = app
        .call(
            "update_debt_statement",
            json!({
                "debtId": debt["id"], "period": "2026-09", "newPeriod": "2026-08",
                "balance": "-1.00", "note": ""
            }),
        )
        .expect_err("a negative balance is rejected");
    assert_eq!(error["code"], "DEBT_STATEMENT_BALANCE_NEGATIVE");
    let rows = app.ok("list_debt_statements", json!({}));
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(rows[0]["period"], "2026-09");
    assert_eq!(rows[0]["balance"], "150000.00");
    assert_eq!(
        app.ok("project_status", json!({}))["unsaved_changes"],
        false
    );
}

/// The saved file of a project with one debt, as JSON, after `change`, opened again.
fn open_changed_file(change: impl FnOnce(&mut Value)) -> Result<(), &'static str> {
    let (_app, _debt, _dir, path) = saved_project();
    let mut value: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    change(&mut value);
    fs::write(&path, value.to_string()).unwrap();
    Project::new().open(&path).map_err(|error| error.code())
}

#[test]
fn test_csd011_ac_23_2_a_file_without_the_debts_section_is_rejected() {
    assert_eq!(
        open_changed_file(|_| {}),
        Ok(()),
        "the untouched file opens"
    );
    let result = open_changed_file(|value| {
        value.as_object_mut().unwrap().remove("debts").unwrap();
    });
    assert_eq!(result, Err("PROJECT_INVALID_FILE"));
}

#[test]
fn test_csd020_ac_33_3_a_debt_without_a_category_is_an_invalid_file() {
    let result = open_changed_file(|value| {
        let debt = &mut value["debts"]["debts"][0];
        assert!(
            debt.get("category").is_some(),
            "the saved debt has a category"
        );
        debt.as_object_mut().unwrap().remove("category");
    });
    assert_eq!(result, Err("PROJECT_INVALID_FILE"));
}

#[test]
fn test_csd019_ac_12_3_selling_a_house_is_an_unsaved_change() {
    let app = harness();
    let house = app.ok(
        "create_house",
        json!({ "input": {
            "name": "Example House", "period": "Monthly",
            "purchase_date": "15/03/2024", "purchase_amount": "250000.00",
            "sell_date": "", "sell_amount": ""
        } }),
    );
    let dir = TempDir::new().unwrap();
    app.window
        .state::<AppState>()
        .project()
        .unwrap()
        .save_as(dir.path().join("example-a.varde"))
        .unwrap();
    assert_eq!(
        app.ok("project_status", json!({}))["unsaved_changes"],
        false,
        "a saved project with a held house"
    );
    app.ok(
        "update_house",
        json!({ "houseId": house["id"], "input": {
            "name": "Example House", "period": "Monthly",
            "purchase_date": "15/03/2024", "purchase_amount": "250000.00",
            "sell_date": "10/06/2026", "sell_amount": "300000.00"
        } }),
    );
    assert_eq!(app.ok("project_status", json!({}))["unsaved_changes"], true);
}
