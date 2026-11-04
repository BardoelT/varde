//! Acceptance-criteria tests of CSD-005, CSD-009 and CSD-010 over IPC: the commands as the
//! frontend calls them, with the codes and shapes it relies on.

use serde_json::{json, Value};

use crate::ipc_tests::{harness, Harness};

/// The error of a command that must fail.
fn failure(app: &Harness, command: &str, args: Value) -> Value {
    app.call(command, args)
        .expect_err(&format!("{command} should have failed"))
}

/// A monthly bank account "Example Current Account" and its id.
fn current_account(app: &Harness) -> String {
    let account = app.ok(
        "create_bank_account",
        json!({ "input": { "name": "Example Current Account", "period": "Monthly" } }),
    );
    account["id"].as_str().unwrap().to_owned()
}

fn add_entry(app: &Harness, id: &str, period: &str, balance: &str) {
    app.ok(
        "add_bank_entry",
        json!({ "accountId": id, "period": period, "balance": balance }),
    );
}

// ---- CSD-005 ----

#[test]
fn test_csd005_ac_29_4_an_invalid_value_rejects_the_whole_edit_over_ipc() {
    let app = harness();
    let account = app.ok(
        "create_investment_account",
        json!({ "input": { "name": "Example Brokerage", "period": "Monthly" } }),
    );
    let id = account["id"].as_str().unwrap();
    app.ok(
        "add_investment_statement",
        json!({ "accountId": id, "period": "2026-09", "balance": "12500.00", "netDeposits": "0.00" }),
    );
    let error = failure(
        &app,
        "update_investment_statement",
        json!({ "accountId": id, "period": "2026-09", "newPeriod": "2026-08", "balance": "-1.00" }),
    );
    assert_eq!(error["code"], "INVESTMENT_BALANCE_NEGATIVE");
    let rows = app.ok("list_investment_statements", json!({}));
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(rows[0]["period"], "2026-09");
    assert_eq!(rows[0]["balance"], "12500.00");
}

// ---- CSD-009 ----

#[test]
fn test_csd009_ac_13_4_an_invalid_value_rejects_the_whole_edit_over_ipc() {
    let app = harness();
    let id = current_account(&app);
    add_entry(&app, &id, "2026-09", "2500.00");
    let error = failure(
        &app,
        "update_bank_entry",
        json!({ "accountId": id, "period": "2026-09", "newPeriod": "2026-08", "balance": "abc", "note": "" }),
    );
    assert_eq!(error["code"], "BANK_ENTRY_BALANCE_INVALID");
    let rows = app.ok("list_bank_entries", json!({}));
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(
        rows[0]["period"], "2026-09",
        "the entry is still for 2026-09"
    );
    assert_eq!(rows[0]["balance"], "2500.00");
}

#[test]
fn test_csd009_ac_24_1_a_bank_entry_alone_counts_for_the_currency_confirmation() {
    // No jobs, no investments, a bank account with 1 entry: the frontend asks before the change.
    let app = harness();
    let id = current_account(&app);
    add_entry(&app, &id, "2026-09", "2500.00");
    let settings = app.ok("get_settings", json!({}));
    assert_eq!(settings["currency"], "EUR");
    assert_eq!(settings["entry_count"], 1);
}

#[test]
fn test_csd009_ac_24_2_a_bank_account_without_entries_needs_no_confirmation() {
    let app = harness();
    current_account(&app);
    assert_eq!(app.ok("get_settings", json!({}))["entry_count"], 0);
    let changed = app.ok("set_currency", json!({ "currency": "USD" }));
    assert_eq!(changed["currency"], "USD");
}

// ---- CSD-010 ----

/// The marker of 2026 as the Insights tab receives it.
fn marker_of_2026(app: &Harness) -> Value {
    let insights = app.ok("get_bank_insights", json!({}));
    let year = insights["years"]
        .as_array()
        .unwrap()
        .iter()
        .find(|year| year["year"] == 2026)
        .unwrap()
        .clone();
    year["marker"].clone()
}

#[test]
fn test_csd010_ac_15_1_the_figures_follow_a_new_entry() {
    let app = harness();
    let id = current_account(&app);
    add_entry(&app, &id, "2026-09", "2500.00");
    assert_eq!(marker_of_2026(&app)["value"], "2500.00");
    add_entry(&app, &id, "2026-10", "3000.00");
    assert_eq!(marker_of_2026(&app)["value"], "3000.00");
    assert_eq!(marker_of_2026(&app)["label"], "3.00k");
}

#[test]
fn test_csd010_ac_15_2_deleting_the_last_entry_empties_the_series() {
    let app = harness();
    let id = current_account(&app);
    add_entry(&app, &id, "2026-09", "2500.00");
    let before = app.ok("get_bank_insights", json!({}));
    assert_eq!(before["years"][0]["year"], 2026, "a chart to begin with");
    app.ok(
        "delete_bank_entry",
        json!({ "accountId": id, "period": "2026-09" }),
    );
    let insights = app.ok("get_bank_insights", json!({}));
    assert_eq!(insights["years"], json!([]), "no years, so no chart");
    assert_eq!(insights["banks"], json!([]));
}
