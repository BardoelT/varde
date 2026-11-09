---
id: CSD-002
title: Project file (save, open, new)
status: approved
owner: Thijs Bardoel
version: 2.1
last_updated: 07/11/2026
related: [CSD-001, CSD-003, CSD-004, CSD-005, CSD-008, CSD-009, CSD-011, CSD-013, CSD-015, CSD-017, CSD-018, CSD-019, CSD-020, CSD-022, CSD-026, CSD-027]
---

# CSD-002: Project file (save, open, new)

## 1. Intent
The app keeps all of the user's data in one local project file instead of a database. Without a clear save model, users can lose edits or overwrite good data with bad. This capability defines how a project is created, opened, saved and protected against accidental loss. It applies to all app data, starting with the employment income of CSD-001, and it is for a single user working on one project at a time.

## 2. Scope
### 2.1 In scope
- New, Open, Save and Save As for the project file, as items of the native macOS File menu.
- Unsaved-changes tracking and the confirmations that protect unsaved data.
- Safe writing, so a failed save never damages an existing file.
- The project file format: its sections, the strict schema and its versioning.
- Saving never overwriting another file without confirmation, and the file extension.
- Reopening the last used project when the app starts.
- A synthetic example project file in the repository that shows every area (REQ-21, REQ-22).

### 2.2 Non-goals
- NG-1: Encryption of the project file. (Remembering the last used file moved into scope in v0.4: REQ-8, REQ-10, REQ-11.)
- NG-2: Autosave and backup or recovery copies made by the app (no `.bak`). The one-off script of REQ-20 writes its own backup copy before it changes a file. The in-memory undo history of CSD-027 is not a backup: it is never written to disk and ends when the project is replaced or the app quits.
- NG-3: Multiple projects open at once.
- NG-4: Cloud sync or sharing of the file.
- NG-5: Buttons for these actions inside the app window, and an Open Recent list.
- NG-6: Remembering more than one file, and opening a file by double-clicking it in Finder.
- NG-7: Migrating project files of an older or newer format inside the app. There is no migration framework or migration code in the app. The one exception is a one-off script outside the app that upgrades a file by hand (REQ-20), because the first changes of the schema after the release (the opening of a debt, CSD-011 REQ-30, and the origin of a settlement, CSD-026 v3.0) make older files with debts invalid.
- NG-8: An "Open example project" item or any other way to reach the example file from inside the app (GitHub issue 30 follow-up). The example is a file in the repository.

## 3. Definitions
| Term | Meaning |
|---|---|
| **Project** | All app data held in memory, plus an optional path to its file. |
| **Project file** | A JSON file with the extension `.varde` holding the project. Its sections and versioning are in REQ-16 to REQ-18. |
| **Gone for good** | The remembered project file does not exist while its parent folder exists and can be read (for example the file was deleted). Any other reason the file cannot be opened is *transient* or a problem of the file itself, and the remembered path is kept (REQ-13). |
| **Unsaved changes** | The in-memory project differs from the last saved or opened state. |

## 4. Requirements

### 4.0 Overview flow
```mermaid
sequenceDiagram
    actor U as User
    participant MENU as macOS File menu
    participant UI as Frontend (ui/)
    participant CMD as commands/
    participant FS as Project file
    U->>MENU: File > Save
    MENU->>UI: menu-action "save"
    UI->>CMD: save_project()
    alt project has no path
        CMD->>U: native save dialog
        U-->>CMD: chosen path
    end
    CMD->>FS: write temporary file, then rename over target
    CMD-->>UI: ok, unsaved indicator cleared
```

**REQ-1: Save** *(event-driven)*
When the user saves, the app shall write all project data to the project file and clear the unsaved indicator, asking for a path in a native save dialog when the project has no path yet or when the user chose Save As.

```gherkin
@AC-1.1
Scenario: Save and reload round-trips
  Given 2 jobs with 5 paychecks in total
  When the user saves to a chosen path and then opens that file in a fresh session
  Then the same 2 jobs and 5 paychecks exist with identical values

@AC-1.2
Scenario: Write failure keeps changes unsaved
  Given unsaved changes
  And the chosen path is not writable
  When the user saves
  Then an error with code "PROJECT_SAVE_FAILED" and no file path or OS detail is shown
  And the unsaved indicator remains

@AC-1.3
Scenario: Path is asked once, then reused
  Given a new project with a job and no path
  When the user saves
  Then a native save dialog is shown and the file is written to the chosen path
  When the user adds a second job and saves again
  Then no dialog is shown and the same file is overwritten

@AC-1.4
Scenario: Save As asks again
  Given a project saved to "example-a.varde"
  When the user chooses Save As and picks "example-b.varde"
  Then "example-b.varde" contains the data
  And later saves go to "example-b.varde"

@AC-1.5
Scenario: Cancelling the dialog saves nothing
  Given a new project with a job and no path
  When the user saves and cancels the native save dialog
  Then no file is written
  And the unsaved indicator remains

@AC-1.6
Scenario: The project is saved with the extension .varde
  Given a new project with a job and no path
  When the user saves it as "example-a"
  Then the file "example-a.varde" is written
```

