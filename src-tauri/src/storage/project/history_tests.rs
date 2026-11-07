//! Acceptance tests of CSD-027 (undo and redo) for the history of a project.
//! Synthetic data only; files live in a temporary directory.

use tempfile::TempDir;

use super::history::MAX_STEPS;
use super::*;
use crate::domain::expenses::ExpenseInput;
use crate::domain::money::Currency;
use crate::domain::{Amounts, JobInput, YearMonth};

fn expense(name: &str, amount: &str) -> ExpenseInput {
    ExpenseInput {
        name: name.to_owned(),
        amount: amount.to_owned(),
        date: "10/09/2026".to_owned(),
        category: "Hobby".to_owned(),
        note: None,
    }
}

/// Runs `change` as one step labelled `label`, as the commands do through `AppState::edit`.
fn step(project: &mut Project, label: StepLabel, change: impl FnOnce(&mut Project)) {
    let before = project.snapshot();
    change(project);
    project.record_step(label, before);
}

fn add_expense(project: &mut Project, name: &str, amount: &str) {
    step(project, StepLabel::add(Record::Expense), |project| {
        project
            .expenses_mut()
            .add_expense(&expense(name, amount))
            .unwrap();
    });
}

fn names(project: &Project) -> Vec<String> {
    let mut names: Vec<String> = project
        .expenses()
        .list()
        .expenses
        .into_iter()
        .map(|expense| expense.name)
        .collect();
    names.sort();
    names
}

fn undo_label(project: &Project) -> Option<String> {
    project.status().undo_label.map(|label| label.to_string())
}

fn redo_label(project: &Project) -> Option<String> {
    project.status().redo_label.map(|label| label.to_string())
}

// ---- REQ-1: undo ----

#[test]
fn test_csd027_ac_1_1_undo_a_delete() {
    let mut project = Project::new();
    add_expense(&mut project, "Example Bike", "900.00");
    let id = project.expenses().list().expenses[0].id;
    step(
        &mut project,
        StepLabel::delete(Record::Expense),
        |project| {
            project.expenses_mut().delete_expense(id).unwrap();
        },
    );
    assert_eq!(names(&project), Vec::<String>::new());

    let undone = project.undo().unwrap();

    assert_eq!(undone.to_string(), "Delete Expense");
    assert_eq!(names(&project), ["Example Bike"]);
}

#[test]
fn test_csd027_ac_1_2_undo_an_edit() {
    let mut project = Project::new();
    add_expense(&mut project, "Example Bike", "900.00");
    let id = project.expenses().list().expenses[0].id;
    step(&mut project, StepLabel::edit(Record::Expense), |project| {
        project
            .expenses_mut()
            .update_expense(id, &expense("Example Bike", "950.00"))
            .unwrap();
    });

    project.undo().unwrap();

    assert_eq!(project.expenses().list().total.to_string(), "900.00");
}

#[test]
fn test_csd027_ac_1_3_undo_steps_back_one_step_at_a_time() {
    let mut project = Project::new();
    add_expense(&mut project, "Example Tea", "20.00");
    add_expense(&mut project, "Example Coffee", "30.00");

    project.undo().unwrap();

    assert_eq!(names(&project), ["Example Tea"]);
}

#[test]
fn test_csd027_ac_1_4_nothing_to_undo() {
    let mut project = Project::new();
    assert_eq!(undo_label(&project), None);
    assert_eq!(project.undo().unwrap_err().code(), "NOTHING_TO_UNDO");
}

// ---- REQ-2: redo ----

#[test]
fn test_csd027_ac_2_1_redo_an_undone_delete() {
    let mut project = Project::new();
    add_expense(&mut project, "Example Bike", "900.00");
    let id = project.expenses().list().expenses[0].id;
    step(
        &mut project,
        StepLabel::delete(Record::Expense),
        |project| {
            project.expenses_mut().delete_expense(id).unwrap();
        },
    );
    project.undo().unwrap();
    assert_eq!(redo_label(&project).as_deref(), Some("Delete Expense"));

    let redone = project.redo().unwrap();

    assert_eq!(redone.to_string(), "Delete Expense");
    assert_eq!(names(&project), Vec::<String>::new());
    assert_eq!(undo_label(&project).as_deref(), Some("Delete Expense"));
}

