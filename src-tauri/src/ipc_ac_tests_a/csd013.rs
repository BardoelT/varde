//! CSD-013 over IPC: expenses and the currency confirmation of CSD-003 REQ-5.

use serde_json::json;

use super::expense;
use crate::ipc_tests::harness;

#[test]
fn test_csd013_ac_17_1_an_expense_alone_asks_for_the_currency_confirmation() {
    let app = harness();
    expense(&app, "Example Roof Repair", "4500.00", "15/09/2026", "Home");
    // No jobs, investments, bank accounts, debts or houses: the expense is the only entry.
    let settings = app.ok("get_settings", json!({}));
    assert_eq!(settings, json!({ "currency": "EUR", "entry_count": 1 }));
    // The frontend asks first; cancelling sends nothing, so the currency and the amount stay.
    assert_eq!(app.ok("get_settings", json!({}))["currency"], "EUR");
    assert_eq!(
        app.ok("list_expenses", json!({}))["expenses"][0]["amount"],
        "4500.00"
    );
    // After a confirmed change the amount is still not converted.
    app.ok("set_currency", json!({ "currency": "USD" }));
    assert_eq!(
        app.ok("list_expenses", json!({}))["expenses"][0]["amount"],
        "4500.00"
    );
}

#[test]
fn test_csd013_ac_17_2_no_expenses_needs_no_confirmation_for_them() {
    let app = harness();
    assert_eq!(app.ok("get_settings", json!({}))["entry_count"], 0);
    let changed = app.ok("set_currency", json!({ "currency": "USD" }));
    assert_eq!(changed, json!({ "currency": "USD", "entry_count": 0 }));
    // Deleting the only expense brings the count back to 0.
    let added = expense(&app, "Example Bike", "900.00", "10/09/2025", "Hobby");
    assert_eq!(app.ok("get_settings", json!({}))["entry_count"], 1);
    app.ok("delete_expense", json!({ "expenseId": added["id"] }));
    assert_eq!(app.ok("get_settings", json!({}))["entry_count"], 0);
}
