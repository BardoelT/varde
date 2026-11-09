//! Number format (CSD-029) over IPC: choosing and saving the format, and
//! typed amounts that are read in it by every area that takes an amount.

use serde_json::{json, Value};
use tempfile::TempDir;

use crate::ipc_tests::{harness_in, Harness};

fn app_with_format(dir: &TempDir, format: &str) -> Harness {
    let app = harness_in(dir.path());
    app.ok("set_number_format", json!({ "numberFormat": format }));
    app
}

#[test]
fn test_csd029_the_number_format_is_chosen_saved_and_restored_over_ipc() {
    let dir = TempDir::new().unwrap();
    let app = harness_in(dir.path());
    assert_eq!(
        app.ok("get_preferences", json!({}))["number_format"],
        "point_comma"
    );
    let answer = app.ok(
        "set_number_format",
        json!({ "numberFormat": "comma_point" }),
    );
    assert_eq!(answer["number_format"], "comma_point");
    assert_eq!(answer["theme"], "system");
    assert_eq!(answer["language"], "en");

    let restarted = harness_in(dir.path());
    assert_eq!(
        restarted.ok("get_preferences", json!({}))["number_format"],
        "comma_point"
    );
    // Theme and language keep the format.
    app.ok("set_language", json!({ "language": "nl" }));
    let answer = app.ok("set_theme", json!({ "theme": "dark" }));
    assert_eq!(answer["number_format"], "comma_point");
}

#[test]
fn test_csd029_ac_9_2_unsupported_formats_are_rejected_over_ipc() {
    let dir = TempDir::new().unwrap();
    let app = app_with_format(&dir, "apostrophe_comma");
    let error = app
        .call("set_number_format", json!({ "numberFormat": "comma" }))
        .unwrap_err();
    assert_eq!(error["code"], "PREFERENCES_NUMBER_FORMAT_UNSUPPORTED");
    assert_eq!(
        app.ok("get_preferences", json!({}))["number_format"],
        "apostrophe_comma"
    );
}

