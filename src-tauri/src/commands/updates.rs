//! In-app updates (CSD-023): `check_for_update` and `install_update`. Rust
//! drives the updater plugin; the webview has no updater permission.

#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive their arguments by value"
)]

use std::future::Future;

use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_updater::{Error as PluginError, Update, UpdaterExt};

use crate::commands::AppState;
use crate::domain::updates::{is_newer, Progress, UpdateCheck, UpdateError};
use crate::error::AppError;

/// Event with the download progress (CSD-023 REQ-6).
pub const PROGRESS_EVENT: &str = "update-progress";

/// A newer version a source found, with what is needed to install it.
pub struct Found<H> {
    /// The newer version.
    pub version: String,
    /// What the source needs to download and install it.
    pub handle: H,
}

/// Where updates come from. The real source is the updater plugin; the tests
/// use canned answers, so no test calls the network (CSD-023 section 7).
pub trait UpdateSource: Send + Sync {
    /// What `install` needs to know about the found update.
    type Handle: Send + Sync;

    /// Fetches the endpoint. `None` when nothing newer is offered.
    fn check(
        &self,
    ) -> impl Future<Output = Result<Option<Found<Self::Handle>>, UpdateError>> + Send;

    /// Downloads the update and verifies its signature.
    fn download(
        &self,
        handle: &Self::Handle,
        on_progress: &mut (dyn FnMut(Progress) + Send),
    ) -> impl Future<Output = Result<Vec<u8>, UpdateError>> + Send;

    /// Replaces the installed app with the downloaded, verified update.
    ///
    /// # Errors
    /// [`UpdateError::SignatureInvalid`] or [`UpdateError::InstallFailed`].
    fn install(&self, handle: &Self::Handle, bytes: &[u8]) -> Result<(), UpdateError>;
}

/// How an install run ended without an error.
#[derive(Debug, PartialEq, Eq)]
pub enum InstallOutcome {
    /// The update is in place, with its version; the caller restarts the app.
    Installed(String),
    /// The endpoint offers nothing newer (any more), so nothing was downloaded.
    UpToDate,
}

/// One check (CSD-023 REQ-3, REQ-4): always asks the source again.
///
/// # Errors
/// The source's error.
pub async fn run_check<S: UpdateSource>(
    source: &S,
    current: &str,
) -> Result<UpdateCheck, UpdateError> {
    Ok(match source.check().await? {
        Some(found) => UpdateCheck::Available {
            current: current.to_owned(),
            version: found.version,
        },
        None => UpdateCheck::UpToDate {
            current: current.to_owned(),
        },
    })
}

/// One install (CSD-023 REQ-5): checks the source again first, so it installs
/// what the endpoint offers now, then downloads, verifies and installs.
///
/// # Errors
/// The source's error. Nothing is replaced unless the install itself ran.
pub async fn run_install<S: UpdateSource>(
    source: &S,
    mut on_progress: impl FnMut(Progress) + Send,
) -> Result<InstallOutcome, UpdateError> {
    let Some(found) = source.check().await? else {
        return Ok(InstallOutcome::UpToDate);
    };
    let bytes = source.download(&found.handle, &mut on_progress).await?;
    source.install(&found.handle, &bytes)?;
    Ok(InstallOutcome::Installed(found.version))
}

/// A signature problem is told apart from the stage's own failure.
fn stage_error(error: &PluginError, stage: UpdateError) -> UpdateError {
    match error {
        PluginError::Minisign(_)
        | PluginError::Base64(_)
        | PluginError::SignatureUtf8(_)
        | PluginError::SignedVersionMismatch { .. }
        | PluginError::MissingSignedVersion => UpdateError::SignatureInvalid,
        _ => stage,
    }
}

/// The updater plugin, configured in `tauri.conf.json` (the one endpoint and
/// the public key).
struct PluginSource<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> UpdateSource for PluginSource<R> {
    type Handle = Update;

    async fn check(&self) -> Result<Option<Found<Update>>, UpdateError> {
        let updater = self
            .app
            .updater_builder()
            // Only a higher release version counts: no downgrades (REQ-4).
            .version_comparator(|current, remote| {
                is_newer(&current.to_string(), &remote.version.to_string())
            })
            .build()
            .map_err(|_| UpdateError::CheckFailed)?;
        let update = updater
            .check()
            .await
            .map_err(|_| UpdateError::CheckFailed)?;
        Ok(update.map(|update| Found {
            version: update.version.clone(),
            handle: update,
        }))
    }

