//! The native macOS menu bar. The project actions live in the File menu
//! (CSD-002 REQ-9), Undo and Redo in the Edit menu (CSD-027 REQ-10) and
//! Settings… in the app menu (CSD-003 REQ-2); everything else is Tauri's
//! standard menu, which keeps copy and paste working in text fields.

use tauri::menu::{
    AboutMetadata, IsMenuItem, Menu, MenuItem, MenuItemBuilder, MenuItemKind, PredefinedMenuItem,
};
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

/// What the About panel shows (CSD-031 REQ-1). Plain text, English only, like
/// the rest of the native menu.
#[derive(Debug, PartialEq, Eq)]
pub struct AboutInfo {
    pub name: &'static str,
    /// The version of the package as built, the one source of the number (REQ-2).
    pub version: &'static str,
    pub description: &'static str,
    pub copyright: &'static str,
    pub license: &'static str,
    pub website: &'static str,
}

/// The content of the About panel.
pub const ABOUT: AboutInfo = AboutInfo {
    name: "Varde",
    version: env!("CARGO_PKG_VERSION"),
    description: "Personal wealth tracking. Your data stays on this computer.",
    copyright: "© 2026 Thijs Bardoel",
    license: "GPL-3.0-only",
    website: "https://github.com/BardoelT/varde",
};

impl AboutInfo {
    /// The text under the name. macOS shows only the name, version, copyright
    /// and these credits, so the description, licence and website go here.
    #[must_use]
    pub fn credits(&self) -> String {
        format!(
            "{}\n\nLicence: {}\n{}",
            self.description, self.license, self.website
        )
    }

    /// The metadata of the native panel, with the app icon passed explicitly so
    /// it also shows in a development run (REQ-1, AC-1.2).
    #[must_use]
    pub fn metadata(&self, icon: Option<tauri::image::Image<'static>>) -> AboutMetadata<'static> {
        AboutMetadata {
            name: Some(self.name.to_owned()),
            version: Some(self.version.to_owned()),
            copyright: Some(self.copyright.to_owned()),
            comments: Some(self.description.to_owned()),
            license: Some(self.license.to_owned()),
            website: Some(self.website.to_owned()),
            credits: Some(self.credits()),
            icon,
            ..AboutMetadata::default()
        }
    }
}

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
    replace_about(app, &app_menu)?;
    let (id, label, accelerator) = SETTINGS;
    let settings = MenuItemBuilder::with_id(id, label)
        .accelerator(accelerator)
        .build(app)?;
    let after_settings = PredefinedMenuItem::separator(app)?;
    app_menu.insert_items(&[&settings, &after_settings], SETTINGS_POSITION)?;

    replace_undo_and_redo(app, &menu)?;
    Ok(menu)
}

/// Swaps the predefined About item of the app menu for one with the Varde
/// content (CSD-031). It keeps the first position.
fn replace_about<R: Runtime>(
    app: &AppHandle<R>,
    app_menu: &tauri::menu::Submenu<R>,
) -> tauri::Result<()> {
    for item in app_menu.items()? {
        if let MenuItemKind::Predefined(predefined) = &item {
            if predefined
                .text()
                .is_ok_and(|text| text.starts_with("About"))
            {
                app_menu.remove(&item)?;
            }
        }
    }
    // The icon is embedded in the binary from the first icon of tauri.conf.json.
    let icon = app
        .default_window_icon()
        .map(|icon| icon.clone().to_owned());
    let about = PredefinedMenuItem::about(app, Some("About Varde"), Some(ABOUT.metadata(icon)))?;
    app_menu.insert(&about, 0)
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
    fn test_csd031_ac_1_1_the_panel_shows_the_app_information() {
        assert_eq!(ABOUT.name, "Varde");
        assert_eq!(
            ABOUT.description,
            "Personal wealth tracking. Your data stays on this computer."
        );
        assert_eq!(ABOUT.copyright, "© 2026 Thijs Bardoel");
        assert_eq!(ABOUT.license, "GPL-3.0-only");
        assert_eq!(ABOUT.website, "https://github.com/BardoelT/varde");
        let metadata = ABOUT.metadata(None);
        assert_eq!(metadata.name.as_deref(), Some("Varde"));
        assert_eq!(metadata.copyright.as_deref(), Some("© 2026 Thijs Bardoel"));
        // macOS shows the credits, so they carry the description, licence and website.
        let credits = metadata.credits.unwrap();
        for text in [ABOUT.description, ABOUT.license, ABOUT.website] {
            assert!(credits.contains(text), "{credits}");
        }
    }

    #[test]
    fn test_csd031_ac_1_3_the_panel_text_is_english() {
        // Constants, not translated texts (CSD-022 NG-6): the language setting never reaches them.
        assert!(ABOUT.credits().starts_with("Personal wealth tracking."));
    }

    #[test]
    fn test_csd031_ac_2_1_the_version_is_the_package_version() {
        assert_eq!(ABOUT.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(ABOUT.metadata(None).version.as_deref(), Some(ABOUT.version));
    }

    #[test]
    fn test_csd031_ac_2_2_package_and_bundle_versions_agree() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let bundle = config["version"].as_str().unwrap();
        assert_eq!(
            bundle,
            env!("CARGO_PKG_VERSION"),
            "tauri.conf.json says {bundle}, Cargo.toml says {}",
            env!("CARGO_PKG_VERSION")
        );
    }

    #[test]
    fn test_csd031_ac_1_2_the_first_configured_icon_is_a_large_png() {
        // The About panel gets the default window icon, which Tauri takes from the
        // first .png in tauri.conf.json: it must be the sharp one.
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let first = config["bundle"]["icon"][0].as_str().unwrap();
        assert_eq!(first, "icons/128x128@2x.png");
    }

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
