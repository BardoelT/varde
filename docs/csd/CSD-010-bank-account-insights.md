---
id: CSD-010
title: Bank account insights
status: approved
owner: Thijs Bardoel
version: 1.3
last_updated: 03/11/2026
related: [CSD-007, CSD-009, CSD-025]
---

# CSD-010: Bank account insights

## 1. Intent
The Bank accounts page lists accounts, entries and a table of yearly totals, but a table does not show how the money at the banks is spread over the accounts and how that changed over the years. This capability adds an insights layer to the Bank accounts page: a chart with one bar per calendar year, stacked from one segment per bank, with a marker at the closing balance of the year. It is for the owner looking back over their own bank balances. The chart frame (axis, bars, tooltip, highlight, motion, size, look, legend, colours, tabs, short labels) is shared and specified in CSD-025. It follows the Investment insights of CSD-007, but a bank entry is only a balance, so the segments are the banks instead of opening balance, deposits and growth. It supersedes CSD-009 NG-6 for this view only to the extent that the chart shows the closing balance per bank.

## 2. Scope
### 2.1 In scope
- An "Insights" tab on the Bank accounts page, next to the existing content ("Overview").
- A yearly stacked bar chart over all accounts with one segment per bank, a closing-balance marker, a legend of the banks, a tooltip and the highlight of one bank.
- Its empty state.

### 2.2 Non-goals
- NG-1: A monthly view, or any granularity other than a calendar year.
- NG-2: A filter, a selection or a hiding of banks.
- NG-3: Growth, a percentage, a year-over-year change, a forecast, or any other figure than the balances and the closing balance. The number of entries is never displayed.
- NG-4: Click-through, zooming, or exporting the chart.
- NG-5: Charts on the Dashboard or on the Net worth page, which has its own chart (CSD-017).
- NG-6: A chart library or any other new dependency.
- NG-7: Estimating balances for years without an entry (CSD-009 NG-8), and carrying a balance forward into a year in which a bank has no entry.
- NG-8: Letting the user choose the colour of a bank.

## 3. Definitions
| Term | Meaning |
|---|---|
| **Year** | A calendar year. An entry counts in the year of its own period only, as in the yearly totals (CSD-009 REQ-19). |
| **Current year** | The calendar year of today, in local time. Its figures run up to the latest entry so far, and are therefore partial. |
| **Segment** | The part of a year's bar that belongs to one bank: the balance of that bank's latest entry in that year. A bank without an entry in that year has no segment in it. |
| **Closing balance** | The sum of the segments of a year, as in the yearly totals of CSD-009 REQ-19. |
| **Colour slot** | The number from 0 to 7 stored with a bank (CSD-009 REQ-3), which decides its colour. |

## 4. Requirements

### 4.0 Overview flow
```mermaid
sequenceDiagram
    actor U as User
    participant UI as Frontend (ui/)
    participant CMD as commands/
    participant DOM as domain/
    U->>UI: opens the Insights tab
    UI->>CMD: get_bank_insights()
    CMD->>DOM: insights(today)
    DOM-->>CMD: banks, axis, and one figure set per year, with segments, pieces and marker
    CMD-->>UI: figures as decimal strings, short labels as text
    UI->>U: draws the received bars per bank, markers, axis and labels, legend
```

**REQ-1: Tabs on the Bank accounts page** *(ubiquitous)*
The Bank accounts page shall offer the tabs "Overview" and "Insights" while at least one bank account exists, with Overview, which holds the content of CSD-009, shown first.

```gherkin
@AC-1.1
Scenario: Overview is shown first
  Given a project with one bank account
  When the user opens the Bank accounts page
  Then the Overview tab is selected
  And it shows the Accounts, Yearly totals and Entries sections

@AC-1.2
Scenario: Switching to Insights
  Given the Bank accounts page shows the Overview tab
  When the user selects the Insights tab
  Then the Overview sections are hidden
  And the insights chart is shown

@AC-1.3
Scenario: No accounts yet
  Given a project without bank accounts
  When the user opens the Bank accounts page
  Then the prompt to add the first bank account is shown
  And there are no tabs
```

