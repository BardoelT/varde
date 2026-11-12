//! Acceptance tests of CSD-032 (transactions) through the real commands: the
//! names, the argument names and the shapes the frontend relies on. The native
//! file dialog is replaced by a path. Synthetic data only.

use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};
use tauri::Manager;
use tempfile::TempDir;

use crate::commands::transactions::import_flow;
use crate::commands::AppState;
use crate::ipc_tests::{harness, Harness};

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("example-bank-october.csv")
}

fn import(app: &Harness, path: PathBuf) -> Result<Option<(usize, usize)>, Value> {
    let state = app.window.state::<AppState>();
    import_flow(&state, || Some(path))
        .map(|summary| summary.map(|found| (found.added, found.skipped)))
        .map_err(|error| serde_json::to_value(&error).unwrap())
}

fn rows(app: &Harness) -> Vec<Value> {
    app.ok("list_transactions", json!({}))["rows"]
        .as_array()
        .unwrap()
        .clone()
}

fn category_id(app: &Harness, name: &str) -> String {
    app.ok("list_transaction_categories", json!({}))
        .as_array()
        .unwrap()
        .iter()
        .find(|category| category["name"] == name)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn row_of<'a>(rows: &'a [Value], name: &str) -> &'a Value {
    rows.iter()
        .find(|row| row["counterparty_name"] == name)
        .unwrap()
}

#[test]
fn test_csd032_ac_1_1_importing_adds_the_transactions_and_marks_the_project_unsaved() {
    let app = harness();
    assert_eq!(
        app.ok("project_status", json!({}))["unsaved_changes"],
        false
    );
    assert_eq!(import(&app, fixture()).unwrap(), Some((8, 0)));
    assert_eq!(rows(&app).len(), 8);
    assert_eq!(app.ok("project_status", json!({}))["unsaved_changes"], true);
}

#[test]
fn test_csd032_ac_1_2_cancelling_the_dialog_changes_nothing() {
    let app = harness();
    let state = app.window.state::<AppState>();
    assert!(import_flow(&state, || None).unwrap().is_none());
    assert_eq!(
        app.ok("project_status", json!({}))["unsaved_changes"],
        false
    );
    assert_eq!(rows(&app).len(), 0);
}

#[test]
fn test_csd032_ac_1_3_an_import_changes_nothing_outside_transactions() {
    let app = harness();
    app.ok(
        "add_expense",
        json!({ "input": { "name": "Example Bike", "amount": "900.00", "date": "10/09/2026", "category": "Hobby", "note": null } }),
    );
    let before = app.ok("list_expenses", json!({}));
    import(&app, fixture()).unwrap();
    assert_eq!(app.ok("list_expenses", json!({})), before);
}

#[test]
fn test_csd032_ac_3_3_an_unreadable_file_is_reported_without_its_path() {
    let app = harness();
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("missing.csv");
    let error = import(&app, missing).unwrap_err();
    assert_eq!(error["code"], "TRANSACTION_IMPORT_UNREADABLE");
    assert!(!error["message"].as_str().unwrap().contains('/'));
    let binary = dir.path().join("binary.csv");
    fs::write(&binary, [0xff, 0xfe, 0x00, 0x80]).unwrap();
    assert_eq!(
        import(&app, binary).unwrap_err()["code"],
        "TRANSACTION_IMPORT_UNREADABLE"
    );
}

#[test]
fn test_csd032_ac_4_1_another_currency_imports_nothing() {
    let app = harness();
    app.ok("set_currency", json!({ "currency": "USD" }));
    let error = import(&app, fixture()).unwrap_err();
    assert_eq!(error["code"], "TRANSACTION_IMPORT_CURRENCY");
    assert_eq!(rows(&app).len(), 0);
}

#[test]
fn test_csd032_ac_5_1_importing_the_same_file_again_skips_everything() {
    let app = harness();
    import(&app, fixture()).unwrap();
    assert_eq!(import(&app, fixture()).unwrap(), Some((0, 8)));
    assert_eq!(rows(&app).len(), 8);
}

#[test]
fn test_csd032_ac_6_1_an_import_is_one_undo_step() {
    let app = harness();
    import(&app, fixture()).unwrap();
    let status = app.ok("project_status", json!({}));
    assert_eq!(status["undo_label"], "Import Transactions");
    let undone = app.ok("undo", json!({}));
    assert_eq!(undone["step"], "Import Transactions");
    assert_eq!(rows(&app).len(), 0);
    assert_eq!(
        app.ok("project_status", json!({}))["redo_label"],
        "Import Transactions"
    );
    app.ok("redo", json!({}));
    assert_eq!(rows(&app).len(), 8);
}

