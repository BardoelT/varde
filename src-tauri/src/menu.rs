//! The native macOS menu bar. The project actions live in the File menu
//! (CSD-002 REQ-9), Undo and Redo in the Edit menu (CSD-027 REQ-10) and
//! Settings… in the app menu (CSD-003 REQ-2); everything else is Tauri's
//! standard menu, which keeps copy and paste working in text fields.

use tauri::menu::{IsMenuItem, Menu, MenuItem, MenuItemBuilder, MenuItemKind, PredefinedMenuItem};
use tauri::{AppHandle, Manager, Runtime};

use crate::storage::{ProjectStatus, StepLabel};

/// Event the frontend listens for; the payload is one of the item ids below.
pub const MENU_EVENT: &str = "menu-action";

/// The project items in menu order: id, label and shortcut.
pub const ITEMS: [(&str, &str, &str); 4] = [
    ("project-new", "New", "CmdOrCtrl+N"),
    ("project-open", "Open…", "CmdOrCtrl+O"),
    ("project-save", "Save", "CmdOrCtrl+S"),
    ("project-save-as", "Save As…", "CmdOrCtrl+Shift+S"),
];

/// The Settings… item of the app menu: id, label and shortcut (⌘,).
pub const SETTINGS: (&str, &str, &str) = ("open-settings", "Settings…", "CmdOrCtrl+,");

/// The Undo and Redo items of the Edit menu: id, verb and shortcut. They
/// replace the predefined ones (CSD-027 REQ-10, CON-3).
pub const UNDO: (&str, &str, &str) = ("project-undo", "Undo", "CmdOrCtrl+Z");
pub const REDO: (&str, &str, &str) = ("project-redo", "Redo", "CmdOrCtrl+Shift+Z");

/// Whether `id` is an item whose action the frontend runs.
#[must_use]
pub fn is_forwarded_item(id: &str) -> bool {
    id == SETTINGS.0 || id == UNDO.0 || id == REDO.0 || ITEMS.iter().any(|(item, _, _)| *item == id)
}

/// The text of an Undo or Redo item: the verb, followed by the step it applies
/// to when there is one, such as "Undo Delete Bank Account" (CSD-027 AC-10.1).
#[must_use]
pub fn history_text(verb: &str, step: Option<StepLabel>) -> String {
    step.map_or_else(|| verb.to_owned(), |step| format!("{verb} {step}"))
}

/// The two Edit items, kept so their text and state can follow the history.
pub struct HistoryItems<R: Runtime> {
    undo: MenuItem<R>,
    redo: MenuItem<R>,
}

/// Makes the Edit items show what Undo and Redo would apply, and disables an
/// item when there is no step for it (CSD-027 REQ-10). Without a built menu, as
/// in the IPC tests, there is nothing to update.
pub fn sync<R: Runtime>(app: &AppHandle<R>, status: ProjectStatus) {
    let Some(items) = app.try_state::<HistoryItems<R>>() else {
        return;
    };
    for (item, verb, step) in [
        (&items.undo, UNDO.1, status.undo_label),
        (&items.redo, REDO.1, status.redo_label),
    ] {
        // A menu that cannot be updated must not fail the command behind it.
        let _ = item.set_text(history_text(verb, step));
        let _ = item.set_enabled(step.is_some());
    }
}

const FILE_MENU: &str = "File";
const EDIT_MENU: &str = "Edit";

/// In the default macOS app menu: About, separator, then the rest. Settings…
/// goes right below About and that separator, as in other macOS apps.
const SETTINGS_POSITION: usize = 2;

/// Builds Tauri's default menu with the project items at the top of File and
/// Settings… in the app menu.
///
/// # Errors
/// Returns an error if the menu or one of its items cannot be created, or if
/// the default menu has no File or app menu.
pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let menu = Menu::default(app)?;
    let mut items = Vec::new();
    for (id, label, accelerator) in ITEMS {
        items.push(
            MenuItemBuilder::with_id(id, label)
                .accelerator(accelerator)
                .build(app)?,
        );
    }
    let separator = PredefinedMenuItem::separator(app)?;

    let file = menu
        .items()?
        .into_iter()
        .find_map(|item| match item {
            MenuItemKind::Submenu(submenu) if submenu.text().is_ok_and(|t| t == FILE_MENU) => {
                Some(submenu)
            }
            _ => None,
        })
        .ok_or(tauri::Error::InvalidWindowHandle)?;
    let mut entries: Vec<&dyn IsMenuItem<R>> = items.iter().map(|i| i as _).collect();
    entries.push(&separator);
    file.insert_items(&entries, 0)?;

    // The app menu is the first one, named after the program.
    let app_menu = menu
        .items()?
        .into_iter()
        .find_map(|item| match item {
            MenuItemKind::Submenu(submenu) => Some(submenu),
            _ => None,
        })
        .ok_or(tauri::Error::InvalidWindowHandle)?;
    let (id, label, accelerator) = SETTINGS;
    let settings = MenuItemBuilder::with_id(id, label)
        .accelerator(accelerator)
        .build(app)?;
    let after_settings = PredefinedMenuItem::separator(app)?;
    app_menu.insert_items(&[&settings, &after_settings], SETTINGS_POSITION)?;

    replace_undo_and_redo(app, &menu)?;
    Ok(menu)
}

