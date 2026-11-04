---
id: CSD-007
title: Investment insights
status: draft
owner: Thijs Bardoel
version: 1.4
last_updated: 03/11/2026
related: [CSD-000, CSD-003, CSD-005, CSD-006, CSD-025]
---

# CSD-007: Investment insights

## 1. Intent
The Investments page lists accounts, statements and a table of yearly totals, but a table does not show how an investment pot grew: how much of the change came from money paid in and how much from the market. This capability adds an insights layer to the Investments page: a chart with one bar per year that starts at the balance the year opened with, adds net deposits, and ends with the capital growth, together with a marker at the closing balance and the growth as a percentage of the opening balance. It is for the owner looking back over their own investments. It follows the Income insights of CSD-006 and supersedes CSD-005 NG-4 for this view only, and CSD-005 NG-3 for the growth percentage of this view only.

## 2. Scope
### 2.1 In scope
- An "Insights" tab on the Investments page, next to the existing content ("Overview").
- A yearly stacked bar chart over all accounts, with a closing-balance marker, a growth percentage per year and a tooltip.
- Its empty state.

### 2.2 Non-goals
- NG-1: A monthly view, or any granularity other than a calendar year.
- NG-2: A filter or a breakdown per account.
- NG-3: Any return figure other than the growth percentage of REQ-6: annualised returns, time-weighted returns, a percentage in the Yearly totals table or anywhere else, and forecasts.
- NG-4: Click-through, zooming, or exporting the chart.
- NG-5: Charts on the Dashboard or on the Net worth page, which has its own chart (CSD-017).
- NG-6: A chart library or any other new dependency.
- NG-7: Estimating balances for years without a statement (CSD-005 NG-7).

## 3. Definitions
| Term | Meaning |
|---|---|
| **Year** | A calendar year. A statement counts in the year of its own period only, as in the yearly totals (CSD-005 REQ-26). |
| **Current year** | The calendar year of today, in local time. Its figures run up to the latest statement so far, and are therefore partial. |
| **Opening balance** | For a year: the sum, over the accounts that have a statement in that year, of the balance of each such account's latest statement before that year. An account without an earlier statement adds nothing: it is taken to start at 0.00 (CSD-005), so its balance shows up as net deposits and capital growth. |
| **Net deposits, capital growth, closing balance** | As in CSD-005: the yearly totals of REQ-26. |
| **Growth percentage** | The capital growth of a year, counting only the accounts that have an opening balance in that year, divided by the opening balance, in percent, with two decimals (CSD-000 CON-8). It leaves net deposits out and ignores when in the year they arrived, so it is a simple yearly figure and not a time-weighted return. |

## 4. Requirements

### 4.0 Overview flow
```mermaid
sequenceDiagram
    actor U as User
    participant UI as Frontend (ui/)
    participant CMD as commands/
    participant DOM as domain/
    U->>UI: opens the Insights tab
    UI->>CMD: get_investment_insights()
    CMD->>DOM: insights(today)
    DOM-->>CMD: one figure set per year, with growth percentage
    CMD-->>UI: figures as decimal strings
    UI->>U: stacked bars, closing markers, growth percentages
```

**REQ-1: Tabs on the Investments page** *(ubiquitous)*
The Investments page shall offer the tabs "Overview" and "Insights" while at least one investment account exists, with Overview, which holds the existing content, shown first.

```gherkin
@AC-1.1
Scenario: Overview is shown first
  Given a project with one investment account
  When the user opens the Investments page
  Then the Overview tab is selected
  And it shows the Accounts, Yearly totals and Statements sections

@AC-1.2
Scenario: Switching to Insights
  Given the Investments page shows the Overview tab
  When the user selects the Insights tab
  Then the Overview sections are hidden
  And the insights chart is shown

@AC-1.3
Scenario: No accounts yet
  Given a project without investment accounts
  When the user opens the Investments page
  Then the prompt to add the first investment account is shown
  And there are no tabs
```

**REQ-2: Year series** *(ubiquitous)*
The app shall return one set of figures for every calendar year from the year of the earliest statement up to and including the later of the current year and the year of the latest statement, but for at most 50 years back from that last year, oldest year first, holding the opening balance, the net deposits, the capital growth, the closing balance and the number of statements (returned, never displayed). A statement older than the 50 years gets no slot, but still counts in the opening balance of the years that follow it (section 3) and in the yearly totals of CSD-005 REQ-26.

