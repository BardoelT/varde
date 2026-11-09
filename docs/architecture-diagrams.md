# Architecture diagrams

Visual companion to [`architecture.md`](architecture.md). The text there explains *why*; these diagrams show *how the parts connect*. All diagrams are Mermaid, so GitHub renders them.

1. [System context](#1-system-context)
2. [Layers and the rules between them](#2-layers-and-the-rules-between-them)
3. [Backend modules](#3-backend-modules)
4. [Domain dependencies](#4-domain-dependencies) (two views)
5. [The series core](#5-the-series-core)
6. [Frontend structure](#6-frontend-structure)
7. [A change, end to end (with undo)](#7-a-change-end-to-end-with-undo)
8. [Opening and saving a project](#8-opening-and-saving-a-project)
9. [App start](#9-app-start)
10. [Native menu and close confirmation](#10-native-menu-and-close-confirmation)
11. [Insights charts](#11-insights-charts)
12. [Project file and data on disk](#12-project-file-and-data-on-disk)
13. [Sale of a house with linked debts](#13-sale-of-a-house-with-linked-debts)
14. [Test layers](#14-test-layers)

## 1. System context

One desktop process, no network, no server. Data lives in two places on the user's disk: the project file they choose, and the app config folder.

```mermaid
flowchart LR
    user(["User"])

    subgraph app["Varde.app (one macOS process)"]
        direction LR
        wv["Webview<br/>ui/: HTML + CSS + JS"]
        rs["Rust backend<br/>src-tauri/"]
        wv <-- "IPC: invoke / events" --> rs
    end

    proj[("Project file<br/>*.varde (JSON)<br/>user-chosen location")]
    cfg[("App config folder<br/>preferences.json<br/>last project path")]
    menu["Native macOS menu bar"]
    dlg["Native file dialogs<br/>(tauri-plugin-dialog)"]

    user --> wv
    user --> menu
    menu -- "menu-action event" --> wv
    rs -- "atomic write / read" --> proj
    rs -- "read / write" --> cfg
    rs --> dlg

    net(["Network"]):::none
    rs -. "no calls" .-> net
    classDef none stroke-dasharray: 4 4,color:#888,stroke:#888
```

## 2. Layers and the rules between them

Dependencies point one way. `domain/` knows nothing about Tauri or IO, and the frontend never computes a financial figure.

```mermaid
flowchart TB
    subgraph FE["Frontend (ui/)"]
        pages["pages/<br/>one file per page or view"]
        shared["shared/<br/>ipc, dom, crud, charts, i18n ..."]
        pages --> shared
    end

    subgraph BE["Backend (src-tauri/src/)"]
        cmds["commands/<br/>parse + bound input, call domain,<br/>map errors"]
        dom["domain/<br/>pure rules and figures<br/>(no Tauri, no IO)"]
        sto["storage/<br/>project file, preferences,<br/>last project, undo history"]
        err["error.rs / domain/error.rs<br/>AppError, DomainError (stable codes)"]
        cmds --> dom
        cmds --> sto
        sto --> dom
        cmds --> err
        dom --> err
    end

    shared == "ipc.js: the only caller of Tauri" ==> cmds
    cmds == "decimal strings, English messages,<br/>error codes" ==> shared

    rule1["Rule: frontend displays only,<br/>Rust computes every figure"]:::note
    rule2["Rule: domain is pure and unit-tested"]:::note
    FE ~~~ rule1
    dom ~~~ rule2
    classDef note fill:none,stroke-dasharray: 4 4
```

## 3. Backend modules

`AppState` holds the one open `Project` behind a mutex. Every command reaches it through `state.project()` (read) or `state.edit(label)` (an undoable change).

```mermaid
flowchart LR
    lib["lib.rs<br/>configure(): manage AppState,<br/>register commands, window events"]
    menu["menu.rs<br/>File and Edit menu items"]

    subgraph commands["commands/ (#[tauri::command])"]
        direction TB
        c_income["income"]
        c_inv["investments"]
        c_bank["bank_accounts"]
        c_debts["debts"]
        c_houses["houses"]
        c_exp["expenses"]
        c_derived["net_worth · ratios · dashboard"]
        c_cfg["settings · preferences"]
        c_proj["project<br/>new/open/save/undo/redo/quit"]
    end

    state["AppState<br/>Mutex&lt;Project&gt; + UTC offset"]

    subgraph storage["storage/"]
        direction TB
        project["project/<br/>Project: data, saved copy, path"]
        history["project/history.rs<br/>History: up to 100 steps"]
        file["file.rs<br/>JSON read/write, atomic,<br/>format_version 2, 50 MB cap"]
        lastp["last_project.rs"]
        prefs["preferences.rs"]
    end

    domain["domain/<br/>ProjectData and the areas"]

    lib --> commands
    lib --> menu
    lib --> state
    commands --> state
    state --> project
    project --> history
    project --> file
    project --> lastp
    project --> domain
    c_cfg --> prefs
    commands --> domain
```

## 4. Domain dependencies

Two views of `domain/`, because one diagram with every edge is unreadable. Arrows mean "uses".

**4a. What the areas build on.** Each area uses the shared building blocks; the four series areas also share the series core. Edges go from a whole group, not from each area.

```mermaid
flowchart LR
    subgraph areas["Areas (hold the data)"]
        direction TB
        subgraph seriesAreas["Series areas"]
            direction TB
            bank["bank_accounts"]
            inv["investments"]
            debts["debts"]
            houses["houses"]
        end
        subgraph ownShape["Own shape"]
            direction TB
            income["income"]
            expenses["expenses"]
        end
    end

    series["series<br/>traits + rules written once:<br/>add / move / delete record,<br/>file checks, closings, insights years"]

    subgraph blocks["Shared building blocks"]
        direction TB
        common["common<br/>Id, Period, YearMonth,<br/>Note, ColourSlot, series_years"]
        money["money<br/>Money: 2 decimals, half-up"]
        charts["charts<br/>Axis, Piece, Marker,<br/>stacking, labels"]
    end

    seriesAreas --> series
    series --> common
    series --> money
    ownShape --> common
    ownShape --> money
    areas --> charts

    houses <-->|"linking: a sale settles<br/>linked debts together"| debts
```

**4b. Where the figures come from.** Net worth, ratios and the dashboard keep no data. They combine the figures the areas already give.

```mermaid
flowchart LR
    subgraph areas["Areas"]
        direction TB
        bank["bank_accounts"]
        inv["investments"]
        debts["debts"]
        houses["houses"]
        income["income"]
        expenses["expenses"]
    end

    subgraph derived["Derived, no data of their own"]
        direction TB
        nw["net_worth<br/>yearly closing figures<br/>of the four series areas"]
        ratios["ratios<br/>income, wealth, growth,<br/>expenses (all six areas)"]
        dash["dashboard<br/>income overview"]
    end

    bank & inv & debts & houses -->|"yearly closings"| nw
    areas -->|"yearly figures"| ratios
    income -->|"overview"| dash
```

All areas and the project settings are collected in `ProjectData` (`domain/project_data.rs`), which is what the project file stores.

## 5. The series core

One set of rules written once on traits. An area supplies its own types and hooks and calls one-line wrappers.

```mermaid
classDiagram
    direction LR

    class SeriesData {
        <<trait>>
        accounts() / records()
        SeriesError → DomainError mapping
        optional hooks
    }
    class SeriesAccount {
        <<trait>>
        name, period, colour slot
    }
    class SeriesRecord {
        <<trait>>
        account id, period, amount, note
    }
    class store {
        <<store.rs>>
        add_record / update_record / delete_record
        all_records / remove_account
        latest
        yearly_closings
        year_figures (insights years + stacked pieces)
        file checks
    }
    class helpers {
        <<mod.rs>>
        AccountInput
        colour slot choice
        case-insensitive name check
        parse_period / record_note
    }

    class BankAccounts
    class Investments {
        hook: net deposits, growth
    }
    class Debts {
        hook: settlement, house link
    }
    class Houses {
        hook: holding window
    }

    store ..> SeriesData : generic over
    SeriesData --> SeriesAccount
    SeriesData --> SeriesRecord
    BankAccounts ..|> SeriesData
    Investments ..|> SeriesData
    Debts ..|> SeriesData
    Houses ..|> SeriesData
    store ..> helpers
```

## 6. Frontend structure

`main.js` wires every page once at start and reloads them whenever the project changes. Pages with two tabs have an overview file and an insights file.

```mermaid
flowchart TB
    html["index.html<br/>all pages and dialogs"]
    main["main.js<br/>init each page, reloadAll(),<br/>menu-action + close-requested listeners"]
    nav["navigation.js<br/>left pane"]

    subgraph pages["pages/"]
        direction LR
        ov["*-overview.js<br/>income · investments · bank<br/>debts · houses · expenses"]
        ins["*-insights.js<br/>same six areas"]
        single["dashboard · net-worth · ratios<br/>settings · project"]
    end

    subgraph shared["shared/"]
        direction LR
        ipc["ipc.js<br/>invoke, listen, errorText"]
        dom["dom.js<br/>$, h, toast, confirm,<br/>submitOnce"]
        crud["crud.js<br/>openDialog, saveDialog,<br/>deleteWithConfirm"]
        rows["list-rows.js · record-form.js<br/>statement-filter.js · field-errors.js"]
        ip["insights-page.js<br/>createSeriesInsights"]
        yc["yearly-chart.js<br/>createYearlyChart(strategy)"]
        ch["chart.js<br/>SVG helpers, toPosition"]
        fmt["format.js · date-mask.js · tabs.js"]
        i18n["i18n.js<br/>t, tn, errorMessage"]
        settle["settle.js<br/>askSettlements"]
    end

    loc["locales/nl*.js<br/>Dutch texts, English text is the key"]
    css["styles/<br/>base → layout → components → pages → charts"]
    rust[["Rust commands"]]

    html --> main
    html --> nav
    main --> pages
    pages --> shared
    ov --> crud & rows & settle
    ins --> ip & yc
    ip --> yc
    yc --> ch
    i18n --> loc
    ipc ==> rust
    html -.-> css
```

## 7. A change, end to end (with undo)

Every command that changes project data goes through `AppState::edit(label)`. The guard snapshots the data first; when it drops after a change that altered something, the snapshot becomes one undo step. A rejected change records nothing.

```mermaid
sequenceDiagram
    autonumber
    actor U as User
    participant P as pages/*-overview.js
    participant C as shared/crud.js
    participant I as ipc.js
    participant H as commands/*
    participant S as AppState.edit(label)
    participant D as domain/*
    participant M as main.js

    U->>P: fill dialog, press Save
    P->>C: saveDialog(save, reload)
    C->>I: invoke("add_bank_entry", args)
    I->>H: Tauri IPC
    H->>H: parse and bound input
    H->>S: lock project, snapshot "before"
    H->>D: apply rule (validate, round, store)
    alt rule rejects
        D-->>H: DomainError (stable code)
        H-->>I: AppError {code, safe message}
        I-->>C: rejection
        C-->>U: error in dialog's error box
        Note over S: guard drops, data unchanged,<br/>no undo step recorded
    else ok
        D-->>H: updated record
        Note over S: guard drops, data changed,<br/>record_step(label, before)
        H-->>I: serialised response
        I-->>C: resolved
        C->>C: close dialog
        C->>P: reload()
        P->>I: invoke("list_*"/"yearly_*")
        P-->>U: table and totals redrawn
        P->>M: onChanged()
        M->>I: project_status, settings, dashboard
    end
```

Undo and redo use the same `History` in `storage/project/history.rs`:

```mermaid
stateDiagram-v2
    [*] --> Clean: new / open
    Clean --> Dirty: confirmed change<br/>(push step, clear redo)
    Dirty --> Dirty: further change<br/>(max 100 steps)
    Dirty --> Dirty: undo / redo<br/>(swap data with step)
    Dirty --> Clean: save, or undo back<br/>to the saved data
    Clean --> Dirty: undo / redo
    Dirty --> [*]: quit (asks to confirm)
    Clean --> [*]: quit
    note right of Dirty
        History is memory only:
        never written to disk or logged,
        dropped on new / open.
        The currency setting is not a step.
    end note
```

## 8. Opening and saving a project

Native dialogs run without the project lock held, so closing the window never deadlocks.

```mermaid
sequenceDiagram
    autonumber
    actor U as User
    participant F as pages/project.js
    participant C as commands/project.rs
    participant P as storage Project
    participant D as Native dialog
    participant FS as file.rs / disk
    participant L as LastProject

    rect rgb(240,240,255)
    Note over U,L: Open
    U->>F: File > Open (menu-action)
    F->>C: open_project
    C->>D: pick *.varde (lock not held)
    D-->>C: path or cancelled
    C->>P: open(path)
    P->>FS: read (≤ 50 MB), deny_unknown_fields,<br/>format_version check, validate sections
    alt invalid or newer file
        FS-->>C: StorageError (no path in message)
        C-->>F: AppError, current project untouched
    else ok
        FS-->>P: ProjectData
        P->>P: data = saved = file, history cleared
        P->>L: remember path
        C-->>F: Opened
        F->>F: reloadAll()
    end
    end

    rect rgb(240,255,240)
    Note over U,L: Save
    U->>F: File > Save
    F->>C: save_project
    alt has no path yet
        C->>D: choose path, add .varde, confirm overwrite
    end
    C->>P: save()
    P->>FS: write temp file, then atomic rename
    P->>P: saved = data (unsaved_changes = false)
    P->>L: remember path
    C-->>F: Saved
    end
```

## 9. App start

```mermaid
flowchart TD
    A["run()"] --> B["read local UTC offset<br/>(before any thread starts)"]
    B --> C["Builder: dialog plugin, configure(),<br/>menu::build"]
    C --> D["setup()"]
    D --> E["app_config_dir"]
    E --> F["Project.restore(LastProject)<br/>reopen last project, else empty<br/>(failure becomes a startup notice)"]
    E --> G["Preferences.load_theme()<br/>set native window theme"]
    G --> H["manage(Preferences)"]
    F --> W["main window loads ui/"]
    H --> W
    W --> J["main.js start()"]
    J --> K["collectStaticTexts()"]
    K --> L["loadPreferences()<br/>theme + language,<br/>then add .theme-ready"]
    L --> M["initDateMasks, init every page,<br/>register listeners"]
    M --> N["take_startup_notice → toast if any"]
    N --> O["reloadAll() pages, in parallel,<br/>one failure never stops another"]
```

## 10. Native menu and close confirmation

Menu items and quit requests are forwarded to the frontend, which owns the confirmation dialogs.

```mermaid
sequenceDiagram
    autonumber
    actor U as User
    participant NM as menu.rs (native menu)
    participant L as lib.rs
    participant F as main.js / project.js
    participant R as commands/project.rs

    U->>NM: Cmd+S / Cmd+O / Cmd+N / Cmd+Z / Cmd+, ...
    NM->>L: on_menu_event(id)
    L->>L: is_forwarded_item(id)?
    L-->>F: emit "menu-action" (id)
    F->>R: save_project / open_project / undo / ...
    R-->>F: result (+ new undo/redo labels)
    F->>NM: Edit items relabelled, e.g. "Undo Delete Bank Account"

    U->>L: close window or Cmd+Q
    L->>L: has_unsaved_changes()? (poisoned lock counts as yes)
    alt unsaved changes
        L->>L: prevent_close / prevent_exit
        L-->>F: emit "close-requested"
        F-->>U: confirm dialog
        U->>F: Quit anyway
        F->>R: quit_app (explicit exit code)
    else clean
        L->>L: close
    end
```

## 11. Insights charts

Rust decides everything a chart shows. The frontend only places received values.

```mermaid
flowchart LR
    subgraph rust["Rust"]
        a["domain/<area>/insights.rs<br/>per year figures"]
        s["domain/series year_figures<br/>(or area's own)"]
        c["domain/charts<br/>stack_from_zero / waterfall,<br/>Axis ticks, Marker, labels"]
        cmd["get_*_insights<br/>{ years, axis, pieces, marker,<br/>growth_label? }"]
        a --> s --> c --> cmd
    end

    subgraph js["Frontend"]
        pg["pages/*-insights.js<br/>strategy: pieceStyle, tooltip rows,<br/>extra marks, labels under axis"]
        yc["createYearlyChart(strategy)<br/>gridline + label per tick,<br/>bar from low..high per piece,<br/>marker line + label"]
        tp["toPosition()<br/>the only decimal-to-number step,<br/>for a y position only"]
        lg["legend, tooltip frame<br/>shared data-legend key<br/>highlights a series in every chart"]
        pg --> yc --> tp
        pg --> lg
    end

    cmd == "decimal strings" ==> pg
```

## 12. Project file and data on disk

```mermaid
erDiagram
    PROJECT_FILE ||--|| SETTINGS : has
    PROJECT_FILE ||--|| INCOME : has
    PROJECT_FILE ||--|| INVESTMENTS : has
    PROJECT_FILE ||--|| BANK_ACCOUNTS : has
    PROJECT_FILE ||--|| DEBTS : has
    PROJECT_FILE ||--|| HOUSES : has
    PROJECT_FILE ||--|| EXPENSES : has

    PROJECT_FILE {
        int format_version "2"
    }
    SETTINGS {
        string currency "one per project"
    }

    INCOME ||--o{ JOB : lists
    JOB ||--o{ ENTRY : "paycheck or annual statement"

    INVESTMENTS ||--o{ INVESTMENT_ACCOUNT : lists
    INVESTMENT_ACCOUNT ||--o{ INVESTMENT_STATEMENT : "one per period"

    BANK_ACCOUNTS ||--o{ BANK_ACCOUNT : lists
    BANK_ACCOUNT ||--o{ BANK_ENTRY : "signed balance per period"

    DEBTS ||--o{ DEBT : lists
    DEBT ||--o{ DEBT_STATEMENT : "balance per period"
    DEBT }o--o| HOUSE : "optional link"

    HOUSES ||--o{ HOUSE : lists
    HOUSE ||--o{ HOUSE_VALUATION : "inside holding window"

    EXPENSES ||--o{ EXPENSE : "dated, category, note"
```

Outside the project file, in the app config folder (apart from any project):

| File | Holds |
|---|---|
| `preferences.json` | theme (System, Light, Dark) and language (English, Dutch) |
| last project path | which `.varde` to reopen at start |

## 13. Sale of a house with linked debts

`domain/houses/linking` is the only way to change a sale, so a house and its debts change together or not at all (CSD-019, CSD-020, CSD-026).

```mermaid
sequenceDiagram
    autonumber
    actor U as User
    participant H as houses-overview.js
    participant S as settle.js
    participant C as commands/houses.rs
    participant L as domain/houses/linking
    participant D as domain/debts

    U->>H: record sale of a house
    H->>C: list_open_debts(house)
    C-->>H: open debts linked to it
    alt there are open linked debts
        H->>S: askSettlements()
        S-->>U: each debt: Repaid or Cancelled?
        U-->>S: choices
        S-->>H: settlements
    end
    H->>C: update_house(sale, settlements)
    C->>L: apply sale
    L->>D: Debts::settle(date + period from sold house)
    D-->>L: settlement + settlement statement (always together)
    L->>L: update house sale
    alt any step fails
        L-->>C: error, nothing changed
    else all ok
        L-->>C: house + debts updated (one undo step)
    end
    Note over L,D: Undo of the sale calls unsettle,<br/>and editing the sale re-settles
```

## 14. Test layers

```mermaid
flowchart TB
    csd["docs/csd/CSD-nnn<br/>REQ-n (EARS) + AC-n.m (Gherkin)"]

    subgraph rust["cargo test"]
        unit["domain unit tests<br/>tests.rs / ac_tests*.rs<br/>test_csdNNN_ac_n_m_..."]
        stor["storage tests<br/>temp directories only"]
        ipc["ipc_*_tests.rs<br/>commands on the Tauri mock runtime"]
        gold["golden_tests.rs<br/>every read command + project file<br/>vs src-tauri/golden/"]
        ex["example_project_tests.rs<br/>writes examples/example.varde"]
    end

    subgraph js["npm test (node:test + jsdom)"]
        harness["ui/tests/harness.js<br/>fake window.__TAURI__ invoke"]
        dom["DOM tests<br/>display ACs, charts, language"]
        fx["fixtures.js<br/>canned answers = golden files"]
    end

    csd --> unit & stor & ipc & dom
    gold -- "read answers" --> fx
    fx --> harness --> dom
    ipc --> gold
    gold --> ex
```