#[test]
fn test_csd027_ac_2_2_nothing_to_redo() {
    let mut project = Project::new();
    assert_eq!(redo_label(&project), None);
    assert_eq!(project.redo().unwrap_err().code(), "NOTHING_TO_REDO");
}

#[test]
fn test_csd027_ac_2_1_redo_steps_forward_one_step_at_a_time() {
    let mut project = Project::new();
    add_expense(&mut project, "Example Tea", "20.00");
    add_expense(&mut project, "Example Coffee", "30.00");
    project.undo().unwrap();
    project.undo().unwrap();

    project.redo().unwrap();

    assert_eq!(names(&project), ["Example Tea"]);
}

// ---- REQ-3: one step per action ----

#[test]
fn test_csd027_ac_3_1_a_cascading_delete_is_one_step() {
    let mut project = Project::new();
    let mut job_id = None;
    step(&mut project, StepLabel::add(Record::Job), |project| {
        job_id = Some(
            project
                .income_mut()
                .create_job(&JobInput {
                    employer: "Example Employer".to_owned(),
                    start_date: Some("01/03/2024".to_owned()),
                    ..JobInput::default()
                })
                .unwrap()
                .id,
        );
    });
    let job_id = job_id.unwrap();
    for month in ["2024-03", "2024-04"] {
        step(&mut project, StepLabel::add(Record::Paycheck), |project| {
            project
                .income_mut()
                .add_paycheck(
                    job_id,
                    YearMonth::parse(month).unwrap(),
                    Amounts::parse("3000.00", "", "").unwrap(),
                )
                .unwrap();
        });
    }
    step(&mut project, StepLabel::delete(Record::Job), |project| {
        project.income_mut().delete_job(job_id).unwrap();
    });
    assert_eq!(project.income().jobs().len(), 0);

    project.undo().unwrap();

    assert_eq!(project.income().jobs().len(), 1);
    assert_eq!(project.income().entry_count(job_id), 2);
}

// ---- REQ-4: what is not a step ----

#[test]
fn test_csd027_ac_4_1_changing_the_currency_is_not_a_step() {
    let mut project = Project::new();
    project.settings_mut().set_currency("USD").unwrap();
    assert_eq!(undo_label(&project), None);
}

#[test]
fn test_csd027_ac_4_1_undo_leaves_the_currency_alone() {
    let mut project = Project::new();
    add_expense(&mut project, "Example Bike", "900.00");
    project.settings_mut().set_currency("USD").unwrap();

    project.undo().unwrap();
    assert_eq!(project.settings().currency, Currency::Usd);
    assert_eq!(names(&project), Vec::<String>::new());

    project.redo().unwrap();
    assert_eq!(project.settings().currency, Currency::Usd);
    assert_eq!(names(&project), ["Example Bike"]);
}

#[test]
fn test_csd027_ac_4_2_saving_is_not_a_step() {
    let dir = TempDir::new().unwrap();
    let mut project = Project::new();
    add_expense(&mut project, "Example Bike", "900.00");
    project.save_as(dir.path().join("example-a.varde")).unwrap();

    assert_eq!(undo_label(&project).as_deref(), Some("Add Expense"));
    project.undo().unwrap();
    assert_eq!(names(&project), Vec::<String>::new());
}

// ---- REQ-5: a new step clears redo ----

#[test]
fn test_csd027_ac_5_1_redo_is_gone_after_a_new_edit() {
    let mut project = Project::new();
    add_expense(&mut project, "Example Tea", "20.00");
    project.undo().unwrap();
    assert!(redo_label(&project).is_some());

    add_expense(&mut project, "Example Cake", "45.00");

    assert_eq!(redo_label(&project), None);
    assert_eq!(project.redo().unwrap_err().code(), "NOTHING_TO_REDO");
}