```gherkin
@AC-2.1
Scenario: Series runs from the first statement to the current year
  Given today is 26/10/2026
  And the earliest statement is for 2024-03
  And there are statements in 2025 and 2026
  When the insights are requested
  Then the series holds the years 2024, 2025 and 2026 in that order

@AC-2.2
Scenario: A single year
  Given today is 26/10/2026
  And the only statement is for 2026-09
  When the insights are requested
  Then the series holds only the year 2026

@AC-2.3
Scenario: No statements
  Given today is 26/10/2026
  And an account without statements
  When the insights are requested
  Then the series is empty

@AC-2.4
Scenario: A statement after the current year extends the series
  Given today is 26/10/2026
  And statements for 2024-03 and 2027-05
  When the insights are requested
  Then the series holds the years 2024, 2025, 2026 and 2027

@AC-2.5
Scenario: A very old statement gets no slot but still counts
  Given today is 26/10/2026
  And an account with a statement for 1950-12 (balance 1000.00, net deposits 1000.00) and one for 2026-06 (balance 1500.00, net deposits 200.00)
  When the insights are requested
  Then the series holds the years 1977 to 2026, which is 50 years
  And 1950 to 1976 have no slot
  And the year 2026 has opening balance 1000.00, net deposits 200.00, capital growth 300.00 and closing balance 1500.00
  And the yearly totals of CSD-005 REQ-26 still hold a row for 1950
```

**REQ-3: Years without statements** *(unwanted behaviour)*
If a year in the series has no statement, then the app shall return that year with all amounts 0.00 and 0 statements.

```gherkin
@AC-3.1
Scenario: Gap year
  Given today is 26/10/2026
  And there are statements in 2022 and 2024 but none in 2023
  When the insights are requested
  Then the year 2023 has opening balance 0.00, net deposits 0.00, capital growth 0.00, closing balance 0.00 and 0 statements
  And the series holds the years 2022, 2023, 2024, 2025 and 2026

@AC-3.2
Scenario: The year after a gap still opens with the balance from before the gap
  Given today is 26/10/2026
  And an account with a statement for 2024-12 (balance 10000.00) and one for 2026-06 (balance 11000.00, net deposits 500.00)
  When the insights are requested
  Then the year 2025 has no statements
  And the year 2026 has opening balance 10000.00, net deposits 500.00, capital growth 500.00 and closing balance 11000.00
```

**REQ-4: Current year is partial** *(state-driven)*
While a year is the current year, the app shall mark it as partial. A year after the current year is not marked partial and looks like any other year.

```gherkin
@AC-4.1
Scenario: Current year is marked
  Given today is 26/10/2026
  And there are statements in 2025 and 2026
  When the insights are requested
  Then the year 2026 is marked partial
  And the year 2025 is not

@AC-4.2
Scenario: Current year without statements is still in the series and marked
  Given today is 26/10/2026
  And the only statement is for 2025-12
  When the insights are requested
  Then the series holds 2025 and 2026
  And 2026 is marked partial and has 0 statements

@AC-4.3
Scenario: A future year is not marked partial
  Given today is 26/10/2026
  And a statement for 2027-05
  When the insights are requested
  Then the year 2027 is not marked partial
```

**REQ-5: The parts add up to the closing balance** *(ubiquitous)*
The app shall compute for each year the opening balance as defined in section 3, take the net deposits, the capital growth and the closing balance from the yearly totals of CSD-005 REQ-26, and the opening balance plus the net deposits plus the capital growth shall equal the closing balance.

