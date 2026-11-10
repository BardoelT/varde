//! Acceptance tests of CSD-024 REQ-8 and REQ-9 (the window title) through the
//! real commands on the mock runtime. Synthetic data only.

use serde_json::json;
use tempfile::TempDir;

use crate::ipc_tests::{harness, Harness};

fn add_job(app: &Harness) {
    app.ok(
        "create_job",
        json!({ "input": {
            "employer": "Example Employer", "title": "",
            "start_date": "01/03/2024", "end_date": ""
        } }),
    );
}

/// Saves the open project like the Save As flow does, then publishes it.
fn save_as(app: &Harness, path: &std::path::Path) {
    app.with_project(|project| project.save_as(path.to_path_buf()).unwrap());
    app.ok("project_status", json!({}));
}

#[test]
fn test_csd024_ac_8_1_a_new_project_is_untitled() {
    let app = harness();
    app.ok("new_project", json!({}));
    assert_eq!(app.title(), "Untitled");
}

#[test]
fn test_csd024_ac_9_1_an_edit_adds_the_dot_to_untitled() {
    let app = harness();
    add_job(&app);
    assert_eq!(app.title(), "Untitled ●");
}

#[test]
fn test_csd024_ac_8_2_and_9_2_an_opened_file_shows_its_name_and_the_dot() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("something.varde");
    let source = harness();
    add_job(&source);
    save_as(&source, &path);

    let app = harness();
    app.with_project(|project| project.open(&path).unwrap());
    app.ok("project_status", json!({}));
    assert_eq!(app.title(), "something.varde");
    add_job(&app);
    assert_eq!(app.title(), "something.varde ●");
}

#[test]
fn test_csd024_ac_8_3_and_9_3_save_as_changes_the_name_and_clears_the_dot() {
    let dir = TempDir::new().unwrap();
    let app = harness();
    add_job(&app);
    save_as(&app, &dir.path().join("example-a.varde"));
    assert_eq!(app.title(), "example-a.varde");
    save_as(&app, &dir.path().join("example-b.varde"));
    assert_eq!(app.title(), "example-b.varde");
}

#[test]
fn test_csd024_ac_8_4_a_new_project_after_a_file_is_untitled() {
    let dir = TempDir::new().unwrap();
    let app = harness();
    save_as(&app, &dir.path().join("something.varde"));
    assert_eq!(app.title(), "something.varde");
    app.ok("new_project", json!({}));
    assert_eq!(app.title(), "Untitled");
}

#[test]
fn test_csd024_ac_9_4_undo_and_redo_move_the_dot() {
    let dir = TempDir::new().unwrap();
    let app = harness();
    save_as(&app, &dir.path().join("something.varde"));
    add_job(&app);
    assert_eq!(app.title(), "something.varde ●");
    app.ok("undo", json!({}));
    assert_eq!(app.title(), "something.varde");
    app.ok("redo", json!({}));
    assert_eq!(app.title(), "something.varde ●");
}

#[test]
fn test_csd024_ac_9_5_a_failed_save_keeps_the_dot() {
    let dir = TempDir::new().unwrap();
    let app = harness();
    save_as(&app, &dir.path().join("something.varde"));
    add_job(&app);
    let missing = dir.path().join("missing-folder").join("example.varde");
    let failed = app.with_project(|project| project.save_as(missing).is_err());
    assert!(failed);
    app.ok("project_status", json!({}));
    assert_eq!(app.title(), "something.varde ●");
}

#[test]
fn test_csd024_ac_8_6_the_title_has_no_folder() {
    let dir = TempDir::new().unwrap();
    let app = harness();
    save_as(&app, &dir.path().join("something.varde"));
    let title = app.title();
    assert_eq!(title, "something.varde");
    assert!(!title.contains('/'));
}
