use crate::menu::{AboutInfo, ABOUT};

/// What the About dialog shows on Windows (CSD-038 REQ-14): the same texts and the one version as
/// the native panel of CSD-031.
#[tauri::command]
#[must_use]
pub fn get_about_info() -> &'static AboutInfo {
    &ABOUT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csd038_ac_14_1_the_about_dialog_has_the_texts_and_version_of_the_native_panel() {
        let info = get_about_info();
        assert_eq!(info.name, "Varde");
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(info.license, "GPL-3.0-only");
    }
}