```gherkin
@AC-5.1
Scenario: Parts of a year
  Given today is 26/10/2026
  And a monthly account with statements 2025-11 (balance 11000.00, net deposits 11000.00), 2025-12 (balance 11500.00, net deposits 200.00) and 2026-05 (balance 12500.00, net deposits 300.00)
  When the insights are requested
  Then 2025 has opening balance 0.00, net deposits 11200.00, capital growth 300.00 and closing balance 11500.00
  And 2026 has opening balance 11500.00, net deposits 300.00, capital growth 700.00 and closing balance 12500.00

@AC-5.6
Scenario: A first statement without net deposits
  Given today is 26/10/2026
  And a monthly account with a statement 2026-03 (balance 1020.00, net deposits 1000.00)
  When the insights are requested
  Then 2026 has opening balance 0.00, net deposits 1000.00, capital growth 20.00 and closing balance 1020.00

@AC-5.2
Scenario: An account that starts in a later year
  Given today is 26/10/2026
  And account "Example Brokerage" with statements 2025-12 (balance 12000.00, net deposits 12000.00) and 2026-03 (balance 12600.00, net deposits 100.00)
  And account "Example Crypto" with a first statement 2026-03 (balance 500.00, net deposits 400.00)
  When the insights are requested
  Then 2026 has opening balance 12000.00, net deposits 500.00, capital growth 600.00 and closing balance 13100.00

@AC-5.3
Scenario: An account without a statement in the year is left out
  Given today is 26/10/2026
  And a monthly account "Example Brokerage" with statements 2025-12 (balance 12000.00, net deposits 12000.00) and 2026-03 (balance 12500.00)
  And a yearly account "Example Pension" with a statement 2025 (balance 80000.00, net deposits 80000.00)
  When the insights are requested
  Then 2025 has opening balance 0.00, net deposits 92000.00 and closing balance 92000.00
  And 2026 has opening balance 12000.00, net deposits 0.00, capital growth 500.00 and closing balance 12500.00

@AC-5.4
Scenario: A net withdrawal and a loss
  Given today is 26/10/2026
  And a monthly account with statements 2025-12 (balance 12000.00, net deposits 12000.00) and 2026-03 (balance 11000.00, net deposits -500.00)
  When the insights are requested
  Then 2026 has opening balance 12000.00, net deposits -500.00, capital growth -500.00 and closing balance 11000.00

@AC-5.5
Scenario: Net deposits and closing balance equal the yearly totals
  Given today is 26/10/2026
  And the statements of AC-5.1 and AC-5.2
  When the insights and the yearly totals are requested
  Then for every year the closing balance, the net deposits and the capital growth of the insights equal those of the yearly totals
```

**REQ-6: Growth percentage** *(ubiquitous)*
The app shall calculate the growth percentage of a year as the capital growth of the accounts that have an opening balance in that year ÷ the opening balance × 100, rounded to two decimal places half-up (CSD-000 CON-8), where a decrease that rounds to 0.00 is given as 0.00 without a sign.

```gherkin
@AC-6.1
Scenario Outline: Growth percentage per year
  Given today is 26/10/2026
  And the opening balance of 2025 is <opening>
  And the capital growth of 2025 is <growth>, all of it from accounts that are in the opening balance
  When the insights are requested
  Then the growth percentage of 2025 is "<percent>"

  Examples:
    | opening   | growth   | percent |
    | 11500.00  | 700.00   | 6.09    |
    | 40000.00  | -1000.00 | -2.50   |
    | 20000.00  | 90.00    | 0.45    |
    | 20000.00  | 0.00     | 0.00    |
    | 40000.00  | 2.00     | 0.01    |
    | 100000.00 | -1.00    | 0.00    |

@AC-6.2
Scenario: Growth of an account without an opening balance is left out of the percentage
  Given today is 26/10/2026
  And account "Example Brokerage" with statements 2025-12 (balance 12000.00, net deposits 12000.00) and 2026-03 (balance 12600.00, net deposits 100.00)
  And account "Example Crypto" with a first statement 2026-03 (balance 500.00, net deposits 400.00)
  When the insights are requested
  Then the capital growth of 2026 is 600.00
  And the growth percentage of 2026 is "4.17"
```

**REQ-7: No growth percentage available** *(unwanted behaviour)*
If the opening balance of a year is 0.00 or the year has no statements, then the app shall give it no growth percentage.

```gherkin
@AC-7.1
Scenario: First year has no percentage
  Given today is 26/10/2026
  And the earliest statement is in 2025
  When the insights are requested
  Then the year 2025 has no growth percentage

@AC-7.2
Scenario: Gap year has no percentage
  Given today is 26/10/2026
  And there are statements in 2022 and 2024 but none in 2023
  When the insights are requested
  Then the year 2023 has no growth percentage

@AC-7.3
Scenario: The current year has a percentage
  Given today is 26/10/2026
  And the opening balance of 2026 is 12500.00 and the capital growth of 2026 so far is 250.00
  When the insights are requested
  Then the growth percentage of 2026 is "2.00"
  And 2026 is marked partial

@AC-7.4
Scenario: A year in which only new accounts have statements has no percentage
  Given today is 26/10/2026
  And account "Example Brokerage" with a statement for 2025-12 (balance 12000.00) and no statement in 2026
  And account "Example Crypto" with a first statement 2026-03 (balance 500.00)
  When the insights are requested
  Then the opening balance of 2026 is 0.00
  And the year 2026 has no growth percentage
```

