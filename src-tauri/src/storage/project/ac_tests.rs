//! Acceptance tests of CSD-002 that the main test module did not name yet:
//! AC-16.1 (a saved empty project has every section) and AC-18.2 (a missing or
//! zero version is an invalid file).

use std::fs;

use tempfile::TempDir;

use super::*;
use crate::domain::{Income, JobInput};

fn add_job(project: &mut Project) {
    project
        .income_mut()
        .create_job(&JobInput {
            employer: "Example Employer".to_owned(),
            start_date: Some("01/03/2024".to_owned()),
            ..JobInput::default()
        })
        .unwrap();
}

#[test]
fn test_csd002_ac_16_1_a_saved_empty_project_has_every_section() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    let mut project = Project::new();
    project.save_as(path.clone()).unwrap();

    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let object = value.as_object().unwrap();
    assert_eq!(object["format_version"], 2);
    for section in [
        "income",
        "investments",
        "bank_accounts",
        "debts",
        "expenses",
        "houses",
        "settings",
    ] {
        assert!(object.contains_key(section), "section {section} is missing");
    }
    // Only the version and the seven sections are in the file.
    assert_eq!(object.len(), 8);
}

#[test]
fn test_csd002_ac_18_3_a_version_1_file_is_invalid_until_the_script_has_upgraded_it() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    let mut project = Project::new();
    project.save_as(path.clone()).unwrap();
    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    value["format_version"] = serde_json::json!(1);
    let old = dir.path().join("version-1.varde");
    fs::write(&old, value.to_string()).unwrap();
    add_job(&mut project);
    let before: Income = project.income().clone();
    let error = project.open(&old).unwrap_err();
    assert_eq!(error.code(), "PROJECT_INVALID_FILE");
    assert_eq!(project.income(), &before);
}

#[test]
fn test_csd002_ac_18_2_a_missing_or_zero_version_is_an_invalid_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    let mut project = Project::new();
    project.save_as(path.clone()).unwrap();
    let good: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();

    add_job(&mut project);
    let before: Income = project.income().clone();

    for (name, version) in [("zero", Some(0)), ("absent", None)] {
        let mut value = good.clone();
        match version {
            Some(number) => value["format_version"] = serde_json::json!(number),
            None => {
                value.as_object_mut().unwrap().remove("format_version");
            }
        }
        let bad = dir.path().join(format!("version-{name}.varde"));
        fs::write(&bad, value.to_string()).unwrap();
        let error = project.open(&bad).unwrap_err();
        assert_eq!(error.code(), "PROJECT_INVALID_FILE", "version {name}");
        assert_eq!(project.income(), &before, "version {name} changes nothing");
    }
}
