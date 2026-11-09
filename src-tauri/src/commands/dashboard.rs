#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]

use tauri::State;

use super::AppState;
use crate::domain::dashboard::{dashboard, Dashboard};
use crate::error::AppError;

/// The eight tiles of the dashboard, computed from the project as it is now,
/// saved or not (CSD-004 REQ-7).
#[tauri::command]
pub fn get_dashboard(state: State<'_, AppState>) -> Result<Dashboard, AppError> {
    let today = state.today_date();
    let project = state.project()?;
    Ok(dashboard(project.data(), today))
}