**REQ-8: Chart** *(ubiquitous)*
The Insights tab shall draw one bar per year in the order received, stacked from the bottom as opening balance, net deposits and capital growth, in which a positive segment rises from the running total and a negative segment hangs down from it, with capital growth in a positive colour when it is positive and a negative colour when it is negative and a negative net deposit in a lighter tint of the net deposits colour with a thin dashed outline, on a y-axis that starts at 0 (or at a rounded-down negative value if a running total is negative) and ends at a rounded-up value of at least the highest running total, with gridlines, short axis labels whose text is provided by the backend (format in CSD-025), a legend with one item for each piece (Opening balance, Net deposits, Capital growth (gain), Capital growth (loss)) and one for the closing-balance marker, and no currency.

```gherkin
@AC-8.1
Scenario: Stacked bars per year
  Given the series holds the years 2025 and 2026
  When the user opens the Insights tab
  Then two bars are drawn, 2025 on the left and 2026 on the right
  And each bar is stacked from opening balance, net deposits and capital growth
  And the legend has the items Opening balance, Net deposits, Capital growth (gain), Capital growth (loss) and Closing balance

@AC-8.2
Scenario: Growth colour follows the sign
  Given 2025 has capital growth 300.00 and 2026 has capital growth -500.00
  When the user opens the Insights tab
  Then the growth segment of 2025 uses the positive colour
  And the growth segment of 2026 uses the negative colour

@AC-8.3
Scenario: Negative segments hang down
  Given 2026 has opening balance 12000.00, net deposits -500.00 and capital growth -500.00
  When the user opens the Insights tab
  Then the net deposits segment of 2026 hangs down from 12000.00 to 11500.00 in a lighter tint with a thin dashed outline
  And the growth segment hangs down from 11500.00 to 11000.00 in the negative colour

@AC-8.4
Scenario: Axis covers the highest level
  Given the highest running total of any year is 13100.00
  When the user opens the Insights tab
  Then the top of the y-axis is at least 13100.00
  And the axis starts at 0

@AC-8.5
Scenario: Gap year is an empty slot
  Given the year 2023 has no statements
  When the user opens the Insights tab
  Then 2023 has a labelled slot without a bar

@AC-8.6
Scenario: Current year is marked
  Given the year 2026 is partial
  When the user opens the Insights tab
  Then the 2026 bar has a lighter fill
  And its label reads "2026 YTD"
```

**REQ-9: Closing-balance marker** *(ubiquitous)*
The Insights tab shall draw for each year with statements a horizontal marker line of 2 px with round ends in the muted text colour, wider than the bar, at the closing balance, with its short value label next to it (above the line), the label text being provided by the backend (format in CSD-025), not connected to the markers of other years, and dashed for the current year.

```gherkin
@AC-9.1
Scenario: Marker at the closing balance
  Given 2025 has closing balance 11500.00
  When the user opens the Insights tab
  Then a marker line is drawn across the 2025 bar at 11500.00
  And its label is the short label provided by the backend for 11500.00

@AC-9.2
Scenario: Marker sits at the top of a gaining bar and the bottom of a losing growth segment
  Given 2025 has capital growth 300.00 and 2026 has capital growth -500.00
  When the user opens the Insights tab
  Then the 2025 marker sits at the top of the stack
  And the 2026 marker sits at the lower end of the growth segment

@AC-9.3
Scenario: Markers are not connected and the current year is dashed
  Given the series holds 2024, a gap year 2025 and the partial year 2026
  When the user opens the Insights tab
  Then no line joins the markers
  And the marker of 2026 is dashed
  And 2025 has no marker
```

**REQ-10: Growth percentage labels** *(ubiquitous)*
The Insights tab shall show the growth percentage of each year as a second line under its year label as "+x.xx%" or "-x.xx%", "0.00%" for no change (also for a decrease that rounds to 0.00, without a sign) and "–" for no growth percentage, using the negative colour only for decreases.