**REQ-2: Open** *(event-driven)*
When the user opens a project file through a native open dialog, the app shall replace the in-memory project with the file's contents.

```gherkin
@AC-2.1
Scenario: Open valid file
  Given a valid project file with 1 job and 2 paychecks
  When the user opens it
  Then 1 job and 2 paychecks are shown

@AC-2.2
Scenario: Invalid file is rejected
  Given a file that is not a valid project file
  And the app holds 1 job in memory
  When the user opens it
  Then an error with code "PROJECT_INVALID_FILE" is shown
  And the 1 job in memory is unchanged

# @AC-2.3 (removed in v1.0): a file from a newer app version has its own error, see AC-18.1.
```

**REQ-3: Unsaved-changes indicator** *(state-driven)*
While the in-memory project differs from the last saved or opened state, the app shall show an unsaved-changes indicator.

```gherkin
@AC-3.1
Scenario: Indicator follows edits
  Given a freshly opened project
  Then no unsaved indicator is shown
  When the user adds a job
  Then the unsaved indicator is shown
  When the user saves
  Then the unsaved indicator is cleared

@AC-3.2
Scenario: Indicator follows undo and redo (CSD-027)
  Given a freshly opened project
  When the user adds a job and chooses Edit > Undo
  Then no unsaved indicator is shown
  When the user chooses Edit > Redo
  Then the unsaved indicator is shown
```

**REQ-4: Confirm close with unsaved changes** *(event-driven)*
When the user closes the app while the unsaved-changes indicator is shown, the app shall ask for confirmation before closing.

```gherkin
@AC-4.1
Scenario: Close prompts
  Given unsaved changes
  When the user closes the app
  Then a confirmation is asked
  And choosing "cancel" keeps the app open with its data

@AC-4.2
Scenario: Close without changes
  Given no unsaved changes
  When the user closes the app
  Then the app closes without asking
```

**REQ-5: Failed save leaves the file intact** *(unwanted behaviour)*
If writing the project file fails, then the app shall leave any existing file at that path unchanged.

```gherkin
@AC-5.1
Scenario: Interrupted write does not corrupt the file
  Given "example-a.varde" holds 1 job
  And the project in memory has 2 jobs
  When the write fails part-way
  Then "example-a.varde" still holds exactly 1 job
  And the error code "PROJECT_SAVE_FAILED" is shown
```

**REQ-6: New project** *(event-driven)*
When the user starts a new project, the app shall replace the in-memory project with an empty one that has no path.

```gherkin
@AC-6.1
Scenario: New project is empty
  Given a project with 1 job saved to "example-a.varde"
  When the user starts a new project
  Then no jobs exist
  And the next save asks for a path

@AC-6.2
Scenario: Saved file is untouched by New
  Given a project with 1 job saved to "example-a.varde"
  When the user starts a new project and then closes it without saving
  Then "example-a.varde" still holds 1 job
```

**REQ-7: Confirm before replacing unsaved data** *(event-driven)*
When the user starts a new project or opens a file while the unsaved-changes indicator is shown, the app shall ask for confirmation before replacing the in-memory project.

```gherkin
@AC-7.1
Scenario: Open prompts when unsaved
  Given unsaved changes
  When the user chooses Open
  Then a confirmation is asked before the open dialog
  And choosing "cancel" keeps the current data

@AC-7.2
Scenario: New project without unsaved changes
  Given no unsaved changes
  When the user starts a new project
  Then no confirmation is asked
```

**REQ-8: Reopen last project** *(event-driven)*
When the app starts, the app shall open the project file that was last opened or saved, or start with an empty project that has no path if there is none.

```gherkin
@AC-8.1
Scenario: First launch has nothing to reopen
  Given no project file was ever opened or saved
  When the app starts
  Then no jobs exist
  And no path is set

@AC-8.2
Scenario: The last project is reopened
  Given a project with 1 job was saved to "example-a.varde" in an earlier session
  When the app starts
  Then 1 job exists
  And the path is "example-a.varde"
  And no unsaved indicator is shown
```

**REQ-9: File menu** *(ubiquitous)*
The app shall offer New, Open, Save and Save As as items of the native macOS File menu with the shortcuts ⌘N, ⌘O, ⌘S and ⇧⌘S, and shall not offer them as buttons in the app window.

```gherkin
@AC-9.1
Scenario: Menu items and shortcuts exist
  Given the app has started
  Then the File menu contains "New" with ⌘N, "Open…" with ⌘O, "Save" with ⌘S and "Save As…" with ⇧⌘S

@AC-9.2
Scenario: Choosing a menu item runs the action
  Given a new project with a job and no path
  When the user chooses File > Save
  Then a native save dialog is shown
  And after choosing a path the unsaved indicator is cleared

@AC-9.3
Scenario: No buttons in the window
  Given the app has started
  Then the app window has no New, Open, Save or Save As buttons
  And the unsaved-changes indicator is still shown in the window

@AC-9.4
Scenario: Menu actions respect unsaved changes
  Given unsaved changes
  When the user chooses File > New or File > Open…
  Then a confirmation is asked before the project is replaced
```

