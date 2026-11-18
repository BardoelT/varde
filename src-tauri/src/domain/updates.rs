//! In-app updates (CSD-023): the safe errors, the "is it newer" rule and the
//! answers the frontend shows. Pure: the network and the install are in
//! `commands/updates.rs`.

use serde::Serialize;
use thiserror::Error;

/// Why an update run failed. One code per stage (CSD-023 REQ-8). The messages
/// are generic on purpose: no URL, path or raw error text ever reaches the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum UpdateError {
    /// The endpoint cannot be reached, or its answer is not a valid update file.
    #[error("Checking for updates failed. Check your connection and try again.")]
    CheckFailed,
    /// The download failed or stopped.
    #[error("Downloading the update failed. Check your connection and try again.")]
    DownloadFailed,
    /// The signature does not match the public key in the app.
    #[error("The update could not be verified, so it was not installed.")]
    SignatureInvalid,
    /// Replacing the app failed.
    #[error("Installing the update failed. Try again.")]
    InstallFailed,
}

impl UpdateError {
    /// Stable machine-readable code for the frontend.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::CheckFailed => "UPDATE_CHECK_FAILED",
            Self::DownloadFailed => "UPDATE_DOWNLOAD_FAILED",
            Self::SignatureInvalid => "UPDATE_SIGNATURE_INVALID",
            Self::InstallFailed => "UPDATE_INSTALL_FAILED",
        }
    }
}

/// The result of a check (CSD-023 REQ-3): nothing newer, or a newer version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum UpdateCheck {
    /// The installed version is the latest.
    UpToDate {
        /// The installed version.
        current: String,
    },
    /// A newer version can be installed.
    Available {
        /// The installed version.
        current: String,
        /// The newer version.
        version: String,
    },
    /// The update is installed, but the app has not restarted: there are
    /// changes the user was not asked about (CSD-023 REQ-14).
    Installed {
        /// The version that is installed.
        version: String,
    },
}

/// Download progress in bytes, as the `update-progress` event carries it
/// (CSD-023 REQ-6). `total` is `None` when the server sent no length.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Progress {
    /// Bytes received so far.
    pub downloaded: u64,
    /// Bytes expected in all, when known.
    pub total: Option<u64>,
}

/// `MAJOR.MINOR.PATCH` as three numbers. Anything else (a pre-release, build
/// metadata, a missing part) is not a release version (CSD-023 NG-6).
fn release_version(text: &str) -> Option<(u64, u64, u64)> {
    let mut parts = text.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    parts.next().is_none().then_some((major, minor, patch))
}

/// Whether `candidate` is a higher release version than `current`. A version
/// that is equal, lower or not a release version is not an update, so a
/// downgrade is never offered (CSD-023 REQ-4).
#[must_use]
pub fn is_newer(current: &str, candidate: &str) -> bool {
    match (release_version(current), release_version(candidate)) {
        (Some(current), Some(candidate)) => candidate > current,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csd023_ac_4_2_a_version_that_is_not_higher_is_not_an_update() {
        assert!(!is_newer("0.2.0", "0.2.0"));
        assert!(!is_newer("0.2.0", "0.1.9"));
        assert!(!is_newer("1.0.0", "0.9.9"));
    }

    #[test]
    fn test_csd023_ac_3_2_a_higher_version_is_an_update() {
        assert!(is_newer("0.1.0", "0.2.0"));
        assert!(is_newer("0.9.9", "0.10.0"));
        assert!(is_newer("0.2.0", "1.0.0"));
    }

    #[test]
    fn test_csd023_ns_6_pre_releases_and_odd_versions_are_not_updates() {
        for candidate in ["0.2.0-rc1", "0.2.0+build", "0.2", "0.2.0.1", "", "x.y.z"] {
            assert!(!is_newer("0.1.0", candidate), "{candidate}");
        }
        assert!(!is_newer("not a version", "0.2.0"));
    }

    #[test]
    fn test_csd023_ac_8_1_each_stage_has_its_code_and_a_safe_message() {
        let cases = [
            (UpdateError::CheckFailed, "UPDATE_CHECK_FAILED"),
            (UpdateError::DownloadFailed, "UPDATE_DOWNLOAD_FAILED"),
            (UpdateError::SignatureInvalid, "UPDATE_SIGNATURE_INVALID"),
            (UpdateError::InstallFailed, "UPDATE_INSTALL_FAILED"),
        ];
        for (error, code) in cases {
            assert_eq!(error.code(), code);
            let message = error.to_string();
            assert!(!message.contains("http"), "{message}");
            assert!(!message.contains('/'), "{message}");
        }
    }
}
