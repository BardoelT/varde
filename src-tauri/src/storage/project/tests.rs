use std::cell::Cell;
use std::fs;

use tempfile::TempDir;

use super::*;
use crate::domain::{Amounts, JobId, JobInput, YearMonth};
use crate::storage::file::{write_atomic_with, FORMAT_VERSION};

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

fn add_paycheck(project: &mut Project, job: JobId, month: &str, gross: &str) {
    project
        .income_mut()
        .add_paycheck(
            job,
            YearMonth::parse(month).unwrap(),
            Amounts::parse(gross, "", "").unwrap(),
        )
        .unwrap();
}

fn saved_project(dir: &TempDir, name: &str) -> (Project, PathBuf) {
    let mut project = Project::new();
    add_job(&mut project, "Example Employer");
    let path = dir.path().join(name);
    project.save_as(path.clone()).unwrap();
    (project, path)
}

fn job_count(project: &Project) -> usize {
    project.income().jobs().len()
}

fn file_job_count(path: &Path) -> usize {
    file::read(path).unwrap().income.jobs().len()
}

// ---- REQ-1: save ----

#[test]
fn test_csd002_ac_1_1_save_and_reload_round_trips() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    let first = add_job(&mut project, "Example Employer");
    let second = add_job(&mut project, "Other Example Employer");
    for month in ["2026-07", "2026-08", "2026-09"] {
        add_paycheck(&mut project, first, month, "3000.00");
    }
    add_paycheck(&mut project, second, "2026-08", "1000.50");
    add_paycheck(&mut project, second, "2026-09", "1000.50");
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();

    let mut fresh = Project::new();
    fresh.open(&path).unwrap();
    assert_eq!(fresh.income(), project.income());
    assert_eq!(job_count(&fresh), 2);
    let paychecks = fresh.income().paychecks(first).unwrap();
    assert_eq!(paychecks.len(), 3);
    assert_eq!(paychecks[0].amounts.gross_salary().to_string(), "3000.00");
}

#[test]
fn test_csd002_ac_1_2_write_failure_keeps_changes_unsaved() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    add_job(&mut project, "Example Employer");
    let missing = dir.path().join("no-such-dir").join("example-a.varde");

    let error = project.save_as(missing.clone()).unwrap_err();
    assert_eq!(error.code(), "PROJECT_SAVE_FAILED");
    let message = error.to_string();
    assert!(!message.contains(dir.path().to_str().unwrap()));
    assert!(!message.contains("No such file"));
    assert_eq!(
        project.status(),
        ProjectStatus {
            unsaved_changes: true,
            has_path: false
        }
    );
    assert!(!missing.exists());
}

#[test]
fn test_csd002_ac_1_3_path_is_asked_once_then_reused() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    add_job(&mut project, "Example Employer");
    let path = dir.path().join("example-a.varde");
    let asked = Cell::new(0);
    let ask = || {
        asked.set(asked.get() + 1);
        Some(path.clone())
    };

    assert_eq!(
        project.save_asking(ask, |_| true).unwrap(),
        SaveOutcome::Saved
    );
    assert_eq!(asked.get(), 1);
    assert_eq!(file_job_count(&path), 1);

    add_job(&mut project, "Second Example Employer");
    assert_eq!(
        project.save_asking(ask, |_| true).unwrap(),
        SaveOutcome::Saved
    );
    assert_eq!(asked.get(), 1, "second save must not ask again");
    assert_eq!(file_job_count(&path), 2);
}

#[test]
fn test_csd002_ac_1_4_save_as_asks_again() {
    let dir = TempDir::new().unwrap();
    let (mut project, path_a) = saved_project(&dir, "example-a.varde");
    let path_b = dir.path().join("example-b.varde");

    let outcome = project
        .save_as_asking(|| Some(path_b.clone()), |_| true)
        .unwrap();
    assert_eq!(outcome, SaveOutcome::Saved);
    assert_eq!(file_job_count(&path_b), 1);

    add_job(&mut project, "Second Example Employer");
    project.save().unwrap();
    assert_eq!(file_job_count(&path_b), 2, "later saves go to example-b");
    assert_eq!(file_job_count(&path_a), 1, "example-a is left as it was");
}