    async fn download(
        &self,
        handle: &Update,
        on_progress: &mut (dyn FnMut(Progress) + Send),
    ) -> Result<Vec<u8>, UpdateError> {
        let mut downloaded: u64 = 0;
        handle
            .download(
                |chunk, total| {
                    downloaded =
                        downloaded.saturating_add(u64::try_from(chunk).unwrap_or(u64::MAX));
                    on_progress(Progress { downloaded, total });
                },
                || {},
            )
            .await
            .map_err(|error| stage_error(&error, UpdateError::DownloadFailed))
    }

    fn install(&self, handle: &Update, bytes: &[u8]) -> Result<(), UpdateError> {
        handle
            .install(bytes)
            .map_err(|error| stage_error(&error, UpdateError::InstallFailed))
    }
}

/// The version of the running app, from the package (CSD-031 REQ-2).
#[tauri::command]
#[must_use]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// Checks the endpoint for a newer version (CSD-023 REQ-3). The answer is not
/// kept anywhere (REQ-10).
///
/// # Errors
/// An update code (REQ-8).
#[tauri::command]
pub async fn check_for_update<R: Runtime>(app: AppHandle<R>) -> Result<UpdateCheck, AppError> {
    Ok(run_check(&PluginSource { app }, env!("CARGO_PKG_VERSION")).await?)
}

/// Whether the app may restart now (CSD-023 REQ-14): it may when nothing is
/// unsaved, or when the user was asked about the unsaved changes and said yes
/// (REQ-7). Changes made after that question have not been asked about.
#[must_use]
pub fn may_restart(unsaved_now: bool, unsaved_confirmed: bool) -> bool {
    !unsaved_now || unsaved_confirmed
}

