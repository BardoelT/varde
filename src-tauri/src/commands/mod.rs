//! `#[tauri::command]` handlers: validate input, call the domain, map errors.

pub mod bank_accounts;
pub mod dashboard;
pub mod debts;
pub mod expenses;
pub mod houses;
pub mod income;
pub mod investments;
pub mod net_worth;
pub mod preferences;
pub mod project;
pub mod ratios;
pub mod settings;

use std::ops::{Deref, DerefMut};
use std::sync::{Mutex, MutexGuard};

use time::{Date, OffsetDateTime, UtcOffset};

use crate::domain::{NumberFormat, ProjectData, YearMonth};
use crate::error::AppError;
use crate::storage::{Project, StepLabel};

/// Text that no amount parser accepts. A typed amount that does not fit the
/// number format becomes this, so the area reports its own invalid-amount
/// error and nothing is guessed (CSD-029 REQ-7).
const NOT_AN_AMOUNT: &str = "?";

/// The canonical text of an amount typed in `format` (CSD-029 REQ-6). The area
/// that receives it still validates and rounds it (CSD-000 CON-8).
#[must_use]
pub(crate) fn typed_amount(format: NumberFormat, text: &str) -> String {
    format
        .typed_to_canonical(text)
        .unwrap_or_else(|| NOT_AN_AMOUNT.to_owned())
}

/// [`typed_amount`] for an amount that may be left out.
#[must_use]
pub(crate) fn typed_amount_opt(format: NumberFormat, text: Option<String>) -> Option<String> {
    text.map(|text| typed_amount(format, &text))
}

/// The project locked for one change. When it goes out of scope after a change
/// that altered the data, the change becomes one undo step (CSD-027 REQ-3).
/// A rejected change alters nothing and records nothing (REQ-7).
pub struct Edit<'a> {
    project: MutexGuard<'a, Project>,
    label: StepLabel,
    before: Option<ProjectData>,
}

impl Deref for Edit<'_> {
    type Target = Project;

    fn deref(&self) -> &Project {
        &self.project
    }
}

impl DerefMut for Edit<'_> {
    fn deref_mut(&mut self) -> &mut Project {
        &mut self.project
    }
}

impl Drop for Edit<'_> {
    fn drop(&mut self) {
        if let Some(before) = self.before.take() {
            self.project.record_step(self.label, before);
        }
    }
}

/// Shared app state: the open project and the local UTC offset, read once at
/// startup while the process is still single-threaded.
pub struct AppState {
    project: Mutex<Project>,
    utc_offset: UtcOffset,
    /// A fixed "today" so tests that compare whole outputs do not depend on
    /// the clock.
    #[cfg(test)]
    fixed_today: Mutex<Option<Date>>,
}

impl AppState {
    /// A fresh empty project (CSD-002 REQ-8).
    #[must_use]
    pub fn new(utc_offset: UtcOffset) -> Self {
        Self {
            project: Mutex::new(Project::new()),
            utc_offset,
            #[cfg(test)]
            fixed_today: Mutex::new(None),
        }
    }

    /// Locks the project. Keep the guard short-lived: never hold it across a
    /// native dialog, or closing the window could deadlock.
    ///
    /// # Errors
    /// Returns `Internal` if a previous holder panicked.
    pub fn project(&self) -> Result<MutexGuard<'_, Project>, AppError> {
        self.project.lock().map_err(|_| AppError::Internal)
    }

    /// Locks the project for one change that is undoable as `label`. Every
    /// command that changes project data (not settings) goes through here, so
    /// there is one place that records steps (CSD-027 CON-1). Keep the guard
    /// short-lived, like [`AppState::project`].
    ///
    /// # Errors
    /// Returns `Internal` if a previous holder panicked.
    pub fn edit(&self, label: StepLabel) -> Result<Edit<'_>, AppError> {
        let project = self.project()?;
        let before = Some(project.snapshot());
        Ok(Edit {
            project,
            label,
            before,
        })
    }

    /// The current calendar month in local time (CSD-001 REQ-5).
    #[must_use]
    pub fn today(&self) -> YearMonth {
        YearMonth::from_date(self.today_date())
    }

    /// The current calendar date in local time (CSD-013 CON-11).
    #[must_use]
    pub fn today_date(&self) -> Date {
        #[cfg(test)]
        if let Some(date) = self.fixed_today.lock().ok().and_then(|date| *date) {
            return date;
        }
        OffsetDateTime::now_utc().to_offset(self.utc_offset).date()
    }

    /// Makes `today` return `date` for the rest of a test.
    #[cfg(test)]
    pub fn fix_today(&self, date: Date) {
        if let Ok(mut fixed) = self.fixed_today.lock() {
            *fixed = Some(date);
        }
    }
}
