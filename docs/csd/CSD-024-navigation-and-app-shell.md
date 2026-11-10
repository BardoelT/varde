---
id: CSD-024
title: Navigation and app shell
status: approved
owner: Thijs Bardoel
version: 0.6
last_updated: 09/11/2026
related: [CSD-002, CSD-003, CSD-004, CSD-022, CSD-027, CSD-031]
---

# CSD-024: Navigation and app shell

## 1. Intent
The app has one window with a left pane that opens the pages, and a native macOS menu bar. The name of the open project file is shown in the window title (GitHub issue 20). Until now the left pane was specified inside the income dashboard CSD (CSD-004), although it belongs to the whole app, and the menu was described only in pieces in CSD-002 and CSD-003. This capability is the one place for the structure of the left pane, which page opens first, how a page is chosen, and what the native menu holds. The pages themselves and the actions behind the menu items stay with their own CSDs.

## 2. Scope
### 2.1 In scope
- The left pane: its groups, the order of its items, and Settings pinned at the bottom.
- The window title: the name of the open project file, and a dot while the project has unsaved changes.
- Which page the app opens on, and choosing a page with the mouse or the keyboard.
- The native menu bar: which items it holds, and that its texts stay English.

### 2.2 Non-goals
- NG-1: Any page in the Insights group other than the Net worth page of CSD-017 and the Ratios page of CSD-018 (moved from CSD-004 NG-5).
- NG-2: Pages that have no function yet: Accounts, Transactions and Budget.
- NG-3: Collapsible groups, and a user-defined order of the items.
- NG-4: Keyboard shortcuts that open a page, other than ⌘, for Settings (CSD-003 REQ-2). Decided for the first release (OQ-1).
- NG-5: The content of a page, its tabs (such as Overview and Insights) and the actions behind the menu items (the owning CSDs).
- NG-6: Remembering the last shown page between sessions (OQ-2).
- NG-7: A status text in the left pane for the project ("New project", "All changes saved"); the window title replaces it (GitHub issue 20).
- NG-8: The macOS "document edited" dot in the close button (it needs `NSWindow` through FFI, which `CLAUDE.md` forbids) and an "— Edited" suffix: the dot is a character in the title (REQ-9).
- NG-9: A translated window title: "Untitled" stays English, like the menu (REQ-6).

## 3. Definitions
| Term | Meaning |
|---|---|
| **Left pane** | The column at the left of the window with the groups of pages and Settings. |
| **Group** | A plain, non-collapsible heading in the left pane with the pages below it. |
| **Page** | One view the left pane can open. Exactly one page is shown at a time. |
| **Window title** | The text the native macOS title bar of the window shows, centred at the top. |
| **Unsaved dot** | The character `●`, placed after the file name in the window title while the project has unsaved changes (CSD-002 REQ-3). |

## 4. Requirements

### 4.0 Overview flow
```mermaid
sequenceDiagram
    actor U as User
    participant MENU as macOS menu bar
    participant UI as Frontend (ui/)
    U->>UI: chooses an item in the left pane
    UI->>U: shows that page, marks the item as current
    U->>MENU: chooses Settings… or a File item
    MENU->>UI: menu-action with the item id
    UI->>U: runs the action (CSD-002, CSD-003)
```

**REQ-1: Structure of the left pane** *(ubiquitous)*
The app shall group the pages in the left pane under plain, non-collapsible headings, in this order: Overview (Dashboard), Finance (Income, Expenses), Wealth (Bank accounts, Investments, Houses), Liabilities (Debts) and Insights (Net worth, Ratios). This requirement is the one place that states the order; the capabilities that add a page do not repeat it.

```gherkin
@AC-1.1
Scenario: Groups and pages in order
  Given the app has started
  Then the left pane shows the headings "Overview", "Finance", "Wealth", "Liabilities" and "Insights" in that order
  And "Overview" holds "Dashboard", "Finance" holds "Income" and "Expenses", "Wealth" holds "Bank accounts", "Investments" and "Houses", "Liabilities" holds "Debts" and "Insights" holds "Net worth" and "Ratios", each in that order

@AC-1.2
Scenario: Pages without a function are not offered
  Given the app has started
  Then there is no "Accounts", "Transactions" or "Budget" in the left pane

@AC-1.3
Scenario: No example data
  Given the app has started
  Then no page shows example transactions, accounts or budgets
```