**REQ-2: Year series** *(ubiquitous)*
The app shall return one set of figures for every calendar year from the year of the earliest entry up to and including the later of the current year and the year of the latest entry, but for at most 50 years back from that last year, oldest year first, holding the closing balance, the number of entries (returned, not displayed) and, per bank with an entry in that year, its segment, together with the axis, the bar pieces, the marker and the short labels of CSD-025 REQ-3. An entry older than the 50 years gets no slot, but still counts in the yearly totals of CSD-009 REQ-19 (a year counts only the entries of its own period).

```gherkin
@AC-2.1
Scenario: Series runs from the first entry to the current year
  Given today is 26/10/2026
  And the earliest entry is for 2024-03
  And there are entries in 2025 and 2026
  When the insights are requested
  Then the series holds the years 2024, 2025 and 2026 in that order

@AC-2.2
Scenario: A single year
  Given today is 26/10/2026
  And the only entry is for 2026-09
  When the insights are requested
  Then the series holds only the year 2026

@AC-2.3
Scenario: No entries
  Given today is 26/10/2026
  And an account without entries
  When the insights are requested
  Then the series is empty

@AC-2.5
Scenario: An entry after the current year extends the series
  Given today is 26/10/2026
  And entries for 2024-03 and 2027-05
  When the insights are requested
  Then the series holds the years 2024, 2025, 2026 and 2027

@AC-2.6
Scenario: A very old entry gets no slot but still counts
  Given today is 26/10/2026
  And "Example Bank" has an entry for 1950-01 with balance 1000.00
  And "Other Bank" has an entry for 2026-03 with balance 500.00
  When the insights are requested
  Then the series holds the years 1977 to 2026, which is 50 years
  And 1950 to 1976 have no slot
  And the closing balance of 2026 is 500.00
  And the yearly totals of CSD-009 REQ-19 still hold a row for 1950 with closing balance 1000.00

@AC-2.4
Scenario: Pieces, marker and labels are delivered
  Given today is 26/10/2026
  And account "Example Current Account" with an entry 2025-12 (balance 2000.00)
  And account "Example Savings" with an entry 2025-12 (balance 8000.00)
  When the insights are requested
  Then 2025 has the pieces 0.00 to 2000.00 and 2000.00 to 10000.00, in the creation order of the banks
  And the marker of 2025 has value 10000.00 and label "10.00k"
  And the axis ticks include 0.00 and 10000.00 with the labels "0" and "10k"
```

**REQ-3: Years without entries** *(unwanted behaviour)*
If a year in the series has no entry, then the app shall return that year with closing balance 0.00, 0 entries and no segments.

```gherkin
@AC-3.1
Scenario: Gap year
  Given today is 26/10/2026
  And there are entries in 2022 and 2024 but none in 2023
  When the insights are requested
  Then the year 2023 has closing balance 0.00, 0 entries and no segments
  And the series holds the years 2022, 2023, 2024, 2025 and 2026
```

**REQ-4: Current year is partial** *(state-driven)*
While a year is the current year, the app shall mark it as partial. A year after the current year is not marked partial and looks like any other year.

```gherkin
@AC-4.1
Scenario: Current year is marked
  Given today is 26/10/2026
  And there are entries in 2025 and 2026
  When the insights are requested
  Then the year 2026 is marked partial
  And the year 2025 is not

@AC-4.2
Scenario: Current year without entries is still in the series and marked
  Given today is 26/10/2026
  And the only entry is for 2025-12
  When the insights are requested
  Then the series holds 2025 and 2026
  And 2026 is marked partial and has 0 entries

@AC-4.3
Scenario: A future year is not marked partial
  Given today is 26/10/2026
  And an entry for 2027-05
  When the insights are requested
  Then the year 2027 is not marked partial
```

**REQ-5: Segments are the latest balances per bank** *(ubiquitous)*
The app shall give each year one segment per bank that has an entry in that year, equal to the balance of that bank's latest entry in that year, ordered by the creation of the banks, and the segments shall add up to the closing balance, which equals the closing balance and the entry count of the yearly totals of CSD-009 REQ-19. The entry count is a returned data field and is never displayed; "entries" in the ACs of this requirement and of REQ-2 to REQ-4 refers to it.