**REQ-10: Remember the last project** *(event-driven)*
When a project is opened, saved or saved under a new name, the app shall remember its path for the next start, and shall leave the remembered path unchanged when the user starts a new project.

```gherkin
@AC-10.1
Scenario: Save remembers the path
  Given a new project with a job
  When the user saves it to "example-a.varde"
  Then the next start reopens "example-a.varde"

@AC-10.2
Scenario: Open and Save As move the remembered path
  Given "example-a.varde" and "example-b.varde" exist
  When the user opens "example-a.varde" and then saves it as "example-c.varde"
  Then the next start reopens "example-c.varde"

@AC-10.3
Scenario: New project keeps the remembered path
  Given a project saved to "example-a.varde"
  When the user starts a new project and quits without saving
  Then the next start reopens "example-a.varde"

@AC-10.4
Scenario: A failed save does not change the remembered path
  Given a project saved to "example-a.varde"
  When the user chooses a path that is not writable and saving fails
  Then the next start still reopens "example-a.varde"
```

**REQ-11: Remembered file cannot be reopened** *(unwanted behaviour)*
If the remembered project file cannot be opened at start, then the app shall start with an empty project that has no path and show a notice once for that start, without the file path.

```gherkin
@AC-11.1
Scenario Outline: The file cannot be opened
  Given the remembered file "example-a.varde" is <problem>
  When the app starts
  Then no jobs exist and no path is set
  And a notice with code "<code>" is available once
  And the notice text does not contain the file path

  Examples:
    | problem                                              | code                 |
    | deleted, while its folder still exists               | PROJECT_OPEN_FAILED  |
    | on a drive that is not mounted                       | PROJECT_OPEN_FAILED  |
    | in a folder that no longer exists                    | PROJECT_OPEN_FAILED  |
    | not a valid project file                             | PROJECT_INVALID_FILE |
    | from a newer version of the app (format_version 3)   | PROJECT_FILE_NEWER   |

# @AC-11.2 (removed in v1.0): the notice is not repeated once the path is forgotten, see AC-12.2.
```

**REQ-12: Forget a remembered file that is gone for good** *(unwanted behaviour)*
If the remembered project file cannot be opened at start because its parent folder exists and the file does not, then the app shall forget the remembered path.

```gherkin
@AC-12.1
Scenario: A deleted file in an existing folder is forgotten
  Given the remembered file "example-a.varde" was deleted
  And its folder "example-folder" exists
  When the app starts
  Then a notice with code "PROJECT_OPEN_FAILED" is shown once
  And the remembered path is forgotten

@AC-12.2
Scenario: The error is not repeated
  Given the remembered path was forgotten at the last start (AC-12.1)
  When the app starts again
  Then no notice is shown
  And no jobs exist and no path is set
```

**REQ-13: Keep the remembered path in every other case** *(unwanted behaviour)*
If the remembered project file cannot be opened at start for any reason other than the one of REQ-12 (its parent folder is missing or cannot be read, or the file exists but is invalid or from a newer version of the app), then the app shall keep the remembered path.

```gherkin
@AC-13.1
Scenario: An unmounted drive does not make the app forget the project
  Given the remembered file "example-a.varde" is on a drive that is not mounted
  When the app starts
  Then an error notice with code "PROJECT_OPEN_FAILED" is shown once
  And the remembered path is still "example-a.varde"

@AC-13.2
Scenario: The project is back at the next start
  Given the remembered file was on a drive that was not mounted at the last start
  And the drive is mounted again
  When the app starts
  Then the project of "example-a.varde" is opened
  And no notice is shown

@AC-13.3
Scenario Outline: A file that exists but cannot be used keeps its path
  Given the remembered file "example-a.varde" exists and is <problem>
  When the app starts
  Then an error notice with code "<code>" is shown once
  And the remembered path is still "example-a.varde"

  Examples:
    | problem                                            | code                 |
    | not a valid project file                           | PROJECT_INVALID_FILE |
    | from a newer version of the app (format_version 3) | PROJECT_FILE_NEWER   |

@AC-13.4
Scenario Outline: A parent folder that is missing or unreadable keeps the path
  Given the remembered file is "example-folder/example-a.varde"
  And the folder "example-folder" is <state>
  When the app starts
  Then an error notice with code "PROJECT_OPEN_FAILED" is shown once
  And the remembered path is still "example-folder/example-a.varde"

  Examples:
    | state                  |
    | missing                |
    | not readable           |
```

**REQ-14: Saving never overwrites another file silently** *(event-driven)*
When the user chooses a path in the save dialog (first save or Save As) where a file already exists, the app shall ask for confirmation before overwriting it, and shall leave the existing file unchanged when the user declines.

```gherkin
@AC-14.1
Scenario: Save As onto an existing file asks first
  Given a project saved to "example-a.varde"
  And a file "example-b.varde" exists
  When the user chooses Save As and picks "example-b.varde"
  Then a confirmation is asked before "example-b.varde" is overwritten

@AC-14.2
Scenario: Declining keeps the existing file
  Given a file "example-b.varde" holding 1 job
  And a project with 2 jobs
  When the user saves it to "example-b.varde" and declines the confirmation
  Then "example-b.varde" still holds exactly 1 job
  And the unsaved indicator remains

@AC-14.3
Scenario: Saving to the project's own file needs no confirmation
  Given a project saved to "example-a.varde"
  When the user chooses Save
  Then no confirmation is asked
```

