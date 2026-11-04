//! Acceptance-criteria tests of the opening and closing of a debt (CSD-011 REQ-30
//! to REQ-33, CSD-026 REQ-15 and REQ-16). Each test is named for the one AC it
//! proves. A manual closing goes through the form path of `linking`, as the app does.

use super::*;
use crate::domain::houses::linking::{self, DebtFormInput};
use crate::domain::houses::Houses;

fn period(text: &str) -> Period {
    parse_period(text).unwrap()
}

fn code<T>(result: Result<T, DomainError>) -> &'static str {
    result.err().map_or("OK", |error| error.code())
}

fn form(name: &str, kind: &str, date: &str, amount: &str) -> DebtFormInput {
    DebtFormInput {
        debt: DebtInput {
            name: name.to_owned(),
            period: Some(kind.to_owned()),
            opening_date: date.to_owned(),
            opening_amount: amount.to_owned(),
        },
        ..DebtFormInput::default()
    }
}

fn with_closing(
    mut input: DebtFormInput,
    date: &str,
    amount: &str,
    outcome: &str,
) -> DebtFormInput {
    input.closing = ClosingInput {
        date: date.to_owned(),
        amount: amount.to_owned(),
        outcome: outcome.to_owned(),
    };
    input
}

/// Creates "Example Mortgage" (monthly) with the opening 15/03/2020 and 200000.00.
fn mortgage(debts: &mut Debts) -> DebtId {
    linking::create_debt(
        &Houses::default(),
        debts,
        &form("Example Mortgage", "Monthly", "15/03/2020", "200000.00"),
    )
    .unwrap()
    .id
}

fn add(
    debts: &mut Debts,
    id: DebtId,
    at: &str,
    amount: &str,
) -> Result<DebtStatement, DomainError> {
    debts.add_statement(
        id,
        period(at),
        DebtBalance::parse(amount).unwrap(),
        Note::default(),
    )
}

fn edit(debts: &mut Debts, id: DebtId, input: &DebtFormInput) -> Result<Debt, DomainError> {
    linking::update_debt(&Houses::default(), debts, id, input)
}

#[test]
fn test_csd011_ac_1_4_a_debt_without_an_opening_is_rejected() {
    let mut debts = Debts::default();
    let result = linking::create_debt(
        &Houses::default(),
        &mut debts,
        &form("Example Mortgage", "Monthly", "", ""),
    );
    assert_eq!(code(result), "DEBT_OPENING_DATE_REQUIRED");
    assert_eq!(debts.debts().len(), 0);
}

#[test]
fn test_csd011_ac_30_1_add_a_debt_with_its_opening() {
    let mut debts = Debts::default();
    let id = mortgage(&mut debts);
    let debt = debts.get(id).unwrap();
    assert_eq!(debt.opening_date().iso(), "2020-03-15");
    assert_eq!(debt.opening_amount().value().to_string(), "200000.00");
}

#[test]
fn test_csd011_ac_30_2_edit_the_opening() {
    let mut debts = Debts::default();
    let id = mortgage(&mut debts);
    add(&mut debts, id, "2026-09", "150000.00").unwrap();
    let input = form("Example Mortgage", "Monthly", "01/03/2020", "210000.00");
    edit(&mut debts, id, &input).unwrap();
    let debt = debts.get(id).unwrap();
    assert_eq!(debt.opening_date().iso(), "2020-03-01");
    assert_eq!(debt.opening_amount().value().to_string(), "210000.00");
    assert_eq!(debts.statement_count(id), 1);
}

#[test]
fn test_csd011_ac_30_3_a_missing_part_of_the_opening_is_rejected() {
    for (date, amount, expected) in [
        ("", "200000.00", "DEBT_OPENING_DATE_REQUIRED"),
        ("15/03/2020", "", "DEBT_OPENING_AMOUNT_REQUIRED"),
        ("", "", "DEBT_OPENING_DATE_REQUIRED"),
    ] {
        let mut debts = Debts::default();
        let result = linking::create_debt(
            &Houses::default(),
            &mut debts,
            &form("Example Mortgage", "Monthly", date, amount),
        );
        assert_eq!(code(result), expected);
        assert_eq!(debts.debts().len(), 0);
    }
}

