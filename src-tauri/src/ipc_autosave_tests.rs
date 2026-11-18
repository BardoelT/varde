//! Acceptance tests of CSD-002 REQ-23 to REQ-26 (autosave) through the real
//! commands on the mock runtime, on projects in a temporary directory.
//! Synthetic data only.

use std::fs;
use std::path::Path;

use serde_json::json;
use tempfile::TempDir;

use crate::ipc_tests::{harness, harness_in, Harness};
use crate::storage::Preferences;

fn add_job(app: &Harness, employer: &str) {
    app.ok(
        "create_job",
        json!({ "input": {
            "employer": employer, "title": "",
            "start_date": "01/03/2024", "end_date": ""
        } }),
    );
}

/// An app with autosave on whose project was saved to `path`.
fn saved_app(path: &Path) -> Harness {
    let app = harness().with_autosave();
    app.with_project(|project| project.save_as(path.to_path_buf()).unwrap());
    app.ok("project_status", json!({}));
    app
}

fn holds(path: &Path, text: &str) -> bool {
    fs::read_to_string(path).is_ok_and(|content| content.contains(text))
}

#[test]
fn test_csd002_ac_23_1_a_change_is_written_at_once() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    let app = saved_app(&path);
    add_job(&app, "Example Employer");
    assert!(holds(&path, "Example Employer"));
    assert_eq!(app.title(), "example-a.varde");
}

#[test]
fn test_csd002_ac_23_2_one_step_is_one_write() {
    let dir = TempDir::new().unwrap();
    let app = saved_app(&dir.path().join("example-a.varde"));
    add_job(&app, "Example Employer");
    assert_eq!(app.state().autosave_writes(), 1);
    add_job(&app, "Example Other");
    assert_eq!(app.state().autosave_writes(), 2);
}

#[test]
fn test_csd002_ac_23_3_undo_and_redo_are_written() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    let app = saved_app(&path);
    add_job(&app, "Example Employer");
    app.ok("undo", json!({}));
    assert!(!holds(&path, "Example Employer"));
    assert_eq!(app.title(), "example-a.varde");
    app.ok("redo", json!({}));
    assert!(holds(&path, "Example Employer"));
}

#[test]
fn test_csd002_ac_23_4_the_project_currency_is_written() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    let app = saved_app(&path);
    app.ok("set_currency", json!({ "currency": "USD" }));
    assert!(holds(&path, "USD"));
    assert_eq!(app.title(), "example-a.varde");
}

#[test]
fn test_csd002_ac_23_5_a_project_without_a_path_writes_nothing() {
    let app = harness().with_autosave();
    add_job(&app, "Example Employer");
    assert_eq!(app.state().autosave_writes(), 0);
    assert_eq!(app.title(), "Untitled ●");
}

#[test]
fn test_csd002_ac_23_6_after_the_first_save_autosave_takes_over() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    let app = harness().with_autosave();
    add_job(&app, "Example Employer");
    app.with_project(|project| project.save_as(path.clone()).unwrap());
    add_job(&app, "Example Other");
    assert!(holds(&path, "Example Employer") && holds(&path, "Example Other"));
}

#[test]
fn test_csd002_ac_23_7_save_as_moves_autosave_to_the_new_path() {
    let dir = TempDir::new().unwrap();
    let (a, b) = (
        dir.path().join("example-a.varde"),
        dir.path().join("example-b.varde"),
    );
    let app = saved_app(&a);
    app.with_project(|project| project.save_as(b.clone()).unwrap());
    add_job(&app, "Example Employer");
    assert!(holds(&b, "Example Employer"));
    assert!(!holds(&a, "Example Employer"));
}

#[test]
fn test_csd002_ac_23_8_a_rejected_form_writes_nothing() {
    let dir = TempDir::new().unwrap();
    let app = saved_app(&dir.path().join("example-a.varde"));
    let rejected = app.call(
        "create_job",
        json!({ "input": { "employer": "", "title": "", "start_date": "01/03/2024", "end_date": "" } }),
    );
    assert!(rejected.is_err());
    assert_eq!(app.state().autosave_writes(), 0);
}

#[test]
fn test_csd002_ac_24_1_autosave_is_on_by_default() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("preferences.json");
    let load = || Preferences::in_dir(dir.path()).load_autosave();
    assert!(load(), "missing file");
    for text in [r#"{"theme": "dark"}"#, r#"{"autosave": "yes"}"#, "not JSON"] {
        fs::write(&file, text).unwrap();
        assert!(load(), "{text}");
    }
    fs::write(&file, r#"{"autosave": false}"#).unwrap();
    assert!(!load());
}