**REQ-15: The extension .varde** *(event-driven)*
When the user saves under a name that does not end in `.varde`, the app shall append `.varde` to the name before writing.

```gherkin
@AC-15.1
Scenario Outline: The extension is appended
  Given a new project with a job
  When the user saves it as "<typed>"
  Then the file "<written>" is written

  Examples:
    | typed           | written             |
    | example-a       | example-a.varde     |
    | example-a.varde | example-a.varde     |

@AC-15.2
Scenario: A name that exists only after appending asks first
  Given a file "example-a.varde" exists
  When the user saves a project as "example-a"
  Then a confirmation is asked before "example-a.varde" is overwritten
```

**REQ-16: Sections of the project file** *(ubiquitous)*
The project file shall hold a `format_version` and the sections `income`, `investments`, `bank_accounts`, `debts`, `expenses`, `houses` and `settings`, and every section shall be present in every file the app writes.

```gherkin
@AC-16.1
Scenario: A saved empty project has every section
  Given a new project
  When the user saves it to "example-a.varde"
  Then the file has "format_version" 2
  And the file has the sections "income", "investments", "bank_accounts", "debts", "expenses", "houses" and "settings"
```

**REQ-17: Strict schema** *(unwanted behaviour)*
If a project file lacks a section, or holds a field the app does not know, then the app shall reject the file with the error code `PROJECT_INVALID_FILE` and leave the in-memory project unchanged.

```gherkin
@AC-17.1
Scenario Outline: A file that does not match the schema is rejected
  Given a project file <problem>
  And the app holds 1 job in memory
  When the user opens it
  Then an error with code "PROJECT_INVALID_FILE" is shown
  And the 1 job in memory is unchanged

  Examples:
    | problem                                             |
    | without the section "settings"                      |
    | without the section "houses"                        |
    | with an extra top-level field "notes"               |
    | with an extra field inside the section "income"     |
    | with a debt that has no "opening_date"              |
    | with a debt that has no "opening_amount"            |
```

**REQ-18: Newer file version** *(unwanted behaviour)*
If the `format_version` of a project file is higher than the version the app understands, then the app shall reject the file with the error code `PROJECT_FILE_NEWER`, whose message says that the file was made by a newer version of the app, and leave the in-memory project unchanged.

```gherkin
@AC-18.1
Scenario: File from a newer app version is rejected with its own message
  Given a project file with "format_version" 3 and the app understands version 2
  And the app holds 1 job in memory
  When the user opens it
  Then an error with code "PROJECT_FILE_NEWER" is shown
  And the message says the file is from a newer version of the app
  And the 1 job in memory is unchanged

@AC-18.2
Scenario Outline: A missing or zero version is an invalid file
  Given a project file with "format_version" <version>
  When the user opens it
  Then an error with code "PROJECT_INVALID_FILE" is shown

  Examples:
    | version |
    | 0       |
    | absent  |

@AC-18.3
Scenario: A version 1 file is invalid until the script has upgraded it
  Given a project file with "format_version" 1
  When the user opens it
  Then an error with code "PROJECT_INVALID_FILE" is shown
  And the in-memory project is unchanged
```

**REQ-19: Amounts in the file are strict** *(unwanted behaviour)*
If a project file holds an amount that is not a decimal string with at most two decimals, then the app shall reject the file with the error code `PROJECT_INVALID_FILE` and leave the in-memory project unchanged, without rounding the amount.

```gherkin
@AC-19.1
Scenario Outline: A file amount with more than two decimals is rejected, not rounded
  Given a project file with a bank account entry whose balance is "<balance>"
  And the app holds 1 job in memory
  When the user opens it
  Then an error with code "PROJECT_INVALID_FILE" is shown
  And the 1 job in memory is unchanged

  Examples:
    | balance  |
    | 2500.505 |
    | 0.001    |
    | -1.234   |

@AC-19.2
Scenario: Trailing zeros in a file amount are accepted
  Given a project file with a bank account entry whose balance is "2500.500"
  When the user opens it
  Then the project opens and the entry has balance 2500.50
```

Only text typed in a form is rounded (CSD-000 CON-8), because the app itself only ever writes amounts with two decimals.

**REQ-20: A one-off script upgrades a file with debts to the current format** *(event-driven)*
When the owner runs the script `scripts/migrate-debt-opening.mjs` on a project file with a `format_version` below 2, or in which a debt has no `opening_date` and `opening_amount`, the script shall first write a copy of the file next to it with the extension `.bak` added, and then write, for each debt without an opening, the opening date as the first day of the period of its first statement (`2025-03` gives `2025-03-01`, `2025` gives `2025-01-01`) and the opening amount as the balance of that statement, and for a debt without statements the day the script runs and `0.00`, and shall list the names of the debts without statements. The script shall also give every settlement the `origin` `Sale` when its debt has a house link and `Manual` when it has none, and set `format_version` to 2. The script shall not run when the file has `format_version` 2 and every debt has an opening and every settlement an origin, and shall leave the original file unchanged when anything fails.

