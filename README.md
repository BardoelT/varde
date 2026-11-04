# Varde

A private, local-only desktop app for tracking personal wealth: what comes in, what goes out, what you own and what you owe. Each area has a table view and an **Insights** tab with a yearly chart.

## What it does today

The left pane groups the pages as **Overview**, **Finance**, **Wealth**, **Liabilities** and **Insights** (the Net worth and Ratios pages), with Settings and the project status at the bottom.

- **Dashboard:** your income this year to date and last month (total gross, gross salary, holiday pay and bonus), summed over all jobs. An annual statement counts in full in this year and not in last month.
- **Income:** any number of jobs. A monthly job has one paycheck per month, a yearly job has one annual statement per year, both with gross salary, holiday pay and bonus. Yearly totals and a list of all paychecks and annual statements sit next to the jobs. Insights charts income per year with the year-over-year growth.
- **Expenses:** dated expenses with a name, amount, category (a fixed list) and a note, with yearly totals and the largest expense of each year. Insights charts the spending per year, stacked by category.
- **Bank accounts:** any number of accounts, each recorded monthly or yearly as a closing balance (negative when overdrawn) and a note. Yearly totals and the total of the latest balances. Insights charts the balance per bank per year with a closing-balance marker.
- **Investments:** any number of investment accounts, each recorded monthly or yearly. A statement holds the closing balance, the net deposits (money put in minus money taken out) and a note; the capital growth is derived from them, and the form calculates the balance or the growth from the other. Yearly totals show closing balance, net deposits and capital growth. Insights charts opening balance, net deposits and capital growth with a closing-balance marker and the growth percentage. Individual stocks, prices and trades are not tracked.
- **Houses:** the houses you own, each bought once and possibly sold, valued monthly or yearly, with the change since the previous valuation and yearly values. A sold house stops counting from its sale year. A debt has a category (Mortgage, Loan, Scholarship or Other), and a Mortgage or Loan can be linked to a house in the debt form; when the house is sold, each open debt is marked Repaid or Cancelled and drops to 0.00. A Repaid debt counts in full in the wealth accumulation from cash, the settled amount of a Cancelled one is taken out of that year's repaid amount. Insights charts the value per house per year.
- **Debts:** what you owe, each recorded monthly or yearly as a remaining balance, with the amount repaid per year. Insights charts the debt per debt per year.
- **Net worth:** one chart of your overall financial state: a stacked bar per year with houses, investments and cash above the zero line and debts below it, and a marker at the net worth. Every figure is the one the chart of its own area shows.
- **Ratios:** one table with the years as columns: total income, how much of it became wealth (bank increase, investment net deposits and debt repaid), the growth of the investments and of the total wealth, and the disposable income with its share of the income.
- **Settings:** the currency (EUR, USD or GBP) of the whole project, the theme (System, Light or Dark) and the language (English or Dutch). Changing the currency does not convert any amount.
- **Project file:** everything is saved in one file you choose, and the app reopens the last one you used.

All insights charts look and behave alike: hovering a legend item highlights that series in every year, and the tooltip lists its rows in the same order as the bar.


## Principles

- **Local only.** All data lives in one project file you choose. There is no cloud sync, no backend server, no telemetry and no network access.
- **macOS only** (Apple Silicon and Intel).
- **Exact money math.** Amounts use `rust_decimal`, never floats, and are in the one currency of the project.
- **Safe saving.** Files are written to a temporary file and renamed over the target, so a failed save never damages an existing project.

## Tech stack

