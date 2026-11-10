---
id: CSD-031
title: About window
status: approved
owner: Thijs Bardoel
version: 0.2
last_updated: 09/11/2026
related: [CSD-000, CSD-022, CSD-024]
---

# CSD-031: About window

## 1. Intent
The "About Varde" item in the app menu opens Tauri's default About panel, which shows little more than the name and a generic icon. This capability makes that panel show the Varde icon and the information a person expects from an About window: the version, what the app is, who made it, the licence and where the source lives. It uses the native macOS About panel, so the panel looks and behaves like the About panel of any other Mac app. It is GitHub issue 54.

## 2. Scope
### 2.1 In scope
- The content of the native About panel opened from the app menu.
- The single source of the version shown.

### 2.2 Non-goals
- NG-1: A custom HTML window or dialog in `ui/` for About.
- NG-2: Dutch text in the panel. It stays English like the rest of the native menu (CSD-024 REQ-6, CSD-022 NG-6).
- NG-3: A build number, commit hash, list of dependencies or credits.
- NG-4: Update checks or any network access (a link opens in the browser only when the user clicks it).
- NG-5: A new menu item or shortcut. "About Varde" stays where it is (CSD-024 REQ-5).

## 3. Definitions
| Term | Meaning |
|---|---|
| **About panel** | The native macOS panel opened by the "About Varde" item of the app menu. |

## 4. Requirements

**REQ-1: Content of the About panel** *(event-driven)*
When the user chooses "About Varde" in the app menu, the app shall show the About panel with the Varde icon, the name "Varde", the version, the description "Personal wealth tracking. Your data stays on this computer.", the copyright "© 2026 Thijs Bardoel", the licence "GPL-3.0-only" and the website `https://github.com/BardoelT/varde`.

```gherkin
@AC-1.1
Scenario: The panel shows the app information
  Given the app has started
  When the user chooses "About Varde" in the app menu
  Then the About panel shows the Varde icon and the name "Varde"
  And it shows the version, the description, the copyright "© 2026 Thijs Bardoel", the licence "GPL-3.0-only" and the website

@AC-1.2
Scenario: The icon also shows in a development run
  Given the app runs with "cargo tauri dev"
  When the user chooses "About Varde"
  Then the About panel shows the Varde icon and not a generic icon

@AC-1.3
Scenario: Dutch language, English panel
  Given the language setting is Dutch
  When the user chooses "About Varde"
  Then every text in the About panel is in English
```

**REQ-2: One version** *(ubiquitous)*
The version shown in the About panel shall be the version of the app as built, with no second copy of the number to keep in step.

```gherkin
@AC-2.1
Scenario: Version follows the package
  Given the package version is "0.1.0"
  When the About panel is built
  Then the version shown is "0.1.0"

@AC-2.2
Scenario: Package and bundle versions disagree
  Given the version in Cargo.toml differs from the version in tauri.conf.json
  When the tests run
  Then a test fails and names both versions
```

## 5. Interfaces and contracts
Rust builds the panel in `src-tauri/src/menu.rs` with Tauri's `AboutMetadata` on the predefined About item of the app menu. The texts, copyright, licence and website are constants in `menu.rs`; the version is read from the package at compile time. The icon is the default window icon, which Tauri embeds from the first `.png` in `bundle.icon` of `tauri.conf.json`; that entry is `icons/128x128@2x.png` (a test pins it). macOS shows only the name, version, copyright, icon and credits, so the description, licence and website are shown as the credits text. The frontend is not involved.

## 6. Constraints
- CON-1: The native About panel is used through Tauri's built-in menu API: no plugin, no capability, no CSP change and no new dependency. Rationale: least privilege per `CLAUDE.md` (as CSD-024 CON-1).
- CON-2: No amount, file name or other project data appears in the panel. Rationale: the panel is about the app, and the repo and screenshots are public.

## 7. Verification
Tests are named `test_csd031_ac_<n>_<m>_<description>` (CSD-000 REQ-6). The metadata is built by a pure function in `menu.rs` that returns the values, so `cargo test test_csd031_` checks AC-1.1 (texts, licence, website, copyright), AC-2.1 and AC-2.2 (the Cargo and `tauri.conf.json` versions are equal). The native panel itself (AC-1.1 icon, AC-1.2, AC-1.3) can only be created on the main thread and is on the manual checklist: run `cargo tauri dev` and a `cargo tauri build` `.app`, choose "About Varde", and look at it.

Done means fmt, clippy (`-D warnings`) and `cargo test --all-features` pass.

## 8. Open questions
None.

## 9. Changelog
- 0.2 (09/11/2026): implementation notes only, behaviour unchanged. The icon comes from the default window icon (no new Tauri feature or crate), and the description, licence and website are the credits text because macOS ignores those fields.
- 0.1 (09/11/2026): initial draft from the grill-me interview of 09/11/2026 on GitHub issue 54: native panel, explicit icon, fixed content, English only.
