//! The example project (CSD-002 REQ-21, REQ-22): `examples/example.varde`, a
//! synthetic project file that shows every area of the app in every state it
//! supports, and the generator that writes it.
//!
//! The generator builds the project through the same commands the UI calls, so
//! the file is always valid for the current format. A deliberate change of the
//! example is written with `EXAMPLE_UPDATE=1 cargo test example`, and the diff
//! is reviewed. Ids are random, so they are replaced by fixed ids in the order
//! they first appear. "Today" is fixed so the reads do not depend on the clock.
//!
//! Coverage checklist (every state the approved CSDs describe):
//! - Income: a monthly job without end date, a yearly job without end date and a
//!   monthly job that ended and has no title; paychecks with holiday pay and a
//!   bonus; annual statements.
//! - Investments: a monthly, a yearly and a sparse account; statements with net
//!   deposits (also a withdrawal) and with notes.
//! - Bank accounts: monthly and yearly accounts, a negative balance, notes.
//! - Expenses: every category, with and without a note.
//! - Houses: a house that was sold (with a purchase, a sale and valuations only
//!   inside its holding window) and a house that is held.
//! - Debts: a mortgage settled as Repaid and a loan settled as Cancelled by the
//!   sale of their house, a loan linked to the held house, a debt without a
//!   house, a debt at 0.00 (the Repaid mark) and a debt closed by hand.
//! - Net worth, ratios and the dashboard follow from the data above.
//! - Every date and period lies in 2023-01 to 2026-10 and every name starts
//!   with "Example".

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde_json::{json, Value};
use tauri::Manager;
use tempfile::TempDir;
use time::{Date, Month};

use crate::commands::AppState;
use crate::golden_tests::read_everything;
use crate::ipc_tests::{harness, Harness};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn example_path() -> PathBuf {
    repo_root().join("examples").join("example.varde")
}

fn fix_today(app: &Harness) {
    let today = Date::from_calendar_date(2026, Month::October, 15).unwrap();
    app.window.state::<AppState>().fix_today(today);
}

/// Replaces every uuid by a fixed one, numbered by first appearance, so the
/// generated file is the same on every run and is still a valid project file.
fn fix_ids(text: &str) -> String {
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
            let fixed = format!("00000000-0000-4000-8000-{:012}", number + 1);
            out.push_str(&fixed);
            index += 36;
        } else {
            let character = text[index..].chars().next().unwrap();
            out.push(character);
            index += character.len_utf8();
        }
    }
    out
}

/// `(year, month)` from `from` to `to` inclusive, every `step` months.
fn months(from: (i32, u32), to: (i32, u32), step: u32) -> Vec<(i32, u32)> {
    let mut out = Vec::new();
    let (mut year, mut month) = from;
    while (year, month) <= to {
        out.push((year, month));
        month += step;
        while month > 12 {
            month -= 12;
            year += 1;
        }
    }
    out
}

fn period((year, month): (i32, u32)) -> String {
    format!("{year}-{month:02}")
}

/// A whole amount as the project file writes it.
fn amount(value: i64) -> String {
    format!("{value}.00")
}

fn id_of(value: &Value) -> Value {
    value["id"].clone()
}

/// Builds the example project through the commands the UI calls.
fn build_example_project(app: &Harness) {
    add_income(app);
    add_investments(app);
    add_bank_accounts(app);
    add_expenses(app);
    add_houses_and_debts(app);
}

/// A monthly job, a yearly job and a monthly job that ended.
fn add_income(app: &Harness) {
    let monthly = app.ok(
        "create_job",
        json!({ "input": { "employer": "Example Employer", "title": "Engineer", "start_date": "01/01/2024" } }),
    );
    for (year, month) in months((2024, 1), (2026, 10), 1) {
        let gross = match year {
            2024 => 3000,
            2025 => 3200,
            _ => 3300,
        };
        let holiday = match (year, month) {
            (2024, 6) => "2400.00",
            (2025, 6) => "2560.00",
            (2026, 6) => "2640.00",
            _ => "",
        };
        let bonus = match (year, month) {
            (2024, 12) | (2026, 3) => "1000.00",
            (2025, 12) => "1200.00",
            _ => "",
        };
        app.ok(
            "add_paycheck",
            json!({ "jobId": id_of(&monthly), "month": period((year, month)), "amounts": { "gross_salary": amount(gross), "holiday_pay": holiday, "bonus": bonus } }),
        );
    }

    let yearly = app.ok(
        "create_job",
        json!({ "input": { "employer": "Example Consultancy", "title": "Advisor", "start_date": "01/01/2023", "period": "Yearly" } }),
    );
    for (year, gross) in [("2023", 36000), ("2024", 38000), ("2025", 40000)] {
        app.ok(
            "add_statement",
            json!({ "jobId": id_of(&yearly), "year": year, "amounts": { "gross_salary": amount(gross), "holiday_pay": "", "bonus": "2000.00" } }),
        );
    }

    let ended = app.ok(
        "create_job",
        json!({ "input": { "employer": "Example Side Job", "start_date": "01/06/2025", "end_date": "31/12/2025" } }),
    );
    for month in months((2025, 6), (2025, 12), 1) {
        app.ok(
            "add_paycheck",
            json!({ "jobId": id_of(&ended), "month": period(month), "amounts": { "gross_salary": "800.00", "holiday_pay": "", "bonus": "" } }),
        );
    }
}