```gherkin
@AC-20.1
Scenario: Opening derived from the first statement
  Given a project file with the monthly debt "Example Mortgage" with statements for 2025-03 (balance 190000.00) and 2025-04 (balance 189000.00) and no opening
  When the owner runs the script on it
  Then the file holds for "Example Mortgage" the "opening_date" "2025-03-01" and the "opening_amount" "190000.00"
  And a copy of the original exists beside it with the extension ".bak" added
  And the app opens the upgraded file

@AC-20.2
Scenario: A yearly debt
  Given a project file with the yearly debt "Example Student Loan" with a statement for 2024 (balance 8000.00) and no opening
  When the owner runs the script on it
  Then the debt has the "opening_date" "2024-01-01" and the "opening_amount" "8000.00"

@AC-20.3
Scenario: A debt without statements
  Given a project file with the debt "Example Card" without statements and without an opening
  When the owner runs the script on it on 04/11/2026
  Then the debt has the "opening_date" "2026-11-04" and the "opening_amount" "0.00"
  And the script lists "Example Card" as a debt to correct in the app

@AC-20.6
Scenario: Settlements get an origin
  Given a project file with "format_version" 1 in which the debt "Example Loan" is linked to "Example House" and has a settlement, and the debt "Example Card" has no house link and has a settlement, and every debt has an opening
  When the owner runs the script on it
  Then the settlement of "Example Loan" has the "origin" "Sale" and the settlement of "Example Card" has the "origin" "Manual"
  And the file has "format_version" 2
  And a copy of the original exists beside it with the extension ".bak" added
  And the app opens the upgraded file

@AC-20.4
Scenario: Nothing to do
  Given a project file with "format_version" 2 in which every debt has an opening and every settlement an origin
  When the owner runs the script on it
  Then the script reports that nothing needs upgrading
  And the file and its modification time are unchanged and no copy is made

@AC-20.5
Scenario: A failure changes nothing
  Given a project file that is not valid JSON
  When the owner runs the script on it
  Then the script reports an error
  And the original file is unchanged
```

**REQ-21: The example project file** *(ubiquitous)*
The repository shall contain one synthetic project file, `examples/example.varde`, that the app opens, that holds records in every area, that shows every state the approved CSDs describe for each area, and that has only fake names (each starting with "Example") and dates and periods from 2023-01 to 2026-10, and it shall be the only `*.varde` file tracked by git.

```gherkin
@AC-21.1
Scenario: The example opens and every area has records
  Given the file "examples/example.varde"
  When the app opens it
  Then the project has no unsaved changes
  And the project currency is "EUR"
  And jobs, paychecks, investment accounts and statements, bank accounts and entries, debts and statements, houses and valuations and expenses are listed

@AC-21.2
Scenario: The example shows every state
  Given the file "examples/example.varde"
  Then it holds a monthly and a yearly job, a job with an end date and a job without a title
  And a monthly and a yearly investment account and bank account
  And an expense in each of the 8 categories
  And a house that was sold on 10/06/2026 and a house that is held
  And debts of the categories "Mortgage", "Loan", "Scholarship" and "Other"
  And a debt settled as "Repaid" and a debt settled as "Cancelled" by the sale of their house
  And a debt closed by hand

@AC-21.3
Scenario: The example is synthetic and inside its window
  Given the file "examples/example.varde"
  Then every employer and every name starts with "Example"
  And no date or period lies before 2023-01 or after 2026-10

@AC-21.4
Scenario: Only the example is tracked
  Given the git repository
  When the tracked files with the extension ".varde" are listed
  Then the list is exactly "examples/example.varde"
```

**REQ-22: The example is generated** *(unwanted behaviour)*
If `examples/example.varde` differs from the output of its generator, then the Rust tests shall fail, and a run with the environment variable `EXAMPLE_UPDATE` set shall rewrite the file.

```gherkin
@AC-22.1
Scenario: The committed example is what the generator writes
  Given the generator builds the example project through the commands the UI calls, with fixed ids and a fixed "today" of 15/10/2026
  When the generated text is compared with "examples/example.varde"
  Then they are identical
  And a difference fails the test with the hint "EXAMPLE_UPDATE=1 cargo test example"
```

## 5. Interfaces and contracts

Menu: Rust builds the File menu items `project-new`, `project-open`, `project-save` and `project-save-as`. Choosing one emits the event `menu-action` with the item id to the frontend, which runs the flow of the action, including the confirmations of REQ-7.

Commands: `take_startup_notice()` (returns the REQ-11 notice once, then nothing), `new_project()`, `open_project()`, `save_project()`, `save_project_as()`, `project_status()` (returns whether there are unsaved changes and whether a path is set) and `quit_app()` (quits without asking: the frontend has already confirmed, REQ-4). The save and open commands show the native dialog themselves and take no path from the frontend.

Project file: JSON with the extension `.varde`, a top-level integer `format_version` (currently 2) and amounts as decimal strings with at most two decimals (CSD-000 CON-8, REQ-19). Each capability owns one section, and all sections are required (REQ-16):

