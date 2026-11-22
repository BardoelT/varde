//! Acceptance tests of CSD-036 (budgets) through the real commands: the names,
//! the argument names and the shapes the frontend relies on. Synthetic data only.

use serde_json::{json, Value};

use crate::ipc_tests::{harness, Harness};

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

fn set(app: &Harness, name: &str, month: &str, amount: &str, scope: &str) -> Value {
    app.ok(
        "set_budget",
        json!({ "categoryId": category_id(app, name), "month": month, "amount": amount, "scope": scope }),
    )
}

fn row<'a>(view: &'a Value, name: &str) -> &'a Value {
    view["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == name)
        .unwrap()
}

#[test]
fn test_csd036_ac_6_2_a_budget_is_set_and_the_month_is_returned() {
    let app = harness();
    let view = set(&app, "Groceries", "2026-10", "400.00", "all");
    assert_eq!(view["month"], "2026-10");
    assert_eq!(row(&view, "Groceries")["budget"], "400.00");
    assert_eq!(row(&view, "Groceries")["remaining"], "400.00");
    let again = app.ok("get_budgets", json!({ "month": "2026-10" }));
    assert_eq!(again, view);
}

#[test]
fn test_csd036_ac_11_1_any_scope_is_one_undo_step() {
    let app = harness();
    set(&app, "Groceries", "2026-01", "400.00", "all");
    set(&app, "Groceries", "2026-10", "450.00", "from");
    let status = app.ok("project_status", json!({}));
    assert_eq!(status["undo_label"], "Edit Budget");
    app.ok("undo", json!({}));
    let view = app.ok("get_budgets", json!({ "month": "2026-11" }));
    assert_eq!(row(&view, "Groceries")["budget"], "400.00");
    app.ok("redo", json!({}));
    let view = app.ok("get_budgets", json!({ "month": "2026-11" }));
    assert_eq!(row(&view, "Groceries")["budget"], "450.00");
}

#[test]
fn test_csd036_ac_7_1_a_budget_is_removed_with_a_scope() {
    let app = harness();
    set(&app, "Groceries", "2026-01", "400.00", "all");
    let view = app.ok(
        "remove_budget",
        json!({ "categoryId": category_id(&app, "Groceries"), "month": "2026-10", "scope": "from" }),
    );
    assert_eq!(row(&view, "Groceries")["budget"], Value::Null);
    let earlier = app.ok("get_budgets", json!({ "month": "2026-09" }));
    assert_eq!(row(&earlier, "Groceries")["budget"], "400.00");
    assert_eq!(
        app.ok("project_status", json!({}))["undo_label"],
        "Delete Budget"
    );
}

#[test]
fn test_csd036_ac_9_1_invalid_changes_are_rejected_and_record_no_step() {
    let app = harness();
    let groceries = category_id(&app, "Groceries");
    let call = |category: &str, month: &str, amount: &str, scope: &str| {
        app.err(
            "set_budget",
            json!({ "categoryId": category, "month": month, "amount": amount, "scope": scope }),
        )["code"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    assert_eq!(
        call(&groceries, "2026-10", "abc", "all"),
        "BUDGET_AMOUNT_INVALID"
    );
    assert_eq!(
        call(&groceries, "2026-10", "-5.00", "all"),
        "BUDGET_AMOUNT_INVALID"
    );
    assert_eq!(
        call(&groceries, "2026-10", "1000000000.01", "all"),
        "BUDGET_AMOUNT_INVALID"
    );
    assert_eq!(
        call("uncategorized", "2026-10", "1.00", "all"),
        "BUDGET_CATEGORY_PROTECTED"
    );
    assert_eq!(
        call(
            "671166f4-5827-4794-bcdc-54d7824fb650",
            "2026-10",
            "1.00",
            "all"
        ),
        "BUDGET_CATEGORY_UNKNOWN"
    );
    assert_eq!(
        call(&groceries, "2026-13", "1.00", "all"),
        "BUDGET_MONTH_INVALID"
    );
    assert_eq!(
        call(&groceries, "2026-10", "1.00", "sometimes"),
        "BUDGET_SCOPE_INVALID"
    );
    assert_eq!(
        app.ok("project_status", json!({}))["undo_label"],
        Value::Null
    );
    assert_eq!(
        app.err("get_budgets", json!({ "month": "2026-13" }))["code"],
        "BUDGET_MONTH_INVALID"
    );
}

#[test]
fn test_csd036_ac_6_6_a_typed_amount_is_read_in_the_number_format() {
    let app = harness();
    let view = set(&app, "Groceries", "2026-10", "400.505", "all");
    assert_eq!(row(&view, "Groceries")["budget"], "400.51");
}

#[test]
fn test_csd036_ac_10_2_deleting_a_category_removes_its_budget_and_undo_restores_both() {
    let app = harness();
    let groceries = category_id(&app, "Groceries");
    set(&app, "Groceries", "2026-01", "400.00", "all");
    app.ok(
        "delete_transaction_category",
        json!({ "categoryId": groceries }),
    );
    let view = app.ok("get_budgets", json!({ "month": "2026-10" }));
    assert!(view["rows"]
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["name"] != "Groceries"));
    app.ok("undo", json!({}));
    let view = app.ok("get_budgets", json!({ "month": "2026-10" }));
    assert_eq!(row(&view, "Groceries")["budget"], "400.00");
}

#[test]
fn test_csd036_ac_13_1_budgets_count_as_entries_for_the_currency_confirmation() {
    let app = harness();
    assert_eq!(app.ok("get_settings", json!({}))["entry_count"], 0);
    set(&app, "Groceries", "2026-10", "400.00", "all");
    assert_eq!(app.ok("get_settings", json!({}))["entry_count"], 1);
}