#[test]
fn test_csd002_ac_24_2_the_choice_is_remembered() {
    let dir = TempDir::new().unwrap();
    let app = harness_in(dir.path());
    let view = app.ok("set_autosave", json!({ "autosave": false }));
    assert_eq!(view["autosave"], false);
    assert!(!Preferences::in_dir(dir.path()).load_autosave());
    assert_eq!(app.ok("get_preferences", json!({}))["autosave"], false);
}

#[test]
fn test_csd002_ac_24_3_with_autosave_off_only_save_writes() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    let app = harness();
    app.with_project(|project| project.save_as(path.clone()).unwrap());
    add_job(&app, "Example Employer");
    assert!(!holds(&path, "Example Employer"));
    assert_eq!(app.title(), "example-a.varde ●");
    app.with_project(|project| project.save().unwrap());
    assert!(holds(&path, "Example Employer"));
}

#[test]
fn test_csd002_ac_24_4_a_preference_that_cannot_be_saved_is_an_error() {
    // Without a config folder nothing can be saved.
    let app = harness();
    let error = app
        .call("set_autosave", json!({ "autosave": false }))
        .unwrap_err();
    assert_eq!(error["code"], "PREFERENCES_SAVE_FAILED");
}

#[test]
fn test_csd002_ac_25_1_switching_on_saves_what_is_pending() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    let app = harness_in(dir.path());
    app.with_project(|project| project.save_as(path.clone()).unwrap());
    add_job(&app, "Example Employer");
    assert!(!holds(&path, "Example Employer"));
    app.ok("set_autosave", json!({ "autosave": true }));
    assert!(holds(&path, "Example Employer"));
    assert_eq!(app.title(), "example-a.varde");
}

#[test]
fn test_csd002_ac_25_2_switching_on_without_a_path_writes_nothing() {
    let dir = TempDir::new().unwrap();
    let app = harness_in(dir.path());
    add_job(&app, "Example Employer");
    app.ok("set_autosave", json!({ "autosave": true }));
    assert_eq!(app.state().autosave_writes(), 0);
    assert_eq!(app.title(), "Untitled ●");
}

#[test]
fn test_csd002_ac_25_3_switching_off_changes_nothing_on_disk() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    let app = harness_in(&dir.path().join("config")).with_autosave();
    app.with_project(|project| project.save_as(path.clone()).unwrap());
    let before = fs::read(&path).unwrap();
    app.ok("set_autosave", json!({ "autosave": false }));
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[cfg(unix)]
mod failing_write {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn lock(dir: &Path, locked: bool) {
        let mode = if locked { 0o500 } else { 0o700 };
        fs::set_permissions(dir, fs::Permissions::from_mode(mode)).unwrap();
    }

    #[test]
    fn test_csd002_ac_26_1_to_26_3_a_failed_write_keeps_the_change_and_retries() {
        let dir = TempDir::new().unwrap();
        let folder = dir.path().join("work");
        fs::create_dir(&folder).unwrap();
        let path = folder.join("example-a.varde");
        let app = saved_app(&path);
        let before = fs::read(&path).unwrap();

        lock(&folder, true);
        add_job(&app, "Example Employer");
        assert_eq!(
            app.state().last_autosave_failure(),
            Some("PROJECT_SAVE_FAILED")
        );
        assert_eq!(fs::read(&path).unwrap(), before, "the file is unchanged");
        assert_eq!(app.title(), "example-a.varde ●");
        let jobs = app.ok("list_jobs", json!({}));
        assert_eq!(
            jobs.as_array().map(Vec::len),
            Some(1),
            "the job is still shown"
        );

        lock(&folder, false);
        add_job(&app, "Example Other");
        assert!(holds(&path, "Example Employer") && holds(&path, "Example Other"));
        assert_eq!(app.title(), "example-a.varde");
    }

    #[test]
    fn test_csd002_ac_26_3_save_retries_by_hand() {
        let dir = TempDir::new().unwrap();
        let folder = dir.path().join("work");
        fs::create_dir(&folder).unwrap();
        let path = folder.join("example-a.varde");
        let app = saved_app(&path);
        lock(&folder, true);
        add_job(&app, "Example Employer");
        lock(&folder, false);
        app.with_project(|project| project.save().unwrap());
        assert!(holds(&path, "Example Employer"));
    }
}