| Section | Capability |
|---|---|
| `income` | Employment income (CSD-001) |
| `investments` | Investments (CSD-005) |
| `bank_accounts` | Bank accounts (CSD-009) |
| `debts` | Debts (CSD-011) |
| `expenses` | Expenses (CSD-013) |
| `houses` | Houses (CSD-015, CSD-019) |
| `settings` | Project settings (CSD-003) |

Every period field in a section is called `period` (the pay period of a job, which was `pay_period` and now holds the same Monthly or Yearly value under the name `period`, and the period of an investment account, bank account, debt or house), so the names `pay_period`, `entry_period`, `statement_period` and `valuation_period` do not exist. The preferences of the person (theme, language) are not in the file (CSD-008, CSD-022). A house keeps its purchase as `purchase_date` and `purchase_amount` and its optional sale as `sell_date` and `sell_amount` (CSD-015, CSD-019).

A debt keeps its opening as `opening_date` and `opening_amount`, both required (CSD-011 REQ-30); its closing is the existing `settlement` (CSD-026), which since `format_version` 2 also holds a required `origin`, `Sale` or `Manual`.

Versioning policy: the version is 1 at the first release. The first change of the schema after 1.0 would normally raise `format_version`, so that an older app shows the `PROJECT_FILE_NEWER` error (REQ-18) instead of a generic parse error. Owner decision of 04/11/2026: the opening of a debt (CSD-011 v3.0) does not raise it, and `format_version` stays 1. An older app that opens a file with the new keys therefore shows `PROJECT_INVALID_FILE` (REQ-17), and a version 1 file without openings is invalid in this app until the one-off script of REQ-20 has upgraded it. The next schema change shall raise the version, and the `origin` of a settlement (CSD-026 v3.0) is that change: `format_version` is 2 from v2.0 of this CSD, a version 1 file is invalid in this app until the script of REQ-20 has upgraded it (AC-18.3), and an older app that opens a version 2 file shows `PROJECT_FILE_NEWER`. There is no migration in the app (NG-7) and no backup copy made by the app (NG-2).

The script `scripts/migrate-debt-opening.mjs` has no dependency (Node only), takes the path of the file as its argument, upgrades a file to `format_version` 2 (openings of debts and the `origin` of settlements, REQ-20), prints the names of debts without statements and no amounts, and is tested with a synthetic file ("Example Mortgage", round amounts) by `node:test` in `npm test`.

Error codes (code plus safe message): `PROJECT_SAVE_FAILED`, `PROJECT_INVALID_FILE`, `PROJECT_OPEN_FAILED`, `PROJECT_FILE_NEWER`.

## 6. Constraints
- CON-1: The project file is plain, unencrypted JSON, written by writing a temporary file and renaming it over the target. It is never committed (`*.varde` is git-ignored), with one exception: the synthetic `examples/example.varde` (REQ-21), which `.gitignore` re-includes by its exact path. Rationale: a project file holds real financial data and the repo is public. The atomic write backs REQ-5.
- CON-2: File access uses only the Tauri dialog plugin, opened from Rust, so no dialog permission is granted to the webview. There is no `fs` plugin, and file paths never come from the frontend. Rationale: least privilege per `CLAUDE.md`. Adding the plugin still needs approval at implementation time.
- CON-3: Amounts in the file are decimal strings with at most two decimals (CSD-000 CON-8) and are never rounded when read: a file amount with more than two decimals is invalid (REQ-19), only typed text is rounded. The file carries `format_version`. A newer version than the app knows is rejected (REQ-18). Rationale: no float rounding, and a clear message instead of a parse error.
- CON-4: Errors shown to the user never contain file paths or OS details. Rationale: `CLAUDE.md` error rules.
- CON-5: Tests use a temporary directory and synthetic data only, never real app data or real financial data. Rationale: `CLAUDE.md` testing and privacy rules, and the repo is public.
- CON-6: The menu uses Tauri's built-in menu API: no plugin and no extra permission. It keeps the standard macOS menus (app, Edit, View, Window), because the Edit menu is what makes copy, paste and undo work in text fields. Its predefined Undo and Redo items are replaced by the project Undo and Redo of CSD-027, which leave ⌘Z and ⇧⌘Z to a focused text field. Rationale: least privilege per `CLAUDE.md`, and no loss of standard behaviour.
- CON-7: The remembered path is kept in `last-project.json` in the app's own config folder, outside the project file, written like the project file (atomic, owner-only). It holds the path only, never project data, and is never committed. Rationale: it must be readable before any project is open, and it can reveal a user name through the path. It uses Tauri's built-in path API: no plugin and no permission. The application preferences of CSD-008 live in a second file, `preferences.json`, in the same folder under the same rules.

## 7. Verification
The unsaved flag and the project state live in Rust, so they are proven by `cargo test`, with file round-trips in a temporary directory. Tests are named `test_csd002_ac_<n>_<m>_<description>` (CSD-000 REQ-6).