- Rust + [Tauri v2](https://tauri.app/) backend in `src-tauri/`
- Vanilla HTML, CSS and JS frontend in `ui/` (no framework, no build step)

The frontend only displays what Rust returns. It never computes financial figures. How the code is organised, and why, is in [docs/architecture.md](docs/architecture.md).

## Installing a release

Download the `.dmg` of the latest version from the [Releases](../../releases) page, open it and drag Varde to Applications. It works on Apple Silicon and Intel Macs.

The app is not signed or notarized yet, so macOS blocks it the first time you open it. To allow it:

1. Try to open Varde once, then close the warning.
2. Open **System Settings > Privacy & Security**, scroll down to the message about Varde and choose **Open Anyway**.
3. Confirm with your password or Touch ID. macOS remembers the choice.

On macOS 14 and older you can instead right-click the app and choose **Open**.

## Getting started

Prerequisites: [rustup](https://rustup.rs/), which installs the Rust toolchain pinned in `rust-toolchain.toml` by itself, and the Tauri CLI.

```bash
cargo install tauri-cli --version "^2"
```

Run the app in development:

```bash
cargo tauri dev
```

Build a release bundle (`.app` / `.dmg`):

```bash
cargo tauri build
```

The app has no frontend build step and no runtime npm dependency. `ui/` is served as-is. Node and npm are only needed to run the frontend tests (see Development).

## Using the app

- **Project actions** live in the native macOS **File** menu: New (⌘N), Open… (⌘O), Save (⌘S) and Save As… (⇧⌘S). An indicator at the bottom of the left pane shows unsaved changes, and the app asks before you quit or replace a project with unsaved edits. On start, the app reopens the last project you used.
- **Settings** open from the Settings item at the bottom of the left pane, or from the app menu with ⌘,.
- **Dates** are shown and typed as `dd/mm/yyyy`, and months as `YYYY-MM`.

> **Project files hold real financial data** and are plain, unencrypted JSON. `*.varde` is git-ignored. Never commit one.

## Development

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo deny check
GOLDEN_UPDATE=1 cargo test golden        # only for a deliberate change of behaviour: rewrites src-tauri/golden/
npm ci                                   # once: installs jsdom, the tests-only dev dependency
npm test                                 # DOM tests of ui/ (Node 26)
```

Format, clippy and the Rust tests must pass before a change is done, and `npm test` too when `ui/` changed. The release run on `macos-latest` repeats all of them.

### Layout

```
ui/                 frontend (HTML, CSS, JS; no build step)
  index.html
  main.js           entry point: wires the pages and reloads them
  navigation.js     left-pane navigation
  pages/            one file per page or view (income-overview, income-insights, ..., dashboard, settings)
  shared/           helpers pages share (ipc, dom, crud, list-rows, record-form, insights-page, format, i18n, settle, date-mask, chart, yearly-chart, tabs)
  locales/          Dutch texts (the English text in the code is the key)
  styles/           CSS by area (base, layout, components, pages, charts)
  tests/            DOM tests (node:test + jsdom) and their harness; not part of the app
src-tauri/
  src/lib.rs        app setup, command registration
  src/menu.rs       native macOS menu bar (File menu, Settings…)
  src/commands/     thin #[tauri::command] handlers
  src/domain/       pure business logic (income, expenses, bank accounts, investments, houses, debts, net worth, ratios, the shared series core, charts and money, preferences, settings), unit-tested
  src/storage/      project file persistence (JSON, atomic write), last project, preferences
  src/error.rs      app error type
  capabilities/     Tauri permission files
  golden/           golden files of the golden tests
docs/
  architecture.md   how the code is organised and the conventions it follows
  csd/              capability specifications
```

### Specifications

Behaviour is defined in Capability Specification Documents in [`docs/csd/`](docs/csd/). Start with [CSD-000](docs/csd/CSD-000-capability-specifications.md), which describes the format (EARS requirements, Gherkin acceptance criteria).

| CSD | Capability |
|---|---|
| [CSD-001](docs/csd/CSD-001-employment-income.md) | Income tracking (employment income) |
| [CSD-002](docs/csd/CSD-002-project-file.md) | Project file (save, open, new, reopen last project) |
| [CSD-003](docs/csd/CSD-003-project-settings.md) | Project settings (currency) |
| [CSD-004](docs/csd/CSD-004-income-dashboard.md) | Income dashboard |
| [CSD-005](docs/csd/CSD-005-investments.md) | Investments (accounts, statements, net deposits, capital growth, yearly totals) |
| [CSD-006](docs/csd/CSD-006-income-insights.md) | Income insights chart |
| [CSD-007](docs/csd/CSD-007-investment-insights.md) | Investment insights chart |
| [CSD-008](docs/csd/CSD-008-application-preferences.md) | Application preferences (theme) |
| [CSD-009](docs/csd/CSD-009-bank-accounts.md) | Bank accounts |
| [CSD-010](docs/csd/CSD-010-bank-account-insights.md) | Bank account insights chart |
| [CSD-011](docs/csd/CSD-011-debts.md) | Debts |
| [CSD-012](docs/csd/CSD-012-debt-insights.md) | Debt insights chart |
| [CSD-013](docs/csd/CSD-013-expenses.md) | Expenses |
| [CSD-014](docs/csd/CSD-014-expense-insights.md) | Expense insights chart |
| [CSD-015](docs/csd/CSD-015-houses.md) | Houses |
| [CSD-016](docs/csd/CSD-016-house-insights.md) | House insights chart |
| [CSD-017](docs/csd/CSD-017-net-worth-insights.md) | Net worth insights (the Net worth page) |
| [CSD-018](docs/csd/CSD-018-ratios.md) | Ratios (the Ratios page) |
| [CSD-020](docs/csd/CSD-020-debt-linked-to-house.md) | House purchase, sale and linked debts |
| [CSD-021](docs/csd/CSD-021-ci-and-release-pipeline.md) | CI and release pipeline |
| [CSD-022](docs/csd/CSD-022-language.md) | Language (English and Dutch) |
| [CSD-024](docs/csd/CSD-024-navigation-and-app-shell.md) | Navigation and app shell |
| [CSD-025](docs/csd/CSD-025-shared-charts.md) | Shared charts |

Only specs with `status: approved` are implemented; check a spec's status before relying on it. Tests reference acceptance criteria in their names, for example `test_csd002_ac_1_1_...` (Rust) and `test_csd024_ac_3_1_...` (DOM tests).

### Contributing

Work on the `development` branch and open a PR from it into `main`; nothing is pushed to `main` directly. Use Conventional Commit messages (`feat:`, `fix:`, `refactor:`, `chore:`). See [CLAUDE.md](CLAUDE.md) for the full project rules.

The repo is public, so fixtures and test data must be obviously fake (for example "Example Bank", round numbers).

## License

Varde is free software under the [GNU General Public License v3.0](LICENSE). You may use, study, change and share it, and a version you distribute must stay under the same license with its source available. The license covers the code only. Your project files stay yours.