#[test]
fn test_csd011_ac_30_4_the_opening_changes_no_figure() {
    let mut debts = Debts::default();
    let id = mortgage(&mut debts);
    let statement = add(&mut debts, id, "2026-09", "150000.00").unwrap();
    let figures = |debts: &Debts| {
        (
            debts.overview().total,
            debts.change(&statement),
            debts.yearly_totals(),
        )
    };
    let before = figures(&debts);
    assert_eq!(before.0.unwrap().to_string(), "150000.00");
    edit(
        &mut debts,
        id,
        &form("Example Mortgage", "Monthly", "15/03/2020", "250000.00"),
    )
    .unwrap();
    assert_eq!(figures(&debts), before);
}

#[test]
fn test_csd011_ac_31_1_closing_rules() {
    for (date, amount, outcome, expected) in [
        (
            "14/03/2020",
            "0.00",
            "Repaid",
            "DEBT_CLOSING_BEFORE_OPENING",
        ),
        ("10/06/2026", "", "Repaid", "DEBT_CLOSING_INCOMPLETE"),
        ("", "0.00", "Repaid", "DEBT_CLOSING_INCOMPLETE"),
        ("10/06/2026", "0.00", "", "DEBT_CLOSING_OUTCOME_REQUIRED"),
        (
            "10/06/2026",
            "1000000000.01",
            "Repaid",
            "DEBT_CLOSING_AMOUNT_TOO_LARGE",
        ),
        (
            "10/06/2026",
            "-1.00",
            "Repaid",
            "DEBT_CLOSING_AMOUNT_NEGATIVE",
        ),
        ("10/06/2026", "abc", "Repaid", "DEBT_CLOSING_AMOUNT_INVALID"),
    ] {
        let mut debts = Debts::default();
        let id = mortgage(&mut debts);
        let input = with_closing(
            form("Example Mortgage", "Monthly", "15/03/2020", "200000.00"),
            date,
            amount,
            outcome,
        );
        assert_eq!(code(edit(&mut debts, id, &input)), expected);
        assert!(debts.get(id).unwrap().settlement().is_none());
    }
}

#[test]
fn test_csd011_ac_31_2_opening_amount_and_date_rules() {
    for (date, amount, expected) in [
        ("31/02/2020", "200000.00", "DEBT_OPENING_DATE_INVALID"),
        ("15/03/2020", "-0.01", "DEBT_OPENING_AMOUNT_NEGATIVE"),
        (
            "15/03/2020",
            "1000000000.01",
            "DEBT_OPENING_AMOUNT_TOO_LARGE",
        ),
        ("15/03/2020", "abc", "DEBT_OPENING_AMOUNT_INVALID"),
    ] {
        let result = linking::create_debt(
            &Houses::default(),
            &mut Debts::default(),
            &form("Example Mortgage", "Monthly", date, amount),
        );
        assert_eq!(code(result), expected);
    }
}

#[test]
fn test_csd011_ac_31_3_same_day_future_date_and_rounding_are_accepted() {
    let mut debts = Debts::default();
    let id = linking::create_debt(
        &Houses::default(),
        &mut debts,
        &form("Example Mortgage", "Monthly", "15/03/2030", "200000.005"),
    )
    .unwrap()
    .id;
    assert_eq!(
        debts.get(id).unwrap().opening_amount().value().to_string(),
        "200000.01"
    );
    let input = with_closing(
        form("Example Mortgage", "Monthly", "15/03/2030", "200000.005"),
        "15/03/2030",
        "0.00",
        "Repaid",
    );
    edit(&mut debts, id, &input).unwrap();
    assert!(debts.get(id).unwrap().settlement().is_some());
}

