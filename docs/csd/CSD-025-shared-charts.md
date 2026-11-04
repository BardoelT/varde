---
id: CSD-025
title: Shared charts
status: approved
owner: Thijs Bardoel
version: 0.5
last_updated: 03/11/2026
related: [CSD-000, CSD-006, CSD-007, CSD-010, CSD-012, CSD-014, CSD-016, CSD-017]
---

# CSD-025: Shared charts

## 1. Intent
The Insights tabs of Income, Investments, Bank accounts, Debts, Expenses, Houses and the Net worth page each draw a yearly stacked bar chart, and each CSD used to repeat the same text for the axis, the bars, the tooltip, the legend, the highlight, the motion and the look. This CSD holds that shared behaviour once. It also fixes one rule for every chart: Rust computes all figures and builds all short labels such as "2.55k", and the frontend only displays what it receives. What is specific to an area (which pieces a bar has, the legend entries, the tooltip rows, growth labels, the empty-state text) stays in the CSD of that area. It is for the owner reading their own history year by year.

## 2. Scope
### 2.1 In scope
- The yearly chart: slots, bars, pieces, axis, zero line, year labels, the marker, the tooltip, the highlight of one series, motion, size and look.
- The legend and colours of charts that have one series per named item (banks, debts, houses).
- The short label format and the payload that carries ticks, labels and ready-made pieces.
- The Overview and Insights tabs mechanics and the empty state.

### 2.2 Non-goals
- NG-1: Any granularity other than a calendar year, a filter or hiding of series, zooming, click-through or export.
- NG-2: A chart library or any other new dependency.
- NG-3: Which figures an area shows and how they are computed. That stays in each area CSD.
- NG-4: Choosing the colour of a series.
- NG-5: Printing currency symbols on the chart.

## 3. Definitions
| Term | Meaning |
|---|---|
| **Slot** | The column of one calendar year in the chart, also when the year has no bar. |
| **Piece** | One stacked part of a bar, delivered with its start and end value (`low`, `high`) and the series it belongs to. A piece that hangs below the zero line has `hangs: true`. |
| **Series** | One thing a piece belongs to and the legend names, for example a bank or an opening balance. |
| **Marker** | A short horizontal line across a bar at one value of the year, such as the closing balance, with a short label above it. |
| **Short label** | The text for an axis tick, a marker or a growth figure, such as "2.55k" (REQ-4). Rust builds it. |
| **Tick** | One gridline of the axis, delivered with its value and its short label. |
| **Named series** | A series that has a user-given name and a colour slot (0 to 7), as a bank has in CSD-009. |

## 4. Requirements

### 4.0 Overview flow
```mermaid
sequenceDiagram
    actor U as User
    participant UI as Frontend (ui/)
    participant CMD as commands/
    participant DOM as domain/
    U->>UI: opens an Insights tab
    UI->>CMD: get_<area>_insights()
    CMD->>DOM: insights(today)
    DOM-->>CMD: years, axis ticks, pieces, markers and short labels
    CMD-->>UI: figures as decimal strings, labels as text
    UI->>U: draws the received values; computes and rounds nothing
```

**REQ-1: Bars and pieces** *(ubiquitous)*
The yearly chart shall draw one slot per year in the order received, with one bar per year made of the pieces received, in the order received, in which a piece with `hangs: false` rises from the zero line and a piece with `hangs: true` hangs down from it, each between its received `low` and `high`.

```gherkin
@AC-1.1
Scenario: One bar per year in the order received
  Given the series holds the years 2025 and 2026
  When the chart is drawn
  Then 2025 is on the left and 2026 on the right
  And each year has one bar

@AC-1.2
Scenario: A piece hangs below the zero line
  Given 2026 has a piece from 0.00 to 5000.00 and a piece with hangs true from -1000.00 to 0.00
  When the chart is drawn
  Then the first piece rises from the zero line
  And the second piece hangs down from the zero line

@AC-1.3
Scenario: Pieces are drawn at the received values
  Given 2026 has a piece from 2500.00 to 10500.00
  When the chart is drawn
  Then the piece spans the height between the ticks that hold 2500.00 and 10500.00
  And the frontend adds up no values to place it
```

