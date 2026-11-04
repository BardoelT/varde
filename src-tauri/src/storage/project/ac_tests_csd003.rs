//! Acceptance tests of CSD-003 (project settings) that need the project file.

use std::fs;

use tempfile::TempDir;

use super::*;
use crate::domain::{Currency, JobInput};

/// AC-7.4: a file without a settings section is rejected and the project in memory is unchanged.
#[test]
fn test_csd003_ac_7_4_a_file_without_settings_is_rejected() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    project
        .income_mut()
        .create_job(&JobInput {
            employer: "Example Employer".to_owned(),
            start_date: Some("01/03/2024".to_owned()),
            ..JobInput::default()
        })
        .unwrap();
    project.settings_mut().set_currency("GBP").unwrap();
    let good = dir.path().join("example-a.varde");
    project.save_as(good.clone()).unwrap();

    let mut value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&good).unwrap()).unwrap();
    value.as_object_mut().unwrap().remove("settings").unwrap();
    let bad = dir.path().join("example-b.varde");
    fs::write(&bad, value.to_string()).unwrap();

    let error = project.open(&bad).unwrap_err();
    assert_eq!(error.code(), "PROJECT_INVALID_FILE");
    assert_eq!(
        project.income().jobs().len(),
        1,
        "the project in memory is unchanged"
    );
    assert_eq!(project.settings().currency, Currency::Gbp);
}