/// Swaps the predefined Undo and Redo of the Edit menu for the project items.
/// Cut, copy, paste and select all stay (CSD-027 CON-3).
fn replace_undo_and_redo<R: Runtime>(app: &AppHandle<R>, menu: &Menu<R>) -> tauri::Result<()> {
    let edit = menu
        .items()?
        .into_iter()
        .find_map(|item| match item {
            MenuItemKind::Submenu(submenu) if submenu.text().is_ok_and(|t| t == EDIT_MENU) => {
                Some(submenu)
            }
            _ => None,
        })
        .ok_or(tauri::Error::InvalidWindowHandle)?;
    for item in edit.items()? {
        let MenuItemKind::Predefined(predefined) = &item else {
            continue;
        };
        if predefined
            .text()
            .is_ok_and(|text| text == UNDO.1 || text == REDO.1)
        {
            edit.remove(&item)?;
        }
    }
    let make = |(id, verb, accelerator): (&str, &str, &str)| {
        MenuItemBuilder::with_id(id, verb)
            .accelerator(accelerator)
            .enabled(false)
            .build(app)
    };
    let undo = make(UNDO)?;
    let redo = make(REDO)?;
    edit.insert_items(&[&undo, &redo], 0)?;
    app.manage(HistoryItems { undo, redo });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Record;

    // Native menus can only be created on the main thread, which unit tests do
    // not run on. The table the menu is built from is tested here; the built
    // menu is part of the manual checklist (CSD-002 section 7).
    #[test]
    fn test_csd002_ac_9_1_menu_table_matches_the_spec() {
        let spec = [
            ("project-new", "New", "CmdOrCtrl+N"),
            ("project-open", "Open…", "CmdOrCtrl+O"),
            ("project-save", "Save", "CmdOrCtrl+S"),
            ("project-save-as", "Save As…", "CmdOrCtrl+Shift+S"),
        ];
        assert_eq!(ITEMS, spec);
    }

    #[test]
    fn test_csd003_ac_2_1_settings_item_is_in_the_app_menu_with_the_shortcut() {
        assert_eq!(SETTINGS, ("open-settings", "Settings…", "CmdOrCtrl+,"));
        // Right below About and its separator.
        assert_eq!(SETTINGS_POSITION, 2);
    }

    #[test]
    fn test_csd027_ac_10_1_edit_items_have_the_shortcuts_and_the_step_in_the_text() {
        assert_eq!(UNDO, ("project-undo", "Undo", "CmdOrCtrl+Z"));
        assert_eq!(REDO, ("project-redo", "Redo", "CmdOrCtrl+Shift+Z"));
        let step = StepLabel::delete(Record::BankAccount);
        assert_eq!(history_text("Undo", Some(step)), "Undo Delete Bank Account");
        assert_eq!(history_text("Redo", None), "Redo");
    }

    #[test]
    fn test_csd027_ac_10_2_the_text_follows_the_step() {
        let add = StepLabel::add(Record::Expense);
        assert_eq!(history_text("Undo", Some(add)), "Undo Add Expense");
        assert_eq!(history_text("Redo", Some(add)), "Redo Add Expense");
    }

    #[test]
    fn only_known_items_are_forwarded_to_the_frontend() {
        for (id, _, _) in ITEMS {
            assert!(is_forwarded_item(id));
        }
        assert!(is_forwarded_item("project-undo"));
        assert!(is_forwarded_item("project-redo"));
        assert!(is_forwarded_item("open-settings"));
        assert!(!is_forwarded_item("quit"));
        assert!(!is_forwarded_item(""));
    }
}