#[test]
fn test_csd002_ac_1_5_cancelling_the_dialog_saves_nothing() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    add_job(&mut project, "Example Employer");

    assert_eq!(
        project.save_asking(|| None, |_| true).unwrap(),
        SaveOutcome::Cancelled
    );
    assert_eq!(
        project.save_as_asking(|| None, |_| true).unwrap(),
        SaveOutcome::Cancelled
    );
    assert!(project.status().unsaved_changes);
    assert!(!project.status().has_path);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn saving_without_a_path_is_an_error_not_a_write() {
    let mut project = Project::new();
    assert_eq!(project.save().unwrap_err().code(), "PROJECT_NO_PATH");
}

#[test]
fn a_path_without_extension_gets_the_project_extension() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    add_job(&mut project, "Example Employer");
    project.save_as(dir.path().join("example")).unwrap();
    assert!(dir.path().join("example.varde").exists());
    assert!(!dir.path().join("example").exists());
}

// ---- REQ-2: open ----

#[test]
fn test_csd002_ac_2_1_open_valid_file() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    let job = add_job(&mut project, "Example Employer");
    add_paycheck(&mut project, job, "2026-08", "3000.00");
    add_paycheck(&mut project, job, "2026-09", "3000.00");
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();

    let mut opened = Project::new();
    let outcome = opened.open_asking(|| Some(path.clone())).unwrap();
    assert_eq!(outcome, OpenOutcome::Opened);
    assert_eq!(job_count(&opened), 1);
    assert_eq!(opened.income().entry_count(job), 2);
    assert!(opened.status().has_path);
}

#[test]
fn test_csd002_ac_2_2_invalid_file_is_rejected() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    add_job(&mut project, "Example Employer");
    let before = project.income().clone();

    let garbage = [
        ("not-json.varde", "this is not a project file"),
        (
            "no-version.varde",
            r#"{"income":{"jobs":[],"paychecks":[]}}"#,
        ),
        ("wrong-shape.varde", r#"{"format_version":1,"income":42}"#),
        (
            "orphan-paycheck.varde",
            r#"{"format_version":1,"income":{"jobs":[],"paychecks":[{
                "job_id":"00000000-0000-4000-8000-000000000000","month":"2026-09",
                "gross_salary":"3000.00","holiday_pay":"0.00","bonus":"0.00"}]}}"#,
        ),
    ];
    for (name, content) in garbage {
        let path = dir.path().join(name);
        fs::write(&path, content).unwrap();
        let error = project.open(&path).unwrap_err();
        assert_eq!(error.code(), "PROJECT_INVALID_FILE", "{name}");
        assert_eq!(
            project.income(),
            &before,
            "{name} must not change the project"
        );
        assert!(!project.status().has_path, "{name} must not set a path");
    }
}

#[test]
fn test_csd002_ac_18_1_file_from_newer_version_is_rejected_with_its_own_code() {
    let dir = TempDir::new().unwrap();
    let (mut project, path) = saved_project(&dir, "example-a.varde");
    let newer = fs::read_to_string(&path)
        .unwrap()
        .replace("\"format_version\": 1", "\"format_version\": 2");
    assert!(newer.contains("\"format_version\": 2"));
    let newer_path = dir.path().join("newer.varde");
    fs::write(&newer_path, newer).unwrap();
    let before = project.income().clone();

    let error = project.open(&newer_path).unwrap_err();
    assert_eq!(error.code(), "PROJECT_FILE_NEWER");
    assert_eq!(project.income(), &before);
}

#[test]
fn opening_a_missing_file_or_a_directory_fails_without_changes() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    add_job(&mut project, "Example Employer");
    let before = project.income().clone();

    let missing = project.open(&dir.path().join("missing.varde")).unwrap_err();
    assert_eq!(missing.code(), "PROJECT_OPEN_FAILED");
    assert!(!missing.to_string().contains(dir.path().to_str().unwrap()));
    let directory = project.open(dir.path()).unwrap_err();
    assert_eq!(directory.code(), "PROJECT_INVALID_FILE");
    assert_eq!(project.income(), &before);
}

#[test]
fn cancelling_the_open_dialog_changes_nothing() {
    let mut project = Project::new();
    add_job(&mut project, "Example Employer");
    let before = project.income().clone();
    assert_eq!(
        project.open_asking(|| None).unwrap(),
        OpenOutcome::Cancelled
    );
    assert_eq!(project.income(), &before);
}

// ---- REQ-3: unsaved indicator ----

