//! Golden tests: one synthetic example project that touches every area, its
//! project file and the output of every read command, compared with files in
//! `golden/` (CLAUDE.md refactoring step 5).
//!
//! They pin today's behaviour so a later refactoring that must not change it
//! is checked, not assumed. A deliberate change of behaviour updates the golden
//! files with `GOLDEN_UPDATE=1 cargo test golden`, and the diff is reviewed.
//! Ids are random, so they are replaced by `id-1`, `id-2`, ... in the order
//! they first appear. "Today" is fixed so the outputs do not depend on the
//! clock.

use std::fs;
use std::path::PathBuf;

use serde_json::{json, Map, Value};
use tauri::Manager;
use tempfile::TempDir;
use time::{Date, Month};

use crate::commands::AppState;
use crate::ipc_tests::{harness, Harness};

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("golden")
        .join(name)
}

fn fix_today(app: &Harness) {
    let today = Date::from_calendar_date(2026, Month::September, 15).unwrap();
    app.window.state::<AppState>().fix_today(today);
}

/// Replaces every uuid by `id-<n>`, numbered by first appearance.
fn normalize_ids(text: &str) -> String {
    fn is_uuid(candidate: &[u8]) -> bool {
        candidate.len() == 36
            && candidate.iter().enumerate().all(|(index, byte)| {
                if matches!(index, 8 | 13 | 18 | 23) {
                    *byte == b'-'
                } else {
                    byte.is_ascii_hexdigit()
                }
            })
    }

    let bytes = text.as_bytes();
    let mut seen: Vec<&str> = Vec::new();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while index < bytes.len() {
        if index + 36 <= bytes.len() && is_uuid(&bytes[index..index + 36]) {
            let id = &text[index..index + 36];
            let number = seen
                .iter()
                .position(|known| *known == id)
                .unwrap_or_else(|| {
                    seen.push(id);
                    seen.len() - 1
                });
            out.push_str("id-");
            out.push_str(&(number + 1).to_string());
            index += 36;
        } else {
            let character = text[index..].chars().next().unwrap();
            out.push(character);
            index += character.len_utf8();
        }
    }
    out
}

/// Compares `actual` with the golden file, or rewrites it when asked to.
fn assert_golden(name: &str, actual: &str) {
    let path = golden_path(name);
    if std::env::var_os("GOLDEN_UPDATE").is_some() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, actual).unwrap();
        return;
    }
    let expected = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("{name} is missing: run GOLDEN_UPDATE=1 cargo test golden"));
    assert!(
        expected == actual,
        "{name} differs from the golden file. If the change is intended, run \
         GOLDEN_UPDATE=1 cargo test golden and review the diff of src-tauri/golden/."
    );
}

fn id_of(value: &Value) -> Value {
    value["id"].clone()
}

/// Builds the example project through the same commands the UI calls.
fn build_example_project(app: &Harness) {
    add_income(app);
    add_investments(app);
    add_bank_accounts(app);
    add_expenses(app);
    add_houses_and_debts(app);
}

/// A monthly and a yearly job.
fn add_income(app: &Harness) {
    let monthly = app.ok(
        "create_job",
        json!({ "input": { "employer": "Example Employer", "title": "Engineer", "start_date": "01/01/2024" } }),
    );
    let yearly = app.ok(
        "create_job",
        json!({ "input": { "employer": "Example Consultancy", "start_date": "01/01/2023", "end_date": "31/12/2025", "period": "Yearly" } }),
    );
    for (month, gross, holiday, bonus) in [
        ("2024-06", "3000.00", "", ""),
        ("2025-06", "3200.00", "2400.00", "500.00"),
        ("2026-03", "3300.00", "", "1000.00"),
        ("2026-08", "3300.00", "", ""),
    ] {
        app.ok(
            "add_paycheck",
            json!({ "jobId": id_of(&monthly), "month": month, "amounts": { "gross_salary": gross, "holiday_pay": holiday, "bonus": bonus } }),
        );
    }
    for (year, gross) in [
        ("2023", "36000.00"),
        ("2024", "38000.00"),
        ("2025", "40000.00"),
    ] {
        app.ok(
            "add_statement",
            json!({ "jobId": id_of(&yearly), "year": year, "amounts": { "gross_salary": gross, "holiday_pay": "", "bonus": "2000.00" } }),
        );
    }
}

