//! The title of the main window (CSD-024 REQ-8, REQ-9).

/// The title shown while the project has no file yet. English only, like the
/// menu (CSD-024 REQ-8).
const UNTITLED: &str = "Untitled";

/// The unsaved-changes dot, placed after one space (CSD-024 REQ-9).
const UNSAVED_DOT: &str = "●";

/// The window title: the file name with its extension (`Untitled` without a
/// file), followed by a space and a dot while there are unsaved changes.
#[must_use]
pub fn window_title(file_name: Option<&str>, unsaved_changes: bool) -> String {
    let name = file_name.unwrap_or(UNTITLED);
    if unsaved_changes {
        format!("{name} {UNSAVED_DOT}")
    } else {
        name.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csd024_ac_8_1_a_new_project_is_untitled() {
        assert_eq!(window_title(None, false), "Untitled");
    }

    #[test]
    fn test_csd024_ac_8_2_the_title_is_the_file_name() {
        assert_eq!(
            window_title(Some("something.varde"), false),
            "something.varde"
        );
    }

    #[test]
    fn test_csd024_ac_9_1_an_unsaved_new_project_has_the_dot() {
        assert_eq!(window_title(None, true), "Untitled ●");
    }

    #[test]
    fn test_csd024_ac_9_2_an_unsaved_file_has_the_dot() {
        assert_eq!(
            window_title(Some("something.varde"), true),
            "something.varde ●"
        );
    }
}