#[test]
fn test_csd002_ac_3_1_indicator_follows_edits() {
    let dir = TempDir::new().unwrap();
    let (_, path) = saved_project(&dir, "example-a.varde");
    let mut project = Project::new();
    project.open(&path).unwrap();
    assert!(!project.status().unsaved_changes);

    add_job(&mut project, "Second Example Employer");
    assert!(project.status().unsaved_changes);

    project.save().unwrap();
    assert!(!project.status().unsaved_changes);
}

#[test]
fn indicator_clears_when_an_edit_is_undone() {
    let dir = TempDir::new().unwrap();
    let (mut project, _) = saved_project(&dir, "example-a.varde");
    let job = add_job(&mut project, "Second Example Employer");
    assert!(project.status().unsaved_changes);
    project.income_mut().delete_job(job).unwrap();
    assert!(!project.status().unsaved_changes);
}

// ---- CSD-001 REQ-29, REQ-30 and CSD-005 REQ-28, REQ-29: moving an entry ----

fn move_paycheck(project: &mut Project, job: JobId, from: &str, to: &str) -> bool {
    let month = |text: &str| YearMonth::parse(text).unwrap();
    project
        .income_mut()
        .update_paycheck(
            job,
            month(from),
            month(to),
            Amounts::parse("3000.00", "", "").unwrap(),
        )
        .is_ok()
}

#[test]
fn test_csd001_ac_29_7_a_move_is_an_unsaved_change() {
    let dir = TempDir::new().unwrap();
    let (mut project, _) = saved_project(&dir, "example-a.varde");
    let job = project.income().jobs()[0].id;
    add_paycheck(&mut project, job, "2026-05", "3000.00");
    project.save().unwrap();
    assert!(!project.status().unsaved_changes);

    assert!(move_paycheck(&mut project, job, "2026-05", "2026-04"));
    assert!(project.status().unsaved_changes);
    // Moving it back restores the saved state, so the indicator clears.
    assert!(move_paycheck(&mut project, job, "2026-04", "2026-05"));
    assert!(!project.status().unsaved_changes);
}

#[test]
fn test_csd001_ac_30_4_a_rejected_move_is_not_an_unsaved_change() {
    let dir = TempDir::new().unwrap();
    let (mut project, _) = saved_project(&dir, "example-a.varde");
    let job = project.income().jobs()[0].id;
    add_paycheck(&mut project, job, "2026-04", "2000.00");
    add_paycheck(&mut project, job, "2026-05", "3000.00");
    project.save().unwrap();

    // Already taken.
    assert!(!move_paycheck(&mut project, job, "2026-05", "2026-04"));
    assert!(!project.status().unsaved_changes);
}

#[test]
fn test_csd005_ac_28_8_a_move_is_an_unsaved_change() {
    use crate::domain::investments::{AccountInput, Balance, NetDeposits, Note, Period};
    let dir = TempDir::new().unwrap();
    let (mut project, _) = saved_project(&dir, "example-a.varde");
    let account = project
        .investments_mut()
        .create_account(&AccountInput {
            name: "Example Brokerage".to_owned(),
            period: Some("Monthly".to_owned()),
        })
        .unwrap()
        .id;
    let period = |text: &str| Period::parse(text).unwrap();
    project
        .investments_mut()
        .add_statement(
            account,
            period("2026-09"),
            Balance::parse("12500.00").unwrap(),
            NetDeposits::default(),
            Note::default(),
        )
        .unwrap();
    project.save().unwrap();
    assert!(!project.status().unsaved_changes);

    project
        .investments_mut()
        .update_statement(
            account,
            period("2026-09"),
            period("2026-08"),
            Balance::parse("12500.00").unwrap(),
            NetDeposits::default(),
            Note::default(),
        )
        .unwrap();
    assert!(project.status().unsaved_changes);
}

// REQ-4 (confirm close) and REQ-7 (confirm before replacing) are dialogs in
// the UI: see the manual checklist in CSD-002 section 7.

// ---- REQ-5: failed save leaves the file intact ----

