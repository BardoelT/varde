//! Tests of CSD-002 REQ-27 (a saved project can always be opened again), REQ-28
//! (one backup per file per session) and REQ-29 (an older file has its own
//! error). Synthetic data only.

use std::fs;

use tempfile::TempDir;

use super::*;
use crate::domain::{JobId, JobInput};
use crate::storage::MAX_FILE_BYTES;

fn add_job(project: &mut Project, employer: &str) -> JobId {
    project
        .income_mut()
        .create_job(&JobInput {
            employer: employer.to_owned(),
            start_date: Some("01/03/2024".to_owned()),
            ..JobInput::default()
        })
        .unwrap()
        .id
}

fn backup_of(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap().to_os_string();
    name.push(".bak");
    path.with_file_name(name)
}

// ---- REQ-27: a project is never saved larger than it can be read ----

#[test]
fn test_csd002_ac_27_1_a_project_over_the_limit_is_not_saved_and_the_old_file_stays() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    let mut project = Project::new();
    project.save_as(path.clone()).unwrap();
    let before = fs::read(&path).unwrap();
    add_job(&mut project, "Example Employer");

    let error = project.write_limited_for_test(&path, 10).unwrap_err();
    assert_eq!(error.code(), "PROJECT_TOO_LARGE");
    assert_eq!(fs::read(&path).unwrap(), before, "the file is unchanged");
    assert!(!error.to_string().contains('/'), "no path in the message");
}

#[test]
fn test_csd002_ac_27_1_a_project_within_the_limit_is_written() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    let mut project = Project::new();
    add_job(&mut project, "Example Employer");
    let size = project.file_size().unwrap();
    project.write_limited_for_test(&path, size).unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().len(),
        size,
        "exactly at the limit is allowed"
    );
}

#[test]
fn test_csd002_ac_27_3_the_limit_of_writing_is_the_limit_of_reading() {
    assert_eq!(MAX_FILE_BYTES, 50 * 1024 * 1024);
}

// ---- REQ-28: one backup per file per session ----

#[test]
fn test_csd002_ac_28_1_the_first_save_over_an_existing_file_keeps_its_old_content() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    let mut first = Project::new();
    add_job(&mut first, "Example Employer");
    first.save_as(path.clone()).unwrap();
    let original = fs::read(&path).unwrap();

    // A new session: the file is opened, changed and saved.
    let mut session = Project::new();
    session.open(&path).unwrap();
    add_job(&mut session, "Example Other Employer");
    session.save().unwrap();

    assert_eq!(
        fs::read(backup_of(&path)).unwrap(),
        original,
        "the backup holds the opened state"
    );
    assert_ne!(
        fs::read(&path).unwrap(),
        original,
        "the project file holds the change"
    );
}

#[test]
fn test_csd002_ac_28_2_later_saves_of_the_same_session_leave_the_backup_alone() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    let mut first = Project::new();
    first.save_as(path.clone()).unwrap();
    let original = fs::read(&path).unwrap();

    let mut session = Project::new();
    session.open(&path).unwrap();
    add_job(&mut session, "Example Employer");
    session.save().unwrap();
    add_job(&mut session, "Example Other Employer");
    session.save().unwrap();
    add_job(&mut session, "Example Third Employer");
    session.save().unwrap();

    assert_eq!(
        fs::read(backup_of(&path)).unwrap(),
        original,
        "still the state at the start of the session"
    );
}

#[test]
fn test_csd002_ac_28_3_a_new_file_has_no_backup() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    let mut project = Project::new();
    add_job(&mut project, "Example Employer");
    project.save_as(path.clone()).unwrap();
    add_job(&mut project, "Example Other Employer");
    project.save().unwrap();
    assert!(!backup_of(&path).exists());
}

#[test]
fn test_csd002_ac_28_4_save_as_over_another_file_backs_that_file_up() {
    let dir = TempDir::new().unwrap();
    let other = dir.path().join("example-b.varde");
    let mut theirs = Project::new();
    add_job(&mut theirs, "Example Employer");
    theirs.save_as(other.clone()).unwrap();
    let their_content = fs::read(&other).unwrap();

    let mut project = Project::new();
    project.save_as(dir.path().join("example-a.varde")).unwrap();
    project.save_as(other.clone()).unwrap();

    assert_eq!(fs::read(backup_of(&other)).unwrap(), their_content);
}

#[test]
fn test_csd002_ac_28_5_a_new_project_backs_up_again_in_the_same_session() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    let mut project = Project::new();
    add_job(&mut project, "Example Employer");
    project.save_as(path.clone()).unwrap();
    let saved = fs::read(&path).unwrap();

    project.reset();
    project.save_as(path.clone()).unwrap();
    assert_eq!(fs::read(backup_of(&path)).unwrap(), saved);
}

#[test]
fn test_csd002_ac_28_6_a_failed_backup_fails_the_save_and_keeps_the_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    let mut first = Project::new();
    add_job(&mut first, "Example Employer");
    first.save_as(path.clone()).unwrap();
    let original = fs::read(&path).unwrap();
    // A directory where the backup should go: it cannot be written.
    fs::create_dir(backup_of(&path)).unwrap();

    let mut session = Project::new();
    session.open(&path).unwrap();
    add_job(&mut session, "Example Other Employer");
    let error = session.save().unwrap_err();
    assert_eq!(error.code(), "PROJECT_SAVE_FAILED");
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(session.status().unsaved_changes, "the change stays unsaved");
}

// ---- REQ-29: an older file has its own error ----

#[test]
fn test_csd002_ac_29_1_an_older_file_has_its_own_error() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    let mut project = Project::new();
    project.save_as(path.clone()).unwrap();
    for version in [1, 2, 3] {
        let mut value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        value["format_version"] = serde_json::json!(version);
        let old = dir.path().join(format!("version-{version}.varde"));
        fs::write(&old, value.to_string()).unwrap();
        let mut other = Project::new();
        add_job(&mut other, "Example Employer");
        let error = other.open(&old).unwrap_err();
        assert_eq!(error.code(), "PROJECT_FILE_OLDER", "version {version}");
        assert_eq!(
            other.income().jobs().len(),
            1,
            "the project in memory is unchanged"
        );
        assert!(!error.to_string().contains('/'));
    }
}