**REQ-2: Axis and zero line** *(ubiquitous)*
The yearly chart shall draw one gridline with its short label for every tick received, from the first tick to the last, and shall always draw the zero line, also when no value is negative, with no currency.

```gherkin
@AC-2.1
Scenario: Gridlines follow the ticks
  Given the ticks 0.00, 5000.00, 10000.00 and 15000.00 with the labels "0", "5k", "10k" and "15k"
  When the chart is drawn
  Then four gridlines are drawn at those values with those labels
  And the zero line is at 0.00

@AC-2.2
Scenario: Axis below zero
  Given the ticks -1000.00, 0.00, 1000.00 and 2000.00
  When the chart is drawn
  Then the axis starts at -1000.00 and ends at 2000.00

@AC-2.3
Scenario: Only positive values
  Given the ticks start at 0.00
  When the chart is drawn
  Then the zero line is drawn at the bottom of the plot
```

**REQ-3: Rust delivers ticks, pieces and labels** *(ubiquitous)*
The app shall build in Rust, and deliver with the series, the axis ticks (equally spaced at a step of 1, 2 or 5 times a power of ten of at least 1.00, from a value at or below the lowest value to a value at or above the highest value of any year, marker included, and always including 0.00), the pieces of every year with their `low`, `high`, `hangs` and `rounding`, the marker of every year that has one, the axis label of every tick (REQ-17), and the short label of every marker (REQ-4).

```gherkin
@AC-3.1
Scenario: Axis covers the highest and the lowest value
  Given the highest value of any year is 13100.00 and the lowest is -1000.00
  When the series is requested
  Then the first tick is at or below -1000.00 and the last tick at or above 13100.00
  And the ticks include 0.00 and are equally spaced

@AC-3.4
Scenario: The step is a whole round amount
  Given the highest value of any year is 13100.00 and the lowest is 0.00
  When the series is requested
  Then the step is 2000.00 or 5000.00
  And no step has decimals

@AC-3.2
Scenario: Ready-made pieces and marker
  Given 2025 has the series segments 2000.00 and 8000.00 with closing balance 10000.00
  When the series is requested
  Then 2025 has the pieces 0.00 to 2000.00 and 2000.00 to 10000.00
  And its marker has value 10000.00 and label "10.00k"

@AC-3.3
Scenario: Year without a bar
  Given 2023 has no figures
  When the series is requested
  Then 2023 has no pieces and no marker
```

**REQ-4: Short label format** *(ubiquitous)*
The app shall write the short label of a marker or a growth figure as the amount rounded to 2 decimals half-up (CSD-000 CON-8) followed by `k` for thousands or `M` for millions, with `M` when the rounded amount in thousands reaches 1000.00, `k` when the rounded amount reaches 1,000.00, no unit and the full amount below 1,000.00, always 2 decimals, and the sign of a negative amount kept as a minus sign (a negative amount that rounds to 0.00 has no sign).

```gherkin
@AC-4.1
Scenario Outline: Short labels
  When the short label of <amount> is built
  Then it reads "<label>"

  Examples:
    | amount         | label     |
    | 0.00           | 0.00      |
    | 950.50         | 950.50    |
    | 999.99         | 999.99    |
    | 2549.99        | 2.55k     |
    | 10000.00       | 10.00k    |
    | 999994.00      | 999.99k   |
    | 1500000.00     | 1.50M     |
    | -2549.99       | -2.55k    |
    | -0.004         | 0.00      |

@AC-4.2
Scenario Outline: The boundary rounds up into the next unit
  When the short label of <amount> is built
  Then it reads "<label>" and never "<never>"

  Examples:
    | amount       | label | never    |
    | 999996.00    | 1.00M | 1000.00k |
    | 999.995      | 1.00k | 1000.00  |
    | -999996.00   | -1.00M | -1000.00k |

@AC-4.3
Scenario: Above the millions
  When the short label of 1000000000.00 is built
  Then it reads "1000.00M"
```