#[test]
fn test_csd011_ac_31_4_an_opening_amount_of_zero_is_accepted() {
    let mut debts = Debts::default();
    let id = linking::create_debt(
        &Houses::default(),
        &mut debts,
        &form("Example Card", "Monthly", "01/01/2026", "0.00"),
    )
    .unwrap()
    .id;
    assert_eq!(
        debts.get(id).unwrap().opening_amount().value().to_string(),
        "0.00"
    );
}

#[test]
fn test_csd011_ac_32_1_a_statement_outside_the_window() {
    let mut debts = Debts::default();
    let id = mortgage(&mut debts);
    let closed = with_closing(
        form("Example Mortgage", "Monthly", "15/03/2020", "200000.00"),
        "10/06/2026",
        "0.00",
        "Repaid",
    );
    edit(&mut debts, id, &closed).unwrap();
    assert_eq!(
        code(add(&mut debts, id, "2020-02", "1000.00")),
        "DEBT_STATEMENT_OUTSIDE_WINDOW"
    );
    assert_eq!(code(add(&mut debts, id, "2020-03", "1000.00")), "OK");
    assert_eq!(code(add(&mut debts, id, "2026-06", "1000.00")), "OK");
    // After the closing the debt is locked (CSD-026 REQ-6).
    assert_eq!(
        code(add(&mut debts, id, "2026-07", "1000.00")),
        "DEBT_SETTLED_LOCKED"
    );
}

#[test]
fn test_csd011_ac_32_2_a_yearly_debt_compares_years() {
    let mut debts = Debts::default();
    let id = linking::create_debt(
        &Houses::default(),
        &mut debts,
        &form("Example Student Loan", "Yearly", "01/09/2018", "12000.00"),
    )
    .unwrap()
    .id;
    assert_eq!(code(add(&mut debts, id, "2018", "12000.00")), "OK");
    assert_eq!(
        code(add(&mut debts, id, "2017", "12000.00")),
        "DEBT_STATEMENT_OUTSIDE_WINDOW"
    );
}

#[test]
fn test_csd011_ac_32_3_moving_a_statement_out_of_the_window() {
    let mut debts = Debts::default();
    let id = mortgage(&mut debts);
    add(&mut debts, id, "2020-04", "199000.00").unwrap();
    let result = debts.update_statement(
        id,
        period("2020-04"),
        period("2020-02"),
        DebtBalance::parse("199000.00").unwrap(),
        Note::default(),
    );
    assert_eq!(code(result), "DEBT_STATEMENT_OUTSIDE_WINDOW");
    assert!(debts
        .all_statements()
        .iter()
        .all(|(_, s)| s.period == period("2020-04")));
}

#[test]
fn test_csd011_ac_32_4_an_opening_date_edit_that_would_orphan_a_statement() {
    let mut debts = Debts::default();
    let id = mortgage(&mut debts);
    add(&mut debts, id, "2020-04", "199000.00").unwrap();
    let input = form("Example Mortgage", "Monthly", "15/05/2020", "200000.00");
    assert_eq!(
        code(edit(&mut debts, id, &input)),
        "DEBT_DATES_EXCLUDE_STATEMENTS"
    );
    assert_eq!(debts.get(id).unwrap().opening_date().iso(), "2020-03-15");
}

#[test]
fn test_csd011_ac_32_5_a_closing_before_the_last_statement_is_rejected() {
    let mut debts = Debts::default();
    let id = mortgage(&mut debts);
    add(&mut debts, id, "2026-09", "150000.00").unwrap();
    let input = with_closing(
        form("Example Mortgage", "Monthly", "15/03/2020", "200000.00"),
        "10/06/2026",
        "150000.00",
        "Repaid",
    );
    assert_eq!(
        code(edit(&mut debts, id, &input)),
        "DEBT_DATES_EXCLUDE_STATEMENTS"
    );
    assert!(debts.get(id).unwrap().settlement().is_none());
}