**REQ-2: The app opens on the Dashboard** *(event-driven)*
When the app has started, the app shall show the Dashboard page and mark its item in the left pane as the current page.

```gherkin
@AC-2.1
Scenario: First page
  Given the app has started with a new project
  Then the Dashboard page is shown
  And "Dashboard" is the only item marked as the current page

@AC-2.2
Scenario: A remembered project does not change the first page
  Given a project was remembered from the last session (CSD-002 REQ-8)
  When the app has started and opened it
  Then the Dashboard page is shown
```

**REQ-3: Choosing a page** *(event-driven)*
When the user chooses an item in the left pane, with the mouse or by pressing Enter or Space on it after reaching it with Tab, the app shall show that page only and mark that item, and no other, as the current page.

```gherkin
@AC-3.1
Scenario: Mouse
  Given the Dashboard is shown
  When the user chooses "Debts"
  Then the Debts page is shown and the Dashboard page is hidden
  And "Debts" is the only item marked as the current page

@AC-3.2
Scenario: Keyboard
  Given the Dashboard is shown
  When the user reaches "Income" with Tab and presses Enter
  Then the Income page is shown
  And "Income" is the only item marked as the current page

@AC-3.3
Scenario: Choosing the current page again
  Given the Income page is shown
  When the user chooses "Income"
  Then the Income page stays shown and "Income" stays marked as the current page
```

**REQ-4: Settings at the bottom** *(ubiquitous)*
The app shall pin "Settings" at the bottom of the left pane, outside the groups, and shall not show a project status text in the left pane.

```gherkin
@AC-4.1
Scenario: Pinned item
  Given the app has started
  Then "Settings" is shown below the group "Insights", outside every group
  And it is the last item of the left pane

@AC-4.2
Scenario: Pinned item stays in place
  Given the user has opened "Net worth"
  Then "Settings" is still shown at the bottom of the left pane

@AC-4.3
Scenario: No project status in the left pane
  Given the app has started
  Then the left pane shows no "New project", "Unsaved changes" or "All changes saved" text
```

**REQ-5: Native menu** *(ubiquitous)*
The app shall show the native macOS menu bar with the standard menus (app, File, Edit, View and Window), where the app menu holds "Settings…" (⌘,) below "About" and the File menu holds "New" (⌘N), "Open…" (⌘O), "Save" (⌘S) and "Save As…" (⇧⌘S) above its standard items, and the Edit menu holds "Undo" (⌘Z) and "Redo" (⇧⌘Z) as specified in CSD-027 REQ-10, and the app menu's "About" shows the panel specified in CSD-031, and no other menu item is specified.

```gherkin
@AC-5.1
Scenario: Items and shortcuts
  Given the app has started
  Then the app menu holds "Settings…" with ⌘, directly below "About"
  And the File menu starts with "New" ⌘N, "Open…" ⌘O, "Save" ⌘S and "Save As…" ⇧⌘S
  And the menu bar has the menus app, File, Edit, View and Window

@AC-5.2
Scenario: Copy and paste keep working
  Given a text field has the focus
  When the user presses ⌘C and ⌘V
  Then the text is copied and pasted
```

**REQ-6: The menu stays English** *(ubiquitous)*
The app shall show the texts of the native menu bar in English, whatever the language setting is (CSD-022 NG-6).

```gherkin
@AC-6.1
Scenario: Dutch language, English menu
  Given the language setting is Dutch
  Then the left pane shows Dutch texts
  And the menu bar shows "Settings…" and "Save As…"
```

**REQ-7: Menu items run in the window** *(event-driven)*
When the user chooses "Settings…" or a File menu item, the app shall run that item's action in the window as specified in CSD-003 REQ-2 and CSD-002 REQ-9, from whichever page is shown.

```gherkin
@AC-7.1
Scenario: Settings from another page
  Given the Income page is shown
  When the user presses ⌘,
  Then the Settings page is shown
  And "Settings" is the only item marked as the current page

@AC-7.2
Scenario: A menu item for an action that needs confirmation
  Given the project has unsaved changes
  When the user chooses File > New
  Then a confirmation is asked before the project is replaced (CSD-002 REQ-7)
```

