//! Acceptance tests of CSD-027 (undo and redo) through the real commands: which
//! commands record a step, what the status reports, what `undo` and `redo`
//! return. Synthetic data only.

use serde_json::{json, Value};

use crate::ipc_tests::{harness, Harness};

fn expense(name: &str, amount: &str, date: &str) -> Value {
    json!({ "input": {
        "name": name, "amount": amount, "date": date, "category": "Hobby", "note": null
    } })
}

fn add_expense(app: &Harness, name: &str, amount: &str) -> Value {
    app.ok("add_expense", expense(name, amount, "10/09/2026"))
}

fn expense_names(app: &Harness) -> Vec<String> {
    let list = app.ok("list_expenses", json!({}));
    let mut names: Vec<String> = list["expenses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|expense| expense["name"].as_str().unwrap().to_owned())
        .collect();
    names.sort();
    names
}

fn status(app: &Harness) -> Value {
    app.ok("project_status", json!({}))
}

#[test]
fn test_csd027_ac_10_1_the_status_names_the_step_to_undo() {
    let app = harness();
    assert_eq!(status(&app)["undo_label"], Value::Null);
    assert_eq!(status(&app)["redo_label"], Value::Null);

    let created = add_expense(&app, "Example Bike", "900.00");
    assert_eq!(status(&app)["undo_label"], "Add Expense");

    app.ok(
        "update_expense",
        json!({ "expenseId": created["id"], "input": {
            "name": "Example Bike", "amount": "950.00", "date": "10/09/2026",
            "category": "Hobby", "note": null
        } }),
    );
    assert_eq!(status(&app)["undo_label"], "Edit Expense");

    app.ok("delete_expense", json!({ "expenseId": created["id"] }));
    assert_eq!(status(&app)["undo_label"], "Delete Expense");
}

#[test]
fn test_csd027_ac_1_1_undo_a_delete_through_the_command() {
    let app = harness();
    let created = add_expense(&app, "Example Bike", "900.00");
    app.ok("delete_expense", json!({ "expenseId": created["id"] }));
    assert_eq!(expense_names(&app), Vec::<String>::new());

    let result = app.ok("undo", json!({}));

    assert_eq!(result["step"], "Delete Expense");
    assert_eq!(result["status"]["undo_label"], "Add Expense");
    assert_eq!(result["status"]["redo_label"], "Delete Expense");
    assert_eq!(expense_names(&app), ["Example Bike"]);

    let redone = app.ok("redo", json!({}));
    assert_eq!(redone["step"], "Delete Expense");
    assert_eq!(expense_names(&app), Vec::<String>::new());
}

#[test]
fn test_csd027_ac_1_4_and_2_2_nothing_to_undo_or_redo_is_a_safe_error() {
    let app = harness();
    let undo = app.call("undo", json!({})).unwrap_err();
    assert_eq!(undo["code"], "NOTHING_TO_UNDO");
    let redo = app.call("redo", json!({})).unwrap_err();
    assert_eq!(redo["code"], "NOTHING_TO_REDO");
}

#[test]
fn test_csd027_ac_7_1_a_rejected_command_records_no_step() {
    let app = harness();
    add_expense(&app, "Example Tea", "20.00");
    app.ok("undo", json!({}));
    assert_eq!(status(&app)["redo_label"], "Add Expense");

    let rejected = app
        .call(
            "add_expense",
            expense("Example Cake", "45.00", "not a date"),
        )
        .unwrap_err();

    assert!(rejected["code"].is_string());
    assert_eq!(status(&app)["redo_label"], "Add Expense");
    assert_eq!(status(&app)["undo_label"], Value::Null);
}

#[test]
fn test_csd027_ac_3_1_deleting_a_job_with_its_paychecks_is_one_step() {
    let app = harness();
    let job = app.ok(
        "create_job",
        json!({ "input": {
            "employer": "Example Employer", "title": "",
            "start_date": "01/03/2024", "end_date": ""
        } }),
    );
    for month in ["2024-03", "2024-04"] {
        app.ok(
            "add_paycheck",
            json!({ "jobId": job["id"], "month": month,
                "amounts": { "gross_salary": "3000.00", "holiday_pay": "", "bonus": "" } }),
        );
    }
    app.ok("delete_job", json!({ "jobId": job["id"] }));
    assert_eq!(app.ok("list_jobs", json!({})), json!([]));

    app.ok("undo", json!({}));

    let jobs = app.ok("list_jobs", json!({}));
    assert_eq!(jobs[0]["entry_count"], 2);
}

#[test]
fn test_csd027_ac_4_1_the_currency_is_not_a_step() {
    let app = harness();
    app.ok("set_currency", json!({ "currency": "USD" }));
    assert_eq!(status(&app)["undo_label"], Value::Null);
}

#[test]
fn test_csd027_ac_8_1_a_new_project_clears_the_history() {
    let app = harness();
    add_expense(&app, "Example Tea", "20.00");
    app.ok("undo", json!({}));
    add_expense(&app, "Example Cake", "45.00");
    app.ok("undo", json!({}));
    assert_eq!(status(&app)["redo_label"], "Add Expense");

    let after = app.ok("new_project", json!({}));

    assert_eq!(after["undo_label"], Value::Null);
    assert_eq!(after["redo_label"], Value::Null);
}

#[test]
fn test_csd027_ac_3_2_undoing_a_sale_with_a_settlement_restores_the_whole_state() {
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
    let statements_before = app.ok("list_debt_statements", json!({}));
    let debts_before = app.ok("list_debts", json!({}));
    let houses_before = app.ok("list_houses", json!({}));

    app.ok(
        "update_house",
        json!({
            "houseId": house["id"],
            "input": {
                "name": "Example House", "purchase_date": "15/03/2024",
                "purchase_amount": "250000.00",
                "sell_date": "10/06/2026", "sell_amount": "300000.00"
            },
            "links": { "settlements": [ { "debt_id": mortgage["id"], "outcome": "Cancelled" } ] }
        }),
    );
    assert_eq!(status(&app)["undo_label"], "Edit House");
    assert_ne!(app.ok("list_debt_statements", json!({})), statements_before);

    app.ok("undo", json!({}));

    assert_eq!(app.ok("list_debt_statements", json!({})), statements_before);
    assert_eq!(app.ok("list_debts", json!({})), debts_before);
    assert_eq!(app.ok("list_houses", json!({})), houses_before);
}
