//! Persistence: the project file (CSD-002) and the application preferences (CSD-008).

mod file;
mod preferences;
mod project;
mod recent;

pub use file::MAX_FILE_BYTES;
pub use preferences::Preferences;
pub use project::{OpenOutcome, Project, ProjectStatus, Record, SaveOutcome, StepLabel};
pub use recent::RecentProjects;

use std::io;

use thiserror::Error;

/// Failures while saving or opening a project file.
///
/// Messages never contain file paths or operating-system details; the cause is
/// kept as the error source only.
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("The project could not be saved.")]
    SaveFailed(#[source] io::Error),
    #[error("The file could not be opened.")]
    OpenFailed(#[source] io::Error),
    #[error("This is not a valid project file.")]
    InvalidFile,
    #[error("This file was made by a newer version of the app.")]
    FileNewer,
    #[error("This file was made by an older version of the app. It has to be upgraded before it can be opened.")]
    FileOlder,
    #[error("The project is too large to save. Remove some data, such as old transactions, and try again.")]
    ProjectTooLarge,
    #[error("The project has no file yet.")]
    NoPath,
    #[error("The theme could not be saved.")]
    PreferencesSaveFailed(#[source] io::Error),
    #[error("There is nothing to undo.")]
    NothingToUndo,
    #[error("There is nothing to redo.")]
    NothingToRedo,
}

impl StorageError {
    /// Stable machine-readable code for the frontend.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::SaveFailed(_) => "PROJECT_SAVE_FAILED",
            Self::OpenFailed(_) => "PROJECT_OPEN_FAILED",
            Self::InvalidFile => "PROJECT_INVALID_FILE",
            Self::FileNewer => "PROJECT_FILE_NEWER",
            Self::FileOlder => "PROJECT_FILE_OLDER",
            Self::ProjectTooLarge => "PROJECT_TOO_LARGE",
            Self::NoPath => "PROJECT_NO_PATH",
            Self::PreferencesSaveFailed(_) => "PREFERENCES_SAVE_FAILED",
            Self::NothingToUndo => "NOTHING_TO_UNDO",
            Self::NothingToRedo => "NOTHING_TO_REDO",
        }
    }
}