#[test]
fn test_csd032_ac_6_2_a_rejected_import_records_no_step() {
    let app = harness();
    let dir = TempDir::new().unwrap();
    let bad = dir.path().join("bad.csv");
    fs::write(&bad, "Datum;Bedrag\r\n").unwrap();
    assert_eq!(
        import(&app, bad).unwrap_err()["code"],
        "TRANSACTION_IMPORT_FORMAT"
    );
    assert_eq!(
        app.ok("project_status", json!({}))["undo_label"],
        Value::Null
    );
}

#[test]
fn test_csd032_ac_7_1_the_unused_columns_are_not_stored() {
    let app = harness();
    import(&app, fixture()).unwrap();
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example.varde");
    app.with_project(|project| project.save_as(path.clone()).unwrap());
    let text = fs::read_to_string(path).unwrap();
    assert!(!text.contains("MANDATE-1") && !text.contains("CRED-1"));
}

#[test]
fn test_csd032_ac_10_1_the_seed_rules_categorise_known_chains_on_import() {
    let app = harness();
    import(&app, fixture()).unwrap();
    let all = rows(&app);
    assert_eq!(
        row_of(&all, "Albert Heijn Example")["category_id"],
        category_id(&app, "Groceries")
    );
    assert_eq!(
        row_of(&all, "Netflix Example")["category_id"],
        category_id(&app, "Subscriptions")
    );
    assert_eq!(row_of(&all, "Example Landlord")["category_id"], Value::Null);
}

#[test]
fn test_csd032_ac_11_1_a_manual_category_is_sticky_over_ipc() {
    let app = harness();
    import(&app, fixture()).unwrap();
    let all = rows(&app);
    let landlord = row_of(&all, "Example Landlord")["id"].clone();
    let housing = category_id(&app, "Housing");
    app.ok(
        "set_transaction_category",
        json!({ "transactionId": landlord, "categoryId": housing }),
    );
    let leisure = category_id(&app, "Leisure");
    app.ok(
        "add_transaction_rule",
        json!({ "text": "Example Landlord", "categoryId": leisure }),
    );
    let result = app.ok(
        "reapply_transaction_rules",
        json!({ "scope": "all_non_manual" }),
    );
    assert_eq!(result["changed"], 0);
    let after = rows(&app);
    let landlord_row = row_of(&after, "Example Landlord");
    assert_eq!(landlord_row["category_id"], housing);
    assert_eq!(landlord_row["manual"], true);
}

#[test]
fn test_csd032_ac_11_3_a_deleted_transaction_cannot_be_edited() {
    let app = harness();
    import(&app, fixture()).unwrap();
    let id = rows(&app).first().unwrap()["id"].clone();
    app.ok("delete_transaction", json!({ "transactionId": id }));
    let error = app.err(
        "set_transaction_category",
        json!({ "transactionId": id, "categoryId": null }),
    );
    assert_eq!(error["code"], "TRANSACTION_NOT_FOUND");
    assert_eq!(rows(&app).len(), 7);
}

#[test]
fn test_csd032_ac_26_4_manual_only_filters_the_list_over_ipc() {
    let app = harness();
    import(&app, fixture()).unwrap();
    let all = rows(&app);
    let id = all.first().unwrap()["id"].clone();
    app.ok(
        "set_transaction_category",
        json!({ "transactionId": id, "categoryId": null }),
    );
    let list = app.ok("list_transactions", json!({ "manualOnly": true }));
    assert_eq!(list["count"], 1);
    assert_eq!(list["rows"][0]["id"], id);
    assert_eq!(rows(&app).len(), all.len());
}

#[test]
fn test_csd032_ac_27_3_the_rule_list_carries_a_count_over_ipc() {
    let app = harness();
    import(&app, fixture()).unwrap();
    let rules = app.ok("list_transaction_rules", json!({}));
    let rules = rules.as_array().unwrap();
    assert_ne!(rules.len(), 0);
    assert!(rules
        .iter()
        .all(|rule| rule["transaction_count"].is_u64() && rule["text"].is_string()));
    let counted: u64 = rules
        .iter()
        .map(|rule| rule["transaction_count"].as_u64().unwrap())
        .sum();
    assert!(
        counted > 0 && counted <= 7,
        "the seed rules categorise some of the 7 rows"
    );
}