```gherkin
@AC-10.1
Scenario Outline: Growth percentage label
  Given the growth percentage of a year is <percent>
  When the user opens the Insights tab
  Then the second line under the label of that year reads "<label>"

  Examples:
    | percent | label  |
    | 6.09    | +6.09% |
    | -2.50   | -2.50% |
    | 0.00    | 0.00%  |
    | none    | –      |

@AC-10.2
Scenario: Only decreases are red
  Given the growth percentage of 2025 is -2.50 and of 2026 is 6.09
  When the user opens the Insights tab
  Then the percentage of 2025 uses the negative colour
  And the percentage of 2026 does not
```

**REQ-11: Details of a year** *(event-driven)*
When the user hovers over or moves keyboard focus to a year, the Insights tab shall show a tooltip with the year's opening balance, net deposits, capital growth, growth percentage and closing balance, in the order and with the empty-year note that REQ-14 lays out, and shall give the same values to screen readers. The tooltip and the screen-reader description show no number of statements.

```gherkin
@AC-11.1
Scenario: Tooltip on hover
  Given 2026 has opening balance 11500.00, net deposits 300.00, capital growth 700.00, closing balance 12500.00 and growth percentage 6.09
  When the user hovers over the 2026 bar
  Then a tooltip shows those five values
  And it shows no number of statements

@AC-11.2
Scenario: Tooltip on keyboard focus
  Given the Insights tab shows the chart
  When the user moves focus to the 2026 bar with the Tab key
  Then the same tooltip is shown
  And the tooltip disappears when focus leaves the bar

@AC-11.3
Scenario: Accessible label
  Given 2026 has capital growth -500.00
  When a screen reader reads the 2026 bar
  Then it announces the year, the five values and that the capital growth is negative
  And it announces no number of statements
```

**REQ-12: No statements yet** *(state-driven)*
While at least one account exists but no statement, the Insights tab shall show a note instead of a chart.

```gherkin
@AC-12.1
Scenario: Accounts without statements
  Given a project with one account and no statements
  When the user opens the Insights tab
  Then the note "No statements yet. The chart appears once there is a statement." is shown
  And no chart is drawn
```

**REQ-13: Current figures** *(event-driven)*
When the user opens the Insights tab, the app shall show figures computed from the project at that moment.

```gherkin
@AC-13.1
Scenario: Figures follow changes
  Given the Insights tab showed closing balance 12500.00 for 2026
  And the user added a statement for 2026-09 with balance 13000.00 on the Overview tab
  When the user opens the Insights tab again
  Then the 2026 marker shows 13000.00

@AC-13.2
Scenario: Last statement deleted
  Given the Insights tab showed a chart
  And the user deleted the only statement on the Overview tab
  When the user opens the Insights tab again
  Then the note of AC-12.1 is shown and no chart is drawn
```

### 4.1 Legend and highlight

**REQ-14: Legend with the selected year** *(event-driven)*
When the user hovers over or moves keyboard focus to a year, the Insights tab shall show in the tooltip of that year the colour swatch of the legend on the rows of opening balance, net deposits, capital growth and closing balance, with, on the capital growth row, the swatch of a gain when the capital growth is not negative and the swatch of a loss when it is, and shall list the rows of the tooltip in the order the bar shows them, top to bottom: Capital growth, Net deposits, Opening balance, then Closing balance, then Growth (%), and shall place the tooltip beside the year's bar, to its right with its top aligned to the top of the bar, to its left when there is no room on the right, and inside the chart area, or centred over its slot when the year has no bar, where it shows the note "No statements" in place of the rows. The legend in the panel header stays.