```gherkin
@AC-5.1
Scenario: Segments of a year
  Given today is 26/10/2026
  And account "Example Current Account" with entries 2026-03 (balance 2000.00) and 2026-09 (balance 2500.00)
  And account "Example Savings" with an entry 2026-06 (balance 8000.00)
  When the insights are requested
  Then 2026 has the segments "Example Current Account" 2500.00 and "Example Savings" 8000.00
  And 2026 has closing balance 10500.00 and 3 entries

@AC-5.2
Scenario: A bank without an entry in the year has no segment
  Given today is 26/10/2026
  And a monthly account "Example Current Account" with entries 2025-12 (balance 2000.00) and 2026-03 (balance 2500.00)
  And a yearly account "Example Savings" with an entry 2025 (balance 8000.00)
  When the insights are requested
  Then 2025 has the segments "Example Current Account" 2000.00 and "Example Savings" 8000.00
  And 2026 has only the segment "Example Current Account" 2500.00

@AC-5.3
Scenario: An overdrawn bank has a negative segment
  Given today is 26/10/2026
  And account "Example Current Account" with an entry 2026-09 (balance 5000.00)
  And account "Example Overdraft" with an entry 2026-09 (balance -1000.00)
  When the insights are requested
  Then 2026 has the segments 5000.00 and -1000.00
  And 2026 has closing balance 4000.00

@AC-5.4
Scenario: Segments are ordered by creation, not by name
  Given today is 26/10/2026
  And accounts "Example Savings" and then "Example Current Account", both with an entry 2026-09
  When the insights are requested
  Then the segments of 2026 are listed "Example Savings" first and "Example Current Account" second

@AC-5.5
Scenario: Closing balance and entries equal the yearly totals
  Given today is 26/10/2026
  And the entries of AC-5.1 and AC-5.2
  When the insights and the yearly totals are requested
  Then for every year the closing balance and the number of entries of the insights equal those of the yearly totals
  And this includes a year after the current year that has an entry
```

**REQ-6: Chart** *(ubiquitous)*
The Insights tab shall draw one bar per year in the order received, stacked from one segment per bank in which positive segments rise from the zero line in the order received and negative segments hang down from it in that order, on a y-axis that includes 0, ends at a rounded-up value of at least the highest positive sum and starts at a rounded-down value of at most the lowest negative sum, with gridlines, short axis labels delivered by Rust (CSD-025 REQ-3, REQ-4 and REQ-17, such as "10k" on the axis and "10.00k" on a marker), a gap year as a labelled slot without a bar, and no currency.

```gherkin
@AC-6.1
Scenario: Stacked bars per year
  Given the series holds the years 2025 and 2026
  When the user opens the Insights tab
  Then two bars are drawn, 2025 on the left and 2026 on the right
  And each bar has one segment per bank with an entry in that year

@AC-6.2
Scenario: Negative segments hang down
  Given 2026 has the segments 5000.00 and -1000.00
  When the user opens the Insights tab
  Then the segment of 5000.00 rises from the zero line
  And the segment of -1000.00 hangs down from the zero line

@AC-6.3
Scenario: Axis covers the highest and the lowest level
  Given the highest positive sum of any year is 13100.00 and the lowest negative sum is -1000.00
  When the user opens the Insights tab
  Then the top of the y-axis is at least 13100.00
  And the bottom of the y-axis is at most -1000.00

@AC-6.4
Scenario: Only positive balances
  Given every segment of every year is positive
  When the user opens the Insights tab
  Then the y-axis starts at 0

@AC-6.5
Scenario: Gap year is an empty slot
  Given the year 2023 has no entries
  When the user opens the Insights tab
  Then 2023 has a labelled slot without a bar

@AC-6.6
Scenario: Current year is marked
  Given the year 2026 is partial
  When the user opens the Insights tab
  Then the 2026 bar has a lighter fill
  And its label reads "2026 YTD"
```

**REQ-7: Closing-balance marker** *(ubiquitous)*
The Insights tab shall draw for each year with entries the marker of CSD-025 REQ-7 at the closing balance, with the short label of the closing balance delivered by Rust.