/// A monthly, a yearly and a sparse account.
fn add_investments(app: &Harness) {
    let brokerage = app.ok(
        "create_investment_account",
        json!({ "input": { "name": "Example Brokerage", "period": "Monthly" } }),
    );
    for (index, month) in (0_i64..).zip(months((2023, 1), (2026, 10), 3)) {
        let (deposits, note) = match index {
            5 => ("600.00".to_owned(), Some("Bought example fund")),
            9 => ("-1500.00".to_owned(), Some("Example withdrawal")),
            _ => ("600.00".to_owned(), None),
        };
        app.ok(
            "add_investment_statement",
            json!({ "accountId": id_of(&brokerage), "period": period(month), "balance": amount(10000 + 750 * index), "netDeposits": deposits, "note": note }),
        );
    }

    let pension = app.ok(
        "create_investment_account",
        json!({ "input": { "name": "Example Pension", "period": "Yearly" } }),
    );
    for (year, balance) in [
        ("2023", 5000),
        ("2024", 6000),
        ("2025", 7200),
        ("2026", 8500),
    ] {
        app.ok(
            "add_investment_statement",
            json!({ "accountId": id_of(&pension), "period": year, "balance": amount(balance), "netDeposits": "800.00", "note": null }),
        );
    }

    let index_fund = app.ok(
        "create_investment_account",
        json!({ "input": { "name": "Example Index Fund", "period": "Monthly" } }),
    );
    for (index, month) in (0_i64..).zip(months((2025, 6), (2026, 10), 2)) {
        app.ok(
            "add_investment_statement",
            json!({ "accountId": id_of(&index_fund), "period": period(month), "balance": amount(2000 + 150 * index), "netDeposits": null, "note": null }),
        );
    }
}

/// Monthly and yearly accounts, one balance below zero.
fn add_bank_accounts(app: &Harness) {
    let current = app.ok(
        "create_bank_account",
        json!({ "input": { "name": "Example Current Account", "period": "Monthly" } }),
    );
    for (index, month) in (0_i64..).zip(months((2023, 1), (2026, 10), 1)) {
        let (balance, note) = if month == (2023, 3) {
            ("-150.00".to_owned(), Some("Overdrawn"))
        } else {
            (amount(1800 + (index * 370) % 1400), None)
        };
        app.ok(
            "add_bank_entry",
            json!({ "accountId": id_of(&current), "period": period(month), "balance": balance, "note": note }),
        );
    }

    let savings = app.ok(
        "create_bank_account",
        json!({ "input": { "name": "Example Savings", "period": "Yearly" } }),
    );
    for (year, balance, note) in [
        ("2023", 4000, None),
        ("2024", 7500, None),
        ("2025", 11000, Some("Emergency fund topped up")),
        ("2026", 13500, None),
    ] {
        app.ok(
            "add_bank_entry",
            json!({ "accountId": id_of(&savings), "period": year, "balance": amount(balance), "note": note }),
        );
    }

    let holiday = app.ok(
        "create_bank_account",
        json!({ "input": { "name": "Example Holiday Fund", "period": "Monthly" } }),
    );
    for (index, month) in (0_i64..).zip(months((2024, 1), (2026, 10), 3)) {
        app.ok(
            "add_bank_entry",
            json!({ "accountId": id_of(&holiday), "period": period(month), "balance": amount(500 + 250 * index), "note": null }),
        );
    }
}

