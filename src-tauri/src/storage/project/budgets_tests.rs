//! Tests of CSD-037 REQ-12 and CSD-002 REQ-30 for the project file: the
//! `budgets` section and the upgrade of a version 4 file on opening.
//! Synthetic data only.

use std::fs;

use tempfile::TempDir;

use super::*;
use crate::domain::common::YearMonth;
use crate::domain::money::Money;
use crate::domain::transactions::BudgetScope;

fn month(year: i32, month: u8) -> YearMonth {
    YearMonth::new(year, month).unwrap()
}

fn saved_value(
    project: &mut Project,
    dir: &TempDir,
    name: &str,
) -> (std::path::PathBuf, serde_json::Value) {
    let path = dir.path().join(name);
    project.save_as(path.clone()).unwrap();
    let value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    (path, value)
}

fn groceries(project: &Project) -> crate::domain::transactions::CategoryId {
    project
        .transactions()
        .categories()
        .iter()
        .find(|category| category.name == "Groceries")
        .unwrap()
        .id
}

#[test]
fn test_csd037_ac_12_1_a_saved_new_project_has_an_empty_budgets_section() {
    let dir = TempDir::new().unwrap();
    let (_, value) = saved_value(&mut Project::new(), &dir, "example-a.varde");
    assert_eq!(value["format_version"], 5);
    assert_eq!(value["budgets"], serde_json::json!([]));
}

#[test]
fn test_csd037_ac_12_5_budgets_survive_saving_and_opening() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    let id = groceries(&project);
    let amount = |text| Money::parse(text).unwrap();
    project
        .transactions_mut()
        .set_budget(id, month(2026, 1), amount("400.00"), BudgetScope::All)
        .unwrap();
    project
        .transactions_mut()
        .set_budget(id, month(2026, 10), amount("500.00"), BudgetScope::Month)
        .unwrap();
    let (path, _) = saved_value(&mut project, &dir, "example-a.varde");
    let mut reopened = Project::new();
    reopened.open(&path).unwrap();
    assert_eq!(
        reopened.transactions().budgets(),
        project.transactions().budgets()
    );
    let view = reopened
        .transactions()
        .budgets_view(month(2026, 10), month(2026, 10));
    let row = view
        .rows
        .iter()
        .find(|row| row.name == "Groceries")
        .unwrap();
    assert_eq!(row.budget, Some(amount("500.00")));
    assert!(row.single_month);
}

#[test]
fn test_csd037_ac_12_4_invalid_budgets_in_the_file_are_rejected() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    let (_, good) = saved_value(&mut project, &dir, "example-a.varde");
    let id = serde_json::to_value(groceries(&project)).unwrap();
    let entry = |amount: &str, month: &str| serde_json::json!({ "category_id": id, "from": [{ "month": month, "amount": amount }], "only": [] });
    let cases = [
        ("negative", serde_json::json!([entry("-1.00", "2026-10")])),
        (
            "three decimals",
            serde_json::json!([entry("400.005", "2026-10")]),
        ),
        ("bad month", serde_json::json!([entry("1.00", "2026-13")])),
        (
            "unknown category",
            serde_json::json!([{ "category_id": "00000000-0000-0000-0000-000000000000", "from": [], "only": [] }]),
        ),
        (
            "same month twice",
            serde_json::json!([{ "category_id": id, "from": [
                { "month": "2026-10", "amount": "1.00" }, { "month": "2026-10", "amount": "2.00" }
            ], "only": [] }]),
        ),
        (
            "same category twice",
            serde_json::json!([entry("1.00", "2026-10"), entry("2.00", "2026-11")]),
        ),
    ];
    for (name, budgets) in cases {
        let mut value = good.clone();
        value["budgets"] = budgets;
        let path = dir.path().join(format!("{}.varde", name.replace(' ', "-")));
        fs::write(&path, value.to_string()).unwrap();
        let error = Project::new().open(&path).unwrap_err();
        assert_eq!(error.code(), "PROJECT_INVALID_FILE", "{name}");
    }
}

/// A version 4 file is the current file without the section `budgets`.
fn version_4_file(dir: &TempDir, with_budgets: bool) -> std::path::PathBuf {
    let mut project = Project::new();
    let (_, mut value) = saved_value(&mut project, dir, "current.varde");
    value["format_version"] = serde_json::json!(4);
    if !with_budgets {
        value.as_object_mut().unwrap().remove("budgets");
    }
    let path = dir.path().join("old.varde");
    fs::write(&path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
    path
}

#[test]
fn test_csd002_ac_30_1_a_version_4_file_opens_without_budgets_and_without_changes() {
    let dir = TempDir::new().unwrap();
    let path = version_4_file(&dir, false);
    let original = fs::read_to_string(&path).unwrap();
    let mut project = Project::new();
    project.open(&path).unwrap();
    assert_eq!(project.transactions().budgets().len(), 0);
    assert!(!project.status().unsaved_changes);
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        original,
        "the file is not touched"
    );
}

#[test]
fn test_csd002_ac_30_2_the_first_save_writes_version_5_and_keeps_the_original() {
    let dir = TempDir::new().unwrap();
    let path = version_4_file(&dir, false);
    let original = fs::read_to_string(&path).unwrap();
    let mut project = Project::new();
    project.open(&path).unwrap();
    project.save().unwrap();
    let backup = fs::read_to_string(dir.path().join("old.varde.bak")).unwrap();
    assert_eq!(backup, original);
    let saved: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(saved["format_version"], 5);
    assert_eq!(saved["budgets"], serde_json::json!([]));
}

#[test]
fn test_csd002_ac_30_3_a_version_4_file_with_a_budgets_section_is_invalid() {
    let dir = TempDir::new().unwrap();
    let path = version_4_file(&dir, true);
    let error = Project::new().open(&path).unwrap_err();
    assert_eq!(error.code(), "PROJECT_INVALID_FILE");
}

#[test]
fn test_csd002_ac_30_4_a_version_3_file_is_still_refused_as_older() {
    let dir = TempDir::new().unwrap();
    let path = version_4_file(&dir, false);
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    value["format_version"] = serde_json::json!(3);
    fs::write(&path, value.to_string()).unwrap();
    let error = Project::new().open(&path).unwrap_err();
    assert_eq!(error.code(), "PROJECT_FILE_OLDER");
}