/// A monthly and a yearly account.
fn add_investments(app: &Harness) {
    let brokerage = app.ok(
        "create_investment_account",
        json!({ "input": { "name": "Example Brokerage", "period": "Monthly" } }),
    );
    let pension = app.ok(
        "create_investment_account",
        json!({ "input": { "name": "Example Pension", "period": "Yearly" } }),
    );
    for (period, balance, note) in [
        ("2025-12", "10000.00", None),
        ("2026-06", "12500.50", Some("Bought example fund")),
        ("2026-08", "12000.00", None),
    ] {
        app.ok(
            "add_investment_statement",
            json!({ "accountId": id_of(&brokerage), "period": period, "balance": balance, "note": note }),
        );
    }
    for (period, balance) in [("2024", "5000.00"), ("2025", "6000.00")] {
        app.ok(
            "add_investment_statement",
            json!({ "accountId": id_of(&pension), "period": period, "balance": balance, "note": null }),
        );
    }
}

fn add_bank_accounts(app: &Harness) {
    let current = app.ok(
        "create_bank_account",
        json!({ "input": { "name": "Example Current Account", "period": "Monthly" } }),
    );
    let savings = app.ok(
        "create_bank_account",
        json!({ "input": { "name": "Example Savings", "period": "Yearly" } }),
    );
    for (period, balance) in [("2025-12", "2000.00"), ("2026-08", "2500.50")] {
        app.ok(
            "add_bank_entry",
            json!({ "accountId": id_of(&current), "period": period, "balance": balance }),
        );
    }
    for (period, balance, note) in [
        ("2024", "-400.00", Some("Overdrawn")),
        ("2025", "1000.00", None),
        ("2027", "1500.00", Some("Future entry")),
    ] {
        app.ok(
            "add_bank_entry",
            json!({ "accountId": id_of(&savings), "period": period, "balance": balance, "note": note }),
        );
    }
    // A record older than the 50 slots of the series (CSD-010 AC-2.6).
    let old = app.ok(
        "create_bank_account",
        json!({ "input": { "name": "Example Old Bank", "period": "Yearly" } }),
    );
    app.ok(
        "add_bank_entry",
        json!({ "accountId": id_of(&old), "period": "1950", "balance": "100.00", "note": "Very old entry" }),
    );
}

fn add_expenses(app: &Harness) {
    for (name, amount, date, category) in [
        ("Example Bike", "500.00", "05/03/2025", "Vehicle"),
        ("Example Trip", "1250.50", "20/07/2025", "Holiday"),
        ("Example Sofa", "800.00", "11/02/2026", "Home"),
        ("Example Class", "120.00", "02/09/2026", "Study"),
        // Older than the 50 slots of the series (CSD-014 AC-1.6).
        ("Example Heirloom", "100.00", "01/01/1950", "Other"),
    ] {
        app.ok(
            "add_expense",
            json!({ "input": { "name": name, "amount": amount, "date": date, "category": category } }),
        );
    }
}