**REQ-5: The frontend displays, it does not compute** *(ubiquitous)*
The frontend shall display every figure and label of a chart as received, without adding, subtracting, rounding or comparing amounts to produce anything shown, and only converting received values to positions on the chart.

```gherkin
@AC-5.1
Scenario: A received label is shown as it is
  Given a received marker with value 12500.00 and label "9.99k"
  When the chart is drawn
  Then the marker shows "9.99k"

@AC-5.2
Scenario: A received tick label is shown as it is
  Given a received tick with value 5000.00 and label "five thousand"
  When the chart is drawn
  Then the gridline at 5000.00 shows "five thousand"

@AC-5.3
Scenario: Pieces are not stacked in the frontend
  Given 2026 has two pieces, 0.00 to 1000.00 and 1000.00 to 1500.00
  When the chart is drawn
  Then the second piece starts where the first ends because the received values say so
  And changing the received low of the second piece to 1200.00 moves it up with no other change
```

**REQ-6: Year labels, partial years and gap years** *(ubiquitous)*
The yearly chart shall label every slot with its year, writing "YYYY YTD" and giving the bar a lighter fill for a partial year, and showing a year without pieces as a labelled slot without a bar.

```gherkin
@AC-6.1
Scenario: Current year is marked
  Given the year 2026 is partial
  When the chart is drawn
  Then the 2026 bar has a lighter fill
  And its label reads "2026 YTD"

@AC-6.2
Scenario: Gap year
  Given the year 2023 has no pieces
  When the chart is drawn
  Then 2023 has a labelled slot without a bar
```

**REQ-7: Marker** *(optional)*
Where a year has a marker, the yearly chart shall draw a horizontal line across its bar, wider than the bar, at the marker value, with the short label received above it, not connected to the markers of other years and dashed for a partial year.

```gherkin
@AC-7.1
Scenario: Marker at its value
  Given 2025 has a marker with value 10000.00 and label "10.00k"
  When the chart is drawn
  Then a line is drawn across the 2025 bar at 10000.00
  And its label reads "10.00k"

@AC-7.2
Scenario: Not connected and dashed when partial
  Given 2024 has a marker, 2025 is a gap year and the partial year 2026 has a marker
  When the chart is drawn
  Then no line joins the markers
  And the marker of 2026 is dashed
  And 2025 has no marker
```

**REQ-8: Tooltip** *(event-driven)*
When the user hovers over or moves keyboard focus to a year, the yearly chart shall show the tooltip of that year beside its bar, to the right of the bar with its top level with the top of the bar or, when it does not fit there, to the left, always inside the chart, and, for a year without a bar, centred over its slot, with the same values given to screen readers. The area decides the rows, in the order the bar shows them from top to bottom, the segments below the zero line last, and a tooltip and a screen-reader description show no number of entries.

```gherkin
@AC-8.1
Scenario: Tooltip on hover and on keyboard focus
  Given the chart shows 2024, 2025 and 2026
  When the user hovers over the 2025 bar
  Then a tooltip with the year and its rows is shown
  When the user moves focus to the 2026 bar with the Tab key
  Then the same kind of tooltip is shown for 2026
  And it disappears when focus leaves the bar

@AC-8.2
Scenario: Placement
  Given the chart shows 2024, 2025 and 2026
  When the user hovers over the 2024 bar
  Then the tooltip is to the right of that bar with its top level with the top of the bar
  When the user hovers over the 2026 bar and the tooltip does not fit to its right
  Then the tooltip is to the left of that bar and inside the chart

@AC-8.3
Scenario: Year without a bar
  Given the year 2023 has no pieces
  When the user hovers over its slot
  Then the tooltip is centred over that slot

@AC-8.4
Scenario: Rows read like the bar
  Given 2026 has the pieces "A" 2500.00, "B" 8000.00 and a hanging piece "C" -1000.00 in the order received
  When the user hovers over the 2026 bar
  Then the tooltip rows are listed "B", "A", "C"

@AC-8.5
Scenario: Screen-reader description
  Given 2026 has a piece with the series "Example Overdraft" and value -1000.00
  When a screen reader reads the 2026 bar
  Then it announces the year and the same values as the tooltip
  And it announces that "Example Overdraft" is negative
```