/// Downloads, verifies and installs the newest version, then restarts the app
/// (CSD-023 REQ-5). Returns only when it ended in an error, when there was
/// nothing newer, or when the update is installed but the restart waits
/// (REQ-14). `unsaved_confirmed` says the frontend asked about unsaved changes
/// and the user agreed (REQ-7).
///
/// # Errors
/// An update code (REQ-8).
#[tauri::command]
pub async fn install_update<R: Runtime>(
    app: AppHandle<R>,
    unsaved_confirmed: bool,
) -> Result<UpdateCheck, AppError> {
    let source = PluginSource { app: app.clone() };
    let progress_app = app.clone();
    let outcome = run_install(&source, move |progress| {
        // A progress event that cannot be sent only costs the bar a step.
        let _ = progress_app.emit(PROGRESS_EVENT, progress);
    })
    .await?;
    match outcome {
        InstallOutcome::UpToDate => Ok(UpdateCheck::UpToDate {
            current: env!("CARGO_PKG_VERSION").to_owned(),
        }),
        InstallOutcome::Installed(version) => {
            // The question was asked before the download. If the project changed
            // since (autosave off, no file yet), restarting would lose that work.
            let unsaved_now = app
                .state::<AppState>()
                .project()
                .map_or(true, |project| project.status().unsaved_changes);
            if may_restart(unsaved_now, unsaved_confirmed) {
                app.restart();
            }
            Ok(UpdateCheck::Installed { version })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Canned answers for the source: what each `check` gives (one per call),
    /// and how download and install end.
    struct Canned {
        checks: Mutex<Vec<Result<Option<&'static str>, UpdateError>>>,
        download: Result<Vec<u8>, UpdateError>,
        install: Result<(), UpdateError>,
        calls: Mutex<Vec<&'static str>>,
    }

    impl Canned {
        fn new(checks: Vec<Result<Option<&'static str>, UpdateError>>) -> Self {
            Self {
                checks: Mutex::new(checks),
                download: Ok(vec![1, 2, 3, 4]),
                install: Ok(()),
                calls: Mutex::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<&'static str> {
            self.calls
                .lock()
                .map(|calls| calls.clone())
                .unwrap_or_default()
        }

        fn note(&self, call: &'static str) {
            if let Ok(mut calls) = self.calls.lock() {
                calls.push(call);
            }
        }
    }

    #[allow(
        clippy::unused_async_trait_impl,
        reason = "the trait is the same as the real source's, whose methods really await"
    )]
    impl UpdateSource for Canned {
        type Handle = String;

        async fn check(&self) -> Result<Option<Found<String>>, UpdateError> {
            self.note("check");
            let answer = {
                let mut checks = self.checks.lock().map_err(|_| UpdateError::CheckFailed)?;
                if checks.is_empty() {
                    return Err(UpdateError::CheckFailed);
                }
                checks.remove(0)
            };
            answer.map(|version| {
                version.map(|version| Found {
                    version: version.to_owned(),
                    handle: version.to_owned(),
                })
            })
        }

        async fn download(
            &self,
            _handle: &String,
            on_progress: &mut (dyn FnMut(Progress) + Send),
        ) -> Result<Vec<u8>, UpdateError> {
            self.note("download");
            on_progress(Progress {
                downloaded: 2,
                total: Some(4),
            });
            on_progress(Progress {
                downloaded: 4,
                total: Some(4),
            });
            self.download.clone()
        }

        fn install(&self, _handle: &String, _bytes: &[u8]) -> Result<(), UpdateError> {
            self.note("install");
            self.install
        }
    }

    fn block_on<T>(future: impl Future<Output = T>) -> T {
        tauri::async_runtime::block_on(future)
    }

    #[test]
    fn test_csd023_ac_3_1_already_the_latest() {
        let source = Canned::new(vec![Ok(None)]);
        let check = block_on(run_check(&source, "0.1.0"));
        assert_eq!(
            check,
            Ok(UpdateCheck::UpToDate {
                current: "0.1.0".into()
            })
        );
    }

    #[test]
    fn test_csd023_ac_3_2_a_newer_version_exists() {
        let source = Canned::new(vec![Ok(Some("0.2.0"))]);
        let check = block_on(run_check(&source, "0.1.0"));
        assert_eq!(
            check,
            Ok(UpdateCheck::Available {
                current: "0.1.0".into(),
                version: "0.2.0".into()
            })
        );
    }

    #[test]
    fn test_csd023_ac_3_3_an_unreachable_endpoint_is_a_check_failure() {
        let source = Canned::new(vec![Err(UpdateError::CheckFailed)]);
        let check = block_on(run_check(&source, "0.1.0"));
        assert_eq!(check, Err(UpdateError::CheckFailed));
    }

    #[test]
    fn test_csd023_ac_4_1_a_second_check_asks_the_source_again() {
        let source = Canned::new(vec![Ok(Some("0.2.0")), Ok(Some("0.3.0"))]);
        let first = block_on(run_check(&source, "0.1.0"));
        let second = block_on(run_check(&source, "0.1.0"));
        assert!(matches!(first, Ok(UpdateCheck::Available { version, .. }) if version == "0.2.0"));
        assert!(matches!(second, Ok(UpdateCheck::Available { version, .. }) if version == "0.3.0"));
    }

    #[test]
    fn test_csd023_ac_5_1_a_successful_install_checks_downloads_and_installs() {
        let source = Canned::new(vec![Ok(Some("0.2.0"))]);
        let outcome = block_on(run_install(&source, |_| {}));
        assert_eq!(outcome, Ok(InstallOutcome::Installed("0.2.0".into())));
        assert_eq!(source.calls(), ["check", "download", "install"]);
    }

    #[test]
    fn test_csd023_ac_5_2_the_install_uses_what_the_endpoint_says_now() {
        // The check that found 0.2.0 is long gone: install asks again and gets 0.3.0.
        let source = Canned::new(vec![Ok(Some("0.3.0"))]);
        let outcome = block_on(run_install(&source, |_| {}));
        assert_eq!(outcome, Ok(InstallOutcome::Installed("0.3.0".into())));
        assert_eq!(source.calls().first(), Some(&"check"));
    }

    #[test]
    fn test_csd023_ac_5_3_nothing_newer_any_more_downloads_nothing() {
        let source = Canned::new(vec![Ok(None)]);
        let outcome = block_on(run_install(&source, |_| {}));
        assert_eq!(outcome, Ok(InstallOutcome::UpToDate));
        assert_eq!(source.calls(), ["check"]);
    }

    #[test]
    fn test_csd023_ac_6_1_progress_reaches_the_caller() {
        let source = Canned::new(vec![Ok(Some("0.2.0"))]);
        let mut seen = Vec::new();
        let outcome = block_on(run_install(&source, |progress| seen.push(progress)));
        assert_eq!(outcome, Ok(InstallOutcome::Installed("0.2.0".into())));
        assert_eq!(
            seen,
            [
                Progress {
                    downloaded: 2,
                    total: Some(4)
                },
                Progress {
                    downloaded: 4,
                    total: Some(4)
                }
            ]
        );
    }

    #[test]
    fn test_csd023_ac_8_1_each_stage_reports_its_own_code() {
        let mut source = Canned::new(vec![Err(UpdateError::CheckFailed)]);
        assert_eq!(
            block_on(run_install(&source, |_| {})),
            Err(UpdateError::CheckFailed)
        );

        source = Canned::new(vec![Ok(Some("0.2.0"))]);
        source.download = Err(UpdateError::DownloadFailed);
        assert_eq!(
            block_on(run_install(&source, |_| {})),
            Err(UpdateError::DownloadFailed)
        );
        assert_eq!(
            source.calls(),
            ["check", "download"],
            "nothing is installed"
        );

        source = Canned::new(vec![Ok(Some("0.2.0"))]);
        source.download = Err(UpdateError::SignatureInvalid);
        assert_eq!(
            block_on(run_install(&source, |_| {})),
            Err(UpdateError::SignatureInvalid)
        );
        assert_eq!(
            source.calls(),
            ["check", "download"],
            "a bad signature installs nothing"
        );

        source = Canned::new(vec![Ok(Some("0.2.0"))]);
        source.install = Err(UpdateError::InstallFailed);
        assert_eq!(
            block_on(run_install(&source, |_| {})),
            Err(UpdateError::InstallFailed)
        );
    }

    #[test]
    fn test_csd023_ac_8_1_signature_errors_are_told_apart_from_the_stage() {
        let signature = PluginError::MissingSignedVersion;
        assert_eq!(
            stage_error(&signature, UpdateError::DownloadFailed),
            UpdateError::SignatureInvalid
        );
        let network = PluginError::Network("status 500 from https://example.invalid/x".into());
        assert_eq!(
            stage_error(&network, UpdateError::DownloadFailed),
            UpdateError::DownloadFailed
        );
        assert_eq!(
            stage_error(
                &PluginError::BinaryNotFoundInArchive,
                UpdateError::InstallFailed
            ),
            UpdateError::InstallFailed
        );
    }

    fn conf() -> serde_json::Value {
        serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap()
    }

    #[test]
    fn test_csd023_ac_11_1_the_endpoint_is_fixed() {
        let conf = conf();
        let endpoints = conf["plugins"]["updater"]["endpoints"].as_array().unwrap();
        assert_eq!(
            endpoints,
            &[serde_json::json!(
                "https://github.com/BardoelT/varde/releases/latest/download/latest.json"
            )]
        );
    }

    #[test]
    fn test_csd023_con_1_the_webview_has_no_updater_permission() {
        let capability = include_str!("../../capabilities/default.json");
        assert!(!capability.contains("updater"), "{capability}");
    }

    #[test]
    fn test_csd023_ac_2_3_the_native_menu_has_no_update_item() {
        let items = crate::menu::ITEMS
            .iter()
            .map(|(id, label, _)| format!("{id} {label}"));
        for item in items {
            assert!(!item.to_lowercase().contains("update"), "{item}");
        }
    }

    #[test]
    fn test_csd021_the_bundle_builds_the_update_files() {
        let conf = conf();
        assert_eq!(conf["bundle"]["createUpdaterArtifacts"], true);
        assert_eq!(conf["bundle"]["targets"], serde_json::json!(["dmg", "app"]));
    }

    #[test]
    fn test_csd023_ac_14_1_unconfirmed_changes_made_during_the_download_stop_the_restart() {
        // Nothing unsaved at the question, something unsaved now: no restart.
        assert!(!may_restart(true, false));
    }

    #[test]
    fn test_csd023_ac_14_2_the_restart_goes_on_when_the_user_agreed_or_nothing_is_unsaved() {
        assert!(may_restart(true, true), "the user agreed to lose them");
        assert!(may_restart(false, false), "nothing to lose");
        assert!(may_restart(false, true));
    }

    #[test]
    fn test_csd023_ac_14_1_the_installed_answer_serializes_with_its_version() {
        let json = serde_json::to_value(UpdateCheck::Installed {
            version: "0.2.0".into(),
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "status": "installed", "version": "0.2.0" })
        );
    }

    #[test]
    fn test_csd023_ac_8_1_the_serialized_error_holds_the_code_and_no_url() {
        let error = AppError::from(UpdateError::DownloadFailed);
        let json = serde_json::to_string(&error).unwrap_or_default();
        assert!(json.contains("UPDATE_DOWNLOAD_FAILED"));
        assert!(!json.contains("http"));
    }
}