#[test]
fn test_csd002_ac_5_1_interrupted_write_does_not_corrupt_the_file() {
    let dir = TempDir::new().unwrap();
    let (_, path) = saved_project(&dir, "example-a.varde");
    let original = fs::read(&path).unwrap();

    let result = write_atomic_with(&path, |file| {
        use std::io::Write;
        file.write_all(b"{\"format_version\": 1, \"inco")?;
        Err(std::io::Error::other("disk full"))
    });

    assert!(result.is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(file_job_count(&path), 1);
    let leftovers: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(
        leftovers.len(),
        1,
        "temporary file must be removed: {leftovers:?}"
    );
}

#[test]
fn test_csd002_ac_5_1_failed_save_reports_the_spec_error_code() {
    let dir = TempDir::new().unwrap();
    let (mut project, path) = saved_project(&dir, "example-a.varde");
    add_job(&mut project, "Second Example Employer");
    // Make the target a directory so the final rename fails.
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();

    let error = project.save().unwrap_err();
    assert_eq!(error.code(), "PROJECT_SAVE_FAILED");
    assert!(project.status().unsaved_changes);
    let leftovers = fs::read_dir(dir.path()).unwrap().count();
    assert_eq!(leftovers, 1, "temporary file must be removed");
}

// ---- REQ-6: new project ----

#[test]
fn test_csd002_ac_6_1_new_project_is_empty() {
    let dir = TempDir::new().unwrap();
    let (mut project, _) = saved_project(&dir, "example-a.varde");
    project.reset();
    assert_eq!(job_count(&project), 0);
    assert_eq!(
        project.status(),
        ProjectStatus {
            unsaved_changes: false,
            has_path: false
        }
    );
    assert_eq!(project.save().unwrap_err().code(), "PROJECT_NO_PATH");
}

#[test]
fn test_csd002_ac_6_2_saved_file_is_untouched_by_new() {
    let dir = TempDir::new().unwrap();
    let (mut project, path) = saved_project(&dir, "example-a.varde");
    project.reset();
    drop(project);
    assert_eq!(file_job_count(&path), 1);
}

// ---- REQ-8, REQ-10, REQ-11: reopen the last project ----

fn memory_in(dir: &TempDir) -> LastProject {
    LastProject::in_dir(&dir.path().join("config"))
}

/// A fresh "process": an empty project that then restores the last file.
fn restart(dir: &TempDir) -> Project {
    let mut project = Project::new();
    project.restore(memory_in(dir));
    project
}

#[test]
fn test_csd002_ac_8_1_first_launch_has_nothing_to_reopen() {
    let dir = TempDir::new().unwrap();
    let mut project = restart(&dir);
    assert_eq!(job_count(&project), 0);
    assert!(!project.status().has_path);
    assert!(!project.status().unsaved_changes);
    assert!(project.take_startup_notice().is_none());
}

#[test]
fn test_csd002_ac_8_2_the_last_project_is_reopened() {
    let dir = TempDir::new().unwrap();
    let mut project = restart(&dir);
    add_job(&mut project, "Example Employer");
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();
    drop(project);

    let mut next = restart(&dir);
    assert_eq!(job_count(&next), 1);
    assert_eq!(
        next.status(),
        ProjectStatus {
            unsaved_changes: false,
            has_path: true
        }
    );
    assert!(next.take_startup_notice().is_none());
    // Saving goes back to the reopened file without asking for a path.
    add_job(&mut next, "Second Example Employer");
    next.save().unwrap();
    assert_eq!(file_job_count(&path), 2);
}

#[test]
fn test_csd002_ac_10_1_save_remembers_the_path() {
    let dir = TempDir::new().unwrap();
    let mut project = restart(&dir);
    add_job(&mut project, "Example Employer");
    project.save_as(dir.path().join("example-a.varde")).unwrap();
    assert_eq!(
        memory_in(&dir).load(),
        Some(dir.path().join("example-a.varde"))
    );
}

#[test]
fn test_csd002_ac_10_2_open_and_save_as_move_the_remembered_path() {
    let dir = TempDir::new().unwrap();
    let (_, path_a) = saved_project(&dir, "example-a.varde");
    let mut project = restart(&dir);
    project.open(&path_a).unwrap();
    assert_eq!(memory_in(&dir).load(), Some(path_a));

    let path_c = dir.path().join("example-c.varde");
    project.save_as(path_c.clone()).unwrap();
    assert_eq!(memory_in(&dir).load(), Some(path_c.clone()));
    let next = restart(&dir);
    assert!(next.status().has_path);
    assert_eq!(job_count(&next), 1);
}

#[test]
fn test_csd002_ac_10_3_new_project_keeps_the_remembered_path() {
    let dir = TempDir::new().unwrap();
    let mut project = restart(&dir);
    add_job(&mut project, "Example Employer");
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();

    project.reset();
    add_job(&mut project, "Unsaved Example Employer");
    drop(project); // quit without saving

    assert_eq!(memory_in(&dir).load(), Some(path));
    let next = restart(&dir);
    assert_eq!(job_count(&next), 1);
}

#[test]
fn test_csd002_ac_10_4_a_failed_save_keeps_the_remembered_path() {
    let dir = TempDir::new().unwrap();
    let mut project = restart(&dir);
    add_job(&mut project, "Example Employer");
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();

    let unwritable = dir.path().join("no-such-dir").join("example-b.varde");
    assert!(project.save_as(unwritable).is_err());
    assert_eq!(memory_in(&dir).load(), Some(path));
}

fn assert_starts_empty_with_notice(project: &mut Project, code: &str, name: &str) {
    assert_eq!(job_count(project), 0, "{name}");
    assert!(!project.status().has_path, "{name}");
    let notice = project.take_startup_notice().expect(name);
    assert_eq!(notice.code(), code, "{name}");
    let text = notice.to_string();
    assert!(!text.contains("example-a"), "{name}: {text}");
    assert!(!text.contains("example-folder"), "{name}: {text}");
    assert!(project.take_startup_notice().is_none(), "{name}: only once");
}

#[test]
fn test_csd002_ac_11_1_the_file_cannot_be_opened() {
    let newer = r#"{"format_version":2}"#;
    let cases = [
        ("deleted", None, "PROJECT_OPEN_FAILED"),
        (
            "invalid",
            Some("this is not a project file"),
            "PROJECT_INVALID_FILE",
        ),
        ("newer", Some(newer), "PROJECT_FILE_NEWER"),
    ];
    for (name, content, code) in cases {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("example-a.varde");
        if let Some(content) = content {
            fs::write(&path, content).unwrap();
        }
        memory_in(&dir).store(&path).unwrap();

        let mut project = restart(&dir);
        assert_starts_empty_with_notice(&mut project, code, name);
    }
}

#[test]
fn test_csd002_ac_12_1_a_deleted_file_in_an_existing_folder_is_forgotten() {
    let dir = TempDir::new().unwrap();
    memory_in(&dir)
        .store(&dir.path().join("example-a.varde"))
        .unwrap();

    let mut project = restart(&dir);
    assert_starts_empty_with_notice(&mut project, "PROJECT_OPEN_FAILED", "deleted");
    assert_eq!(memory_in(&dir).load(), None);
}

#[test]
fn test_csd002_ac_12_2_the_error_is_not_repeated() {
    let dir = TempDir::new().unwrap();
    memory_in(&dir)
        .store(&dir.path().join("example-a.varde"))
        .unwrap();
    let mut first = restart(&dir);
    assert!(first.take_startup_notice().is_some());

    let mut second = restart(&dir);
    assert!(second.take_startup_notice().is_none());
    assert_eq!(job_count(&second), 0);
    assert!(!second.status().has_path);
}

#[test]
fn test_csd002_ac_13_1_an_unmounted_drive_does_not_make_the_app_forget_the_project() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-folder").join("example-a.varde");
    memory_in(&dir).store(&path).unwrap();

    let mut project = restart(&dir);
    assert_starts_empty_with_notice(&mut project, "PROJECT_OPEN_FAILED", "unmounted");
    assert_eq!(memory_in(&dir).load(), Some(path));
}

