//! Acceptance-criteria tests of CSD-011 and CSD-012 that the other test files of this area
//! do not carry under their AC id. Each test is named for the one AC it proves.

use super::*;
use crate::domain::money::Money;
use crate::domain::YearMonth;

fn period(text: &str) -> Period {
    parse_period(text).unwrap()
}

fn today() -> YearMonth {
    YearMonth::parse("2026-10").unwrap()
}

fn balance(text: &str) -> DebtBalance {
    DebtBalance::parse(text).unwrap()
}

fn debt(debts: &mut Debts, name: &str, kind: &str) -> DebtId {
    debts
        .create_debt(&DebtInput {
            name: name.to_owned(),
            period: Some(kind.to_owned()),
        })
        .unwrap()
        .id
}

fn add(debts: &mut Debts, id: DebtId, at: &str, amount: &str) {
    debts
        .add_statement(id, period(at), balance(amount), Note::default())
        .unwrap();
}

fn money(text: &str) -> Money {
    Money::parse(text).unwrap()
}

fn code<T>(result: Result<T, DomainError>) -> &'static str {
    result.err().map_or("OK", |error| error.code())
}

fn years(debts: &Debts) -> Vec<i32> {
    debts
        .insights(today())
        .years
        .iter()
        .map(|y| y.year)
        .collect()
}

// ---- CSD-011 ----

#[test]
fn test_csd011_ac_9_2_a_yearly_statement_for_a_monthly_debt_is_rejected() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    let result = debts.add_statement(id, period("2025"), balance("1000.00"), Note::default());
    assert_eq!(code(result), "DEBT_PERIOD_MISMATCH");
    assert_eq!(debts.statement_count(id), 0);
}

#[test]
fn test_csd011_ac_11_2_line_breaks_in_a_note_are_kept() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    debts
        .add_statement(
            id,
            period("2026-09"),
            balance("1000.00"),
            debt_note("line one\nline two").unwrap(),
        )
        .unwrap();
    let rows = debts.all_statements();
    assert_eq!(rows[0].1.note.as_str(), "line one\nline two");
    assert_eq!(rows[0].1.note.as_str().lines().count(), 2);
}

#[test]
fn test_csd011_ac_12_2_a_note_can_be_cleared() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    debts
        .add_statement(
            id,
            period("2026-09"),
            balance("150000.00"),
            debt_note("First").unwrap(),
        )
        .unwrap();
    debts
        .update_statement(
            id,
            period("2026-09"),
            period("2026-09"),
            balance("150000.00"),
            debt_note("").unwrap(),
        )
        .unwrap();
    let rows = debts.all_statements();
    assert_eq!(rows[0].1.note.as_str(), "");
    assert_eq!(rows[0].1.balance.value(), money("150000.00"));
}

#[test]
fn test_csd011_ac_15_2_copy_previous_for_a_debt_without_statements_gives_nothing() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    let prefill = debts.prefill_from_latest(id).unwrap();
    assert_eq!(prefill.balance, None);
}

#[test]
fn test_csd011_ac_15_3_a_balance_of_zero_is_copied_as_it_is() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2026-08", "0.00");
    let prefill = debts.prefill_from_latest(id).unwrap();
    assert_eq!(
        prefill.balance.map(|value| value.to_string()),
        Some("0.00".to_owned())
    );
}

#[test]
fn test_csd011_ac_17_6_an_older_statement_does_not_change_the_latest_balance() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2026-09", "150000.00");
    add(&mut debts, id, "2026-01", "160000.00");
    let overview = debts.overview();
    assert_eq!(overview.debts[0].latest_balance, Some(money("150000.00")));
    assert_eq!(overview.debts[0].latest_period, Some(period("2026-09")));
    assert_eq!(overview.total, Some(money("150000.00")));
}

#[test]
fn test_csd011_ac_18_3_a_yearly_statement_counts_as_december() {
    let mut debts = Debts::default();
    let monthly = debt(&mut debts, "Example Mortgage", "Monthly");
    let yearly = debt(&mut debts, "Example Student Loan", "Yearly");
    add(&mut debts, monthly, "2025-11", "100.00");
    add(&mut debts, monthly, "2025-12", "100.00");
    add(&mut debts, yearly, "2025", "100.00");
    let order: Vec<(String, String)> = debts
        .all_statements()
        .iter()
        .map(|(d, s)| (d.name.clone(), s.period.to_string()))
        .collect();
    assert_eq!(
        order,
        [
            ("Example Mortgage".to_owned(), "2025-12".to_owned()),
            ("Example Student Loan".to_owned(), "2025".to_owned()),
            ("Example Mortgage".to_owned(), "2025-11".to_owned()),
        ]
    );
}

