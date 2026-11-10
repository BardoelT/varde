//! Discrete mode (CSD-030) over IPC: choosing, saving and restoring the mode
//! next to the other preferences. Synthetic data only.

use serde_json::json;
use tempfile::TempDir;

use crate::ipc_tests::harness_in;

#[test]
fn test_csd030_ac_2_1_the_mode_is_off_by_default_over_ipc() {
    let dir = TempDir::new().unwrap();
    let app = harness_in(dir.path());
    assert_eq!(app.ok("get_preferences", json!({}))["discrete_mode"], false);
}

#[test]
fn test_csd030_ac_4_1_the_mode_is_saved_and_restored_over_ipc() {
    let dir = TempDir::new().unwrap();
    let app = harness_in(dir.path());
    let answer = app.ok("set_discrete_mode", json!({ "discreteMode": true }));
    assert_eq!(answer["discrete_mode"], true);

    let restarted = harness_in(dir.path());
    assert_eq!(
        restarted.ok("get_preferences", json!({}))["discrete_mode"],
        true
    );
}

#[test]
fn test_csd030_ac_3_4_choosing_the_current_mode_changes_nothing() {
    let dir = TempDir::new().unwrap();
    let app = harness_in(dir.path());
    let answer = app.ok("set_discrete_mode", json!({ "discreteMode": false }));
    assert_eq!(answer["discrete_mode"], false);
    assert_eq!(app.ok("get_preferences", json!({}))["discrete_mode"], false);
}

#[test]
fn test_csd030_ac_4_4_the_other_preferences_keep_the_mode_and_it_keeps_them() {
    let dir = TempDir::new().unwrap();
    let app = harness_in(dir.path());
    app.ok("set_theme", json!({ "theme": "dark" }));
    app.ok("set_language", json!({ "language": "nl" }));
    app.ok(
        "set_number_format",
        json!({ "numberFormat": "comma_point" }),
    );
    let answer = app.ok("set_discrete_mode", json!({ "discreteMode": true }));
    assert_eq!(answer["theme"], "dark");
    assert_eq!(answer["language"], "nl");
    assert_eq!(answer["number_format"], "comma_point");

    let answer = app.ok("set_theme", json!({ "theme": "light" }));
    assert_eq!(answer["discrete_mode"], true);
    let answer = app.ok(
        "set_number_format",
        json!({ "numberFormat": "point_comma" }),
    );
    assert_eq!(answer["discrete_mode"], true);
}

#[test]
fn test_csd030_ac_14_1_a_failed_save_keeps_the_mode_over_ipc() {
    use std::os::unix::fs::PermissionsExt;
    let config = TempDir::new().unwrap();
    let app = harness_in(config.path());
    app.ok("set_discrete_mode", json!({ "discreteMode": false }));

    std::fs::set_permissions(config.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
    let error = app
        .call("set_discrete_mode", json!({ "discreteMode": true }))
        .unwrap_err();
    std::fs::set_permissions(config.path(), std::fs::Permissions::from_mode(0o700)).unwrap();

    assert_eq!(error["code"], "PREFERENCES_SAVE_FAILED");
    let message = error["message"].as_str().unwrap();
    assert!(
        !message.contains(config.path().to_str().unwrap()),
        "{message}"
    );
    assert_eq!(app.ok("get_preferences", json!({}))["discrete_mode"], false);
}

#[test]
fn test_csd030_ac_4_3_the_project_file_holds_no_discrete_mode() {
    let dir = TempDir::new().unwrap();
    let app = harness_in(dir.path());
    app.ok("set_discrete_mode", json!({ "discreteMode": true }));
    let path = dir.path().join("example.varde");
    app.with_project(|project| project.save_as(path.clone()).unwrap());
    let text = std::fs::read_to_string(path).unwrap();
    assert!(!text.contains("discrete"), "{text}");
}