#[test]
fn test_csd002_ac_13_2_the_project_is_back_at_the_next_start() {
    let dir = TempDir::new().unwrap();
    let folder = dir.path().join("example-folder");
    let path = folder.join("example-a.varde");
    memory_in(&dir).store(&path).unwrap();
    let mut first = restart(&dir);
    assert!(first.take_startup_notice().is_some());

    fs::create_dir(&folder).unwrap();
    let mut project = Project::new();
    add_job(&mut project, "Example Employer");
    project.save_as(path.clone()).unwrap();
    let mut second = restart(&dir);
    assert_eq!(job_count(&second), 1);
    assert!(second.take_startup_notice().is_none());
}

#[test]
fn test_csd002_ac_13_3_a_file_that_exists_but_cannot_be_used_keeps_its_path() {
    let cases = [
        (
            "invalid",
            "this is not a project file",
            "PROJECT_INVALID_FILE",
        ),
        ("newer", r#"{"format_version":2}"#, "PROJECT_FILE_NEWER"),
    ];
    for (name, content, code) in cases {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("example-a.varde");
        fs::write(&path, content).unwrap();
        memory_in(&dir).store(&path).unwrap();

        let mut project = restart(&dir);
        assert_starts_empty_with_notice(&mut project, code, name);
        assert_eq!(memory_in(&dir).load(), Some(path), "{name}");
    }
}

#[cfg(unix)]
#[test]
fn test_csd002_ac_13_4_a_folder_that_is_not_readable_keeps_the_path() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().unwrap();
    let folder = dir.path().join("example-folder");
    fs::create_dir(&folder).unwrap();
    let path = folder.join("example-a.varde");
    let mut project = Project::new();
    add_job(&mut project, "Example Employer");
    project.save_as(path.clone()).unwrap();
    memory_in(&dir).store(&path).unwrap();
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o000)).unwrap();

    let mut restarted = restart(&dir);
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o700)).unwrap();
    assert_starts_empty_with_notice(&mut restarted, "PROJECT_OPEN_FAILED", "unreadable");
    assert_eq!(memory_in(&dir).load(), Some(path));
}