```gherkin
@AC-7.1
Scenario: Marker at the closing balance
  Given 2025 has the segments 2000.00 and 8000.00 and closing balance 10000.00
  When the user opens the Insights tab
  Then a marker line is drawn across the 2025 bar at 10000.00
  And its label reads "10.00k"

@AC-7.2
Scenario: Marker sits at the net sum when a segment is negative
  Given 2026 has the segments 5000.00 and -1000.00
  When the user opens the Insights tab
  Then the 2026 marker sits at 4000.00, inside the positive stack

@AC-7.3
Scenario: Markers are not connected and the current year is dashed
  Given the series holds 2024, a gap year 2025 and the partial year 2026
  When the user opens the Insights tab
  Then no line joins the markers
  And the marker of 2026 is dashed
  And 2025 has no marker
```

**REQ-8: A colour per bank** *(ubiquitous)*
The Insights tab shall draw every segment of a bank in the colour of the bank's colour slot, taken from a palette of eight colours with a variant for light mode and one for dark mode, each of at most 60% HSL saturation, so that a bank has the same colour in every year, in the legend and in the tooltip, and keeps it when other banks are deleted.

```gherkin
@AC-8.1
Scenario: A bank has one colour everywhere
  Given bank "Example Savings" has colour slot 1 and entries in 2025 and 2026
  When the user opens the Insights tab
  Then its segments in 2025 and 2026, its legend swatch and its tooltip swatch have the colour of slot 1

@AC-8.2
Scenario: Colours survive a deletion
  Given banks with colour slots 0, 1 and 2 and entries
  When the user deletes the bank with slot 0 on the Overview tab and opens the Insights tab
  Then the other banks are still drawn in the colours of slots 1 and 2

# @AC-8.3 (removed in v1.0): the palette in both modes is specified in CSD-025 REQ-14 (AC-14.2).

@AC-8.4
Scenario: More than eight banks
  Given nine banks with entries
  When the user opens the Insights tab
  Then the first and the ninth bank share a colour
  And each is named in the legend and in the tooltip
```

**REQ-9: Legend of banks** *(ubiquitous)*
The Insights tab shall show the legend of CSD-025 REQ-13 for every bank that has at least one entry, in the order the banks were created, followed by the closing-balance marker.

```gherkin
@AC-9.1
Scenario: Legend lists the banks in creation order
  Given banks "Example Savings" and then "Example Current Account", both with entries
  When the user opens the Insights tab
  Then the legend lists "Example Savings" and "Example Current Account" in that order, each with its colour swatch
  And the legend shows the closing-balance marker

@AC-9.2
Scenario: A bank without entries is not in the legend
  Given a bank "Example Empty" with no entries and a bank "Example Savings" with an entry
  When the user opens the Insights tab
  Then the legend lists only "Example Savings"

# @AC-9.3 (removed in v1.0): wrapping of many names is specified in CSD-025 REQ-13 (AC-13.1).
# @AC-9.4 (removed in v1.0): the ellipsis for a long name is specified in CSD-025 REQ-13 (AC-13.2).

@AC-9.5
Scenario: A deleted bank leaves the legend
  Given the legend lists "Example Savings" and "Example Current Account"
  When the user deletes "Example Savings" on the Overview tab and opens the Insights tab
  Then the legend lists only "Example Current Account"
```

**REQ-10: Details of a year** *(event-driven)*
When the user hovers over or moves keyboard focus to a year, the Insights tab shall show a tooltip with the year, one row per segment with the bank's swatch, full name and balance in the order the bar shows them from top to bottom (the highest segment first, the segments below the zero line last), then the closing balance and, for the current year, that it is partial, placed as CSD-025 REQ-8 says, with the same values given to screen readers. The tooltip and the screen-reader description show no number of entries, and a year without entries shows the note "No entries".