#[test]
fn test_csd032_ac_25_2_delete_all_is_one_undo_step() {
    let app = harness();
    import(&app, fixture()).unwrap();
    let before = rows(&app).len();
    let texts = |app: &Harness| -> Vec<Value> {
        let rules = app.ok("list_transaction_rules", json!({}));
        rules
            .as_array()
            .unwrap()
            .iter()
            .map(|rule| rule["text"].clone())
            .collect()
    };
    let rules = texts(&app);
    app.ok("delete_all_transactions", json!({}));
    assert_eq!(rows(&app), Vec::<Value>::new());
    assert_eq!(texts(&app), rules);
    app.ok("undo", json!({}));
    assert_eq!(rows(&app).len(), before);
}

#[test]
fn test_csd032_ac_8_4_uncategorized_cannot_be_renamed_or_deleted() {
    let app = harness();
    for command in ["rename_transaction_category", "delete_transaction_category"] {
        let error = app.err(
            command,
            json!({ "categoryId": "uncategorized", "name": "Other things" }),
        );
        assert_eq!(error["code"], "TRANSACTION_CATEGORY_PROTECTED", "{command}");
    }
}

#[test]
fn test_csd032_ac_8_2_categories_are_added_and_renamed_over_ipc() {
    let app = harness();
    let added = app.ok(
        "add_transaction_category",
        json!({ "name": "Example Pets" }),
    );
    let renamed = app.ok(
        "rename_transaction_category",
        json!({ "categoryId": added["id"], "name": "Pets" }),
    );
    assert_eq!(renamed["name"], "Pets");
    let error = app.err("add_transaction_category", json!({ "name": "pets" }));
    assert_eq!(error["code"], "TRANSACTION_CATEGORY_NAME_TAKEN");
}

#[test]
fn test_csd032_ac_13_1_deleting_a_category_over_ipc_clears_its_rules_and_transactions() {
    let app = harness();
    import(&app, fixture()).unwrap();
    let groceries = category_id(&app, "Groceries");
    app.ok(
        "delete_transaction_category",
        json!({ "categoryId": groceries }),
    );
    let all = rows(&app);
    assert_eq!(
        row_of(&all, "Albert Heijn Example")["category_id"],
        Value::Null
    );
    let rules = app.ok("list_transaction_rules", json!({}));
    assert!(rules
        .as_array()
        .unwrap()
        .iter()
        .all(|rule| rule["category_id"] != groceries));
    // One undo step brings the category, its rules and its transactions back.
    app.ok("undo", json!({}));
    assert_eq!(
        row_of(&rows(&app), "Albert Heijn Example")["category_id"],
        groceries
    );
}

#[test]
fn test_csd032_ac_9_1_rules_are_reordered_over_ipc() {
    let app = harness();
    let rules = app.ok("list_transaction_rules", json!({}));
    let last = rules.as_array().unwrap().last().unwrap()["id"].clone();
    app.ok(
        "move_transaction_rule",
        json!({ "ruleId": last, "position": 0 }),
    );
    let after = app.ok("list_transaction_rules", json!({}));
    assert_eq!(after.as_array().unwrap().first().unwrap()["id"], last);
    let error = app.err(
        "add_transaction_rule",
        json!({ "text": "Example", "categoryId": "not-an-id" }),
    );
    assert_eq!(error["code"], "TRANSACTION_RULE_CATEGORY_INVALID");
}

#[test]
fn test_csd032_ac_15_1_the_list_is_filtered_and_cut_by_rust() {
    let app = harness();
    import(&app, fixture()).unwrap();
    let page = app.ok("list_transactions", json!({ "limit": 3 }));
    assert_eq!(page["rows"].as_array().unwrap().len(), 3);
    assert_eq!(page["count"], 8);
    // 2500.00 + 15.50 in, 23.97 + 45.10 + 13.99 + 1000.00 + 200.00 + 60.00 out.
    assert_eq!(page["total"], "1172.44");
    let filtered = app.ok(
        "list_transactions",
        json!({ "filter": { "ids": ["uncategorized"] } }),
    );
    assert_eq!(filtered["count"], 6);
    let february = app.ok(
        "list_transactions",
        json!({ "filter": { "from": "2026-11", "to": "2026-12" } }),
    );
    assert_eq!(february["count"], 0);
    assert_eq!(february["total"], "0.00");
}

#[test]
fn test_csd032_ac_18_1_transactions_count_as_entries_for_the_currency_confirmation() {
    let app = harness();
    assert_eq!(app.ok("get_settings", json!({}))["entry_count"], 0);
    import(&app, fixture()).unwrap();
    assert_eq!(app.ok("get_settings", json!({}))["entry_count"], 8);
}