**REQ-9: Highlight one series** *(event-driven)*
When the mouse is over a piece or a legend item of a series, or keyboard focus is on a legend item, the yearly chart shall keep all pieces of that series at full colour in every year, ease all other pieces and the other legend items to 70% opacity, and set the legend item and the tooltip row of that series in the full text colour and a heavier font weight. While nothing is hovered or focused, nothing is highlighted, and keyboard focus on a year highlights nothing.

```gherkin
@AC-9.1
Scenario: Hovering a piece
  Given 2025 and 2026 each have pieces of the series "A" and "B"
  When the user hovers over the "A" piece of 2025
  Then all "A" pieces, in 2025 and 2026, keep their full colour
  And all "B" pieces are eased down to 70% opacity
  And the "A" legend item is in the full text colour and a heavier font weight

@AC-9.2
Scenario: Hovering or focusing a legend item
  Given the legend lists "A" and "B"
  When the user hovers over the legend item "A" or moves keyboard focus to it
  Then the pieces of "A" keep their full colour in every year
  And it ends when the pointer or the focus leaves the item

@AC-9.3
Scenario: Leaving the piece
  Given the user hovers over a piece
  When the user moves the mouse to empty space in the year's column
  Then nothing is highlighted and the tooltip is still shown

@AC-9.4
Scenario: Keyboard focus on a year highlights nothing
  Given the chart is shown
  When the user moves focus to the 2025 bar with the Tab key
  Then nothing is highlighted
```

**REQ-10: Motion** *(event-driven)*
When a tooltip appears or moves to another year, or the highlight changes, the yearly chart shall animate the change over 120 to 150 ms with an ease-out curve, and while the user prefers reduced motion it shall show tooltips and highlight changes at once.

```gherkin
@AC-10.1
Scenario: Tooltip fades in and glides
  Given the chart is shown
  When the user hovers over the 2025 bar and then over the 2026 bar
  Then the tooltip fades in over 120 to 150 ms and glides to its new place instead of jumping

@AC-10.2
Scenario: Reduced motion
  Given the user prefers reduced motion
  When the user hovers over the 2025 bar and then over the 2026 bar
  Then each tooltip appears at once without sliding or gliding
  And the highlight changes at once
```

**REQ-11: Size** *(ubiquitous)*
The yearly chart shall have the same size and behaviour in every area: a plot height that scales with the height of the window so that, with the page scrolled to the top, the chart ends at the bottom of the window and is never lower than 260 px, and a slot at least 64 px wide, with horizontal scrolling when the slots do not fit.

```gherkin
@AC-11.1
Scenario: The plot follows the height of the window
  Given the chart is shown and the page is scrolled to the top
  When the user makes the window 200 px higher
  Then the plot is about 200 px higher and the chart still ends at the bottom of the window
  And when the window is made so low that the plot would be below 260 px, the plot stays 260 px high

@AC-11.2
Scenario: Slots are at least 64 px wide
  Given the series holds 20 years in a narrow window
  When the chart is drawn
  Then each slot is at least 64 px wide
  And the chart scrolls horizontally
```