```gherkin
@AC-14.1
Scenario: Swatches match the legend
  Given 2025 has opening balance 10700.00, net deposits 500.00, capital growth 3800.00 and closing balance 15000.00
  When the user hovers over the 2025 bar
  Then the tooltip rows Opening balance, Net deposits, Capital growth and Closing balance each show the swatch of the same colour or line as in the legend
  And the Capital growth row shows the gain swatch
  And the row Growth (%) shows no swatch

@AC-14.2
Scenario: A loss shows the loss swatch
  Given 2023 has capital growth -800.00
  When the user hovers over the 2023 bar
  Then the Capital growth row shows the loss swatch

@AC-14.3
Scenario: Zero growth shows the gain swatch
  Given 2024 has statements and capital growth 0.00
  When the user hovers over the 2024 bar
  Then the Capital growth row shows the gain swatch

@AC-14.4
Scenario: Tooltip beside the bar
  Given the chart shows the years 2024, 2025 and 2026
  When the user hovers over the 2024 bar
  Then the tooltip is shown to the right of that bar
  And its top is level with the top of that bar

@AC-14.5
Scenario: Tooltip flips near the right edge
  Given the chart shows the years 2024, 2025 and 2026
  When the user hovers over the 2026 bar and the tooltip does not fit to its right
  Then the tooltip is shown to the left of that bar
  And it stays inside the chart area

@AC-14.9
Scenario: Rows follow the bar from top to bottom
  Given 2025 has opening balance 10700.00, net deposits 500.00, capital growth 3800.00, closing balance 15000.00 and growth percentage 35.51
  When the user hovers over the 2025 bar
  Then the tooltip rows are listed Capital growth, Net deposits, Opening balance, Closing balance, Growth (%) in that order
  And no row is listed above Capital growth

@AC-14.6
Scenario: Year without a bar
  Given the year 2023 has no statements
  When the user hovers over its slot
  Then the tooltip is centred over that slot
  And it shows no swatch
  And it shows the note "No statements" and no rows

@AC-14.7
Scenario: Header legend stays
  Given the Insights tab shows the chart
  When the user hovers over a year
  Then the legend in the panel header is still shown

@AC-14.8
Scenario: Keyboard focus shows the same tooltip
  Given the Insights tab shows the chart
  When the user moves focus to the 2025 bar with the Tab key
  Then the tooltip is shown beside that bar with the same swatches
```

**REQ-15: Highlight while hovering** *(event-driven)*
In addition to the highlight of one series of CSD-025 REQ-9, the Insights tab shall, while the mouse is over the closing-balance marker or the Closing balance item of the legend, thicken the marker of every year from 2 px to 3 px, and shall give gain and loss their own legend items.

```gherkin
@AC-15.2
Scenario: Hovering the closing-balance marker
  Given 2025 has closing balance 15000.00
  When the user hovers over its marker
  Then the Closing balance item is set in the full text colour and a heavier font weight in the tooltip and in the header legend
  And the markers of every year thicken from 2 px to 3 px

@AC-15.3
Scenario: Gain and loss are separate legend items
  Given 2025 has capital growth 3800.00 and 2023 has capital growth -800.00
  When the user hovers over the capital growth segment of 2023
  Then the Capital growth (loss) item is highlighted in the header legend
  And the Capital growth (gain) item is not highlighted
  And only the loss segments, in every year, keep their full colour, so the gain segment of 2025 is eased down to 70% opacity
```

# @AC-15.1 and AC-15.4 to AC-15.10 (removed in v1.3): replaced by CSD-025 AC-9.1 to AC-9.4.

**REQ-16: Motion** *(removed in v1.3)*
Moved to CSD-025 REQ-10. AC-16.1 to AC-16.3 are replaced by CSD-025 AC-10.1.

**REQ-17: Reduced motion** *(removed in v1.3)*
Moved to CSD-025 REQ-10. AC-17.1 is replaced by CSD-025 AC-10.2.

**REQ-18: Colours and withdrawals** *(ubiquitous)*
The Insights tab shall draw the chart with the size and the look of CSD-025 REQ-11 and REQ-12, in which the opening balance is a neutral slate blue-grey, the net deposits the accent blue, a capital gain green and a capital loss coral red, and the net withdrawal segment a lighter tint of the net deposits colour with a thin dashed outline, so that neighbouring series can be told apart.

```gherkin
@AC-18.3
Scenario: Marker and withdrawals
  Given 2025 has a net withdrawal
  Then its marker is a 2 px line with round ends in the muted text colour
  And its marker value is 10 px in medium weight
  And the net withdrawal segment is a lighter tint of the net deposits colour with a thin dashed outline

@AC-18.6
Scenario: Colour roles in both modes
  Given the Insights tab shows a chart with a gain year and a loss year
  When the app is in light mode and then in dark mode
  Then the opening balance is drawn in a neutral slate blue-grey
  And the net deposits are drawn in the accent blue
  And the capital gain is drawn in green and the capital loss in coral red
  And each segment is clearly distinguishable from the segment next to it
```

