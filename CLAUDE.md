# CLAUDE.md

Instructions for Claude when working in this repository. Keep this file short and current.
If you (Claude) make the same mistake twice, propose an addition here.

## Project

Personal wealth tracking desktop app: bank accounts, houses, investments, debts, expenses, income and net worth over time. There is no transaction model: individual stocks, prices and trades are out of scope.

- **Stack:** Rust + Tauri v2. Frontend: vanilla HTML + CSS + JS (no framework, no build step) in `ui/`, Rust backend in `src-tauri/`.
- **Platform:** macOS only (Apple Silicon + Intel). Do not add Windows/Linux code paths or CI targets unless asked.
- **Data:** stored **locally only** in a user-chosen project file (JSON, no database; see CSD-002). No cloud sync, no backend server.
- **Repo:** public on GitHub. Assume everything committed is visible to the world.

## Commands

```bash
cargo tauri dev                          # run the app locally
cargo tauri build                        # release build (.app / .dmg)
cargo fmt --all                          # format
cargo clippy --all-targets --all-features -- -D warnings   # lint (must be clean)
cargo test --all-features                # all tests
cargo test <name>                        # single test
cargo deny check                         # licenses, advisories, banned crates
npm ci                                   # once: installs jsdom (tests-only dev dependency)
npm test                                 # DOM tests of ui/ (node:test + jsdom, Node 26)
GOLDEN_UPDATE=1 cargo test golden        # rewrite src-tauri/golden/ after a deliberate behaviour change; review the diff
```

The app has no frontend build step and no runtime npm dependency — `ui/` is served as-is by Tauri. `jsdom` is the one tests-only dev dependency (CSD-021 CON-5): exact-pinned in `package.json`, installed from `package-lock.json`, never bundled. Use only `node:test` and `node:assert` besides it.

Before saying a task is done: fmt, clippy and the cargo tests must pass, and `npm test` too when `ui/` changed.

## Layout

```
ui/                   # frontend (UI only — HTML + CSS + JS, no business logic, no build step)
  index.html
  main.js             # entry point: wires the pages and reloads them
  navigation.js       # left-pane navigation
  pages/              # one file per page: <page>-overview.js and <page>-insights.js for pages with two tabs, <page>.js for the rest (dashboard, net-worth, ratios, settings, project)
  shared/             # helpers pages share: ipc, dom, crud, list-rows, record-form, insights-page, format, i18n, settle, date-mask, chart, yearly-chart, tabs
  locales/            # Dutch texts (nl*.js); the English text in the code is the key (CSD-022)
  styles/             # CSS by area: base, layout, components, pages, charts
  tests/              # DOM tests (node:test + jsdom) and their harness; not part of the app
src-tauri/
  src/main.rs         # thin entry point
  src/lib.rs          # app setup, command registration
  src/commands/       # #[tauri::command] handlers: validate input, call domain, map errors
  src/domain/         # pure business logic: no Tauri, no IO. Areas: income, expenses, bank_accounts, investments, houses, debts, net_worth, ratios; shared: common, series, charts, money, preferences, settings
  src/storage/        # project file persistence (JSON, atomic write), last project, preferences (theme, language)
  src/menu.rs         # native macOS menu bar
  src/error.rs        # app error type
  capabilities/       # Tauri permission files
  golden/             # golden files for the golden tests (see Testing)
  tauri.conf.json
docs/
  architecture.md     # how the code is organised and the conventions it follows
  csd/                # capability specifications
```

Rule: `domain/` is pure and unit-tested. Commands stay thin. The frontend never computes financial figures; it only displays what Rust returns. `docs/architecture.md` explains the structure, including the bank accounts area and the series core that bank accounts, investments, debts and houses share (`domain/series`).

## Rust rules

### Safety
- **No `unsafe`.** Enforced via `Cargo.toml`:
  ```toml
  [lints.rust]
  unsafe_code = "forbid"
  ```
  Do not work around this (no `unsafe` in macros, no new dependencies whose main purpose is FFI without asking).
- No `unwrap()`, `expect()`, `panic!`, `todo!`, `unimplemented!` or indexing that can panic in non-test code. Return `Result`.
- No `as` casts between numeric types where truncation is possible; use `TryFrom`/`try_into()`.

### Lints
Keep this in `Cargo.toml` and don't silence lints without a comment explaining why:
```toml
[lints.clippy]
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
indexing_slicing = "warn"
float_arithmetic = "warn"
module_name_repetitions = "allow"
```
`#[allow(...)]` must be scoped as narrowly as possible and include a reason: `#[allow(clippy::x, reason = "...")]`.