// ---- REQ-14 and REQ-15: overwrite confirmation and the extension ----

#[test]
fn test_csd002_ac_14_1_save_as_onto_an_existing_file_asks_first() {
    let dir = TempDir::new().unwrap();
    let (mut project, _) = saved_project(&dir, "example-a.varde");
    let path_b = dir.path().join("example-b.varde");
    fs::write(&path_b, "existing").unwrap();
    let asked = Cell::new(0);

    project
        .save_as_asking(
            || Some(path_b.clone()),
            |path| {
                asked.set(asked.get() + 1);
                assert_eq!(path, path_b);
                true
            },
        )
        .unwrap();
    assert_eq!(asked.get(), 1);
    assert_eq!(file_job_count(&path_b), 1);
}

#[test]
fn test_csd002_ac_14_2_declining_keeps_the_existing_file() {
    let dir = TempDir::new().unwrap();
    let path_b = dir.path().join("example-b.varde");
    let mut other = Project::new();
    add_job(&mut other, "Example Employer");
    other.save_as(path_b.clone()).unwrap();
    let mut project = Project::new();
    add_job(&mut project, "Example Employer");
    add_job(&mut project, "Second Example Employer");

    let outcome = project
        .save_as_asking(|| Some(path_b.clone()), |_| false)
        .unwrap();
    assert_eq!(outcome, SaveOutcome::Cancelled);
    assert_eq!(file_job_count(&path_b), 1);
    assert!(project.status().unsaved_changes);
    assert!(!project.status().has_path);
}

#[test]
fn test_csd002_ac_14_3_saving_to_the_own_file_needs_no_confirmation() {
    let dir = TempDir::new().unwrap();
    let (mut project, _) = saved_project(&dir, "example-a.varde");
    add_job(&mut project, "Second Example Employer");
    let outcome = project
        .save_asking(|| panic!("no dialog"), |_| panic!("no confirmation"))
        .unwrap();
    assert_eq!(outcome, SaveOutcome::Saved);
}

#[test]
fn test_csd002_ac_15_1_the_extension_is_appended() {
    let cases = [
        ("example-a", "example-a.varde"),
        ("example-a.varde", "example-a.varde"),
        ("example-a.txt", "example-a.txt.varde"),
    ];
    for (typed, written) in cases {
        let dir = TempDir::new().unwrap();
        let mut project = Project::new();
        add_job(&mut project, "Example Employer");
        project.save_as(dir.path().join(typed)).unwrap();
        assert!(dir.path().join(written).exists(), "{typed}");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1, "{typed}");
    }
}

#[test]
fn test_csd002_ac_15_2_a_name_that_exists_only_after_appending_asks_first() {
    let dir = TempDir::new().unwrap();
    let existing = dir.path().join("example-a.varde");
    fs::write(&existing, "existing").unwrap();
    let mut project = Project::new();
    add_job(&mut project, "Example Employer");
    let asked = Cell::new(0);

    let outcome = project
        .save_as_asking(
            || Some(dir.path().join("example-a")),
            |path| {
                asked.set(asked.get() + 1);
                assert_eq!(path, existing);
                false
            },
        )
        .unwrap();
    assert_eq!(asked.get(), 1);
    assert_eq!(outcome, SaveOutcome::Cancelled);
    assert_eq!(fs::read_to_string(&existing).unwrap(), "existing");
}

