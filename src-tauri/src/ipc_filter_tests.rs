//! Acceptance tests of CSD-028 (filters in the statement tables) through the
//! real list commands. Synthetic data only.

use serde_json::{json, Value};
use tauri::Manager;
use tempfile::TempDir;

use crate::commands::AppState;
use crate::ipc_tests::{harness, Harness};

fn bank_account(app: &Harness, name: &str, period: &str) -> Value {
    app.ok(
        "create_bank_account",
        json!({ "input": {
            "name": name, "period": period,
            "opening_date": "01/01/2000", "opening_amount": "0.00"
        } }),
    )
}

fn bank_entry(app: &Harness, account: &Value, period: &str, balance: &str) {
    app.ok(
        "add_bank_entry",
        json!({ "accountId": account["id"], "period": period, "balance": balance }),
    );
}

fn periods(rows: &Value) -> Vec<String> {
    rows.as_array()
        .unwrap()
        .iter()
        .map(|row| row["period"].as_str().unwrap().to_owned())
        .collect()
}

/// "Example Bank" (monthly) with 2026-01 and 2026-03, "Yearly Example" with 2025.
fn bank_project() -> (Harness, Value, Value) {
    let app = harness();
    let bank = bank_account(&app, "Example Bank", "Monthly");
    let yearly = bank_account(&app, "Yearly Example", "Yearly");
    bank_entry(&app, &bank, "2026-01", "100.00");
    bank_entry(&app, &bank, "2026-03", "300.00");
    bank_entry(&app, &yearly, "2025", "50.00");
    (app, bank, yearly)
}

#[test]
fn test_csd028_ac_10_1_no_filter_lists_every_bank_entry() {
    let (app, _, _) = bank_project();
    let all = app.ok("list_bank_entries", json!({}));
    assert_eq!(periods(&all), ["2026-03", "2026-01", "2025"]);
    assert_eq!(app.ok("list_bank_entries", json!({ "filter": null })), all);
    assert_eq!(
        app.ok(
            "list_bank_entries",
            json!({ "filter": { "from": null, "to": null, "ids": [] } })
        ),
        all
    );
}

#[test]
fn test_csd028_ac_3_4_a_yearly_entry_ignores_the_period_range() {
    let (app, _, _) = bank_project();
    let rows = app.ok(
        "list_bank_entries",
        json!({ "filter": { "from": "2026-02", "to": "2026-02", "ids": [] } }),
    );
    assert_eq!(periods(&rows), ["2025"]);
}

#[test]
fn test_csd028_ac_3_1_the_range_includes_both_ends_over_ipc() {
    let (app, _, _) = bank_project();
    let rows = app.ok(
        "list_bank_entries",
        json!({ "filter": { "from": "2026-01", "to": "2026-03", "ids": [] } }),
    );
    assert_eq!(periods(&rows), ["2026-03", "2026-01", "2025"]);
}

#[test]
fn test_csd028_ac_3_5_a_yearly_entry_follows_the_entity_filter() {
    let (app, bank, _) = bank_project();
    let rows = app.ok(
        "list_bank_entries",
        json!({ "filter": { "from": "2026-02", "to": null, "ids": [bank["id"]] } }),
    );
    assert_eq!(periods(&rows), ["2026-03"]);
}

#[test]
fn test_csd028_ac_4_2_two_accounts_are_listed() {
    let (app, bank, yearly) = bank_project();
    let rows = app.ok(
        "list_bank_entries",
        json!({ "filter": { "from": null, "to": null, "ids": [bank["id"], yearly["id"]] } }),
    );
    assert_eq!(rows.as_array().unwrap().len(), 3);
}

#[test]
fn test_csd028_ac_10_2_an_unknown_id_returns_no_rows_and_no_error() {
    let (app, _, _) = bank_project();
    let rows = app.ok(
        "list_bank_entries",
        json!({ "filter": { "from": null, "to": null, "ids": ["00000000-0000-0000-0000-000000000000"] } }),
    );
    assert_eq!(rows, json!([]));
}