```gherkin
@AC-10.1
Scenario: Tooltip on hover
  Given 2026 has the segments "Example Current Account" 2500.00 and "Example Savings" 8000.00 and closing balance 10500.00
  When the user hovers over the 2026 bar
  Then a tooltip shows both banks with their swatch and balance and the closing balance 10500.00
  And it shows no number of entries

@AC-10.2
Scenario: Tooltip on keyboard focus
  Given the Insights tab shows the chart
  When the user moves focus to the 2026 bar with the Tab key
  Then the same tooltip is shown
  And the tooltip disappears when focus leaves the bar

@AC-10.3
Scenario: Accessible label
  Given 2026 has a segment "Example Overdraft" with balance -1000.00
  When a screen reader reads the 2026 bar
  Then it announces the year, each bank with its balance and the closing balance
  And it announces no number of entries
  And it announces that "Example Overdraft" is negative

@AC-10.4
Scenario: Partial year
  Given the year 2026 is partial
  When the user hovers over the 2026 bar
  Then the tooltip says that the year is partial

# @AC-10.5 (removed in v1.0): the placement of the tooltip is specified in CSD-025 REQ-8 (AC-8.2).

@AC-10.6
Scenario: Year without a bar
  Given the year 2023 has no entries
  When the user hovers over its slot
  Then the tooltip is centred over that slot
  And it shows no bank rows
  And it shows the note "No entries"

@AC-10.7
Scenario: Rows follow the bar from top to bottom
  Given 2026 has the segments "Example Current Account" 2500.00, "Example Savings" 8000.00 and "Example Overdraft" -1000.00 in creation order
  When the user hovers over the 2026 bar
  Then the tooltip rows are listed "Example Savings", "Example Current Account", "Example Overdraft"
  And the closing balance 9500.00 is listed after them

@AC-10.8
Scenario: Only positive segments
  Given 2025 has the segments "Example Current Account" 2000.00 and "Example Savings" 8000.00
  When the user hovers over the 2025 bar
  Then the tooltip rows are listed "Example Savings" first and "Example Current Account" second
  And the closing balance is listed last
```

**REQ-11: Highlight one bank** *(removed in v1.0)*
Moved to CSD-025 REQ-9, which holds the highlight of one series for every chart. AC-11.1 to AC-11.6 are replaced by CSD-025 AC-9.1 to AC-9.4.

**REQ-12: Motion and reduced motion** *(removed in v1.0)*
Moved to CSD-025 REQ-10. AC-12.1 to AC-12.3 are replaced by CSD-025 AC-10.1 and AC-10.2.

**REQ-13: Size, look and accessible segments** *(ubiquitous)*
The Insights tab shall draw the chart with the size and the look of CSD-025 REQ-11 and REQ-12, with every segment carrying an accessible label with the bank, the year and the balance.

```gherkin
@AC-13.4
Scenario: Segment labels
  Given 2026 has a segment of "Example Savings" with balance 8000.00
  When a screen reader reads that segment
  Then it announces "Example Savings", 2026 and 8000.00

# @AC-13.1, @AC-13.2, @AC-13.3 (removed in v1.0): bars, gridlines, zero line and tooltip look are specified in CSD-025 REQ-12 (AC-12.1 to AC-12.3).
# @AC-13.5 (removed in v1.0): the always-drawn zero line is specified in CSD-025 REQ-2 (AC-2.3).
# @AC-13.6, @AC-13.7 (removed in v1.0): size and plot height are specified in CSD-025 REQ-11 (AC-11.1, AC-11.2).
```

**REQ-14: No entries yet** *(state-driven)*
While at least one account exists but no entry, the Insights tab shall show a note instead of a chart.

```gherkin
@AC-14.1
Scenario: Accounts without entries
  Given a project with one account and no entries
  When the user opens the Insights tab
  Then the note "No entries yet. The chart appears once there is an entry." is shown
  And no chart is drawn
```

**REQ-15: Current figures** *(event-driven)*
When the user opens the Insights tab, the app shall show figures computed from the project at that moment.

```gherkin
@AC-15.1
Scenario: Figures follow changes
  Given the Insights tab showed closing balance 2500.00 for 2026
  And the user added an entry for 2026-10 with balance 3000.00 on the Overview tab
  When the user opens the Insights tab again
  Then the 2026 marker shows 3000.00

@AC-15.2
Scenario: Last entry deleted
  Given the Insights tab showed a chart
  And the user deleted the only entry on the Overview tab
  When the user opens the Insights tab again
  Then the note of AC-14.1 is shown and no chart is drawn

@AC-15.3
Scenario: A moved entry follows to its new year
  Given a monthly account with an entry for 2026-01
  When the user changes its period to 2025-12 and opens the Insights tab
  Then the series holds 2025 and 2026, and the segment is in 2025
```