# @AC-18.1, AC-18.2, AC-18.4, AC-18.5 and AC-18.7 (removed in v1.3): replaced by CSD-025 AC-11.x and AC-12.x.

## 5. Interfaces and contracts

Command: `get_investment_insights()` returns the series, oldest year first:

```json
{
  "years": [
    { "year": 2025, "opening_balance": "0.00", "net_deposits": "11200.00", "capital_growth": "300.00", "growth_percent": null, "closing_balance": "11500.00", "statement_count": 2, "is_partial": false },
    { "year": 2026, "opening_balance": "11500.00", "net_deposits": "300.00", "capital_growth": "700.00", "growth_percent": "6.09", "closing_balance": "12500.00", "statement_count": 1, "is_partial": true }
  ]
}
```

`statement_count` is returned but never displayed. `years` is empty when there are no statements. `growth_percent` is a decimal string with two decimals or `null`. Amounts are decimal strings. The text of the short axis labels and of the marker value labels is built by Rust and delivered with the series, and the frontend only displays it (format: CSD-025). Rust decides "today" (local time), the years, the partial flag and the growth percentage, and the frontend only displays what it receives.

## 6. Constraints
- CON-1: Figures and the growth percentage are computed in Rust and displayed as received; the frontend only scales values to positions on the chart. Rationale: `CLAUDE.md` layout rule.
- CON-2: Amounts and the percentage use `rust_decimal::Decimal` and are rounded as CSD-000 CON-8 states (two decimals, half-up). Rationale: `CLAUDE.md` money rules and one rounding rule project-wide.
- CON-3: The chart is plain SVG drawn in `ui/`, with no chart library, no new crate and no network access. Rationale: `CLAUDE.md` dependency and Tauri security rules.
- CON-4: Chart colours come from CSS variables and are legible in light and dark mode, and no meaning is carried by colour alone (signs and labels state it as well). Rationale: the app already supports both modes, and colour-blind readers.
- CON-5: The Overview tab behaves as before, and the income insights chart behaves exactly as before; helpers shared by both charts may move into a common `ui/` module without changing the income chart's output. Rationale: this capability adds a view and changes no existing behaviour.
- CON-6: Tests and fixtures use synthetic data only. Rationale: the repo is public.
- CON-7: The figures are computed from the stored statements each time they are needed and are never stored in the project file. Rationale: stored figures would go stale when a statement changes.
- CON-8: The highlight of CSD-025 REQ-9 and of REQ-15 is drawn with CSS classes and carries no meaning by colour alone. Rationale: readers who cannot tell the colours apart, and no new dependency.
- CON-9: Motion is drawn with CSS transitions only and respects the reduced-motion preference of CSD-025 REQ-10. Rationale: no new dependency, and users who ask for less motion.

## 7. Verification
The figures are proven by `cargo test`. Tests are named `test_csd007_ac_<n>_<m>_<description>` (CSD-000 REQ-6).

| What | How |
|---|---|
| Rust tests (data) | `cargo test test_csd007_` |
| Display ACs (REQ-8 to REQ-18 and AC-1.1 to AC-1.3) | DOM tests in `ui/tests/ac-b-investment-insights.test.js` (`npm test`). Checked by hand: where the tooltip sits beside a bar and its flip near the right edge (AC-14.4, AC-14.5) and its centring over the slot of a year without a bar (AC-14.6, the other steps are tested), because jsdom has no layout. The shared chart behaviour has DOM tests under CSD-025 |

Not covered by a named test yet (4 of 55 ACs; no test carries the AC id): AC-14.1, AC-14.4, AC-14.5, AC-14.9. AC-14.4 and AC-14.5 are checked by hand (layout). AC-14.1 and AC-14.9 name the tooltip row "Growth (%)" while the screen shows "Growth"; the test waits for that to be settled.

Done means fmt, clippy (`-D warnings`), `cargo test --all-features` and `npm test` pass.

## 8. Open questions
None.