#[test]
fn test_csd028_ac_3_3_a_range_that_ends_before_it_starts_returns_no_rows() {
    let (app, bank, _) = bank_project();
    let rows = app.ok(
        "list_bank_entries",
        json!({ "filter": { "from": "2026-03", "to": "2026-01", "ids": [bank["id"]] } }),
    );
    assert_eq!(rows, json!([]));
}

#[test]
fn test_csd028_ac_12_1_an_id_of_a_deleted_account_matches_nothing() {
    let (app, bank, yearly) = bank_project();
    app.ok("delete_bank_account", json!({ "accountId": bank["id"] }));
    let rows = app.ok(
        "list_bank_entries",
        json!({ "filter": { "from": null, "to": null, "ids": [bank["id"], yearly["id"]] } }),
    );
    assert_eq!(periods(&rows), ["2025"]);
}

#[test]
fn test_csd028_ac_14_1_listing_with_a_filter_records_no_step() {
    let (app, bank, _) = bank_project();
    app.ok("undo", json!({}));
    let before = app.ok("project_status", json!({}));
    app.ok(
        "list_bank_entries",
        json!({ "filter": { "from": "2026-02", "to": null, "ids": [bank["id"]] } }),
    );
    assert_eq!(app.ok("project_status", json!({})), before);
}

fn expense(app: &Harness, name: &str, amount: &str, date: &str, category: &str) {
    app.ok(
        "add_expense",
        json!({ "input": {
            "name": name, "amount": amount, "date": date, "category": category, "note": null
        } }),
    );
}

fn expense_project() -> Harness {
    let app = harness();
    expense(&app, "Example Lamp", "20.00", "31/01/2026", "Home");
    expense(&app, "Example Chair", "30.00", "01/02/2026", "Home");
    expense(&app, "Example Trip", "100.00", "15/02/2026", "Holiday");
    app
}

#[test]
fn test_csd028_ac_9_2_the_total_without_a_filter_is_the_total_of_everything() {
    let app = expense_project();
    assert_eq!(app.ok("list_expenses", json!({}))["total"], "150.00");
}

#[test]
fn test_csd028_ac_9_1_the_total_follows_the_filter() {
    let app = expense_project();
    let list = app.ok(
        "list_expenses",
        json!({ "filter": { "from": null, "to": null, "ids": ["Home"] } }),
    );
    assert_eq!(list["expenses"].as_array().unwrap().len(), 2);
    assert_eq!(list["total"], "50.00");
}