**REQ-8: Window title shows the open file** *(ubiquitous)*
The app shall show the name of the open project file, with its extension and without its folder, as the window title, and the text `Untitled` while the project has no path. The title holds nothing else: no app name and no path.

```gherkin
@AC-8.1
Scenario: New project
  Given the app has started with a new project
  Then the window title is "Untitled"

@AC-8.2
Scenario: Opened file
  Given the user opens the project file "/Users/example/Documents/something.varde"
  Then the window title is "something.varde"

@AC-8.3
Scenario: Save As changes the name
  Given a project saved to "example-a.varde"
  When the user chooses Save As and picks "example-b.varde"
  Then the window title is "example-b.varde"

@AC-8.4
Scenario: New project after a file
  Given the project file "something.varde" is open
  When the user starts a new project
  Then the window title is "Untitled"

@AC-8.5
Scenario: A remembered project is reopened at startup
  Given a project was remembered from the last session (CSD-002 REQ-8)
  When the app has started and opened it
  Then the window title is the name of that file
  And when it could not be opened (CSD-002 REQ-11 to REQ-13) the window title is "Untitled"

@AC-8.6
Scenario: No folder, same in every language
  Given the project file "/Users/example/Documents/something.varde" is open
  And the language setting is Dutch
  Then the window title is "something.varde"
  And the window title does not contain "/Users" or "Documents"
```

**REQ-9: Unsaved dot in the window title** *(state-driven)*
While the project has unsaved changes (CSD-002 REQ-3), the app shall show one space and the unsaved dot `●` after the name in the window title, and shall show neither while it has none.

```gherkin
@AC-9.1
Scenario: New project with an edit
  Given the app has started with a new project
  When the user adds a job
  Then the window title is "Untitled ●"

@AC-9.2
Scenario: Opened file with an edit
  Given the project file "something.varde" is open
  When the user adds a job
  Then the window title is "something.varde ●"

@AC-9.3
Scenario: Saving clears the dot
  Given the window title is "something.varde ●"
  When the user saves
  Then the window title is "something.varde"

@AC-9.4
Scenario: Undo and redo move the dot (CSD-027)
  Given the project file "something.varde" is open
  When the user adds a job and chooses Edit > Undo
  Then the window title is "something.varde"
  When the user chooses Edit > Redo
  Then the window title is "something.varde ●"

@AC-9.5
Scenario: A failed save keeps the dot
  Given the window title is "something.varde ●"
  And the chosen path is not writable
  When the user saves
  Then the window title is still "something.varde ●"
```

## 5. Interfaces and contracts

Window title: Rust sets the title of the main window after every change of the project state (open, save, Save As, new project, edit, undo, redo) and at startup, with the pure function `window_title(file_name: Option<&str>, unsaved_changes: bool) -> String` in `domain/`. The command layer passes the last part of the path only, so the folder never reaches the title. The title is never set from the frontend.

Menu: Rust builds the menu. The items `open-settings`, `project-new`, `project-open`, `project-save` and `project-save-as` emit the event `menu-action` with the item id to the frontend, which runs the action. The page elements and their ids are frontend-internal.

## 6. Constraints
- CON-1: The menu uses Tauri's built-in menu API: no plugin and no extra permission. Rationale: least privilege per `CLAUDE.md` (as CSD-002 CON-6 and CSD-003 CON-3).
- CON-2: The standard macOS Edit menu stays, except that its predefined Undo and Redo are replaced by the project Undo and Redo of CSD-027, which leave ⌘Z and ⇧⌘Z to a focused text field (CSD-027 REQ-11, CON-3). Rationale: it makes copy and paste work in text fields, and typing can still be undone there (CSD-002 CON-6).
- CON-4: The window title is set with Tauri's built-in window API from Rust: no plugin and no new capability (`core:window:allow-set-title` is not added). Rationale: least privilege per `CLAUDE.md`, and the file name stays in Rust (CSD-002 error rule: no paths to the UI).
- CON-3: The left pane is part of the page, not of one capability: a capability that adds a page adds its item through this CSD. Rationale: one place for the order.