| What | How |
|---|---|
| Rust tests | `cargo test test_csd002_`. One requirement, e.g. REQ-1: `cargo test test_csd002_ac_1_` |
| AC-9.1 | `cargo test test_csd002_ac_9_` checks the table the menu is built from (ids, labels, shortcuts). Native menus can only be created on the main thread, so the built menu is on the manual checklist |
| DOM tests (`npm test`) | `dialogs.test.js`: AC-3.1, AC-4.1, AC-7.1, AC-7.2, AC-9.3 (indicator, close, Open and New with and without unsaved changes, no buttons in the window); `ac-d-preferences.test.js`: AC-9.2, AC-9.4 (File > Save calls the save and clears the indicator, File > New and Open ask first). AC-4.2 is decided in Rust (`test_csd002_ac_4_2_...`, the close asks only when `has_unsaved_changes`) |
| Script (REQ-20, AC-20.1 to AC-20.5) | `npm test`: `scripts/tests/migrate-debt-opening.test.js` (`node:test`, synthetic fixtures in a temporary directory) |
| Example project (REQ-21, REQ-22) | `cargo test example_project` (`src-tauri/src/example_project_tests.rs`: `test_csd002_ac_21_1_...` to `test_csd002_ac_21_4_...` and `test_csd002_ac_22_1_...`). After a deliberate change of the generator or of the format: `EXAMPLE_UPDATE=1 cargo test example`, then review the diff of `examples/` |
| Manual checklist (native parts only) | The real File menu items and shortcuts (AC-9.1 to AC-9.4), the native save dialog, and a real window close without unsaved changes (AC-4.2: the mock runtime has no event loop to send it) |

Not covered by a named test yet (0 of 54 ACs): none. Every AC has a test that carries its id; the native parts are on the manual list above.

Done means fmt, clippy (`-D warnings`), `cargo test --all-features` and `npm test` pass.

## 8. Open questions
- OQ-1: (resolved) Tests include the CSD number, `test_csd<NNN>_ac_n_m_...`. CSD-000 is now 2.1 and `CLAUDE.md` uses the same naming.
- OQ-2: (removed in v1.0) Obsolete: the SQLite text in `CLAUDE.md` is gone.
- OQ-3: (resolved 03/11/2026) A typed name without `.varde` gets the extension appended, it is not rejected (REQ-15 stays as written).
- OQ-4: (resolved 03/11/2026) A remembered file is gone for good only when its parent folder exists and the file does not (REQ-12). When the parent folder is missing or cannot be read, the error is transient and the path is kept (REQ-13).
- OQ-5: (resolved 03/11/2026) A remembered file that exists but is invalid or from a newer app version keeps its remembered path and shows the error (REQ-13, AC-13.3).
- OQ-6: Should the app give a version 1 file its own message that names the upgrade script (a new error code), instead of the generic `PROJECT_INVALID_FILE` (AC-18.3)? The draft keeps the generic code, with no new code and no change to the notice texts. Owner: Thijs. Blocks: REQ-18, AC-18.3.