**REQ-12: Look** *(ubiquitous)*
The yearly chart shall draw bars at most 50% of the width of their slot and at most 44 px wide, with the outer corners of a piece whose `rounding` is `top` or `bottom` rounded by 3 px and a 1 px gap between stacked pieces in place of an outline, 1 px gridlines at 55% opacity of the border colour, a 1 px zero line in the muted text colour at 70% opacity, axis and year labels in the muted text colour at regular font weight, marker labels in 10 px medium weight, and tooltips of 10 px corner radius, 12 px by 14 px padding, a shadow of at least 16 px blur and at most 12% opacity, a 12 px title in semibold and row labels in the muted text colour, in light and in dark mode.

```gherkin
@AC-12.1
Scenario: Bars
  Given the series holds the years 2024, 2025 and 2026
  When the chart is drawn
  Then no bar is wider than 50% of its slot or 44 px
  And the piece with rounding "top" has its top corners rounded by 3 px
  And the piece with rounding "bottom" has its bottom corners rounded by 3 px
  And stacked pieces are separated by a 1 px gap and have no outline

@AC-12.2
Scenario: Gridlines, zero line and labels
  Given the chart is shown with a hanging piece
  Then the gridlines are 1 px lines at 55% opacity of the border colour
  And the zero line is 1 px in the muted text colour at 70% opacity
  And the axis and year labels are in the muted text colour at regular font weight

@AC-12.3
Scenario: Tooltip
  Given the user hovers over a year
  Then the tooltip has a 10 px corner radius, 12 px by 14 px padding and a soft shadow
  And its title is semibold and its row labels are in the muted text colour
```

**REQ-13: Legend of a chart with named series** *(ubiquitous)*
A chart with named series shall show a legend with a colour swatch and the plain-text name (never HTML) of every series, in the order the area gives, wrapping onto more lines when it does not fit, with no limit on the number of series, and a name that does not fit on one legend item cut with an ellipsis, the full name staying available in a tooltip and to screen readers.

```gherkin
@AC-13.1
Scenario: Many series wrap
  Given twelve named series
  When the chart is shown in a narrow window
  Then the legend shows all twelve names, wrapped onto more than one line

@AC-13.2
Scenario: A long name is cut with an ellipsis
  Given a series with a name of 100 characters
  When the chart is shown
  Then its legend item shows the name cut with an ellipsis
  And the full name is available in a tooltip and to screen readers

@AC-13.3
Scenario: A name is not markup
  Given a series named "<b>Example</b>"
  When the chart is shown
  Then the legend shows the characters "<b>Example</b>" and no formatting
```

**REQ-14: Colour of a named series** *(ubiquitous)*
A chart with named series shall draw every piece, legend swatch and tooltip swatch of a series in the colour of its colour slot, from a palette of eight colours with a variant for light mode and one for dark mode, each of at most 60% HSL saturation, so that a series keeps its colour in every year and when other series are deleted, and a ninth series shares the colour of the first.

```gherkin
@AC-14.1
Scenario: One colour everywhere
  Given a series with colour slot 1 and figures in 2025 and 2026
  When the chart is shown
  Then its pieces in both years, its legend swatch and its tooltip swatch have the colour of slot 1

@AC-14.2
Scenario: The palette in both modes
  Given the chart is shown
  When the app is in light mode and then in dark mode
  Then the eight colours are distinct from each other in each mode
  And each has at most 60% HSL saturation

@AC-14.3
Scenario: More than eight series
  Given nine series
  When the chart is shown
  Then the first and the ninth series share a colour
  And each is named in the legend and in the tooltip
```

**REQ-15: Tabs** *(event-driven)*
When the user selects a tab, or moves between the tabs with the arrow keys, the page shall show that tab and hide the other and, when it is the Insights tab, request the figures again so the chart never shows stale figures.

