//! The recent list and the files the system hands over (CSD-002 REQ-31, REQ-33,
//! REQ-36).

use std::fs;

use tempfile::TempDir;

use super::*;
use crate::domain::JobInput;

fn config(dir: &TempDir) -> RecentProjects {
    RecentProjects::in_dir(&dir.path().join("config"))
}

fn restart(dir: &TempDir) -> Project {
    let mut project = Project::new();
    project.restore(config(dir));
    project
}

fn jobs(project: &Project) -> usize {
    project.income().jobs().len()
}

/// A project file with one job, written by a project of its own.
fn file_with_job(dir: &TempDir, name: &str) -> PathBuf {
    let mut project = Project::new();
    project
        .income_mut()
        .create_job(&JobInput {
            employer: "Example Employer".to_owned(),
            start_date: Some("01/03/2024".to_owned()),
            ..JobInput::default()
        })
        .unwrap();
    let path = dir.path().join(name);
    project.save_as(path.clone()).unwrap();
    path
}

fn path(dir: &TempDir, name: &str) -> PathBuf {
    dir.path().join(name)
}

#[test]
fn test_csd002_ac_31_1_open_and_save_put_the_path_on_top() {
    let dir = TempDir::new().unwrap();
    let a = file_with_job(&dir, "example-a.varde");
    let b = file_with_job(&dir, "example-b.varde");
    let mut project = restart(&dir);
    project.open(&a).unwrap();
    project.open(&b).unwrap();
    assert_eq!(project.recent_paths(), vec![b.clone(), a.clone()]);

    project.reset();
    project.save_as(path(&dir, "example-c.varde")).unwrap();
    assert_eq!(
        project.recent_paths(),
        vec![path(&dir, "example-c.varde"), b, a]
    );
}

#[test]
fn test_csd002_ac_31_4_new_project_and_a_failed_save_change_nothing() {
    let dir = TempDir::new().unwrap();
    let a = file_with_job(&dir, "example-a.varde");
    let mut project = restart(&dir);
    project.open(&a).unwrap();

    project.reset();
    assert_eq!(project.recent_paths(), vec![a.clone()]);
    let unwritable = dir.path().join("no-such-dir").join("example-b.varde");
    assert!(project.save_as(unwritable).is_err());
    assert_eq!(project.recent_paths(), vec![a]);
}

#[test]
fn test_csd002_ac_31_5_the_first_entry_is_reopened_at_start() {
    let dir = TempDir::new().unwrap();
    let a = file_with_job(&dir, "example-a.varde");
    let b = file_with_job(&dir, "example-b.varde");
    let mut project = restart(&dir);
    project.open(&a).unwrap();
    project.open(&b).unwrap();
    drop(project);

    let next = restart(&dir);
    assert_eq!(next.path_for_test(), Some(b));
}

#[test]
fn test_csd002_ac_31_6_the_old_remembered_path_is_reopened() {
    let dir = TempDir::new().unwrap();
    let a = file_with_job(&dir, "example-a.varde");
    let folder = dir.path().join("config");
    fs::create_dir_all(&folder).unwrap();
    let text = serde_json::json!({ "path": a.to_str().unwrap() }).to_string();
    fs::write(folder.join("last-project.json"), text).unwrap();

    let project = restart(&dir);
    assert_eq!(jobs(&project), 1);
    assert_eq!(project.recent_paths(), vec![a]);
}

#[test]
fn test_csd002_ac_32_5_clear_menu_empties_the_list_and_keeps_the_project() {
    let dir = TempDir::new().unwrap();
    let a = file_with_job(&dir, "example-a.varde");
    let mut project = restart(&dir);
    project.open(&a).unwrap();

    project.clear_recent();
    assert_eq!(project.recent_paths(), Vec::<PathBuf>::new());
    assert_eq!(jobs(&project), 1);
    assert!(!project.status().unsaved_changes);

    let mut next = restart(&dir);
    assert_eq!(jobs(&next), 0);
    assert!(!next.status().has_path);
    assert!(next.take_startup_notice().is_none());
}

#[test]
fn test_csd002_ac_33_1_a_recent_file_opens() {
    let dir = TempDir::new().unwrap();
    let a = file_with_job(&dir, "example-a.varde");
    let b = file_with_job(&dir, "example-b.varde");
    let mut project = restart(&dir);
    project.open(&a).unwrap();
    project.open(&b).unwrap();
    project.reset();

    project.open_recent(1).unwrap();
    assert_eq!(jobs(&project), 1);
    assert_eq!(project.path_for_test(), Some(a.clone()));
    assert_eq!(project.recent_paths(), vec![a, b]);
}