#[test]
fn test_csd011_ac_32_6_the_settlement_statement_is_inside_the_window() {
    let mut debts = Debts::default();
    let id = mortgage(&mut debts);
    add(&mut debts, id, "2026-03", "150000.00").unwrap();
    let input = with_closing(
        form("Example Mortgage", "Monthly", "15/03/2020", "200000.00"),
        "10/06/2026",
        "150000.00",
        "Repaid",
    );
    edit(&mut debts, id, &input).unwrap();
    assert!(debts
        .all_statements()
        .iter()
        .any(|(_, s)| s.is_settlement() && s.period == period("2026-06")));
}

fn rows(debts: &Debts) -> Vec<String> {
    debts
        .statement_rows()
        .into_iter()
        .map(|row| match row {
            StatementRow::Statement(debt, s) => format!("{} {}", debt.name, s.period),
            StatementRow::Opening(debt) => {
                format!("{} opening {}", debt.name, debt.opening_date().iso())
            }
        })
        .collect()
}

#[test]
fn test_csd011_ac_33_1_the_opening_row_is_the_oldest_row_of_its_debt() {
    let mut debts = Debts::default();
    let id = mortgage(&mut debts);
    add(&mut debts, id, "2026-08", "150500.00").unwrap();
    add(&mut debts, id, "2026-09", "150000.00").unwrap();
    assert_eq!(
        rows(&debts),
        [
            "Example Mortgage 2026-09",
            "Example Mortgage 2026-08",
            "Example Mortgage opening 2020-03-15"
        ]
    );
}

#[test]
fn test_csd011_ac_33_2_the_opening_row_sits_below_a_statement_of_the_same_period() {
    let mut debts = Debts::default();
    let id = mortgage(&mut debts);
    add(&mut debts, id, "2020-03", "199000.00").unwrap();
    assert_eq!(
        rows(&debts),
        [
            "Example Mortgage 2020-03",
            "Example Mortgage opening 2020-03-15"
        ]
    );
}

#[test]
fn test_csd011_ac_33_3_a_debt_without_statements_still_has_its_opening_row() {
    let mut debts = Debts::default();
    linking::create_debt(
        &Houses::default(),
        &mut debts,
        &form("Example Card", "Monthly", "01/01/2026", "0.00"),
    )
    .unwrap();
    assert_eq!(rows(&debts), ["Example Card opening 2026-01-01"]);
}

#[test]
fn test_csd011_ac_33_4_the_opening_row_is_not_a_statement() {
    let mut debts = Debts::default();
    let id = mortgage(&mut debts);
    let statement = add(&mut debts, id, "2026-09", "150000.00").unwrap();
    assert!(debts.change(&statement).is_none());
    assert_eq!(debts.overview().total.unwrap().to_string(), "150000.00");
    assert_eq!(debts.total_statement_count(), 1);
}

#[test]
fn test_csd026_ac_15_1_a_manual_closing() {
    let mut debts = Debts::default();
    let id = linking::create_debt(
        &Houses::default(),
        &mut debts,
        &form("Example Card", "Monthly", "01/01/2025", "5000.00"),
    )
    .unwrap()
    .id;
    add(&mut debts, id, "2026-03", "800.00").unwrap();
    let input = with_closing(
        form("Example Card", "Monthly", "01/01/2025", "5000.00"),
        "10/06/2026",
        "800.00",
        "Repaid",
    );
    edit(&mut debts, id, &input).unwrap();
    let settlement = debts.get(id).unwrap().settlement().unwrap().clone();
    assert_eq!(settlement.outcome, Outcome::Repaid);
    assert_eq!(settlement.date.iso(), "2026-06-10");
    assert_eq!(settlement.amount.value().to_string(), "800.00");
    let (_, last) = debts
        .all_statements()
        .into_iter()
        .find(|(_, s)| s.is_settlement())
        .unwrap();
    assert_eq!(last.period, period("2026-06"));
    assert_eq!(last.note.as_str(), "Repaid on closing");
    assert_eq!(
        debts.overview().debts[0]
            .latest_balance
            .unwrap()
            .to_string(),
        "0.00"
    );
}