```gherkin
@AC-15.1
Scenario: Switching to Insights
  Given the page shows the Overview tab
  When the user selects the Insights tab
  Then the Overview sections are hidden
  And the chart is shown

@AC-15.2
Scenario: Arrow keys move between tabs
  Given focus is on the Overview tab
  When the user presses the right arrow key
  Then the Insights tab is selected and has focus

@AC-15.3
Scenario: Figures follow changes
  Given the Insights tab showed a marker for 2026 at 2500.00
  And the user changed the 2026 figure to 3000.00 on the Overview tab
  When the user opens the Insights tab again
  Then the 2026 marker shows 3000.00
```

**REQ-16: Empty state** *(state-driven)*
While the series is empty, the Insights tab shall show the note of its area instead of a chart.

```gherkin
@AC-16.1
Scenario: Nothing to draw
  Given the series is empty
  When the user opens the Insights tab
  Then the note of the area is shown
  And no chart is drawn

@AC-16.2
Scenario: The last figure is deleted
  Given the Insights tab showed a chart
  And the user deleted the only figure on the Overview tab
  When the user opens the Insights tab again
  Then the note is shown and no chart is drawn
```

**REQ-17: Axis label format** *(ubiquitous)*
The app shall write the label of an axis tick as a whole number without decimals, in thousands with `k` when the step is at least 1,000.00 and below 1,000,000.00, in millions with `M` when the step is at least 1,000,000.00, and in full otherwise, the same unit for every tick of the axis.

```gherkin
@AC-17.1
Scenario Outline: Axis labels have no decimals
  Given the axis step is <step>
  When the label of the tick <tick> is built
  Then it reads "<label>"

  Examples:
    | step       | tick       | label |
    | 500.00     | 1500.00    | 1500  |
    | 2000.00    | 10000.00   | 10k   |
    | 2000.00    | -2000.00   | -2k   |
    | 500000.00  | 1500000.00 | 1500k |
    | 1000000.00 | 3000000.00 | 3M    |
    | 5000.00    | 0.00       | 0     |

@AC-17.2
Scenario: Markers keep their short label
  Given the axis labels are "0", "5k" and "10k"
  And the marker of 2025 has value 2549.99
  When the series is requested
  Then the marker label is "2.55k"
```

## 5. Interfaces and contracts

Every `get_<area>_insights()` command returns the common part below next to its own fields (the legend entries, the amounts for the tooltip, growth figures). Amounts are decimal strings with two decimals (CSD-000 CON-8), labels are text. Field names:

```json
{
  "axis": {
    "ticks": [
      { "value": "-1000.00", "label": "-1k" },
      { "value": "0.00", "label": "0" },
      { "value": "5000.00", "label": "5k" },
      { "value": "10000.00", "label": "10k" }
    ]
  },
  "years": [
    {
      "year": 2025,
      "is_partial": false,
      "pieces": [
        { "series": "bank-…", "low": "0.00", "high": "2000.00", "hangs": false, "rounding": "none" },
        { "series": "bank-…", "low": "2000.00", "high": "10000.00", "hangs": false, "rounding": "top" }
      ],
      "marker": { "value": "10000.00", "label": "10.00k" }
    },
    { "year": 2026, "is_partial": true, "pieces": [], "marker": null }
  ]
}
```

- `axis.ticks` is in ascending order. The first and the last tick are the ends of the axis. The tick with value `0.00` is the zero line.
- `pieces` are in drawing order. `series` is the legend key of the piece (the area decides it). `rounding` is `top`, `bottom` or `none`: `top` for the highest piece that rises, `bottom` for the lowest piece that hangs. A piece of 0.00 is not delivered.
- `marker` is `null` for a year without one.
- Labels of an area that carry a figure, such as the growth label of CSD-006 and CSD-007, are delivered the same way as strings named `<what>_label`, built by Rust. The year label ("2026 YTD") is text, translated by the frontend from `year` and `is_partial`.
- The minus sign in a label is the hyphen-minus "-", the same as in tables.
- Amounts for tooltips and screen readers are delivered as decimal strings; the frontend only adds the digit grouping of CSD-022.