/// Expenses of every category.
fn add_expenses(app: &Harness) {
    for (name, value, date, category, note) in [
        ("Example Bike", "500.00", "05/03/2023", "Vehicle", ""),
        ("Example Trip", "1250.50", "20/07/2023", "Holiday", ""),
        (
            "Example Washing Machine",
            "650.00",
            "14/11/2023",
            "Appliances",
            "",
        ),
        ("Example Gym", "300.00", "10/01/2024", "Sport", ""),
        ("Example Sofa", "800.00", "11/02/2024", "Home", ""),
        ("Example Camera", "450.75", "22/06/2024", "Hobby", ""),
        ("Example Night Class", "120.00", "02/09/2024", "Study", ""),
        (
            "Example Car Repair",
            "900.00",
            "15/12/2024",
            "Vehicle",
            "Replaced the example brakes",
        ),
        ("Example Ski Trip", "1800.00", "08/02/2025", "Holiday", ""),
        ("Example Kitchen", "2400.00", "03/04/2025", "Home", ""),
        ("Example Laptop", "1100.00", "19/09/2025", "Appliances", ""),
        ("Example Gift", "75.50", "24/12/2025", "Other", ""),
        ("Example Course", "600.00", "12/03/2026", "Study", ""),
        ("Example Tennis Racket", "180.00", "07/05/2026", "Sport", ""),
        ("Example Paint Set", "95.00", "30/08/2026", "Hobby", ""),
        ("Example Dentist", "250.00", "06/10/2026", "Other", ""),
    ] {
        app.ok(
            "add_expense",
            json!({ "input": { "name": name, "amount": value, "date": date, "category": category, "note": note } }),
        );
    }
}

fn debt_input(
    name: &str,
    period: &str,
    category: &str,
    opening_date: &str,
    opening_amount: i64,
    house: &Value,
) -> Value {
    json!({
        "name": name,
        "period": period,
        "category": category,
        "opening_date": opening_date,
        "opening_amount": amount(opening_amount),
        "house_id": house
    })
}

/// A sold house and a held house, with the debts that go with them, a debt
/// without a house, a repaid debt and a debt closed by hand.
fn add_houses_and_debts(app: &Harness) {
    let sold = app.ok(
        "create_house",
        json!({ "input": { "name": "Example House", "period": "Monthly", "purchase_date": "15/03/2023", "purchase_amount": "250000.00" } }),
    );
    let held = app.ok(
        "create_house",
        json!({ "input": { "name": "Example Lodge", "period": "Yearly", "purchase_date": "01/01/2025", "purchase_amount": "100000.00" } }),
    );
    for (index, month) in (0_i64..).zip(months((2023, 6), (2026, 3), 3)) {
        let note = (index == 4).then_some("Valued by an example agent");
        app.ok(
            "add_house_valuation",
            json!({ "houseId": id_of(&sold), "period": period(month), "value": amount(255_000 + 3500 * index), "note": note }),
        );
    }
    for (year, value) in [("2025", 105_000), ("2026", 110_000)] {
        app.ok(
            "add_house_valuation",
            json!({ "houseId": id_of(&held), "period": year, "value": amount(value) }),
        );
    }

    let mortgage = app.ok(
        "create_debt",
        json!({ "input": debt_input("Example Mortgage", "Monthly", "Mortgage", "15/03/2023", 200_000, &sold["id"]) }),
    );
    let home_loan = app.ok(
        "create_debt",
        json!({ "input": debt_input("Example Home Loan", "Monthly", "Loan", "15/03/2023", 30_000, &sold["id"]) }),
    );
    for (index, month) in (1_i64..).zip(months((2023, 6), (2026, 3), 3)) {
        let note = (index == 1).then_some("First example payment");
        app.ok(
            "add_debt_statement",
            json!({ "debtId": id_of(&mortgage), "period": period(month), "balance": amount(200_000 - 2500 * index), "note": note }),
        );
        app.ok(
            "add_debt_statement",
            json!({ "debtId": id_of(&home_loan), "period": period(month), "balance": amount(30_000 - 1500 * index) }),
        );
    }

    let lodge_loan = app.ok(
        "create_debt",
        json!({ "input": debt_input("Example Lodge Loan", "Yearly", "Loan", "01/01/2025", 40_000, &held["id"]) }),
    );
    for (year, balance) in [("2025", 38_000), ("2026", 36_000)] {
        app.ok(
            "add_debt_statement",
            json!({ "debtId": id_of(&lodge_loan), "period": year, "balance": amount(balance) }),
        );
    }

    add_debts_without_a_house(app);

    // The sale settles both debts of the house.
    app.ok(
        "update_house",
        json!({
            "houseId": id_of(&sold),
            "input": {
                "name": "Example House",
                "purchase_date": "15/03/2023",
                "purchase_amount": "250000.00",
                "sell_date": "10/06/2026",
                "sell_amount": "300000.00"
            },
            "links": { "settlements": [
                { "debt_id": mortgage["id"], "outcome": "Repaid" },
                { "debt_id": home_loan["id"], "outcome": "Cancelled" }
            ] }
        }),
    );
}

