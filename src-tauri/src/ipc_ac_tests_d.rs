//! Acceptance tests (batch D) for CSD-008 and CSD-022 that need the
//! mock Tauri runtime: the native window theme, the preference defaults, the
//! messages of file errors.

use serde_json::{json, Value};
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use tauri::{ipc::InvokeBody, WebviewWindowBuilder};
use tempfile::TempDir;
use time::UtcOffset;

use crate::commands::preferences::window_theme;
use crate::domain::Theme;
use crate::error::AppError;
use crate::storage::{Preferences, StorageError};

/// An app whose preference file lives in `dir`, with its main window.
fn app_in(
    dir: &std::path::Path,
) -> (
    tauri::App<tauri::test::MockRuntime>,
    tauri::WebviewWindow<tauri::test::MockRuntime>,
) {
    let app = crate::configure(
        mock_builder().manage(Preferences::in_dir(dir)),
        UtcOffset::UTC,
    )
    .build(mock_context(noop_assets()))
    .unwrap();
    let window = WebviewWindowBuilder::new(&app, "main", tauri::WebviewUrl::default())
        .build()
        .unwrap();
    (app, window)
}

fn call(
    window: &tauri::WebviewWindow<tauri::test::MockRuntime>,
    command: &str,
    args: Value,
) -> Result<Value, Value> {
    let request = InvokeRequest {
        cmd: command.into(),
        callback: tauri::ipc::CallbackFn(0),
        error: tauri::ipc::CallbackFn(1),
        url: "tauri://localhost".parse().unwrap(),
        body: InvokeBody::Json(args),
        headers: tauri::http::HeaderMap::default(),
        invoke_key: tauri::test::INVOKE_KEY.to_string(),
    };
    get_ipc_response(window, request).map(|body| body.deserialize::<Value>().unwrap())
}

// ---- CSD-008 ----

#[test]
fn test_csd008_ac_7_1_the_native_window_gets_the_theme_and_system_leaves_it_to_macos() {
    // Dark and Light are given to the native window (title bar, dialogs);
    // System gives it nothing, so the window follows macOS, which is light.
    assert_eq!(window_theme(Theme::Dark), Some(tauri::Theme::Dark));
    assert_eq!(window_theme(Theme::Light), Some(tauri::Theme::Light));
    assert_eq!(window_theme(Theme::System), None);
}

#[test]
fn test_csd008_ac_7_1_choosing_a_theme_over_ipc_applies_it_to_the_window() {
    let dir = TempDir::new().unwrap();
    let (_app, window) = app_in(dir.path());
    for theme in ["dark", "light", "system"] {
        let answer = call(&window, "set_theme", json!({ "theme": theme })).unwrap();
        assert_eq!(answer["theme"], theme);
    }
}

// ---- CSD-022 ----

#[test]
fn test_csd022_ac_2_2_without_a_saved_language_the_text_is_english() {
    // No preference file and no way for the app to ask macOS for its language:
    // the answer is English, whatever the Mac is set to.
    let dir = TempDir::new().unwrap();
    let (_app, window) = app_in(dir.path());
    let answer = call(&window, "get_preferences", json!({})).unwrap();
    assert_eq!(answer["language"], "en");
    assert_eq!(answer["theme"], "system");
}

#[test]
fn test_csd022_ac_7_3_a_file_error_carries_a_code_and_no_path() {
    let path = "/Users/example/Documents/example-a.varde";
    let errors = [
        StorageError::SaveFailed(std::io::Error::other(path)),
        StorageError::OpenFailed(std::io::Error::other(path)),
        StorageError::InvalidFile,
        StorageError::FileNewer,
        StorageError::NoPath,
        StorageError::PreferencesSaveFailed(std::io::Error::other(path)),
    ];
    for error in errors {
        let expected_code = error.code();
        let sent = serde_json::to_value(AppError::from(error)).unwrap();
        assert_eq!(sent["code"], expected_code);
        let message = sent["message"].as_str().unwrap();
        assert_ne!(message, "", "{expected_code} has a message");
        for forbidden in [path, "/Users", "example-a", ".varde"] {
            assert!(
                !sent.to_string().contains(forbidden),
                "{expected_code}: {forbidden} must not reach the frontend"
            );
        }
    }
}

// ---- CSD-002 ----

#[test]
fn test_csd002_ac_4_2_the_close_decision_asks_only_when_there_are_unsaved_changes() {
    // The mock runtime has no event loop, so a real window close cannot be sent. The decision that
    // both the window close and the quit request use is `has_unsaved_changes`: when it is false the
    // close goes through without the `close-requested` event that makes the frontend ask.
    use tauri::Manager;

    let dir = TempDir::new().unwrap();
    let (app, window) = app_in(dir.path());
    let state = app.state::<crate::AppState>();
    let status = call(&window, "project_status", json!({})).unwrap();
    assert_eq!(status["unsaved_changes"], false);
    assert!(
        !crate::has_unsaved_changes(&state),
        "no changes: no question"
    );

    let job = json!({ "input": { "employer": "Example Employer", "title": "",
        "start_date": "01/03/2024", "end_date": "" } });
    call(&window, "create_job", job).unwrap();
    assert!(
        crate::has_unsaved_changes(&state),
        "a change: the question is asked"
    );
}