#[test]
fn a_project_without_memory_still_saves_and_opens() {
    let dir = TempDir::new().unwrap();
    let (project, path) = saved_project(&dir, "example-a.varde");
    assert_eq!(job_count(&project), 1);
    assert_eq!(file_job_count(&path), 1);
    assert_eq!(memory_in(&dir).load(), None);
}

// ---- CSD-003: project settings ----

use crate::domain::Currency;

#[test]
fn test_csd003_ac_7_1_currency_survives_save_and_open() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    add_job(&mut project, "Example Employer");
    project.settings_mut().set_currency("GBP").unwrap();
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();

    let mut fresh = Project::new();
    fresh.open(&path).unwrap();
    assert_eq!(fresh.settings().currency, Currency::Gbp);
    assert!(!fresh.status().unsaved_changes);
}

#[test]
fn test_csd003_ac_7_3_the_settings_are_a_section_of_the_file() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    project.settings_mut().set_currency("USD").unwrap();
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(value["settings"], serde_json::json!({ "currency": "USD" }));
    assert!(value["income"].get("currency").is_none());
}

#[test]
fn test_csd003_ac_4_2_a_currency_change_is_an_unsaved_change() {
    let dir = TempDir::new().unwrap();
    let (mut project, _) = saved_project(&dir, "example-a.varde");
    assert!(!project.status().unsaved_changes);
    project.settings_mut().set_currency("GBP").unwrap();
    assert!(project.status().unsaved_changes);
    project.save().unwrap();
    assert!(!project.status().unsaved_changes);
}

#[test]
fn test_csd003_ac_4_3_choosing_the_current_currency_changes_nothing() {
    let dir = TempDir::new().unwrap();
    let (mut project, _) = saved_project(&dir, "example-a.varde");
    project.settings_mut().set_currency("EUR").unwrap();
    assert!(!project.status().unsaved_changes);
    // Changing and changing back is no change either.
    project.settings_mut().set_currency("USD").unwrap();
    assert!(project.status().unsaved_changes);
    project.settings_mut().set_currency("EUR").unwrap();
    assert!(!project.status().unsaved_changes);
}

#[test]
fn test_csd003_ac_3_1_a_new_project_starts_in_eur() {
    let dir = TempDir::new().unwrap();
    let (mut project, _) = saved_project(&dir, "example-a.varde");
    project.settings_mut().set_currency("USD").unwrap();
    project.reset();
    assert_eq!(project.settings().currency, Currency::Eur);
}

// ---- annual statements in the project file (CSD-001 0.9) ----

fn add_yearly_job(project: &mut Project, employer: &str) -> JobId {
    project
        .income_mut()
        .create_job(&JobInput {
            employer: employer.to_owned(),
            start_date: Some("01/03/2024".to_owned()),
            period: Some("Yearly".to_owned()),
            ..JobInput::default()
        })
        .unwrap()
        .id
}

#[test]
fn test_csd002_ac_1_1_annual_statements_round_trip() {
    use crate::domain::Year;
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    let monthly = add_job(&mut project, "Example Employer");
    add_paycheck(&mut project, monthly, "2026-09", "3000.00");
    let yearly = add_yearly_job(&mut project, "Older Example Employer");
    for (y, gross) in [("2024", "40000.00"), ("2025", "42000.00")] {
        project
            .income_mut()
            .add_statement(
                yearly,
                Year::parse(y).unwrap(),
                Amounts::parse(gross, "", "3000").unwrap(),
            )
            .unwrap();
    }
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();

    let mut fresh = Project::new();
    fresh.open(&path).unwrap();
    assert_eq!(fresh.income(), project.income());
    assert_eq!(fresh.income().entry_count(yearly), 2);
    assert_eq!(fresh.income().entry_count(monthly), 1);
    let totals = fresh.income().yearly_totals();
    assert_eq!(totals[0].year, 2026);
    assert_eq!(totals[1].total_gross.to_string(), "45000.00");
    assert!(!fresh.status().unsaved_changes);
}