#[test]
fn test_csd026_ac_15_3_the_closing_rules_apply() {
    let mut debts = Debts::default();
    let id = linking::create_debt(
        &Houses::default(),
        &mut debts,
        &form("Example Card", "Monthly", "01/01/2025", "5000.00"),
    )
    .unwrap()
    .id;
    let input = with_closing(
        form("Example Card", "Monthly", "01/01/2025", "5000.00"),
        "31/12/2024",
        "0.00",
        "Repaid",
    );
    assert_eq!(
        code(edit(&mut debts, id, &input)),
        "DEBT_CLOSING_BEFORE_OPENING"
    );
}

#[test]
fn test_csd026_ac_15_4_a_cancelled_manual_closing_is_the_cancelled_amount() {
    let mut debts = Debts::default();
    let id = linking::create_debt(
        &Houses::default(),
        &mut debts,
        &form("Example Card", "Monthly", "01/01/2025", "5000.00"),
    )
    .unwrap()
    .id;
    add(&mut debts, id, "2025-12", "800.00").unwrap();
    let input = with_closing(
        form("Example Card", "Monthly", "01/01/2025", "5000.00"),
        "10/06/2026",
        "800.00",
        "Cancelled",
    );
    edit(&mut debts, id, &input).unwrap();
    assert_eq!(debts.cancelled_in(2026).to_string(), "800.00");
    assert_eq!(debts.cancelled_in(2025).to_string(), "0.00");
}

#[test]
fn test_csd026_ac_15_5_the_closed_debt_is_locked() {
    let mut debts = Debts::default();
    let id = mortgage(&mut debts);
    let input = with_closing(
        form("Example Mortgage", "Monthly", "15/03/2020", "200000.00"),
        "10/06/2026",
        "0.00",
        "Repaid",
    );
    edit(&mut debts, id, &input).unwrap();
    assert_eq!(
        code(add(&mut debts, id, "2026-07", "100.00")),
        "DEBT_SETTLED_LOCKED"
    );
}

#[test]
fn test_csd026_ac_16_1_clearing_removes_only_the_settlement_statement() {
    let mut debts = Debts::default();
    let id = mortgage(&mut debts);
    add(&mut debts, id, "2026-03", "800.00").unwrap();
    let closed = with_closing(
        form("Example Mortgage", "Monthly", "15/03/2020", "200000.00"),
        "10/06/2026",
        "800.00",
        "Repaid",
    );
    edit(&mut debts, id, &closed).unwrap();
    let mut clear = form("Example Mortgage", "Monthly", "15/03/2020", "200000.00");
    clear.confirm_undo = true;
    edit(&mut debts, id, &clear).unwrap();
    assert!(debts.get(id).unwrap().settlement().is_none());
    assert_eq!(debts.statement_count(id), 1);
    assert_eq!(debts.all_statements()[0].1.period, period("2026-03"));
}

#[test]
fn test_csd026_ac_16_2_clearing_without_confirmation() {
    let mut debts = Debts::default();
    let id = mortgage(&mut debts);
    let closed = with_closing(
        form("Example Mortgage", "Monthly", "15/03/2020", "200000.00"),
        "10/06/2026",
        "0.00",
        "Repaid",
    );
    edit(&mut debts, id, &closed).unwrap();
    let clear = form("Example Mortgage", "Monthly", "15/03/2020", "200000.00");
    assert_eq!(
        code(edit(&mut debts, id, &clear)),
        "DEBT_CLOSING_UNDO_UNCONFIRMED"
    );
    assert!(debts.get(id).unwrap().settlement().is_some());
}