## 5. Interfaces and contracts

Command: `get_bank_insights()` returns the banks that have at least one entry, in creation order, and the series, oldest year first, together with the common part of CSD-025 section 5 (`axis`, and per year `pieces` and `marker`; the `series` of a piece is the id of its bank):

```json
{
  "banks": [
    { "id": "…", "name": "Example Current Account", "colour_slot": 0 },
    { "id": "…", "name": "Example Savings", "colour_slot": 1 }
  ],
  "axis": { "ticks": [ { "value": "0.00", "label": "0" }, { "value": "5000.00", "label": "5k" }, { "value": "10000.00", "label": "10k" }, { "value": "15000.00", "label": "15k" } ] },
  "years": [
    { "year": 2025, "closing_balance": "10000.00", "entry_count": 2, "is_partial": false,
      "segments": [ { "account_id": "…", "balance": "2000.00" }, { "account_id": "…", "balance": "8000.00" } ],
      "pieces": [ { "series": "…", "low": "0.00", "high": "2000.00", "hangs": false, "rounding": "none" },
                  { "series": "…", "low": "2000.00", "high": "10000.00", "hangs": false, "rounding": "top" } ],
      "marker": { "value": "10000.00", "label": "10.00k" } },
    { "year": 2026, "closing_balance": "2500.00", "entry_count": 1, "is_partial": true,
      "segments": [ { "account_id": "…", "balance": "2500.00" } ],
      "pieces": [ { "series": "…", "low": "0.00", "high": "2500.00", "hangs": false, "rounding": "top" } ],
      "marker": { "value": "2500.00", "label": "2.50k" } }
  ]
}
```

`entry_count` is returned but never displayed. `banks` and `years` are empty when there are no entries. `segments` follow the creation order of the banks and are empty for a gap year, which also has no pieces and `"marker": null`. Amounts are decimal strings. Rust decides "today" (local time), the years, the partial flag, the axis, the pieces and the short labels, and the frontend only displays what it receives. Error codes are those of CSD-009.

## 6. Constraints
- CON-1: The chart reuses the shared chart code of the frontend (CSD-025 CON-1) and adds no chart library or other dependency. Rationale: NG-6, and `CLAUDE.md` asks before adding dependencies.
- CON-2: The frontend displays figures and labels built in Rust and computes no amounts, not the sum of the segments, not the stack of the pieces, not the axis and not a short label (CSD-025 CON-2). Rationale: `CLAUDE.md` layout rule.
- CON-3: No network access. Rationale: `CLAUDE.md` Tauri security rules.
- CON-4: Bank names are drawn as plain text, never as HTML. Rationale: a name is typed by the user and must not run as markup.
- CON-5: Tests and fixtures use synthetic data only. Rationale: the repo is public.
- CON-6: (removed in v1.0) The palette tokens are specified in CSD-025 CON-5.
- CON-7: Money rounding, also for the short labels, follows CSD-000 CON-8. Rationale: one rounding rule for the whole application.

## 7. Verification
The series, the axis, the pieces, the markers and the short labels are computed in Rust, so REQ-2 to REQ-5 are proven by `cargo test`. Tests are named `test_csd010_ac_<n>_<m>_<description>` (CSD-000 REQ-6). The shared chart behaviour (look, size, motion, label format) is verified in CSD-025.

| What | How |
|---|---|
| Rust tests (the data, the axis ticks, the marker value and label) | `cargo test test_csd010_`. One requirement, e.g. REQ-5: `cargo test test_csd010_ac_5_` |
| Display ACs (the screen) | No DOM test carries an AC id of this CSD yet, so its display ACs are checked by hand. |