#[test]
fn test_csd029_ac_10_1_a_failed_save_keeps_the_format_over_ipc() {
    use std::os::unix::fs::PermissionsExt;
    let config = TempDir::new().unwrap();
    let app = app_with_format(&config, "point_comma");

    std::fs::set_permissions(config.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
    let error = app
        .call(
            "set_number_format",
            json!({ "numberFormat": "comma_point" }),
        )
        .unwrap_err();
    std::fs::set_permissions(config.path(), std::fs::Permissions::from_mode(0o700)).unwrap();

    assert_eq!(error["code"], "PREFERENCES_SAVE_FAILED");
    let message = error["message"].as_str().unwrap();
    assert!(
        !message.contains(config.path().to_str().unwrap()),
        "{message}"
    );
    assert_eq!(
        app.ok("get_preferences", json!({}))["number_format"],
        "point_comma"
    );
}

fn bank_account(app: &Harness) -> Value {
    app.ok(
        "create_bank_account",
        json!({ "input": { "name": "Example Account", "period": "Monthly" } }),
    )
}

#[test]
fn test_csd029_ac_6_2_a_bank_balance_typed_in_the_format_is_stored_canonical() {
    let dir = TempDir::new().unwrap();
    let app = app_with_format(&dir, "comma_point");
    let account = bank_account(&app);
    let entry = app.ok(
        "add_bank_entry",
        json!({ "accountId": account["id"], "period": "2026-09", "balance": "1.234,50" }),
    );
    assert_eq!(entry["balance"], "1234.50");
    let entry = app.ok(
        "update_bank_entry",
        json!({ "accountId": account["id"], "period": "2026-09", "newPeriod": "2026-09",
                "balance": "-3000,505" }),
    );
    assert_eq!(entry["balance"], "-3000.51");
}

#[test]
fn test_csd029_ac_7_1_a_bank_balance_that_does_not_fit_the_format_is_rejected() {
    let dir = TempDir::new().unwrap();
    let app = app_with_format(&dir, "comma_point");
    let account = bank_account(&app);
    for typed in ["1234.50", "12.34,50", "1,234.50", "10,9x"] {
        let error = app
            .call(
                "add_bank_entry",
                json!({ "accountId": account["id"], "period": "2026-09", "balance": typed }),
            )
            .unwrap_err();
        assert_eq!(error["code"], "BANK_ENTRY_BALANCE_INVALID", "{typed}");
    }
    assert_eq!(app.ok("list_bank_entries", json!({})), json!([]));
}

#[test]
fn test_csd029_ac_6_1_the_default_format_reads_a_grouping_comma() {
    let app = harness_in(TempDir::new().unwrap().path());
    let account = bank_account(&app);
    let entry = app.ok(
        "add_bank_entry",
        json!({ "accountId": account["id"], "period": "2026-09", "balance": "1,234.50" }),
    );
    assert_eq!(entry["balance"], "1234.50");
}

#[test]
fn test_csd029_ac_6_1_expenses_are_read_in_the_format() {
    let dir = TempDir::new().unwrap();
    let app = app_with_format(&dir, "apostrophe_comma");
    let expense = app.ok(
        "add_expense",
        json!({ "input": { "name": "Example Bike", "amount": "1'234,50",
                           "date": "05/03/2025", "category": "Other" } }),
    );
    assert_eq!(expense["amount"], "1234.50");
    let error = app
        .call(
            "add_expense",
            json!({ "input": { "name": "Example Bike", "amount": "1.234,50",
                               "date": "05/03/2025", "category": "Other" } }),
        )
        .unwrap_err();
    assert_eq!(error["code"], "EXPENSE_AMOUNT_INVALID");
}

#[test]
fn test_csd029_ac_6_1_paychecks_are_read_in_the_format() {
    let dir = TempDir::new().unwrap();
    let app = app_with_format(&dir, "comma_point");
    let job = app.ok(
        "create_job",
        json!({ "input": { "employer": "Example Employer", "title": null,
                           "start_date": "01/01/2024", "end_date": null, "period": "Monthly" } }),
    );
    let entry = app.ok(
        "add_paycheck",
        json!({ "jobId": job["id"], "month": "2026-09",
                "amounts": { "gross_salary": "3.000,50", "holiday_pay": "", "bonus": "250,25" } }),
    );
    assert_eq!(entry["gross_salary"], "3000.50");
    assert_eq!(entry["bonus"], "250.25");
    let error = app
        .call(
            "add_paycheck",
            json!({ "jobId": job["id"], "month": "2026-10",
                    "amounts": { "gross_salary": "3000.50", "holiday_pay": "", "bonus": "" } }),
        )
        .unwrap_err();
    assert_eq!(error["code"], "PAYCHECK_AMOUNT_INVALID");
}

#[test]
fn test_csd029_ac_6_1_debts_houses_and_investments_are_read_in_the_format() {
    let dir = TempDir::new().unwrap();
    let app = app_with_format(&dir, "comma_point");

    let debt = app.ok(
        "create_debt",
        json!({ "input": { "name": "Example Loan", "period": "Monthly",
                           "opening_date": "01/01/2000", "opening_amount": "200.000,00" } }),
    );
    assert_eq!(debt["opening_amount"], "200000.00");
    let statement = app.ok(
        "add_debt_statement",
        json!({ "debtId": debt["id"], "period": "2000-02", "balance": "199.000,50" }),
    );
    assert_eq!(statement["balance"], "199000.50");

    let house = app.ok(
        "create_house",
        json!({ "input": { "name": "Example House", "period": "Monthly",
                           "purchase_date": "01/01/2000", "purchase_amount": "250.000,00" } }),
    );
    assert_eq!(house["purchase_amount"], "250000.00");
    let valuation = app.ok(
        "add_house_valuation",
        json!({ "houseId": house["id"], "period": "2000-02", "value": "260.000,00" }),
    );
    assert_eq!(valuation["value"], "260000.00");

    let account = app.ok(
        "create_investment_account",
        json!({ "input": { "name": "Example Fund", "period": "Monthly" } }),
    );
    let statement = app.ok(
        "add_investment_statement",
        json!({ "accountId": account["id"], "period": "2026-09",
                "balance": "12.500,50", "netDeposits": "-250,25" }),
    );
    assert_eq!(statement["balance"], "12500.50");
    assert_eq!(statement["net_deposits"], "-250.25");
}