### Money and numbers
- **Never use `f32`/`f64` for money.** Use `rust_decimal::Decimal` (or integer minor units) everywhere: domain, storage, and IPC.
- Amounts are in the project currency, one per project and a project setting (CSD-003), so amounts are never mixed. Add an explicit conversion before ever allowing a second currency.
- Rounding is one rule (CSD-000 CON-8): 2 decimals, half-up (the midpoint rounds away from zero), for amounts and for percentages and ratios alike. `Money` is the amount type; never round anywhere else.
- Dates: use `time` or `chrono` with explicit time zones; store UTC, display local.
- Dates the user sees or types are `dd/mm/yyyy` (months `YYYY-MM`), in the UI, in messages and in all docs and CSDs. The project file and IPC use ISO 8601 internally (CSD-000 CON-7).

### Errors
- Library-style errors with `thiserror` in `domain/` and `storage/`; one `AppError` in `error.rs`.
- Errors returned to the frontend are serialized to a safe message and code. Never leak file paths, SQL or stack traces to the UI.

### Style
- Prefer newtypes over primitive obsession (`AccountId(Uuid)`, not `String`).
- Make invalid states unrepresentable (enums over boolean flags).
- Small functions; doc comments (`///`) on all public items in `domain/`.
- Async only where IO requires it; don't block the main thread in commands.

### Dependencies
- Ask before adding a new crate. Prefer well-maintained, widely used crates.
- Must pass `cargo deny check` (licenses: MIT/Apache-2.0/BSD-compatible).
- Commit `Cargo.lock`.

## Tauri security
- **Least privilege capabilities.** Only enable the permissions a feature needs in `src-tauri/capabilities/`. Do not add `shell`, `fs` with broad scopes, or `http` plugins without asking.
- `tauri.conf.json` sets a strict CSP: no `unsafe-eval`, no remote scripts, no remote content. `style-src 'unsafe-inline'` exists only for one inline style attribute in `ui/index.html`; removing it is a later security-setting change that needs approval. CSD-021 covers the release workflow.
- Treat every `#[tauri::command]` input as untrusted: validate and bound it before use.
- No network calls at all unless explicitly requested (e.g. a future exchange-rate fetch must be opt-in and documented).
- Don't use `dangerousDisableAssetCspModification` or similar `dangerous*` options.

## Privacy and the public repo
This app handles personal financial data and the repo is public.
- **Never commit real financial data.** Never commit `*.varde` or any real project file (`*.varde` is git-ignored). Stage explicit paths, never `git add -A` or `git add .`.
- Fixtures and test data must be obviously fake (e.g. "Example Bank", round numbers).
- **No secrets in the repo.** Signing certificates, notarization credentials and API keys live only in GitHub Actions secrets. Never print them in logs.
- No telemetry, analytics or crash reporting that sends data off the machine.
- Don't log amounts, account names or balances at `info` level or above.
- Don't read or modify `.env*`, keychains, or anything outside the project directory.

## Testing
- Unit tests for all `domain/` logic, especially money math, currency handling and edge cases (zero, negative, very large values, rounding).
- Use property tests (`proptest`) for arithmetic invariants where useful.
- Golden tests (`golden_tests.rs`) pin the output of every read command and the project file against `src-tauri/golden/`; refactors must not change them. Update only with `GOLDEN_UPDATE=1 cargo test golden` for a deliberate change, and review the diff.
- Storage tests run against a temporary directory, never real project files.
- Bug fix = failing test first, then the fix.
- Display ACs (what the screen shows) are covered by DOM tests in `ui/tests/` (`node:test` + jsdom, a fake Tauri bridge fed with the golden files). Test titles carry the AC id: `test_csd024_ac_3_1_...`. Rust computes every figure, so DOM tests feed canned answers and check only what is displayed.
- Don't delete or weaken existing tests to make a change pass.

## Git and PRs
- Work on the `development` branch, not one branch per change; never push to `main`. All changes reach `main` through a PR from `development`.
- Small, focused commits (one concern each) with Conventional Commit messages (`feat:`, `fix:`, `refactor:`, `chore:`).
- PR description: what changed, why, and how it was tested.
- GitHub Actions run only when a release tag is pushed (`release.yml`, `macos-latest`): fmt, clippy, tests and `cargo deny` first. Nothing runs on pull requests or merges, so run the checks locally before opening a PR.

## Specifications (CSD)
- All capability specs (CSDs) live in `docs/csd/`. Read `docs/csd/CSD-000-capability-specifications.md` before reading, writing, or implementing any CSD.
- Requirements use EARS, acceptance criteria use Gherkin (tagged @AC-n.m), and flows use Mermaid.
- Before changing behaviour, find and read the governing CSD; only implement CSDs with `status: approved`.
- Never build what a CSD lists as a non-goal. Ask when the spec is ambiguous about behaviour or interfaces.
- Reference REQ-/AC- IDs in tests (test name: `test_csd<NNN>_ac_n_m_…`), commits, and PR descriptions.

## Ask first
Stop and ask before:
- adding dependencies or Tauri plugins/permissions
- touching `.github/workflows/`, signing/notarization config, or `tauri.conf.json` security settings
- adding any network access
- large refactors across modules
