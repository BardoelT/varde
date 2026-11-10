#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Runtime, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

use super::AppState;
use crate::error::AppError;
use crate::menu;
use crate::storage::{OpenOutcome, Project, ProjectStatus, SaveOutcome, StepLabel};

/// Result of a command that may have been cancelled in a native dialog.
#[derive(Serialize)]
pub struct DialogResult {
    /// `false` when the user cancelled the dialog and nothing happened.
    done: bool,
    status: ProjectStatus,
}

/// What an undo or a redo applied, and the project as it is afterwards
/// (CSD-027 section 5).
#[derive(Serialize)]
pub struct HistoryResult {
    step: StepLabel,
    status: ProjectStatus,
}

const FILTER_NAME: &str = "Varde project";
const FILTER_EXTENSIONS: &[&str] = &["varde"];

/// Shows the native "save" dialog. Returns `None` when cancelled.
fn pick_save_path<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.dialog()
        .file()
        .add_filter(FILTER_NAME, FILTER_EXTENSIONS)
        .set_file_name("project.varde")
        .blocking_save_file()
        .and_then(|path| path.into_path().ok())
}

/// Shows the native "open" dialog. Returns `None` when cancelled.
fn pick_open_path<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.dialog()
        .file()
        .add_filter(FILTER_NAME, FILTER_EXTENSIONS)
        .blocking_pick_file()
        .and_then(|path| path.into_path().ok())
}

/// Asks before another file is replaced (CSD-002 REQ-14).
///
/// The native save panel has already asked when the user picked an existing
/// file, so the app asks only when appending `.varde` made the name point at a
/// file the panel did not know about (REQ-15, AC-15.2).
fn confirm_overwrite<R: Runtime>(app: &AppHandle<R>, picked: &Path, target: &Path) -> bool {
    if picked == target {
        return true;
    }
    app.dialog()
        .message("A file with this name already exists. Do you want to replace it?")
        .title("Replace file?")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Replace".to_owned(),
            "Cancel".to_owned(),
        ))
        .blocking_show()
}

/// Whether a save to `picked` may go ahead: it may unless it would replace
/// another file and the user declines. Asked before the project lock is taken,
/// so a dialog never holds it.
fn may_save(
    state: &AppState,
    picked: Option<&PathBuf>,
    confirm: impl FnOnce(&Path, &Path) -> bool,
) -> Result<bool, AppError> {
    let Some(picked) = picked else {
        return Ok(true);
    };
    let target = state.project()?.save_target(picked.clone());
    Ok(!target.replaces_other_file || confirm(picked, &target.path))
}

#[cfg(test)]
fn status_of(state: &AppState) -> Result<ProjectStatus, AppError> {
    Ok(state.project()?.status())
}

/// Saves to the current path, or after a native dialog when there is none yet.
///
/// The project lock is released while the dialog is open, so closing the
/// window never waits on it.
fn save_flow(
    state: &AppState,
    ask_path: impl FnOnce() -> Option<PathBuf>,
    confirm: impl FnOnce(&Path, &Path) -> bool,
) -> Result<DialogResult, AppError> {
    let has_path = state.project()?.status().has_path;
    let path = if has_path { None } else { ask_path() };
    let confirmed = may_save(state, path.as_ref(), confirm)?;
    let mut project = state.project()?;
    let outcome = if has_path {
        project.save()?;
        SaveOutcome::Saved
    } else {
        project.save_asking(|| path, |_| confirmed)?
    };
    Ok(DialogResult {
        done: outcome == SaveOutcome::Saved,
        status: project.status(),
    })
}

/// Save As: always asks for a path.
fn save_as_flow(
    state: &AppState,
    ask_path: impl FnOnce() -> Option<PathBuf>,
    confirm: impl FnOnce(&Path, &Path) -> bool,
) -> Result<DialogResult, AppError> {
    let path = ask_path();
    let confirmed = may_save(state, path.as_ref(), confirm)?;
    let mut project = state.project()?;
    let outcome = project.save_as_asking(|| path, |_| confirmed)?;
    Ok(DialogResult {
        done: outcome == SaveOutcome::Saved,
        status: project.status(),
    })
}

fn open_flow(
    state: &AppState,
    ask_path: impl FnOnce() -> Option<PathBuf>,
) -> Result<DialogResult, AppError> {
    let path = ask_path();
    let mut project = state.project()?;
    let outcome = project.open_asking(|| path)?;
    Ok(DialogResult {
        done: outcome == OpenOutcome::Opened,
        status: project.status(),
    })
}

/// Makes the Edit menu follow `status` and hands the status back (CSD-027
/// REQ-10). Every command that can change the history returns its status through
/// here, so the menu never lags behind.
fn publish<R: Runtime>(app: &AppHandle<R>, state: &AppState, project: &Project) -> ProjectStatus {
    let status = project.status();
    menu::sync(app, status);
    state.sync_title(project);
    status
}

#[tauri::command]
pub fn project_status<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<ProjectStatus, AppError> {
    Ok(publish(&app, &state, &*state.project()?))
}

/// Takes back the most recent step (CSD-027 REQ-1).
#[tauri::command]
pub fn undo<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<HistoryResult, AppError> {
    let mut project = state.project()?;
    let step = project.undo()?;
    Ok(HistoryResult {
        step,
        status: publish(&app, &state, &project),
    })
}