// ---- REQ-6: limit ----

#[test]
fn test_csd027_ac_6_1_the_101st_step_drops_the_oldest() {
    let mut project = Project::new();
    for number in 0..=MAX_STEPS {
        add_expense(&mut project, &format!("Example {number:03}"), "1.00");
    }
    assert_eq!(names(&project).len(), MAX_STEPS + 1);

    for _ in 0..MAX_STEPS {
        project.undo().unwrap();
    }

    assert_eq!(names(&project), ["Example 000"]);
    assert_eq!(undo_label(&project), None);
    assert_eq!(project.undo().unwrap_err().code(), "NOTHING_TO_UNDO");
}

// ---- REQ-7: a failed change records nothing ----

#[test]
fn test_csd027_ac_7_1_a_rejected_change_leaves_the_history_alone() {
    let mut project = Project::new();
    add_expense(&mut project, "Example Tea", "20.00");
    project.undo().unwrap();

    step(&mut project, StepLabel::add(Record::Expense), |project| {
        let rejected = ExpenseInput {
            date: "not a date".to_owned(),
            ..expense("Example Cake", "45.00")
        };
        assert!(project.expenses_mut().add_expense(&rejected).is_err());
    });

    assert_eq!(redo_label(&project).as_deref(), Some("Add Expense"));
    assert_eq!(undo_label(&project), None);
}

// ---- REQ-8: history belongs to the open project ----

#[test]
fn test_csd027_ac_8_1_opening_a_project_clears_the_history() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    let mut other = Project::new();
    add_expense(&mut other, "Example Bike", "900.00");
    other.save_as(path.clone()).unwrap();

    let mut project = Project::new();
    add_expense(&mut project, "Example Tea", "20.00");
    add_expense(&mut project, "Example Cake", "45.00");
    project.undo().unwrap();

    project.open(&path).unwrap();

    assert_eq!(undo_label(&project), None);
    assert_eq!(redo_label(&project), None);
}

#[test]
fn test_csd027_ac_8_1_a_new_project_has_no_history() {
    let mut project = Project::new();
    add_expense(&mut project, "Example Tea", "20.00");

    project.reset();

    assert_eq!(undo_label(&project), None);
    assert_eq!(redo_label(&project), None);
}

#[test]
fn test_csd027_ac_8_2_a_failed_open_keeps_the_history() {
    let dir = TempDir::new().unwrap();
    let bad = dir.path().join("bad.varde");
    std::fs::write(&bad, "not json").unwrap();
    let mut project = Project::new();
    add_expense(&mut project, "Example Tea", "20.00");

    assert!(project.open(&bad).is_err());

    assert_eq!(names(&project), ["Example Tea"]);
    assert_eq!(undo_label(&project).as_deref(), Some("Add Expense"));
}

// ---- REQ-9: undo and the saved state ----

#[test]
fn test_csd027_ac_9_1_undo_back_to_the_saved_state_clears_the_indicator() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    let mut project = Project::new();
    project.save_as(path.clone()).unwrap();
    let on_disk = std::fs::read(&path).unwrap();

    add_expense(&mut project, "Example Tea", "20.00");
    assert!(project.status().unsaved_changes);
    project.undo().unwrap();

    assert!(!project.status().unsaved_changes);
    assert_eq!(std::fs::read(&path).unwrap(), on_disk);

    project.redo().unwrap();
    assert!(project.status().unsaved_changes);
}

#[test]
fn test_csd027_ac_9_2_undo_past_a_save() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("example-a.varde");
    let mut project = Project::new();
    add_expense(&mut project, "Example Tea", "20.00");
    project.save_as(path.clone()).unwrap();
    assert!(!project.status().unsaved_changes);
    let on_disk = std::fs::read(&path).unwrap();

    project.undo().unwrap();

    assert_eq!(names(&project), Vec::<String>::new());
    assert!(project.status().unsaved_changes);
    assert_eq!(std::fs::read(&path).unwrap(), on_disk);
}
