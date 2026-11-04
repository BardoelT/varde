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
    json!({ "name": name, "period": period, "opening_date": "01/01/2000", "opening_amount": "200000.00" })
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
    // The statement and the Opening row of the debt (CSD-011 REQ-33).
    assert_eq!(rows.as_array().unwrap().len(), 2);
    assert_eq!(rows[0]["period"], "2026-09");
    assert_eq!(rows[0]["balance"], "150000.00");
    assert_eq!(rows[1]["opening"], true);
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

/// "Example Mortgage" with 200000.00 at 2025-12 and 190000.00 at 2026-03, linked to a house
/// sold on 10/06/2026 and settled with `outcome` (CSD-026 REQ-9, REQ-10).
fn sold_mortgage(outcome: &str) -> Harness {
    let app = harness();
    let house = app.ok(
        "create_house",
        json!({ "input": { "name": "Example House", "period": "Monthly", "purchase_date": "15/03/2024", "purchase_amount": "250000.00" } }),
    );
    let mortgage = app.ok(
        "create_debt",
        json!({ "input": { "opening_date": "01/01/2000", "opening_amount": "200000.00", "name": "Example Mortgage", "period": "Monthly", "category": "Mortgage", "house_id": house["id"] } }),
    );
    for (period, balance) in [("2025-12", "200000.00"), ("2026-03", "190000.00")] {
        app.ok(
            "add_debt_statement",
            json!({ "debtId": mortgage["id"], "period": period, "balance": balance }),
        );
    }
    app.ok(
        "update_house",
        json!({
            "houseId": house["id"],
            "input": {
                "name": "Example House",
                "purchase_date": "15/03/2024",
                "purchase_amount": "250000.00",
                "sell_date": "10/06/2026",
                "sell_amount": "300000.00"
            },
            "links": { "settlements": [ { "debt_id": mortgage["id"], "outcome": outcome } ] }
        }),
    );
    app
}

fn repaid_2026(app: &Harness) -> Value {
    let totals = app.ok("yearly_debt_totals", json!({}));
    let total = totals
        .as_array()
        .unwrap()
        .iter()
        .find(|total| total["year"] == 2026)
        .unwrap()
        .clone();
    let insights = app.ok("get_debt_insights", json!({}));
    let year = insights["years"]
        .as_array()
        .unwrap()
        .iter()
        .find(|year| year["year"] == 2026)
        .unwrap()
        .clone();
    assert_eq!(total["repaid"], year["repaid"], "one repaid figure");
    total["repaid"].clone()
}

#[test]
fn test_csd011_ac_19_12_a_cancelled_debt_shows_the_cancelled_adjusted_repaid_amount() {
    let app = sold_mortgage("Cancelled");
    assert_eq!(repaid_2026(&app), "10000.00");
}

#[test]
fn test_csd012_ac_5_6_the_insights_repaid_is_cancelled_adjusted() {
    let app = sold_mortgage("Cancelled");
    let insights = app.ok("get_debt_insights", json!({}));
    let year = insights["years"]
        .as_array()
        .unwrap()
        .iter()
        .find(|year| year["year"] == 2026)
        .unwrap();
    assert_eq!(year["repaid"], "10000.00");
}

#[test]
fn test_csd026_ac_11_1_the_debts_overview_shows_the_cancelled_adjusted_repaid_amount() {
    let app = sold_mortgage("Cancelled");
    let totals = app.ok("yearly_debt_totals", json!({}));
    let year = totals
        .as_array()
        .unwrap()
        .iter()
        .find(|total| total["year"] == 2026)
        .unwrap();
    assert_eq!(year["repaid"], "10000.00");
}

#[test]
fn test_csd018_ac_5_9_the_ratios_repaid_amount_equals_the_debts_overview() {
    let app = sold_mortgage("Cancelled");
    assert_eq!(repaid_2026(&app), "10000.00");
    // No bank or investment figure: the wealth accumulation is the repaid amount alone.
    let ratios = app.ok("get_ratios", json!({}));
    let year = ratios["years"]
        .as_array()
        .unwrap()
        .iter()
        .find(|year| year["year"] == 2026)
        .unwrap();
    assert_eq!(year["wealth_accumulation"], "10000.00");
}