## 9. Changelog
- 1.4 (04/11/2026): New AC-7.4: a year in which only new accounts have statements has an opening balance of 0.00 and therefore no growth percentage (REQ-7). The behaviour was already built and tested; the AC states it. Back to `draft` until re-approved.
- 1.3 (03/11/2026): The copies of the look, size and motion text are gone; this CSD points to CSD-025 (REQ-15 reduced to the marker thickening and the gain and loss legend items, REQ-16 and REQ-17 removed in favour of CSD-025 REQ-10, REQ-18 reduced to the colour roles and the withdrawal segment). Back to `draft` until re-approved.
- 1.2 (03/11/2026): Verification (section 7) lists the real test filters and the ACs that no named test covers yet; claims of tests that do not exist are removed.
- 1.1 (03/11/2026): The series runs to the later of the current year and the latest year with a statement, so future years get a slot, and spans at most 50 years back from its last year, older statements still count in the opening balance and the yearly totals (REQ-2, new AC-2.4, AC-2.5; owner decision, CSD-005 OQ-4 resolved). A future year is not partial (REQ-4, new AC-4.3). Back to `draft` until re-approved.
- 1.0 (03/11/2026): The growth percentage is rounded to two decimals half-up instead of one decimal with banker's rounding (CSD-000 CON-8): REQ-6 (AC-6.1 values and two new rows, AC-6.2), AC-7.3, REQ-10 (label "+x.xx%", AC-10.1, AC-10.2), AC-11.1, AC-14.9, the JSON example and CON-2; a decrease that rounds to 0.00 shows no sign. The text of the short axis labels and the marker value labels is provided by the backend, the format is in CSD-025 (REQ-8, REQ-9, AC-9.1, section 5). Back to draft for re-approval.
- 0.8 (29/10/2026): The plot height scales with the height of the window instead of being a fixed 390 px: with the page scrolled to the top the chart ends at the bottom of the window, and the plot is never lower than 260 px. It is recomputed when the window is resized.
- 0.7 (29/10/2026): The Reports page is replaced by the Net worth page of CSD-017 (NG-5; GitHub issue 24). No other change.
- 0.6 (29/10/2026): The plot of every yearly chart is 390 px high (was 260 px), 50% taller. No other behaviour change.
- 0.5 (29/10/2026): Statement counts are no longer displayed: REQ-11 (AC-11.1, AC-11.3) and REQ-14 (AC-14.1, AC-14.6) drop the count from the tooltip and the screen-reader description, an empty year shows "No statements"; the count stays in the data (REQ-2, section 5). REQ-14 orders the tooltip rows as the bar shows them, top to bottom (AC-14.9 added). REQ-8 (AC-8.1) and REQ-15 (AC-15.1 to AC-15.3 amended, AC-15.8 to AC-15.10 added): the legend has one hoverable and focusable item per piece, and hovering or focusing a legend item, or hovering a segment, highlights that series in every year. REQ-18 sets the shared chart size, an always-drawn zero line and the colour roles (AC-18.6, AC-18.7 added). New ACs are manual checklist items.
- 0.4 (27/10/2026): Quieter and more refined charts. REQ-8 (AC-8.3) and REQ-9: the hatch becomes a lighter tint with a dashed outline and the marker a 2 px muted line. REQ-15 (AC-15.1, AC-15.2, AC-15.6) replaces the chip and outline by a full-colour segment, eased dimming of the rest and a heavier legend label. New REQ-16 motion, REQ-17 reduced motion, REQ-18 refined look (AC-16.1 to AC-18.5), CON-9. The tooltip placement of REQ-14 is unchanged. Approved by the owner. Same change for the income chart in CSD-006 0.3.
- 0.3 (27/10/2026): The first statement of an account has a capital growth (CSD-005 0.8), so the "New accounts" segment is gone: it is removed from the definitions, REQ-2, REQ-3, REQ-5, REQ-8, REQ-11, REQ-14, REQ-15, the legend, the tooltip and the `get_investment_insights()` payload. REQ-5 gets AC-5.6, the growth percentage counts only accounts with an opening balance (REQ-6, AC-6.2). Approved by the owner.
- 0.2 (27/10/2026): Legend with the selected year and highlight on hover. New section 4.1 with REQ-14 (swatches in the tooltip, tooltip beside the bar, AC-14.1 to AC-14.8) and REQ-15 (AC-15.1 to AC-15.7), CON-8. REQ-11 points to REQ-14. Approved by the owner. Same change for the income chart in CSD-006 0.2.
- 0.1 (27/10/2026): initial draft, approved by the owner. REQ-9 clarified while implementing: the marker value sits above the line, so it never collides with the next year.