#[test]
fn test_csd002_ac_33_3_a_file_gone_for_good_is_forgotten() {
    let dir = TempDir::new().unwrap();
    let a = file_with_job(&dir, "example-a.varde");
    let b = file_with_job(&dir, "example-b.varde");
    let mut project = restart(&dir);
    project.open(&a).unwrap();
    project.open(&b).unwrap();
    fs::remove_file(&a).unwrap();
    project.reset();
    project.open(&b).unwrap();

    let error = project.open_recent(1).unwrap_err();
    assert_eq!(error.code(), "PROJECT_OPEN_FAILED");
    assert!(!error.to_string().contains("example-a"));
    assert_eq!(jobs(&project), 1, "the open project is unchanged");
    assert_eq!(project.recent_paths(), vec![b]);
}

#[test]
fn test_csd002_ac_33_4_any_other_problem_keeps_the_entry() {
    let cases = [
        (
            "invalid",
            Some("not a project file"),
            "PROJECT_INVALID_FILE",
        ),
        (
            "newer",
            Some(r#"{"format_version":6}"#),
            "PROJECT_FILE_NEWER",
        ),
        ("unmounted", None, "PROJECT_OPEN_FAILED"),
    ];
    for (name, content, code) in cases {
        let dir = TempDir::new().unwrap();
        let b = file_with_job(&dir, "example-b.varde");
        let a = match content {
            Some(text) => {
                let a = path(&dir, "example-a.varde");
                fs::write(&a, text).unwrap();
                a
            }
            // A missing folder is transient, not gone for good (REQ-13).
            None => path(&dir, "example-folder").join("example-a.varde"),
        };
        let mut project = restart(&dir);
        config(&dir).record(&a).unwrap();
        project.open(&b).unwrap();
        // `a` was listed first before opening `b`; list it again second.
        config(&dir).record(&b).unwrap();

        let error = project.open_recent(1).unwrap_err();
        assert_eq!(error.code(), code, "{name}");
        assert_eq!(jobs(&project), 1, "{name}");
        assert!(project.recent_paths().contains(&a), "{name}");
    }
}

#[test]
fn test_csd002_ac_33_5_an_index_outside_the_list_is_rejected() {
    let dir = TempDir::new().unwrap();
    let a = file_with_job(&dir, "example-a.varde");
    let mut project = restart(&dir);
    project.open(&a).unwrap();
    assert!(project.open_recent(1).is_err());
    assert!(project.open_recent(usize::MAX).is_err());
    assert_eq!(project.recent_paths(), vec![a]);
}

#[test]
fn test_csd002_ac_36_1_the_handed_file_wins_at_start() {
    let dir = TempDir::new().unwrap();
    let a = file_with_job(&dir, "example-a.varde");
    let b = file_with_job(&dir, "example-b.varde");
    let mut first = restart(&dir);
    first.open(&a).unwrap();
    drop(first);

    let mut project = Project::new();
    project.restore_with(config(&dir), Some(b.clone()));
    assert_eq!(project.path_for_test(), Some(b.clone()));
    assert_eq!(project.recent_paths(), vec![b, a]);
}

#[test]
fn test_csd002_ac_36_2_a_handed_file_that_cannot_be_opened_leaves_it_empty() {
    let dir = TempDir::new().unwrap();
    let a = file_with_job(&dir, "example-a.varde");
    let mut first = restart(&dir);
    first.open(&a).unwrap();
    drop(first);
    let bad = path(&dir, "example-bad.varde");
    fs::write(&bad, "not a project file").unwrap();

    let mut project = Project::new();
    project.restore_with(config(&dir), Some(bad));
    assert_eq!(jobs(&project), 0);
    assert!(!project.status().has_path);
    let notice = project.take_startup_notice().unwrap();
    assert_eq!(notice.code(), "PROJECT_INVALID_FILE");
    assert!(!notice.to_string().contains("example-bad"));
    assert!(project.take_startup_notice().is_none());
    assert_eq!(project.recent_paths(), vec![a]);
}

#[test]
fn test_csd002_ac_36_3_without_a_handed_file_the_remembered_one_opens() {
    let dir = TempDir::new().unwrap();
    let a = file_with_job(&dir, "example-a.varde");
    let mut first = restart(&dir);
    first.open(&a).unwrap();
    drop(first);

    let mut project = Project::new();
    project.restore_with(config(&dir), None);
    assert_eq!(project.path_for_test(), Some(a));
}