// ---- CSD-012 ----

#[test]
fn test_csd012_ac_2_2_a_single_year() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2026-09", "100.00");
    assert_eq!(years(&debts), [2026]);
}

#[test]
fn test_csd012_ac_2_3_no_statements_means_an_empty_series() {
    let mut debts = Debts::default();
    debt(&mut debts, "Example Mortgage", "Monthly");
    let insights = debts.insights(today());
    assert_eq!(insights.years.len(), 0);
    assert_eq!(insights.debts.len(), 0);
}

#[test]
fn test_csd012_ac_2_4_a_statement_after_the_current_year_extends_the_series() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2024-03", "100.00");
    add(&mut debts, id, "2027-05", "100.00");
    assert_eq!(years(&debts), [2024, 2025, 2026, 2027]);
}

#[test]
fn test_csd012_ac_2_5_a_very_old_statement_gets_no_slot_but_still_counts() {
    let mut debts = Debts::default();
    let mortgage = debt(&mut debts, "Example Mortgage", "Monthly");
    let loan = debt(&mut debts, "Example Loan", "Monthly");
    add(&mut debts, mortgage, "1950-01", "1000.00");
    add(&mut debts, loan, "2026-03", "500.00");
    let insights = debts.insights(today());
    let listed: Vec<i32> = insights.years.iter().map(|y| y.year).collect();
    assert_eq!(listed.len(), 50);
    assert_eq!((listed[0], listed[49]), (1977, 2026));
    assert!(
        insights
            .years
            .iter()
            .all(|year| year.segments.iter().all(|s| s.debt_id != mortgage)),
        "no year of the series has a segment for the 1950 statement"
    );
    let totals = debts.yearly_totals();
    assert!(totals.iter().any(|total| total.year == 1950));
}

#[test]
fn test_csd012_ac_4_1_the_current_year_is_marked_partial() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2025-05", "100.00");
    add(&mut debts, id, "2026-05", "100.00");
    let insights = debts.insights(today());
    let partial: Vec<(i32, bool)> = insights
        .years
        .iter()
        .map(|y| (y.year, y.is_partial))
        .collect();
    assert_eq!(partial, [(2025, false), (2026, true)]);
}

#[test]
fn test_csd012_ac_4_3_a_future_year_is_not_partial() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Yearly");
    add(&mut debts, id, "2025", "100.00");
    add(&mut debts, id, "2027", "100.00");
    let insights = debts.insights(today());
    let partial: Vec<(i32, bool)> = insights
        .years
        .iter()
        .map(|y| (y.year, y.is_partial))
        .collect();
    assert_eq!(partial, [(2025, false), (2026, true), (2027, false)]);
}

#[test]
fn test_csd012_ac_7_2_the_marker_sits_at_the_top_of_the_stack() {
    let mut debts = Debts::default();
    let a = debt(&mut debts, "Example Mortgage", "Yearly");
    let b = debt(&mut debts, "Example Loan", "Yearly");
    add(&mut debts, a, "2025", "2000.00");
    add(&mut debts, b, "2025", "8000.00");
    let insights = debts.insights(today());
    let year = &insights.years[0];
    let top = year.pieces.iter().map(|piece| piece.high).max().unwrap();
    assert_eq!(top, money("10000.00"));
    assert_eq!(year.marker.as_ref().unwrap().value, top);
}

#[test]
fn test_csd012_ac_15_1_the_figures_follow_a_change_of_the_statements() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2026-03", "9000.00");
    let before = debts.insights(today());
    let marker = |insights: &DebtInsights| {
        insights
            .years
            .last()
            .unwrap()
            .marker
            .as_ref()
            .unwrap()
            .value
    };
    assert_eq!(marker(&before), money("9000.00"));
    add(&mut debts, id, "2026-10", "8500.00");
    assert_eq!(marker(&debts.insights(today())), money("8500.00"));
}

#[test]
fn test_csd012_ac_15_2_after_the_last_statement_is_deleted_there_is_nothing_to_draw() {
    let mut debts = Debts::default();
    let id = debt(&mut debts, "Example Mortgage", "Monthly");
    add(&mut debts, id, "2026-03", "9000.00");
    assert_eq!(debts.insights(today()).years.len(), 1);
    debts.delete_statement(id, period("2026-03")).unwrap();
    let insights = debts.insights(today());
    assert!(insights.years.is_empty(), "no chart is drawn");
    assert_eq!(insights.debts.len(), 0);
}