## 9. Changelog
- 2.1 (07/11/2026): Undo and redo of project data (CSD-027, GitHub issue 43): NG-2 says the in-memory undo history is not a backup, CON-6 says the Edit menu's Undo and Redo become the project items, new AC-3.2 (the indicator follows undo and redo). Saving is unchanged and does not clear the history. Back to `draft` until re-approved.
- 2.0 (04/11/2026): A settlement holds a required `origin`, `Sale` or `Manual` (CSD-026 v3.0, CSD-020 REQ-33), so the schema changes and `format_version` becomes 2, as the versioning policy requires for the next schema change: new AC-18.3 (a version 1 file is invalid until the script upgraded it), the newer-version examples now use version 3, AC-16.1 expects version 2. The one-off script of REQ-20 also adds the origin (`Sale` for a settlement on a linked debt, `Manual` otherwise) and sets the version (new AC-20.6, AC-20.4 reworded), and `examples/example.varde` is regenerated (REQ-21, REQ-22). Owner decisions from the grill-me interview of 04/11/2026. Still `draft`.
- 1.5 (04/11/2026): New REQ-21 (AC-21.1 to AC-21.4) and REQ-22 (AC-22.1): the repository holds one synthetic example project, `examples/example.varde`, written by a test through the commands the UI calls and compared with the committed file; it is the one `*.varde` file that is tracked (CON-1 amended, `.gitignore` exception). New NG-8: no way to reach the example from inside the app. Owner decisions from the grill-me interview of 04/11/2026 (GitHub issue 30). Back to `draft` until re-approved.
- 1.4 (04/11/2026): A debt in the file must have `opening_date` and `opening_amount` (CSD-011 v3.0, REQ-17 examples); `format_version` stays 1 by owner decision, so older apps show `PROJECT_INVALID_FILE` for such files and the next schema change raises the version. NG-7 and NG-2 reworded: the app has no migration and makes no backup copy, but a one-off script outside the app upgrades a file (new REQ-20, AC-20.1 to AC-20.5), writing its own `.bak`. Owner decisions from the grill-me interview of 04/11/2026 (GitHub issue 51). Back to `draft` until re-approved.
- 1.3 (04/11/2026): New REQ-19 (AC-19.1, AC-19.2): the file stays strict, an amount with more than two decimals makes it invalid (`PROJECT_INVALID_FILE`) and is not rounded, because only typed amounts are rounded (CSD-000 CON-8, v4.0). The behaviour was already built; it is now stated. Back to `draft` until re-approved.
- 1.2 (03/11/2026): Verification (section 7) lists the real test filters and the ACs that no named test covers yet; claims of tests that do not exist are removed. Section 5 lists the command `quit_app`, and `related` adds CSD-004, CSD-017, CSD-018 and CSD-022. Back to `draft` until re-approved.
- 1.1 (03/11/2026): Owner decisions. REQ-11 lists the five failure cases with their codes (AC-11.1; the newer version case is new) and the notice is shown once per start. REQ-12 is now exact: the path is forgotten only when the parent folder exists and the file does not (AC-12.1 is the deleted-file scenario, the old AC-12.1 became AC-12.2). REQ-13 keeps the path in every other case, including a file that exists but is invalid or from a newer version (behaviour change: it used to be forgotten; new AC-13.3, AC-13.4). Definition "Gone for good" added. Section 5 names the `period` field of a job and the `sell_date` and `sell_amount` house fields. `PROJECT_FILE_NEWER` confirmed. OQ-3 to OQ-5 resolved. Back to `draft` until re-approved.
- 1.0 (03/11/2026): First release, strict schema. The project file extension is `.varde` (was `.wealth`). New REQ-16 to REQ-18: the sections of the file are listed and all required (`income`, `investments`, `bank_accounts`, `debts`, `expenses`, `houses`, `settings`), unknown fields and missing sections are rejected, a newer `format_version` has its own error `PROJECT_FILE_NEWER` (AC-2.3 removed, see AC-18.1); `format_version` stays 1, with the policy that the first schema change after 1.0 raises it; no migration (NG-7), no `.bak`. The `period` field replaces `pay_period`, `entry_period`, `statement_period` and `valuation_period`. New REQ-14 and REQ-15: saving never overwrites another file without confirmation, and `.varde` is appended. REQ-11 split: the remembered path is forgotten only when the file is gone for good (REQ-12), and kept on a transient error such as an unmounted drive (REQ-13); AC-11.2 moved to AC-12.1. OQ-2 removed as obsolete, OQ-3 to OQ-5 added. Display and dialog ACs are covered by DOM tests. Back to `draft` until re-approved.
- 0.14 (30/10/2026): The `assets` section is renamed `houses`; the reader also accepts `assets` and the app writes `houses` (CSD-015 REQ-24, AC-24.6). Debts may carry a link to a house and a settlement (CSD-020). `format_version` stays 1. Back to `draft` until re-approved (GitHub issue 31).
- 0.13 (29/10/2026): The project file gets a `houses` section (CSD-015). `format_version` stays 1: a file without it opens with no houses, as the app is not released yet. Back to `draft` until the owner approves.
- 0.12 (29/10/2026): Wording only: the `expenses` section belongs to the expenses (CSD-013), no longer called "major expenses". No behaviour change.
- 0.11 (29/10/2026): The project file gets an `expenses` section (CSD-013). `format_version` stays 1: a file without it opens with no expenses, as the app is not released yet. Back to `draft` until the owner approves.
- 0.10 (28/10/2026): The project file gets a `debts` section (CSD-011). `format_version` stays 1: a file without it opens with no debts, as the app is not released yet. Back to `draft` until the owner approves.
- 0.9 (28/10/2026): The project file gets a `bank_accounts` section (CSD-009). `format_version` stays 1: a file without it opens with no bank accounts, as the app is not released yet. Back to `draft` until the owner approves.
- 0.8 (28/10/2026): CON-7 notes that the application preferences of CSD-008 live next to `last-project.json`. No behaviour change.
- 0.7 (26/10/2026): The project file gets an `investments` section (CSD-005). `format_version` stays 1: a file without it opens with no investment accounts, as the app is not released yet.
- 0.6 (25/10/2026): The project file gets a `settings` section (CSD-003). `format_version` stays 1: a file without it opens with the defaults, as the app is not released yet.
- 0.5 (25/10/2026): Dates are written dd/mm/yyyy (CSD-000 2.2, CON-7). No behaviour change: the project file keeps ISO 8601 dates internally.
- 0.4 (25/10/2026): The app reopens the last used project at start. REQ-8 changes from "Start empty" to "Reopen last project" (AC-8.1 reworded, AC-8.2 added), new REQ-10 and REQ-11, NG-1 narrowed, NG-6 and CON-7 added, command `take_startup_notice`.
- 0.3 (25/10/2026): Project actions move from four in-window buttons to the native macOS File menu (REQ-9, NG-5, CON-6, menu event in section 5). Behaviour of REQ-1 to REQ-8 is unchanged. CON-2 reworded: dialogs open from Rust, so no webview permission is needed.
- 0.2 (25/10/2026): Resolved OQ-1. Verification uses the `test_csd002_` filter.
- 0.1 (25/10/2026): initial draft, split out of CSD-001 v0.3 (its REQ-14 to REQ-21 and CON-7 to CON-9).