/// A scholarship, a debt at 0.00 (the Repaid mark) and a debt closed by hand.
fn add_debts_without_a_house(app: &Harness) {
    let student = app.ok(
        "create_debt",
        json!({ "input": debt_input("Example Student Loan", "Yearly", "Scholarship", "01/01/2023", 12_000, &Value::Null) }),
    );
    for (year, balance) in [
        ("2023", 10_000),
        ("2024", 8000),
        ("2025", 6000),
        ("2026", 4000),
    ] {
        app.ok(
            "add_debt_statement",
            json!({ "debtId": id_of(&student), "period": year, "balance": amount(balance) }),
        );
    }

    // At 0.00: the Repaid mark.
    let car = app.ok(
        "create_debt",
        json!({ "input": debt_input("Example Car Loan", "Yearly", "Loan", "01/01/2023", 9000, &Value::Null) }),
    );
    for (year, balance) in [("2023", 6000), ("2024", 3000), ("2025", 0)] {
        app.ok(
            "add_debt_statement",
            json!({ "debtId": id_of(&car), "period": year, "balance": amount(balance) }),
        );
    }

    // Closed by hand: a debt without a house.
    let card = app.ok(
        "create_debt",
        json!({ "input": debt_input("Example Credit Card", "Monthly", "Other", "01/01/2026", 1500, &Value::Null) }),
    );
    for (month, balance) in [
        ((2026, 1), 1200),
        ((2026, 2), 900),
        ((2026, 3), 600),
        ((2026, 4), 300),
    ] {
        app.ok(
            "add_debt_statement",
            json!({ "debtId": id_of(&card), "period": period(month), "balance": amount(balance) }),
        );
    }
    let mut closed = debt_input(
        "Example Credit Card",
        "Monthly",
        "Other",
        "01/01/2026",
        1500,
        &Value::Null,
    );
    closed["closing"] = json!({ "date": "15/05/2026", "amount": "300.00", "outcome": "Repaid" });
    app.ok(
        "update_debt",
        json!({ "debtId": id_of(&card), "input": closed }),
    );
}

/// The example project as the generator writes it.
fn generate() -> String {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example.varde");
    let app = harness();
    fix_today(&app);
    build_example_project(&app);
    app.window
        .state::<AppState>()
        .project()
        .unwrap()
        .save_as(path.clone())
        .unwrap();
    fix_ids(&fs::read_to_string(&path).unwrap())
}

/// An app that has opened the committed example file.
fn open_example() -> Harness {
    let app = harness();
    fix_today(&app);
    app.window
        .state::<AppState>()
        .project()
        .unwrap()
        .open(&example_path())
        .unwrap();
    app
}

/// Every string of the file, with the key it sits under.
fn strings(value: &Value, key: &str, out: &mut Vec<(String, String)>) {
    match value {
        Value::String(text) => out.push((key.to_owned(), text.clone())),
        Value::Array(items) => items.iter().for_each(|item| strings(item, key, out)),
        Value::Object(map) => map.iter().for_each(|(name, item)| strings(item, name, out)),
        _ => {}
    }
}

/// `YYYY`, `YYYY-MM` or `YYYY-MM-DD` as `(year, month)`, month 0 for a year.
fn as_date_text(text: &str) -> Option<(i32, u32)> {
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    let parts: Vec<&str> = text.split('-').collect();
    match parts.as_slice() {
        [year] if year.len() == 4 && digits(year) => Some((year.parse().ok()?, 0)),
        [year, month] | [year, month, _]
            if year.len() == 4 && month.len() == 2 && digits(year) && digits(month) =>
        {
            Some((year.parse().ok()?, month.parse().ok()?))
        }
        _ => None,
    }
}

#[test]
fn test_csd002_ac_22_1_the_committed_example_is_what_the_generator_writes() {
    let generated = generate();
    let path = example_path();
    if std::env::var_os("EXAMPLE_UPDATE").is_some() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &generated).unwrap();
        return;
    }
    let committed = fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!("examples/example.varde is missing: run EXAMPLE_UPDATE=1 cargo test example")
    });
    assert!(
        committed == generated,
        "examples/example.varde differs from the generator. If the change is intended, run \
         EXAMPLE_UPDATE=1 cargo test example and review the diff of examples/."
    );
}