/// Applies again the step that was last undone (CSD-027 REQ-2).
#[tauri::command]
pub fn redo<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<HistoryResult, AppError> {
    let mut project = state.project()?;
    let step = project.redo()?;
    Ok(HistoryResult {
        step,
        status: publish(&app, &state, &project),
    })
}

/// The reason the last project was not reopened at start, once (CSD-002 REQ-11).
#[tauri::command]
pub fn take_startup_notice(state: State<'_, AppState>) -> Result<Option<AppError>, AppError> {
    Ok(state.project()?.take_startup_notice().map(AppError::from))
}

#[tauri::command]
pub fn new_project<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<ProjectStatus, AppError> {
    let mut project = state.project()?;
    project.reset();
    Ok(publish(&app, &state, &project))
}

#[tauri::command(async)]
pub fn open_project<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<DialogResult, AppError> {
    let mut result = open_flow(&state, || pick_open_path(&app))?;
    result.status = publish(&app, &state, &*state.project()?);
    Ok(result)
}

#[tauri::command(async)]
pub fn save_project<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<DialogResult, AppError> {
    let mut result = save_flow(
        &state,
        || pick_save_path(&app),
        |picked, target| confirm_overwrite(&app, picked, target),
    )?;
    result.status = publish(&app, &state, &*state.project()?);
    Ok(result)
}

#[tauri::command(async)]
pub fn save_project_as<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<DialogResult, AppError> {
    let mut result = save_as_flow(
        &state,
        || pick_save_path(&app),
        |picked, target| confirm_overwrite(&app, picked, target),
    )?;
    result.status = publish(&app, &state, &*state.project()?);
    Ok(result)
}

/// Quits without asking: the frontend has already confirmed (CSD-002 REQ-4).
#[tauri::command]
pub fn quit_app<R: Runtime>(app: AppHandle<R>) {
    app.exit(0);
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use time::UtcOffset;

    use super::*;
    use crate::domain::JobInput;

    fn state_with_job() -> AppState {
        let state = AppState::new(UtcOffset::UTC);
        state
            .project()
            .unwrap()
            .income_mut()
            .create_job(&JobInput {
                employer: "Example Employer".into(),
                start_date: Some("01/03/2024".into()),
                ..JobInput::default()
            })
            .unwrap();
        state
    }

    #[test]
    fn test_csd002_ac_1_3_flow_asks_once_and_reuses_the_path() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("example-a.varde");
        let state = state_with_job();
        let mut asked = 0;

        let first = save_flow(
            &state,
            || {
                asked += 1;
                Some(path.clone())
            },
            |_, _| true,
        )
        .unwrap();
        assert!(first.done);
        assert!(!first.status.unsaved_changes);
        assert!(path.exists());

        let second = save_flow(
            &state,
            || {
                asked += 1;
                None
            },
            |_, _| true,
        )
        .unwrap();
        assert!(second.done);
        assert_eq!(asked, 1);
    }

    #[test]
    fn test_csd002_ac_1_5_flow_reports_a_cancelled_dialog() {
        let dir = TempDir::new().unwrap();
        let state = state_with_job();

        let saved = save_flow(&state, || None, |_, _| true).unwrap();
        assert!(!saved.done);
        assert!(saved.status.unsaved_changes);
        let saved_as = save_as_flow(&state, || None, |_, _| true).unwrap();
        assert!(!saved_as.done);
        let opened = open_flow(&state, || None).unwrap();
        assert!(!opened.done);
        assert!(opened.status.unsaved_changes);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn test_csd002_ac_1_4_flow_save_as_moves_the_project() {
        let dir = TempDir::new().unwrap();
        let state = state_with_job();
        let path_a = dir.path().join("example-a.varde");
        let path_b = dir.path().join("example-b.varde");
        save_flow(&state, || Some(path_a.clone()), |_, _| true).unwrap();
        save_as_flow(&state, || Some(path_b.clone()), |_, _| true).unwrap();
        assert!(path_b.exists());
        let status = status_of(&state).unwrap();
        assert!(status.has_path && !status.unsaved_changes);
    }

    #[test]
    fn test_csd002_ac_14_2_flow_declining_keeps_the_existing_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("example-b.varde");
        std::fs::write(&path, "existing").unwrap();
        let state = state_with_job();

        let result = save_as_flow(&state, || Some(path.clone()), |_, _| false).unwrap();
        assert!(!result.done);
        assert!(result.status.unsaved_changes);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "existing");

        let result = save_as_flow(&state, || Some(path.clone()), |_, _| true).unwrap();
        assert!(result.done);
        assert_ne!(std::fs::read_to_string(&path).unwrap(), "existing");
    }

    #[test]
    fn open_flow_replaces_the_project() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("example-a.varde");
        let source = state_with_job();
        save_flow(&source, || Some(path.clone()), |_, _| true).unwrap();

        let target = AppState::new(UtcOffset::UTC);
        let result = open_flow(&target, || Some(path.clone())).unwrap();
        assert!(result.done);
        assert_eq!(target.project().unwrap().income().jobs().len(), 1);
    }
}