#[test]
fn test_csd032_ac_9_6_a_rule_creates_its_category_in_one_undo_step() {
    let app = harness();
    let categories = app.ok("list_transaction_categories", json!({}));
    let count = categories.as_array().unwrap().len();
    let rule = app.ok(
        "add_transaction_rule",
        json!({ "text": "Example Gym", "newCategory": "Example Sport" }),
    );
    let after = app.ok("list_transaction_categories", json!({}));
    assert_eq!(after.as_array().unwrap().len(), count + 1);
    assert_eq!(
        rule["category_id"],
        after.as_array().unwrap().last().unwrap()["id"]
    );
    app.ok("undo", json!({}));
    assert_eq!(
        app.ok("list_transaction_categories", json!({}))
            .as_array()
            .unwrap()
            .len(),
        count
    );
    let rules = app.ok("list_transaction_rules", json!({}));
    assert!(rules
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["text"] != "Example Gym"));
}

#[test]
fn test_csd032_ac_9_7_a_rejected_name_over_ipc_changes_nothing() {
    let app = harness();
    let error = app.err(
        "add_transaction_rule",
        json!({ "text": "Example Gym", "newCategory": "groceries" }),
    );
    assert_eq!(error["code"], "TRANSACTION_CATEGORY_NAME_TAKEN");
    assert_eq!(
        app.ok("project_status", json!({}))["undo_label"],
        Value::Null
    );
}

#[test]
fn test_csd032_ac_24_1_yearly_totals_over_ipc() {
    let app = harness();
    assert_eq!(app.ok("yearly_transaction_totals", json!({})), json!([]));
    import(&app, fixture()).unwrap();
    let totals = app.ok("yearly_transaction_totals", json!({}));
    assert_eq!(
        totals,
        json!([{
            "year": 2026,
            "income": "2515.50",
            "spent": "1343.06",
            "net": "1172.44",
            "transaction_count": 8
        }])
    );
}

#[test]
fn test_csd032_ac_11_5_unset_manual_over_ipc_is_one_undo_step() {
    let app = harness();
    import(&app, fixture()).unwrap();
    let id = rows(&app).first().unwrap()["id"].clone();
    let groceries = category_id(&app, "Groceries");
    app.ok(
        "set_transaction_category",
        json!({ "transactionId": id, "categoryId": groceries }),
    );
    app.ok("unset_transaction_manual", json!({ "transactionId": id }));
    let after = rows(&app);
    let row = after.iter().find(|row| row["id"] == id).unwrap();
    assert_eq!(row["manual"], false);
    assert_eq!(row["category_id"], groceries);
    assert_eq!(
        app.ok("project_status", json!({}))["undo_label"],
        "Edit Transaction"
    );
    app.ok("undo", json!({}));
    let undone = rows(&app);
    assert_eq!(
        undone.iter().find(|row| row["id"] == id).unwrap()["manual"],
        true
    );
    let error = app.err(
        "unset_transaction_manual",
        json!({ "transactionId": "nope" }),
    );
    assert_eq!(error["code"], "TRANSACTION_NOT_FOUND");
}

#[test]
fn test_csd033_ac_5_1_insights_over_ipc_default_to_years_and_take_a_granularity() {
    let app = harness();
    let today = time::Date::from_calendar_date(2026, time::Month::October, 15).unwrap();
    app.window.state::<AppState>().fix_today(today);
    let empty = app.ok("get_transaction_insights", json!({}));
    assert_eq!(empty["years"], json!([]));
    assert_eq!(empty["granularity"], "yearly");
    import(&app, fixture()).unwrap();
    let yearly = app.ok("get_transaction_insights", json!({}));
    assert_eq!(yearly["years"].as_array().unwrap().len(), 1);
    assert_eq!(yearly["years"][0]["total"], "1343.06");
    assert_eq!(yearly["years"][0]["income"], "2515.50");
    assert_eq!(yearly["years"][0]["marker"]["value"], "2515.50");
    assert_eq!(yearly["years"][0]["month"], Value::Null);
    let monthly = app.ok(
        "get_transaction_insights",
        json!({ "granularity": "monthly" }),
    );
    assert_eq!(monthly["granularity"], "monthly");
    let last = monthly["years"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(last["total"], "1343.06");
    assert_eq!(last["month"], 10);
    assert!(monthly["axis"]["ticks"].as_array().unwrap().len() >= 2);
    let error = app.err(
        "get_transaction_insights",
        json!({ "granularity": "weekly" }),
    );
    assert!(error.is_string() || error.is_object());
}