#[test]
fn test_csd002_ac_21_1_the_example_opens_and_every_area_has_records() {
    let app = open_example();
    let reads = read_everything(&app);
    assert_eq!(
        app.ok("project_status", json!({})),
        json!({ "unsaved_changes": false, "has_path": true })
    );
    for command in [
        "list_jobs",
        "list_entries",
        "list_investment_accounts",
        "list_investment_statements",
        "list_bank_accounts",
        "list_bank_entries",
        "list_debts",
        "list_debt_statements",
        "list_houses",
        "list_house_valuations",
        "list_expenses",
    ] {
        let value = &reads[command];
        let filled = match value {
            Value::Array(items) => !items.is_empty(),
            Value::Object(map) => map
                .values()
                .any(|part| part.as_array().is_some_and(|items| !items.is_empty())),
            _ => false,
        };
        assert!(filled, "{command} is empty on the example project: {value}");
    }
    assert_eq!(reads["get_settings"]["currency"], "EUR");
}

#[test]
fn test_csd002_ac_21_2_the_example_shows_every_state() {
    let app = open_example();
    let file: Value = serde_json::from_str(&fs::read_to_string(example_path()).unwrap()).unwrap();

    let jobs = file["income"]["jobs"].as_array().unwrap();
    let periods: Vec<&str> = jobs
        .iter()
        .filter_map(|job| job["period"].as_str())
        .collect();
    assert!(periods.contains(&"Monthly") && periods.contains(&"Yearly"));
    assert!(jobs.iter().any(|job| !job["end_date"].is_null()));
    assert!(jobs.iter().any(|job| job["end_date"].is_null()));
    assert!(jobs.iter().any(|job| job["title"].is_null()));

    for section in ["investments", "bank_accounts"] {
        let accounts = file[section]["accounts"].as_array().unwrap();
        let periods: Vec<&str> = accounts
            .iter()
            .filter_map(|account| account["period"].as_str())
            .collect();
        assert!(
            periods.contains(&"Monthly") && periods.contains(&"Yearly"),
            "{section}"
        );
    }

    let expenses = file["expenses"].to_string();
    for category in [
        "Home",
        "Vehicle",
        "Holiday",
        "Sport",
        "Study",
        "Appliances",
        "Hobby",
        "Other",
    ] {
        assert!(
            expenses.contains(&format!("\"{category}\"")),
            "no expense of category {category}"
        );
    }

    let reads = read_everything(&app);
    let houses = reads["list_houses"].to_string();
    assert!(houses.contains("sell_date") && houses.contains("purchase_date"));
    let debts = file["debts"].to_string();
    for outcome in ["Repaid", "Cancelled"] {
        assert!(
            debts.contains(&format!("\"{outcome}\"")),
            "no debt settled as {outcome}"
        );
    }
    for category in ["Mortgage", "Loan", "Scholarship", "Other"] {
        assert!(
            debts.contains(&format!("\"{category}\"")),
            "no debt of category {category}"
        );
    }
}

#[test]
fn test_csd002_ac_21_3_the_example_is_synthetic_and_inside_its_window() {
    let file: Value = serde_json::from_str(&fs::read_to_string(example_path()).unwrap()).unwrap();
    let mut found = Vec::new();
    strings(&file, "", &mut found);
    assert_ne!(found.len(), 0);
    for (key, text) in &found {
        if matches!(key.as_str(), "employer" | "name") {
            assert!(
                text.starts_with("Example"),
                "{key} {text:?} does not start with Example"
            );
        }
        if let Some((year, month)) = as_date_text(text) {
            assert!(
                (2023..=2026).contains(&year) && (year, month) <= (2026, 10),
                "{key} {text:?} lies outside 2023-01 to 2026-10"
            );
        }
    }
}

#[test]
fn test_csd002_ac_21_4_only_the_example_project_file_is_tracked() {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root())
        .args(["ls-files", "--", "*.varde"])
        .output();
    let Ok(output) = output else {
        return; // no git: nothing to check
    };
    if !output.status.success() {
        return; // not a git checkout (a source archive)
    }
    let tracked = String::from_utf8(output.stdout).unwrap();
    assert_eq!(tracked.trim(), "examples/example.varde");
}