## 7. Verification
The left pane and the page switching are checked by DOM tests (`npm test`, dev-only test harness in `ui/tests/`, `navigation.test.js`, titles carry the AC id). Tests are named `test_csd024_ac_<n>_<m>_<description>` (CSD-000 REQ-6).

| What | How |
|---|---|
| Display ACs with a DOM test | `npm test`: `navigation.test.js` (AC-1.1 to AC-1.3, AC-2.1, AC-2.2, AC-3.1 to AC-3.3, AC-4.1 to AC-4.3, AC-6.1, AC-7.1, AC-7.2) |
| Window title (REQ-8, REQ-9) | `cargo test test_csd024_`: `window_title` as a pure function (all four strings, `Untitled` and the dot), and the command layer with the Tauri mock runtime (the title after open, save, Save As, new, edit, undo, redo and the failed save, AC-8.1 to AC-8.6, AC-9.1 to AC-9.5). jsdom cannot see a native title |
| Menu tables (REQ-5, REQ-7) | `cargo test test_csd002_ac_9_` and `cargo test test_csd003_ac_2_1` check the tables the menu is built from. Native menus can only be created on the main thread, so the built menu (AC-5.1, AC-5.2, AC-6.1 menu texts) is on the manual checklist |

Not covered by a named test yet (2 of 27 ACs; no test carries the AC id): AC-5.1, AC-5.2. Both need the native menu bar and a real text field, which no automatic test can build (muda creates menus on the main thread only, and a native paste is an OS action). The parts of AC-5.1 that are tables are proven by `test_csd002_ac_9_1_...` and `test_csd003_ac_2_1_...`; the rest is on the manual checklist.

Done means fmt, clippy (`-D warnings`), `cargo test --all-features` and `npm test` pass.

## 8. Open questions
- OQ-1: (resolved 03/11/2026) No keyboard shortcuts for pages in the first release; only ⌘, for Settings exists and NG-4 stays.
- OQ-2: (resolved 03/11/2026) The app always opens on the Dashboard and does not remember the last page (REQ-2, AC-2.2, NG-6).
- OQ-3: (resolved 03/11/2026) The native menu is specified only as the standard menus (app, File, Edit, View, Window) plus the items already named; it is not listed item by item (REQ-5).

## 9. Changelog
- 0.6 (09/11/2026): REQ-5 says the content of the "About" item is specified in CSD-031 (GitHub issue 54); related adds CSD-031. No other change. Back to `draft` until re-approved.
- 0.5 (09/11/2026): The name of the open file in the window title (GitHub issue 20). New REQ-8 (AC-8.1 to AC-8.6: the file name with extension, `Untitled` for a new project, no folder, no app name, English only) and REQ-9 (AC-9.1 to AC-9.5: the dot `●` after the name while there are unsaved changes). REQ-4 now pins only Settings and the project status text leaves the left pane (new AC-4.3, NG-7); the unsaved indicator of CSD-002 REQ-3 becomes the dot. New NG-8, NG-9, CON-4, and the window title in section 5 and 7. Owner decisions from the grill-me interview of 09/11/2026. Back to `draft` until re-approved.
- 0.4 (07/11/2026): Undo and redo of project data (CSD-027, GitHub issue 43): REQ-5 names the Edit items Undo (⌘Z) and Redo (⇧⌘Z), CON-2 says they replace the predefined ones. Back to `draft` until re-approved.
- 0.3 (03/11/2026): Verification (section 7) lists the real test filters and the ACs that no named test covers yet; claims of tests that do not exist are removed.
- 0.2 (03/11/2026): Owner decisions, OQ-1 to OQ-3 resolved. No page shortcuts in the first release (NG-4 stays), the app always opens on the Dashboard (new NG-6 for remembering the last page), and REQ-5 now says the menu is the standard menus (app, File, Edit, View, Window) without a full item list (AC-5.1 names the five menus). Back to `draft` until re-approved.
- 0.1 (03/11/2026): initial draft. The left pane requirements move here from CSD-004 REQ-1, AC-1.1, AC-1.2 and NG-5 (decision D9); the native menu, which CSD-002 REQ-9 and CSD-003 REQ-2 specify item by item, gets one overview (REQ-5 to REQ-7). Describes the behaviour of the app today.