## 6. Constraints
- CON-1: The charts reuse one set of shared frontend code (`ui/shared/`) and add no chart library or other dependency. Rationale: NG-2, and `CLAUDE.md` asks before adding dependencies.
- CON-2: The frontend computes no money amount and no label text: no adding, subtracting, rounding or comparing of amounts for display, not for the axis, not for the stack of pieces, not for a marker. Positions come from received values only. Rationale: `CLAUDE.md` layout rule, and one rounding rule (CON-3) in one place.
- CON-3: Rounding of every label follows CSD-000 CON-8 (2 decimals, half-up). Rationale: figures agree across screens and tests.
- CON-4: Series names are drawn as plain text, never as HTML. Rationale: a name is typed by the user and must not run as markup.
- CON-5: The palette is defined once as colour tokens per mode, and no colour of a series is written inline in the page code. Rationale: one place to adjust, and both modes stay in step.
- CON-6: No network access. Rationale: `CLAUDE.md` Tauri security rules.
- CON-7: Tests and fixtures use synthetic data only. Rationale: the repo is public.

## 7. Verification
The ticks, pieces, markers and short labels are built in Rust, so REQ-3 and REQ-4 are proven by `cargo test`, named `test_csd025_ac_<n>_<m>_<description>` (CSD-000 REQ-6). Everything the screen shows is checked by DOM tests with a received series as input (`npm test`, `ui/tests/charts.test.js`, `tabs.test.js` and `empty-states.test.js`), with the AC id in the title. The area CSDs check their own data with their own `cargo test` filter.

| What | How |
|---|---|
| Rust tests (REQ-3, REQ-4) | `cargo test test_csd025_`. One requirement, e.g. REQ-4: `cargo test test_csd025_ac_4_` |
| DOM tests | `npm test`: `charts.test.js` (AC-1.x, AC-2.x, AC-5.x to AC-14.x), `tabs.test.js` (AC-15.1 to AC-15.3), `empty-states.test.js` (AC-16.1, AC-16.2) |

Not covered by a named test yet (9 of 49 ACs; no test carries the AC id): AC-8.2, AC-8.3, AC-10.1, AC-11.1, AC-12.1 to AC-12.3, AC-13.1, AC-14.2.

Done means fmt, clippy (`-D warnings`), `cargo test --all-features` and `npm test` pass.

## 8. Open questions
- OQ-1: (resolved 03/11/2026) Rust chooses the axis ticks (REQ-3).
- OQ-2: (resolved 03/11/2026) Axis labels never show decimals (REQ-17); the short labels of markers and growth figures keep 2 decimals (REQ-4).
- OQ-3: (resolved 03/11/2026) The sign is "-" everywhere, in labels and in tables.
- OQ-4: (resolved 03/11/2026) CSD-006 and CSD-007 now point here instead of holding copies of the look, size and motion text (CSD-010 already did).

## 9. Changelog
- 0.5 (03/11/2026): OQ-4 resolved: CSD-006 1.3 and CSD-007 1.3 point to this CSD. No requirement changes.
- 0.4 (03/11/2026): Verification (section 7) lists the real test filters and the ACs that no named test covers yet; claims of tests that do not exist are removed.
- 0.3 (03/11/2026): The section 5 example shows the whole-number axis label "5k" (REQ-17). Back to `draft` until re-approved.
- 0.2 (03/11/2026): axis ticks use a whole round step and axis labels never show decimals (REQ-3: AC-3.4 added; new REQ-17, AC-17.1, AC-17.2); REQ-4 applies to markers and growth figures; the minus sign is "-" everywhere; OQ-1 to OQ-3 resolved.
- 0.1 (03/11/2026): initial draft. The yearly chart and named-series framework that the Insights CSDs repeated (bars, axis, tooltip, highlight, motion, size, look, legend, colours, tabs, empty state), the short label format and the label payload built by Rust (decision D2: the frontend computes and rounds nothing; 999,996 shows as "1.00M").
