//! CSD-003 over IPC: the currency, and the entry count that decides whether the frontend asks
//! for a confirmation (REQ-5).

use serde_json::json;

use super::{expense, job, named, paycheck, statement};
use crate::ipc_tests::harness;

#[test]
fn test_csd003_ac_3_2_jobs_and_paychecks_have_no_currency_of_their_own() {
    let app = harness();
    app.ok("set_currency", json!({ "currency": "USD" }));
    let job_id = job(&app, "Example Employer", "01/03/2024", "Monthly");
    paycheck(&app, &job_id, "2026-09", "3000.00", "", "");

    assert_eq!(app.ok("get_settings", json!({}))["currency"], "USD");
    let entries = app.ok("list_entries", json!({}));
    assert_eq!(entries[0]["gross_salary"], "3000.00");
    assert!(entries[0].get("currency").is_none());
    let jobs = app.ok("list_jobs", json!({}));
    assert!(jobs[0].get("currency").is_none());
}

#[test]
fn test_csd003_ac_5_4_an_annual_statement_counts_for_the_currency_confirmation() {
    let app = harness();
    let job_id = job(&app, "Example Employer", "01/01/2020", "Yearly");
    assert_eq!(
        app.ok("get_settings", json!({}))["entry_count"],
        0,
        "a yearly job alone needs no confirmation"
    );
    statement(&app, &job_id, "2025", "40000.00", "", "");
    assert_eq!(app.ok("get_settings", json!({}))["entry_count"], 1);
    // The amount is not converted.
    app.ok("set_currency", json!({ "currency": "USD" }));
    assert_eq!(
        app.ok("list_entries", json!({}))[0]["gross_salary"],
        "40000.00"
    );
}

#[test]
fn test_csd003_ac_5_3_a_job_without_entries_needs_no_confirmation() {
    let app = harness();
    job(&app, "Example Employer", "01/03/2024", "Monthly");
    let settings = app.ok("get_settings", json!({}));
    assert_eq!(settings["entry_count"], 0);
    let changed = app.ok("set_currency", json!({ "currency": "USD" }));
    assert_eq!(changed["currency"], "USD");
}

#[test]
fn test_csd003_ac_5_6_an_investment_account_without_statements_needs_no_confirmation() {
    let app = harness();
    app.ok(
        "create_investment_account",
        json!({ "input": named("Example Brokerage", "Monthly") }),
    );
    assert_eq!(app.ok("get_settings", json!({}))["entry_count"], 0);
    let changed = app.ok("set_currency", json!({ "currency": "USD" }));
    assert_eq!(changed["currency"], "USD");
}

#[test]
fn test_csd003_ac_5_8_a_bank_account_without_entries_needs_no_confirmation() {
    let app = harness();
    app.ok(
        "create_bank_account",
        json!({ "input": named("Example Current Account", "Monthly") }),
    );
    assert_eq!(app.ok("get_settings", json!({}))["entry_count"], 0);
    let changed = app.ok("set_currency", json!({ "currency": "USD" }));
    assert_eq!(changed["currency"], "USD");
}

#[test]
fn test_csd003_ac_5_10_a_debt_without_statements_needs_no_confirmation() {
    let app = harness();
    app.ok(
        "create_debt",
        json!({ "input": { "name": "Example Loan", "period": "Monthly", "opening_date": "01/01/2000", "opening_amount": "200000.00" } }),
    );
    assert_eq!(app.ok("get_settings", json!({}))["entry_count"], 0);
    let changed = app.ok("set_currency", json!({ "currency": "USD" }));
    assert_eq!(changed["currency"], "USD");
}

#[test]
fn test_csd003_ac_5_11_an_expense_alone_counts_for_the_currency_confirmation() {
    let app = harness();
    expense(&app, "Example Kitchen", "4500.00", "05/03/2026", "Other");
    let settings = app.ok("get_settings", json!({}));
    assert_eq!(
        settings["entry_count"], 1,
        "no jobs, accounts, debts or houses"
    );
    // Cancelling the confirmation never reaches Rust: the currency and the amount stay.
    assert_eq!(settings["currency"], "EUR");
    assert_eq!(
        app.ok("list_expenses", json!({}))["expenses"][0]["amount"],
        "4500.00"
    );
}

#[test]
fn test_csd003_ac_5_12_a_project_without_expenses_needs_no_confirmation_for_them() {
    let app = harness();
    assert_eq!(app.ok("get_settings", json!({}))["entry_count"], 0);
    let changed = app.ok("set_currency", json!({ "currency": "USD" }));
    assert_eq!(changed, json!({ "currency": "USD", "entry_count": 0 }));
}

#[test]
fn test_csd003_ac_5_14_a_house_without_valuations_needs_no_confirmation() {
    let app = harness();
    app.ok(
        "create_house",
        json!({ "input": {
            "name": "Example House", "period": "Monthly",
            "purchase_date": "01/01/1900", "purchase_amount": "0.00",
        } }),
    );
    assert_eq!(app.ok("get_settings", json!({}))["entry_count"], 0);
    let changed = app.ok("set_currency", json!({ "currency": "USD" }));
    assert_eq!(changed["currency"], "USD");
}