#[test]
fn test_csd028_ac_3_2_an_expense_counts_in_the_month_of_its_date() {
    let app = expense_project();
    let list = app.ok(
        "list_expenses",
        json!({ "filter": { "from": "2026-02", "to": null, "ids": [] } }),
    );
    let names: Vec<&str> = list["expenses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|expense| expense["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Example Trip", "Example Chair"]);
    assert_eq!(list["total"], "130.00");
}

#[test]
fn test_csd028_ac_3_3_a_filter_matching_nothing_has_a_zero_total() {
    let app = expense_project();
    let list = app.ok(
        "list_expenses",
        json!({ "filter": { "from": "2027-01", "to": null, "ids": [] } }),
    );
    assert_eq!(list["expenses"], json!([]));
    assert_eq!(list["total"], "0.00");
}

#[test]
fn test_csd028_ac_5_1_income_filters_on_the_job_and_keeps_annual_statements() {
    let app = harness();
    let monthly = app.ok(
        "create_job",
        json!({ "input": { "employer": "Example Employer", "start_date": "01/03/2024", "end_date": "" } }),
    );
    let yearly = app.ok(
        "create_job",
        json!({ "input": { "employer": "Older Example Employer", "start_date": "01/03/2020", "period": "Yearly" } }),
    );
    let amounts = json!({ "gross_salary": "3000.00", "holiday_pay": "", "bonus": "" });
    app.ok(
        "add_paycheck",
        json!({ "jobId": monthly["id"], "month": "2026-01", "amounts": amounts }),
    );
    app.ok(
        "add_paycheck",
        json!({ "jobId": monthly["id"], "month": "2026-03", "amounts": amounts }),
    );
    app.ok(
        "add_statement",
        json!({ "jobId": yearly["id"], "year": "2025", "amounts": amounts }),
    );

    let by_range = app.ok(
        "list_entries",
        json!({ "filter": { "from": "2026-02", "to": null, "ids": [] } }),
    );
    assert_eq!(periods(&by_range), ["2026-03", "2025"]);

    let by_job = app.ok(
        "list_entries",
        json!({ "filter": { "from": null, "to": null, "ids": [monthly["id"]] } }),
    );
    assert_eq!(periods(&by_job), ["2026-03", "2026-01"]);
}

#[test]
fn test_csd028_ac_5_1_investments_filter_on_the_account() {
    let app = harness();
    let brokerage = app.ok(
        "create_investment_account",
        json!({ "input": { "name": "Example Brokerage", "period": "Monthly" } }),
    );
    let pension = app.ok(
        "create_investment_account",
        json!({ "input": { "name": "Example Pension", "period": "Monthly" } }),
    );
    for (account, period) in [(&brokerage, "2026-01"), (&pension, "2026-02")] {
        app.ok(
            "add_investment_statement",
            json!({ "accountId": account["id"], "period": period, "balance": "1000.00", "netDeposits": "0.00" }),
        );
    }
    let rows = app.ok(
        "list_investment_statements",
        json!({ "filter": { "from": null, "to": null, "ids": [pension["id"]] } }),
    );
    assert_eq!(periods(&rows), ["2026-02"]);
}

#[test]
fn test_csd028_ac_5_1_debts_filter_on_the_debt_and_the_opening_row_follows() {
    let app = harness();
    let debt_input = |name: &str| json!({ "name": name, "period": "Monthly", "opening_date": "01/01/2000", "opening_amount": "200000.00" });
    let mortgage = app.ok(
        "create_debt",
        json!({ "input": debt_input("Example Mortgage") }),
    );
    let loan = app.ok(
        "create_debt",
        json!({ "input": debt_input("Example Loan") }),
    );
    for (debt, balance) in [(&mortgage, "150000.00"), (&loan, "5000.00")] {
        app.ok(
            "add_debt_statement",
            json!({ "debtId": debt["id"], "period": "2026-09", "balance": balance }),
        );
    }
    let rows = app.ok(
        "list_debt_statements",
        json!({ "filter": { "from": null, "to": null, "ids": [loan["id"]] } }),
    );
    assert_eq!(
        rows.as_array().unwrap().len(),
        2,
        "a statement and the opening row"
    );
    assert!(rows
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["debt_name"] == "Example Loan"));

    // The opening row counts in the month of its date (2000-01).
    let recent = app.ok(
        "list_debt_statements",
        json!({ "filter": { "from": "2026-01", "to": null, "ids": [] } }),
    );
    assert_eq!(recent.as_array().unwrap().len(), 2);
    assert!(recent
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["opening"] == false));
}

#[test]
fn test_csd028_ac_5_1_houses_filter_on_the_house() {
    let app = harness();
    let house_input = |name: &str| json!({ "name": name, "period": "Monthly", "purchase_date": "01/01/1900", "purchase_amount": "0.00" });
    let first = app.ok(
        "create_house",
        json!({ "input": house_input("Example House") }),
    );
    let second = app.ok(
        "create_house",
        json!({ "input": house_input("Example Cottage") }),
    );
    for (house, period) in [(&first, "2026-01"), (&second, "2026-02")] {
        app.ok(
            "add_house_valuation",
            json!({ "houseId": house["id"], "period": period, "value": "300000.00" }),
        );
    }
    let rows = app.ok(
        "list_house_valuations",
        json!({ "filter": { "from": null, "to": null, "ids": [first["id"]] } }),
    );
    assert_eq!(periods(&rows), ["2026-01"]);
}

#[test]
fn test_csd028_ac_13_3_the_saved_project_holds_no_filter() {
    let (app, bank, _) = bank_project();
    app.ok(
        "list_bank_entries",
        json!({ "filter": { "from": "2026-02", "to": null, "ids": [bank["id"]] } }),
    );
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-filter.varde");
    app.window
        .state::<AppState>()
        .project()
        .unwrap()
        .save_as(path.clone())
        .unwrap();
    let saved = std::fs::read_to_string(path).unwrap();
    assert!(!saved.to_lowercase().contains("filter"));
}