#[test]
fn test_csd001_the_file_keeps_pay_periods_and_statements_in_their_own_list() {
    use crate::domain::Year;
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    add_job(&mut project, "Example Employer");
    let yearly = add_yearly_job(&mut project, "Older Example Employer");
    project
        .income_mut()
        .add_statement(
            yearly,
            Year::parse("2025").unwrap(),
            Amounts::parse("40000.00", "", "").unwrap(),
        )
        .unwrap();
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();

    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(value["format_version"], 1);
    let periods: Vec<&str> = value["income"]["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|j| j["period"].as_str().unwrap())
        .collect();
    assert_eq!(periods.len(), 2);
    assert!(periods.contains(&"Monthly") && periods.contains(&"Yearly"));
    let statement = &value["income"]["statements"][0];
    assert_eq!(statement["year"], "2025");
    assert_eq!(statement["gross_salary"], "40000.00");
    assert_eq!(value["income"]["paychecks"].as_array().unwrap().len(), 0);
}

#[test]
fn a_file_with_an_entry_of_the_wrong_kind_is_invalid() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    // An annual statement for a job that has no pay period, so it is monthly.
    fs::write(
        &path,
        r#"{"format_version":1,"income":{
            "jobs":[{"id":"671166f4-5827-4794-bcdc-54d7824fb650","employer":"Example Employer",
                "title":null,"start_date":"2024-03-01","end_date":null}],
            "paychecks":[],
            "statements":[{"job_id":"671166f4-5827-4794-bcdc-54d7824fb650","year":"2025",
                "gross_salary":"40000.00","holiday_pay":"0.00","bonus":"0.00"}]}}"#,
    )
    .unwrap();
    let mut project = Project::new();
    assert_eq!(
        project.open(&path).unwrap_err().code(),
        "PROJECT_INVALID_FILE"
    );
}

// ---- constraints ----

#[test]
fn test_csd002_con_3_file_has_version_and_decimal_strings() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    let job = add_job(&mut project, "Example Employer");
    add_paycheck(&mut project, job, "2026-09", "3000.00");
    let path = dir.path().join("example-a.varde");
    project.save_as(path.clone()).unwrap();

    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(value["format_version"], u64::from(FORMAT_VERSION));
    let paycheck = &value["income"]["paychecks"][0];
    assert_eq!(paycheck["gross_salary"], "3000.00");
    assert_eq!(paycheck["holiday_pay"], "0.00");
    assert_eq!(paycheck["month"], "2026-09");
    assert_eq!(value["income"]["jobs"][0]["start_date"], "2024-03-01");
    assert_eq!(value["income"]["jobs"][0].get("currency"), None);
    assert_eq!(value["settings"]["currency"], "EUR");
}

#[test]
fn test_csd002_con_1_project_file_is_private_to_the_owner() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().unwrap();
    let (_, path) = saved_project(&dir, "example-a.varde");
    let mode = fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(mode & 0o077, 0, "mode {mode:o}");
}

// ---- REQ-17: a misspelled section must not wipe data ----

#[test]
fn test_csd002_ac_17_1_a_file_that_does_not_match_the_schema_is_rejected() {
    type Change = fn(&mut serde_json::Value);
    let dir = TempDir::new().unwrap();
    let (mut project, path) = saved_project(&dir, "example-a.varde");
    let good: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let cases: [(&str, Change); 4] = [
        ("without settings", |v| {
            v.as_object_mut().unwrap().remove("settings");
        }),
        ("without houses", |v| {
            v.as_object_mut().unwrap().remove("houses");
        }),
        ("extra top-level field", |v| {
            v["notes"] = serde_json::json!("x");
        }),
        ("misspelled section", |v| {
            let accounts = v.as_object_mut().unwrap().remove("bank_accounts").unwrap();
            v["bank_acounts"] = accounts;
        }),
    ];
    for (name, change) in cases {
        let mut value = good.clone();
        change(&mut value);
        let bad = dir.path().join("bad.varde");
        fs::write(&bad, value.to_string()).unwrap();
        let error = project.open(&bad).unwrap_err();
        assert_eq!(error.code(), "PROJECT_INVALID_FILE", "{name}");
        assert_eq!(job_count(&project), 1, "{name}");
    }
    for section in [
        "income",
        "investments",
        "bank_accounts",
        "debts",
        "expenses",
        "houses",
        "settings",
    ] {
        let mut value = good.clone();
        value[section]["notes"] = serde_json::json!("x");
        let bad = dir.path().join("bad.varde");
        fs::write(&bad, value.to_string()).unwrap();
        let error = project.open(&bad).unwrap_err();
        assert_eq!(
            error.code(),
            "PROJECT_INVALID_FILE",
            "extra field in {section}"
        );
        assert_eq!(job_count(&project), 1, "{section}");
    }
}
