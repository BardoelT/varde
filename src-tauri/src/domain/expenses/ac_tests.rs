//! Acceptance tests of CSD-013 that no other test in this area carries by name.

use super::*;

fn input(name: &str, amount: &str, date: &str, category: &str) -> ExpenseInput {
    ExpenseInput {
        name: name.to_owned(),
        amount: amount.to_owned(),
        date: date.to_owned(),
        category: category.to_owned(),
        note: None,
    }
}

/// AC-3.2: today is in 2026 and the only expense is from 2025: 2026 is in the series, partial and empty.
#[test]
fn test_csd014_ac_3_2_the_current_year_without_expenses_is_in_the_series_and_marked() {
    let mut expenses = Expenses::default();
    expenses
        .add_expense(&input("Example Trip", "250.00", "15/12/2025", "Holiday"))
        .unwrap();
    let result = expenses.insights(2026);
    let years: Vec<i32> = result.years.iter().map(|year| year.year).collect();
    assert_eq!(years, [2025, 2026]);
    let current = &result.years[1];
    assert!(current.is_partial);
    assert_eq!(current.total.to_string(), "0.00");
    assert_eq!(current.expense_count, 0);
    assert_eq!(current.segments.len(), 0);
}

/// CSD-014 AC-13.3: an edit that moves an expense to another year moves its segment with it.
#[test]
fn test_csd014_ac_13_3_a_moved_expense_follows_to_its_new_year() {
    let mut expenses = Expenses::default();
    let id = expenses
        .add_expense(&input("Example Sofa", "800.00", "10/01/2026", "Home"))
        .unwrap()
        .id;
    let before = expenses.insights(2026);
    assert_eq!(before.years.len(), 1);

    expenses
        .update_expense(id, &input("Example Sofa", "800.00", "20/12/2025", "Home"))
        .unwrap();
    let after = expenses.insights(2026);
    let years: Vec<i32> = after.years.iter().map(|year| year.year).collect();
    assert_eq!(years, [2025, 2026], "the series holds 2025 and 2026");
    assert_eq!(after.years[0].segments.len(), 1, "the segment is in 2025");
    assert_eq!(after.years[0].total.to_string(), "800.00");
    assert_eq!(after.years[1].segments.len(), 0, "and no longer in 2026");
}

/// AC-8.3: deleting the last expense of a year removes the year from the yearly totals.
#[test]
fn test_csd013_ac_8_3_deleting_the_last_expense_of_a_year_removes_the_year() {
    let mut expenses = Expenses::default();
    let only_2025 = expenses
        .add_expense(&input("Example Bike", "900.00", "10/09/2025", "Hobby"))
        .unwrap()
        .id;
    expenses
        .add_expense(&input(
            "Example Roof Repair",
            "4500.00",
            "15/09/2026",
            "Home",
        ))
        .unwrap();
    let years = |expenses: &Expenses| {
        expenses
            .yearly_totals()
            .iter()
            .map(|total| total.year)
            .collect::<Vec<_>>()
    };
    assert_eq!(years(&expenses), [2026, 2025]);

    expenses.delete_expense(only_2025).unwrap();
    assert_eq!(years(&expenses), [2026], "2025 is no longer listed");
    assert_eq!(expenses.total_count(), 1);
}
