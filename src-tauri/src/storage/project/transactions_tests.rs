//! Tests of CSD-032 for the project file and the undo history: the sections,
//! the strict schema and the memory budget. Synthetic data only.

use std::fs;

use tempfile::TempDir;

use super::history::History;
use super::*;
use crate::domain::money::Currency;

const HEADER: &str = "Rekeningnummer;Transactiedatum;Valutacode;CreditDebet;Bedrag;Tegenrekeningnummer;Tegenrekeninghouder;Valutadatum;Betaalwijze;Omschrijving;Type betaling;Machtigingsnummer;Incassant ID;Referentie;Boekdatum";

fn export(count: usize) -> String {
    let mut lines = vec![HEADER.to_owned()];
    for n in 0..count {
        lines.push(format!(
            "NL00EXAM0000000001;01-10-2026;EUR;Afschrijvingen;1,00; ;Example Shop;01-10-2026;Betaalautomaat;Example;;;;R{n};01-10-2026"
        ));
    }
    lines.join("\r\n") + "\r\n"
}

fn project_with(count: usize) -> Project {
    let mut project = Project::new();
    project
        .transactions_mut()
        .import(&export(count), Currency::Eur)
        .unwrap();
    project
}

fn saved_value(project: &mut Project, dir: &TempDir) -> (std::path::PathBuf, serde_json::Value) {
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();
    let value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    (path, value)
}

#[test]
fn test_csd032_ac_17_1_transactions_survive_save_and_open() {
    let dir = TempDir::new().unwrap();
    let mut project = project_with(3);
    let id = project.transactions().categories().first().unwrap().id;
    project
        .transactions_mut()
        .add_rule("Example Shop", id)
        .unwrap();
    let (path, value) = saved_value(&mut project, &dir);
    assert_eq!(value["transactions"].as_array().unwrap().len(), 3);
    assert!(value["transaction_categories"].is_array());
    assert!(value["transaction_rules"].is_array());

    let mut reopened = Project::new();
    reopened.open(&path).unwrap();
    assert_eq!(reopened.transactions(), project.transactions());
    assert!(!reopened.status().unsaved_changes);
}

#[test]
fn test_csd032_ac_17_2_a_version_2_file_is_invalid_until_upgraded() {
    let dir = TempDir::new().unwrap();
    let (path, mut value) = saved_value(&mut project_with(1), &dir);
    value["format_version"] = serde_json::json!(2);
    for section in [
        "transactions",
        "transaction_categories",
        "transaction_rules",
    ] {
        value.as_object_mut().unwrap().remove(section);
    }
    fs::write(&path, value.to_string()).unwrap();
    let error = Project::new().open(&path).unwrap_err();
    assert_eq!(error.code(), "PROJECT_INVALID_FILE");
}

#[test]
fn test_csd032_ac_17_4_a_transaction_with_an_unknown_category_is_invalid() {
    let dir = TempDir::new().unwrap();
    let (path, mut value) = saved_value(&mut project_with(1), &dir);
    value["transactions"][0]["category_id"] =
        serde_json::json!("671166f4-5827-4794-bcdc-54d7824fb650");
    fs::write(&path, value.to_string()).unwrap();
    assert_eq!(
        Project::new().open(&path).unwrap_err().code(),
        "PROJECT_INVALID_FILE"
    );
}

#[test]
fn test_csd032_ac_17_5_an_amount_with_three_decimals_is_invalid() {
    let dir = TempDir::new().unwrap();
    let (path, mut value) = saved_value(&mut project_with(1), &dir);
    value["transactions"][0]["amount"] = serde_json::json!("-23.975");
    fs::write(&path, value.to_string()).unwrap();
    assert_eq!(
        Project::new().open(&path).unwrap_err().code(),
        "PROJECT_INVALID_FILE"
    );
    value["transactions"][0]["amount"] = serde_json::json!("-23.97");
    value["transactions"][0]["unknown"] = serde_json::json!("x");
    fs::write(&path, value.to_string()).unwrap();
    assert!(Project::new().open(&path).is_err());
}

#[test]
fn test_csd032_ac_17_7_a_file_without_a_new_section_is_invalid() {
    for section in [
        "transactions",
        "transaction_categories",
        "transaction_rules",
    ] {
        let dir = TempDir::new().unwrap();
        let (path, mut value) = saved_value(&mut project_with(1), &dir);
        value.as_object_mut().unwrap().remove(section);
        fs::write(&path, value.to_string()).unwrap();
        assert_eq!(
            Project::new().open(&path).unwrap_err().code(),
            "PROJECT_INVALID_FILE",
            "{section}"
        );
    }
}

#[test]
fn test_csd032_ac_19_1_undo_works_with_a_large_project() {
    let mut project = project_with(20_000);
    let id = project.transactions().transactions().first().unwrap().id;
    let before = project.snapshot();
    project.transactions_mut().set_category(id, None).unwrap();
    project.record_step(StepLabel::edit(Record::Transaction), before);
    project.undo().unwrap();
    assert!(
        !project
            .transactions()
            .transactions()
            .first()
            .unwrap()
            .manual
    );
}

#[test]
fn test_csd032_ac_19_2_the_budget_drops_the_oldest_steps_but_keeps_the_newest() {
    // 1,000 transactions weigh about 500 KB; a 1 MB budget holds two lists.
    let mut project = project_with(1000);
    let mut history = History::default();
    for step in 0..10 {
        let before = project.snapshot();
        let id = project.transactions().transactions().first().unwrap().id;
        let category = project.transactions().categories().get(step).map(|c| c.id);
        project
            .transactions_mut()
            .set_category(id, category)
            .unwrap();
        history.push_with_budget(StepLabel::edit(Record::Transaction), before, 1024 * 1024);
    }
    assert!(history.undo_len() < 10);
    assert!(history.undo_len() >= 1);
}

#[test]
fn test_csd032_ac_19_3_steps_that_share_a_list_cost_it_once() {
    // Ten steps that never touch the transactions all share one list.
    let mut project = project_with(1000);
    let mut history = History::default();
    for _ in 0..10 {
        let before = project.snapshot();
        project.settings_mut().set_currency("EUR").unwrap();
        history.push_with_budget(StepLabel::edit(Record::Transaction), before, 1024 * 1024);
    }
    assert_eq!(history.undo_len(), 10);
}

#[test]
fn test_csd032_ac_17_3_the_script_gives_a_version_2_file_the_default_sections() {
    // The script and the app each hold the default categories and rules; this
    // proves they agree. It needs Node, as `npm test` does.
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("scripts")
        .join("migrate-debt-opening.mjs");
    if std::process::Command::new("node")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("node is not installed: the script is not run");
        return;
    }
    let dir = TempDir::new().unwrap();
    let mut fresh = Project::new();
    let (path, mut value) = saved_value(&mut fresh, &dir);
    value["format_version"] = serde_json::json!(2);
    for section in [
        "transactions",
        "transaction_categories",
        "transaction_rules",
    ] {
        value.as_object_mut().unwrap().remove(section);
    }
    fs::write(&path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
    let output = std::process::Command::new("node")
        .arg(&script)
        .arg(&path)
        .output()
        .unwrap();
    assert!(output.status.success());
    let mut upgraded = Project::new();
    upgraded.open(&path).unwrap();
    assert_eq!(upgraded.transactions(), fresh.transactions());
}
