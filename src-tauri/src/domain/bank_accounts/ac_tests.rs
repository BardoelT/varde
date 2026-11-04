//! Acceptance-criteria tests of CSD-009 and CSD-010 that the other test files carry no AC id for.

use super::*;
use crate::domain::common::YearMonth;
use crate::domain::money::Money;

fn today() -> YearMonth {
    YearMonth::parse("2026-10").unwrap()
}

fn period(text: &str) -> Period {
    parse_period(text).unwrap()
}

fn account(banks: &mut BankAccounts, name: &str, kind: &str) -> BankAccountId {
    banks
        .create_account(&BankAccountInput {
            name: name.to_owned(),
            period: Some(kind.to_owned()),
        })
        .unwrap()
        .id
}

fn add(banks: &mut BankAccounts, id: BankAccountId, at: &str, amount: &str, note: &str) {
    banks
        .add_entry(
            id,
            period(at),
            BankBalance::parse(amount).unwrap(),
            bank_note(note).unwrap(),
        )
        .unwrap();
}

// ---- CSD-009 ----

#[test]
fn test_csd009_ac_12_2_an_emptied_note_clears_the_note() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2026-09", "2500.00", "First");
    assert_eq!(banks.all_entries()[0].1.note.as_str(), "First");
    banks
        .update_entry(
            id,
            period("2026-09"),
            period("2026-09"),
            BankBalance::parse("2500.00").unwrap(),
            bank_note("").unwrap(),
        )
        .unwrap();
    let entries = banks.all_entries();
    assert_eq!(
        entries[0].1.note.as_str(),
        "",
        "the entry has an empty note"
    );
    assert_eq!(entries[0].1.balance.value().to_string(), "2500.00");
}

#[test]
fn test_csd009_ac_17_6_an_older_entry_does_not_change_the_latest_balance() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2026-09", "2500.00", "");
    add(&mut banks, id, "2026-01", "900.00", "");
    let overview = banks.overview();
    let shown = &overview.accounts[0];
    assert_eq!(shown.latest_balance.unwrap().to_string(), "2500.00");
    assert_eq!(shown.latest_period.unwrap().to_string(), "2026-09");
    assert_eq!(overview.total.unwrap().to_string(), "2500.00");
}

#[test]
fn test_csd009_ac_18_3_a_yearly_entry_counts_as_december() {
    let mut banks = BankAccounts::default();
    let monthly = account(&mut banks, "Example Current Account", "Monthly");
    let yearly = account(&mut banks, "Example Savings", "Yearly");
    add(&mut banks, monthly, "2025-11", "1.00", "");
    add(&mut banks, monthly, "2025-12", "1.00", "");
    add(&mut banks, yearly, "2025", "1.00", "");
    let order: Vec<String> = banks
        .all_entries()
        .iter()
        .map(|(account, entry)| format!("{} {}", account.name, entry.period))
        .collect();
    assert_eq!(
        order,
        [
            "Example Current Account 2025-12",
            "Example Savings 2025",
            "Example Current Account 2025-11",
        ],
        "the entry for 2025-12 and the entry for 2025 come before the entry for 2025-11"
    );
}

// ---- CSD-010 ----

#[test]
fn test_csd010_ac_4_2_the_current_year_without_entries_is_in_the_series_and_partial() {
    let mut banks = BankAccounts::default();
    let id = account(&mut banks, "Example Current Account", "Monthly");
    add(&mut banks, id, "2025-12", "100.00", "");
    let years = banks.insights(today()).years;
    assert_eq!(
        years.iter().map(|y| y.year).collect::<Vec<_>>(),
        [2025, 2026]
    );
    assert!(years[1].is_partial, "2026 is marked partial");
    assert_eq!(years[1].entry_count, 0, "and has 0 entries");
}

#[test]
fn test_csd010_ac_6_4_with_only_positive_balances_the_axis_starts_at_zero() {
    let mut banks = BankAccounts::default();
    let current = account(&mut banks, "Example Current Account", "Monthly");
    let savings = account(&mut banks, "Example Savings", "Yearly");
    add(&mut banks, current, "2025-12", "2000.00", "");
    add(&mut banks, savings, "2026", "8000.00", "");
    let insights = banks.insights(today());
    assert!(insights
        .years
        .iter()
        .flat_map(|y| y.segments.iter())
        .all(|s| !s.balance.is_negative()));
    let first = insights.axis.ticks.first().unwrap();
    assert_eq!(first.value.to_string(), "0.00");
    assert!(insights.axis.ticks.last().unwrap().value.to_string() != "0.00");
}

#[test]
fn test_csd010_ac_7_1_the_marker_is_at_the_closing_balance_with_a_short_label() {
    let mut banks = BankAccounts::default();
    let current = account(&mut banks, "Example Current Account", "Monthly");
    let savings = account(&mut banks, "Example Savings", "Monthly");
    add(&mut banks, current, "2025-12", "2000.00", "");
    add(&mut banks, savings, "2025-12", "8000.00", "");
    let year = &banks.insights(today()).years[0];
    assert_eq!(year.closing_balance.to_string(), "10000.00");
    let marker = year.marker.as_ref().unwrap();
    assert_eq!(marker.value.to_string(), "10000.00");
    assert_eq!(marker.label, "10.00k");
}

#[test]
fn test_csd010_ac_7_2_the_marker_sits_at_the_net_sum_when_a_segment_is_negative() {
    let mut banks = BankAccounts::default();
    let current = account(&mut banks, "Example Current Account", "Monthly");
    let overdraft = account(&mut banks, "Example Overdraft", "Monthly");
    add(&mut banks, current, "2026-06", "5000.00", "");
    add(&mut banks, overdraft, "2026-06", "-1000.00", "");
    let year = banks
        .insights(today())
        .years
        .into_iter()
        .find(|y| y.year == 2026)
        .unwrap();
    let marker = year.marker.as_ref().unwrap();
    assert_eq!(marker.value.to_string(), "4000.00");
    let top = year.pieces.iter().map(|p| p.high).max().unwrap();
    assert_eq!(top.to_string(), "5000.00");
    assert!(
        marker.value > Money::ZERO && marker.value < top,
        "inside the positive stack"
    );
}
