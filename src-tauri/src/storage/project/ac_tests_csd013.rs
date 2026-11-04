//! Acceptance tests of CSD-013 (expenses) that need the project file.

use std::fs;

use tempfile::TempDir;

use super::*;
use crate::domain::expenses::ExpenseInput;

fn input(name: &str, amount: &str) -> ExpenseInput {
    ExpenseInput {
        name: name.to_owned(),
        amount: amount.to_owned(),
        date: "10/09/2026".to_owned(),
        category: "Hobby".to_owned(),
        note: None,
    }
}

/// AC-7.6: an edit of a saved expense is an unsaved change.
#[test]
fn test_csd013_ac_7_6_an_edit_is_an_unsaved_change() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    let id = project
        .expenses_mut()
        .add_expense(&input("Example Bike", "900.00"))
        .unwrap()
        .id;
    project.save_as(dir.path().join("example-a.varde")).unwrap();
    assert!(!project.status().unsaved_changes);

    project
        .expenses_mut()
        .update_expense(id, &input("Example Bike", "950.00"))
        .unwrap();
    assert!(project.status().unsaved_changes);
}

/// AC-16.2: a file without an "expenses" section is rejected and the project in memory stays.
#[test]
fn test_csd013_ac_16_2_a_file_without_the_expenses_section_is_rejected() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    project
        .expenses_mut()
        .add_expense(&input("Example Bike", "900.00"))
        .unwrap();
    let good = dir.path().join("example-a.varde");
    project.save_as(good.clone()).unwrap();

    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&good).unwrap()).unwrap();
    value.as_object_mut().unwrap().remove("expenses").unwrap();
    let bad = dir.path().join("example-b.varde");
    fs::write(&bad, value.to_string()).unwrap();

    let error = project.open(&bad).unwrap_err();
    assert_eq!(error.code(), "PROJECT_INVALID_FILE");
    assert_eq!(
        project.expenses().total_count(),
        1,
        "the project in memory is unchanged"
    );
}
