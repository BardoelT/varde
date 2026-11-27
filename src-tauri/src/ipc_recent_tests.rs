//! Acceptance tests of CSD-002 REQ-32, REQ-33, REQ-35 and REQ-36 through the
//! real commands on the mock runtime. Synthetic data only.

use std::path::{Path, PathBuf};

use serde_json::json;
use tauri::Manager;
use tempfile::TempDir;

use crate::ipc_tests::{harness, Harness};
use crate::storage::RecentProjects;

/// A project file with one job.
fn file_with_job(dir: &TempDir, name: &str) -> PathBuf {
    let app = harness();
    app.ok(
        "create_job",
        json!({ "input": {
            "employer": "Example Employer", "title": "",
            "start_date": "01/03/2024", "end_date": ""
        } }),
    );
    let path = dir.path().join(name);
    app.with_project(|project| project.save_as(path.clone()).unwrap());
    path
}

/// An app that keeps its recent list in the folder of `dir`.
fn app_in(dir: &TempDir) -> Harness {
    let app = harness();
    app.with_project(|project| {
        project.restore_with(RecentProjects::in_dir(&dir.path().join("config")), None);
    });
    app
}

fn job_count(app: &Harness) -> usize {
    app.with_project(|project| project.income().jobs().len())
}

fn recent(app: &Harness) -> Vec<PathBuf> {
    app.with_project(|project| project.recent_paths())
}

fn hand_over(app: &Harness, paths: Vec<PathBuf>) {
    crate::handle_system_paths(app.window.app_handle(), paths);
}

#[test]
fn test_csd002_ac_33_1_a_recent_file_opens_by_its_index() {
    let dir = TempDir::new().unwrap();
    let a = file_with_job(&dir, "example-a.varde");
    let b = file_with_job(&dir, "example-b.varde");
    let app = app_in(&dir);
    app.with_project(|project| {
        project.open(&a).unwrap();
        project.open(&b).unwrap();
    });
    app.ok("new_project", json!({}));

    let result = app.ok("open_recent_project", json!({ "index": 1 }));
    assert_eq!(result["done"], true);
    assert_eq!(job_count(&app), 1);
    assert_eq!(app.title(), "example-a.varde");
    assert_eq!(recent(&app), vec![a, b]);
}

#[test]
fn test_csd002_ac_33_3_a_gone_file_fails_without_a_path_and_is_forgotten() {
    let dir = TempDir::new().unwrap();
    let a = file_with_job(&dir, "example-a.varde");
    let b = file_with_job(&dir, "example-b.varde");
    let app = app_in(&dir);
    app.with_project(|project| {
        project.open(&a).unwrap();
        project.open(&b).unwrap();
    });
    std::fs::remove_file(&a).unwrap();

    let error = app.err("open_recent_project", json!({ "index": 1 }));
    assert_eq!(error["code"], "PROJECT_OPEN_FAILED");
    assert!(!error.to_string().contains("example-a"), "{error}");
    assert_eq!(job_count(&app), 1);
    assert_eq!(recent(&app), vec![b]);
}

#[test]
fn test_csd002_ac_33_5_an_index_outside_the_list_has_no_effect() {
    let dir = TempDir::new().unwrap();
    let a = file_with_job(&dir, "example-a.varde");
    let app = app_in(&dir);
    app.with_project(|project| project.open(&a).unwrap());
    let error = app.err("open_recent_project", json!({ "index": 7 }));
    assert_eq!(error["code"], "PROJECT_OPEN_FAILED");
    assert_eq!(job_count(&app), 1);
    assert_eq!(recent(&app), vec![a]);
}

#[test]
fn test_csd002_ac_35_1_a_file_handed_to_the_running_app_waits_for_the_frontend() {
    let dir = TempDir::new().unwrap();
    let a = file_with_job(&dir, "example-a.varde");
    let app = app_in(&dir);
    app.ok("take_startup_notice", json!({}));

    hand_over(&app, vec![a.clone()]);
    assert_eq!(job_count(&app), 0, "nothing opens before the confirmation");

    let result = app.ok("open_pending_file", json!({}));
    assert_eq!(result["done"], true);
    assert_eq!(job_count(&app), 1);
    assert_eq!(recent(&app), vec![a]);

    // Nothing is left to open.
    let again = app.ok("open_pending_file", json!({}));
    assert_eq!(again["done"], false);
}

#[test]
fn test_csd002_ac_35_4_and_35_5_invalid_paths_are_ignored_and_the_first_is_used() {
    let dir = TempDir::new().unwrap();
    let a = file_with_job(&dir, "example-a.varde");
    let b = file_with_job(&dir, "example-b.varde");
    let app = app_in(&dir);
    app.ok("take_startup_notice", json!({}));

    hand_over(
        &app,
        vec![
            Path::new("example-a.varde").to_path_buf(),
            dir.path().join("x.txt"),
        ],
    );
    assert_eq!(app.ok("open_pending_file", json!({}))["done"], false);

    hand_over(&app, vec![a.clone(), b]);
    app.ok("open_pending_file", json!({}));
    assert_eq!(recent(&app), vec![a]);
}

#[test]
fn test_csd002_ac_35_6_a_handed_file_with_invalid_content_changes_nothing() {
    let dir = TempDir::new().unwrap();
    let a = file_with_job(&dir, "example-a.varde");
    let app = app_in(&dir);
    app.with_project(|project| project.open(&a).unwrap());
    app.ok("take_startup_notice", json!({}));
    let bad = dir.path().join("example-bad.varde");
    std::fs::write(&bad, "not a project file").unwrap();

    hand_over(&app, vec![bad]);
    let error = app.err("open_pending_file", json!({}));
    assert_eq!(error["code"], "PROJECT_INVALID_FILE");
    assert!(!error.to_string().contains("example-bad"), "{error}");
    assert_eq!(job_count(&app), 1);
    assert_eq!(recent(&app), vec![a]);
}

#[test]
fn test_csd002_ac_36_1_a_file_handed_before_the_frontend_started_is_the_start_file() {
    let dir = TempDir::new().unwrap();
    let a = file_with_job(&dir, "example-a.varde");
    let b = file_with_job(&dir, "example-b.varde");
    let app = app_in(&dir);
    app.with_project(|project| project.open(&a).unwrap());

    hand_over(&app, vec![b.clone()]);
    assert_eq!(app.title(), "example-b.varde");
    assert_eq!(recent(&app), vec![b, a]);
    assert!(app.ok("take_startup_notice", json!({})).is_null());
}

#[test]
fn test_csd002_ac_36_2_a_start_file_that_cannot_be_opened_leaves_it_empty() {
    let dir = TempDir::new().unwrap();
    let a = file_with_job(&dir, "example-a.varde");
    let app = app_in(&dir);
    app.with_project(|project| project.open(&a).unwrap());
    let bad = dir.path().join("example-bad.varde");
    std::fs::write(&bad, "not a project file").unwrap();

    hand_over(&app, vec![bad]);
    assert_eq!(job_count(&app), 0);
    let notice = app.ok("take_startup_notice", json!({}));
    assert_eq!(notice["code"], "PROJECT_INVALID_FILE");
    assert!(!notice.to_string().contains("example-bad"), "{notice}");
    assert_eq!(recent(&app), vec![a]);
}