/// A house that is sold with two linked debts settled, a house that is held,
/// and a debt without a house.
fn add_houses_and_debts(app: &Harness) {
    let sold = app.ok(
        "create_house",
        json!({ "input": { "name": "Example House", "period": "Monthly", "purchase_date": "15/03/2024", "purchase_amount": "250000.00" } }),
    );
    let held = app.ok(
        "create_house",
        json!({ "input": { "name": "Example Lodge", "period": "Yearly", "purchase_date": "01/01/2025", "purchase_amount": "100000.00" } }),
    );
    for (period, value, note) in [
        ("2025-12", "260000.00", None),
        ("2026-04", "270000.00", Some("Valued by an example agent")),
    ] {
        app.ok(
            "add_house_valuation",
            json!({ "houseId": id_of(&sold), "period": period, "value": value, "note": note }),
        );
    }
    for (period, value) in [("2025", "105000.00"), ("2026", "110000.00")] {
        app.ok(
            "add_house_valuation",
            json!({ "houseId": id_of(&held), "period": period, "value": value }),
        );
    }
    let mortgage = app.ok(
        "create_debt",
        json!({ "input": { "opening_date": "01/01/2000", "opening_amount": "150000.00", "name": "Example Mortgage", "period": "Monthly", "category": "Mortgage", "house_id": sold["id"] } }),
    );
    let loan = app.ok(
        "create_debt",
        json!({ "input": { "opening_date": "01/01/2000", "opening_amount": "20000.00", "name": "Example Home Loan", "period": "Monthly", "category": "Loan", "house_id": sold["id"] } }),
    );
    let student = app.ok(
        "create_debt",
        json!({ "input": { "opening_date": "01/01/2000", "opening_amount": "8000.00", "name": "Example Student Loan", "period": "Yearly", "category": "Other" } }),
    );
    for (debt, period, balance) in [
        (&mortgage, "2025-12", "150000.00"),
        (&mortgage, "2026-04", "148000.50"),
        (&loan, "2026-04", "20000.00"),
        (&student, "2025", "8000.00"),
        (&student, "2026", "7000.00"),
    ] {
        app.ok(
            "add_debt_statement",
            json!({ "debtId": id_of(debt), "period": period, "balance": balance }),
        );
    }
    app.ok(
        "update_house",
        json!({
            "houseId": id_of(&sold),
            "input": {
                "name": "Example House",
                "purchase_date": "15/03/2024",
                "purchase_amount": "250000.00",
                "sell_date": "10/06/2026",
                "sell_amount": "300000.00"
            },
            "links": { "settlements": [
                { "debt_id": mortgage["id"], "outcome": "Repaid" },
                { "debt_id": loan["id"], "outcome": "Cancelled" }
            ] }
        }),
    );
}

/// Every read command the UI calls, in a fixed order.
pub(crate) fn read_everything(app: &Harness) -> Value {
    let reads = [
        "project_status",
        "get_settings",
        "list_jobs",
        "list_entries",
        "yearly_totals",
        "get_income_insights",
        "list_investment_accounts",
        "list_investment_statements",
        "yearly_investment_totals",
        "get_investment_insights",
        "list_bank_accounts",
        "list_bank_entries",
        "yearly_bank_totals",
        "get_bank_insights",
        "list_debts",
        "list_debt_statements",
        "yearly_debt_totals",
        "get_debt_insights",
        "list_houses",
        "list_house_valuations",
        "yearly_house_values",
        "get_house_insights",
        "list_expenses",
        "yearly_expense_totals",
        "get_expense_insights",
        "get_net_worth_insights",
        "get_ratios",
        "get_dashboard",
    ];
    let mut out = Map::new();
    for command in reads {
        out.insert(command.to_owned(), app.ok(command, json!({})));
    }
    Value::Object(out)
}

fn pretty(value: &Value) -> String {
    let mut text = serde_json::to_string_pretty(value).unwrap();
    text.push('\n');
    text
}

#[test]
fn golden_the_example_project_file_and_every_read_command() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example.varde");

    let built = harness();
    fix_today(&built);
    build_example_project(&built);
    built
        .window
        .state::<AppState>()
        .project()
        .unwrap()
        .save_as(path.clone())
        .unwrap();
    let from_memory = read_everything(&built);

    // The file as written.
    let file_text = fs::read_to_string(&path).unwrap();
    assert_golden("example-project.json", &normalize_ids(&file_text));

    // The same project reopened from that file answers the same.
    let reopened = harness();
    fix_today(&reopened);
    reopened
        .window
        .state::<AppState>()
        .project()
        .unwrap()
        .open(&path)
        .unwrap();
    let from_file = read_everything(&reopened);
    assert_eq!(from_memory["list_jobs"], from_file["list_jobs"]);
    assert_eq!(
        from_memory["get_net_worth_insights"],
        from_file["get_net_worth_insights"]
    );

    // Opening marks nothing as changed; a saved project reads the same as the
    // one that was built, apart from the unsaved indicator.
    let mut expected = from_memory.clone();
    let mut actual = from_file.clone();
    expected["project_status"] = Value::Null;
    actual["project_status"] = Value::Null;
    assert_eq!(expected, actual);

    assert_golden("ipc-reads.json", &normalize_ids(&pretty(&from_file)));
}