Not covered by a named test yet (27 of 45 ACs; no test carries the AC id): AC-1.1 to AC-1.3, AC-4.2, AC-6.1, AC-6.2, AC-6.4, AC-6.6, AC-7.1 to AC-7.3, AC-8.1, AC-8.2, AC-8.4, AC-9.1, AC-9.5, AC-10.1 to AC-10.4, AC-10.6 to AC-10.8, AC-13.4, AC-14.1, AC-15.1, AC-15.2.

Done means fmt, clippy (`-D warnings`), `cargo test --all-features` and `npm test` pass.

## 8. Open questions
- OQ-1: (resolved in v0.2) The legend lists only banks that have at least one entry (REQ-9), because a bank without entries has no segment to identify.
- OQ-2: (resolved 03/11/2026) An entry after the current year extends the series to the latest year with an entry, so the closing balances equal the yearly totals of CSD-009 REQ-19 (REQ-2, AC-2.5, AC-5.5). The series spans at most 50 years back from its last year; an older entry gets no slot but still counts (AC-2.6). A future year is not marked partial (REQ-4, AC-4.3).

## 9. Changelog
- 1.3 (03/11/2026): Verification (section 7) lists the real test filters and the ACs that no named test covers yet; claims of tests that do not exist are removed. The claim that display ACs have DOM tests is removed.
- 1.2 (03/11/2026): The axis tick labels in AC-2.4 and the section 5 example are whole numbers ("0", "10k"), as CSD-025 REQ-17 says; markers keep 2 decimals. Back to `draft` until re-approved.
- 1.1 (03/11/2026): Resolved OQ-2: the series runs to the later of the current year and the latest year with an entry (REQ-2, new AC-2.5), spans at most 50 years back from its last year, older entries still count (new AC-2.6), a future year is not partial (REQ-4, new AC-4.3), AC-5.5 covers future years. Back to `draft` until re-approved.
- 1.0 (03/11/2026): The shared chart text moved to CSD-025: REQ-11 (highlight) and REQ-12 (motion) removed in favour of CSD-025 REQ-9 and REQ-10, REQ-13 reduced to the size and look reference and the accessible segment label (AC-13.1 to AC-13.3, AC-13.5 to AC-13.7 removed), AC-8.3, AC-9.3, AC-9.4 and AC-10.5 removed, CON-6 removed. REQ-7 uses the marker of CSD-025 and the short label "10.00k" (was "10k"). Rust delivers the axis, the pieces, the marker and the short labels (REQ-2, new AC-2.4, section 5, CON-2). Rounding follows CSD-000 CON-8 (new CON-7). Verification: display ACs are covered by DOM tests, ACs without a test are marked not covered. New OQ-2 (entries in the future). Back to draft for re-approval.
- 0.6 (29/10/2026): The plot height scales with the height of the window instead of being a fixed 390 px: with the page scrolled to the top the chart ends at the bottom of the window, and the plot is never lower than 260 px. It is recomputed when the window is resized.
- 0.5 (29/10/2026): The Reports page is replaced by the Net worth page of CSD-017 (NG-5; GitHub issue 24). No other change.
- 0.4 (29/10/2026): The plot of every yearly chart is 390 px high (was 260 px), 50% taller. No other behaviour change.
- 0.3 (29/10/2026): Entry counts are no longer displayed: REQ-10 (AC-10.1, AC-10.3 amended) drops the count from the tooltip and the screen-reader description, an empty year shows "No entries" (AC-10.6), NG-3 and REQ-5 say the count is returned but never shown. REQ-10 orders the tooltip rows as the bar shows them, top to bottom, segments below the zero line last (AC-10.7, AC-10.8 added). REQ-13 sets the shared chart size (plot height 260 px, slot at least 64 px) and an always-drawn zero line (AC-13.5, AC-13.6 added). The legend hover and focus highlight of REQ-11 is unchanged. New ACs are manual checklist items.
- 0.2 (28/10/2026): Resolved OQ-1: the legend lists only banks with at least one entry, as REQ-9 says.
- 0.1 (28/10/2026): initial draft, from GitHub issue 13 and the grill-me interview. It follows CSD-007 (tabs, year series, partial year, marker, tooltip, highlight, motion, look), with one segment per bank instead of opening balance, deposits and growth.
