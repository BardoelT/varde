//! Acceptance tests over IPC for CSD-003 and CSD-013. They call the real commands the way the
//! frontend does (see `ipc_tests.rs`); the test names carry the AC id.

use serde_json::{json, Value};

use crate::ipc_tests::Harness;

mod csd003;
mod csd013;

fn named(name: &str, period: &str) -> Value {
    json!({ "name": name, "period": period })
}

fn job(app: &Harness, employer: &str, start: &str, period: &str) -> String {
    let job = app.ok(
        "create_job",
        json!({ "input": { "employer": employer, "start_date": start, "period": period } }),
    );
    job["id"].as_str().unwrap().to_owned()
}

fn paycheck(app: &Harness, job_id: &str, month: &str, gross: &str, holiday: &str, bonus: &str) {
    app.ok(
        "add_paycheck",
        json!({ "jobId": job_id, "month": month, "amounts": {
            "gross_salary": gross, "holiday_pay": holiday, "bonus": bonus
        } }),
    );
}

fn statement(app: &Harness, job_id: &str, year: &str, gross: &str, holiday: &str, bonus: &str) {
    app.ok(
        "add_statement",
        json!({ "jobId": job_id, "year": year, "amounts": {
            "gross_salary": gross, "holiday_pay": holiday, "bonus": bonus
        } }),
    );
}

fn expense(app: &Harness, name: &str, amount: &str, date: &str, category: &str) -> Value {
    app.ok(
        "add_expense",
        json!({ "input": { "name": name, "amount": amount, "date": date, "category": category } }),
    )
}
