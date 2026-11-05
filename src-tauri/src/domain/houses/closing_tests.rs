//! Tests for the closing of a debt by hand, its edit in place and the debts of a sold
//! house: CSD-026 REQ-1, REQ-4 to REQ-6 and REQ-15 to REQ-18, CSD-020 REQ-16 and REQ-17.
//! Each test is named for the one AC it proves. All names and amounts are synthetic.

use crate::domain::common::Note;
use crate::domain::debts::{
    debt_note, parse_period, DebtBalance, DebtCategory, DebtId, DebtInput, Origin, Outcome,
};
use crate::domain::error::DomainError;
use crate::domain::houses::linking::{self, DebtFormInput, DebtLinksInput, SettlementChoice};
use crate::domain::houses::{HouseId, HouseInput};
use crate::domain::ProjectData;

fn code<T>(result: Result<T, DomainError>) -> &'static str {
    match result {
        Ok(_) => "OK",
        Err(error) => error.code(),
    }
}

fn house_input(name: &str, sold: Option<(&str, &str)>) -> HouseInput {
    HouseInput {
        name: name.to_owned(),
        period: Some("Monthly".to_owned()),
        purchase_date: Some("15/03/2024".to_owned()),
        purchase_amount: Some("250000.00".to_owned()),
        sell_date: sold.map(|(date, _)| date.to_owned()),
        sell_amount: sold.map(|(_, amount)| amount.to_owned()),
    }
}

fn links(choices: &[(DebtId, &str)], confirm_undo: bool) -> DebtLinksInput {
    DebtLinksInput {
        settlements: choices
            .iter()
            .map(|(id, outcome)| SettlementChoice {
                debt_id: id.to_string(),
                outcome: (*outcome).to_owned(),
            })
            .collect(),
        confirm_undo,
    }
}

/// A house, held or already sold on the date given, with no debts.
fn house(data: &mut ProjectData, name: &str, sold_on: Option<&str>) -> HouseId {
    let sold = sold_on.map(|date| (date, "300000.00"));
    linking::create_house(
        &mut data.houses,
        &mut data.debts,
        &house_input(name, sold),
        &links(&[], false),
    )
    .unwrap()
    .id
}

/// Saves the Edit house form of `id` with the sale given and the choices.
fn save_house(
    data: &mut ProjectData,
    id: HouseId,
    sold_on: Option<&str>,
    choices: &[(DebtId, &str)],
    confirm_undo: bool,
) -> Result<(), DomainError> {
    let name = data.houses.get(id).unwrap().name.clone();
    let sold = sold_on.map(|date| (date, "300000.00"));
    linking::update_house(
        &mut data.houses,
        &mut data.debts,
        id,
        &house_input(&name, sold),
        &links(choices, confirm_undo),
    )
    .map(|_| ())
}

/// A monthly debt of the category given (opening 01/01/2000 and 200000.00), linked to a house
/// or not, with no statements.
fn new_debt(data: &mut ProjectData, name: &str, category: &str, house: Option<HouseId>) -> DebtId {
    linking::create_debt(
        &data.houses,
        &mut data.debts,
        &DebtFormInput {
            debt: DebtInput {
                name: name.to_owned(),
                period: Some("Monthly".to_owned()),
                ..DebtInput::example()
            },
            category: Some(category.to_owned()),
            house_id: house.map(|house| house.to_string()),
            ..DebtFormInput::default()
        },
    )
    .unwrap()
    .id
}

fn statement(data: &mut ProjectData, id: DebtId, at: &str, balance: &str) {
    if data.debts.statement_count(id) == 0 {
        let first = DebtInput::first_balance(data.debts.get(id).unwrap(), balance);
        data.debts.update_debt(id, &first).unwrap();
    }
    data.debts
        .add_statement(
            id,
            parse_period(at).unwrap(),
            DebtBalance::parse(balance).unwrap(),
            debt_note("").unwrap(),
        )
        .unwrap();
}

/// What the debt form sends on a save: the closing fields as typed, the house and the category.
struct Save<'a> {
    category: &'a str,
    house: Option<HouseId>,
    closing: Option<(&'a str, &'a str, &'a str)>,
    confirm_undo: bool,
    opening: Option<(&'a str, &'a str)>,
    name: Option<&'a str>,
}

impl<'a> Save<'a> {
    fn new(category: &'a str, house: Option<HouseId>) -> Self {
        Self {
            category,
            house,
            closing: None,
            confirm_undo: false,
            opening: None,
            name: None,
        }
    }

    fn closing(mut self, date: &'a str, amount: &'a str, outcome: &'a str) -> Self {
        self.closing = Some((date, amount, outcome));
        self
    }

    fn confirmed(mut self) -> Self {
        self.confirm_undo = true;
        self
    }

    fn run(&self, data: &mut ProjectData, id: DebtId) -> Result<(), DomainError> {
        let debt = data.debts.get(id).unwrap();
        let (opening_date, opening_amount) = self.opening.unwrap_or(("01/01/2000", "200000.00"));
        let mut form = DebtFormInput {
            debt: DebtInput {
                name: self.name.map_or_else(|| debt.name.clone(), str::to_owned),
                period: None,
                opening_date: opening_date.to_owned(),
                opening_amount: opening_amount.to_owned(),
            },
            category: Some(self.category.to_owned()),
            house_id: self.house.map(|house| house.to_string()),
            confirm_undo: self.confirm_undo,
            ..DebtFormInput::default()
        };
        if let Some((date, amount, outcome)) = self.closing {
            form.closing.date = date.to_owned();
            form.closing.amount = amount.to_owned();
            form.closing.outcome = outcome.to_owned();
        }
        linking::update_debt(&data.houses, &mut data.debts, id, &form).map(|_| ())
    }
}

/// The settlement of a debt as outcome, origin, ISO date and amount, if it has one.
fn settlement(data: &ProjectData, id: DebtId) -> Option<(Outcome, Origin, String, String)> {
    data.debts.get(id).unwrap().settlement().map(|s| {
        (
            s.outcome,
            s.origin,
            s.date.iso(),
            s.amount.value().to_string(),
        )
    })
}

/// The settlement statements of a debt as period and note.
fn settlement_statements(data: &ProjectData, id: DebtId) -> Vec<(String, String)> {
    data.debts
        .all_statements()
        .into_iter()
        .filter(|(debt, statement)| debt.id == id && statement.is_settlement())
        .map(|(_, statement)| {
            (
                statement.period.to_string(),
                statement.note.as_str().to_owned(),
            )
        })
        .collect()
}

fn regular_statements(data: &ProjectData, id: DebtId) -> Vec<(String, String)> {
    data.debts
        .all_statements()
        .into_iter()
        .filter(|(debt, statement)| debt.id == id && !statement.is_settlement())
        .map(|(_, statement)| {
            (
                statement.period.to_string(),
                statement.balance.value().to_string(),
            )
        })
        .collect()
}

/// A held house with "Example Mortgage" (2026-03: 190000.00) and "Example Loan"
/// (2026-03: 10000.00) linked to it.
fn house_with_two_debts() -> (ProjectData, HouseId, DebtId, DebtId) {
    let mut data = ProjectData::default();
    let house = house(&mut data, "Example House", None);
    let mortgage = new_debt(&mut data, "Example Mortgage", "Mortgage", Some(house));
    let loan = new_debt(&mut data, "Example Loan", "Loan", Some(house));
    statement(&mut data, mortgage, "2026-03", "190000.00");
    statement(&mut data, loan, "2026-03", "10000.00");
    (data, house, mortgage, loan)
}

/// "Example House" sold on 10/06/2026 with "Example Loan" (2026-03: 4000.00) settled as Cancelled
/// by the sale.
fn sold_house_with_settled_loan() -> (ProjectData, HouseId, DebtId) {
    let mut data = ProjectData::default();
    let house = house(&mut data, "Example House", None);
    let loan = new_debt(&mut data, "Example Loan", "Loan", Some(house));
    statement(&mut data, loan, "2026-03", "4000.00");
    save_house(
        &mut data,
        house,
        Some("10/06/2026"),
        &[(loan, "Cancelled")],
        false,
    )
    .unwrap();
    (data, house, loan)
}

// ---- CSD-026 REQ-1: a sale skips closed debts ----

#[test]
fn test_csd026_ac_1_5_a_debt_that_is_closed_already_is_skipped() {
    let (mut data, house, mortgage, loan) = house_with_two_debts();
    Save::new("Loan", Some(house))
        .closing("15/05/2026", "10000.00", "Repaid")
        .run(&mut data, loan)
        .unwrap();
    let open = linking::open_debts(&data.debts, house, "10/06/2026").unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].debt_id, mortgage);
    save_house(
        &mut data,
        house,
        Some("10/06/2026"),
        &[(mortgage, "Repaid")],
        false,
    )
    .unwrap();
    let (_, origin, _, _) = settlement(&data, mortgage).unwrap();
    assert_eq!(origin, Origin::Sale);
    assert_eq!(
        settlement(&data, loan),
        Some((
            Outcome::Repaid,
            Origin::Manual,
            "2026-05-15".to_owned(),
            "10000.00".to_owned()
        ))
    );
}

#[test]
fn test_csd026_ac_1_6_a_choice_for_a_debt_that_is_closed_already_is_rejected() {
    let (mut data, house, mortgage, loan) = house_with_two_debts();
    Save::new("Loan", Some(house))
        .closing("15/05/2026", "10000.00", "Repaid")
        .run(&mut data, loan)
        .unwrap();
    let result = save_house(
        &mut data,
        house,
        Some("10/06/2026"),
        &[(mortgage, "Repaid"), (loan, "Cancelled")],
        false,
    );
    assert_eq!(code(result), "HOUSE_SETTLEMENT_INVALID");
    assert!(data.houses.get(house).unwrap().is_held());
    assert!(!data.debts.is_settled(mortgage));
}

// ---- CSD-026 REQ-4: undo leaves manual closings alone ----

#[test]
fn test_csd026_ac_4_4_undo_leaves_a_manual_closing_alone() {
    let (mut data, house, mortgage, loan) = house_with_two_debts();
    Save::new("Loan", Some(house))
        .closing("15/05/2026", "10000.00", "Repaid")
        .run(&mut data, loan)
        .unwrap();
    save_house(
        &mut data,
        house,
        Some("10/06/2026"),
        &[(mortgage, "Repaid")],
        false,
    )
    .unwrap();
    save_house(&mut data, house, None, &[], true).unwrap();
    assert!(settlement(&data, mortgage).is_none());
    assert_eq!(
        settlement(&data, loan).map(|(outcome, ..)| outcome),
        Some(Outcome::Repaid)
    );
    assert_eq!(settlement_statements(&data, loan).len(), 1);
}

#[test]
fn test_csd026_ac_4_5_undo_leaves_an_edited_closing_alone() {
    let (mut data, house, loan) = sold_house_with_settled_loan();
    Save::new("Loan", Some(house))
        .closing("20/05/2026", "4000.00", "Cancelled")
        .run(&mut data, loan)
        .unwrap();
    save_house(&mut data, house, None, &[], true).unwrap();
    assert_eq!(
        settlement(&data, loan),
        Some((
            Outcome::Cancelled,
            Origin::Manual,
            "2026-05-20".to_owned(),
            "4000.00".to_owned()
        ))
    );
}

// ---- CSD-026 REQ-5: only a closing by sale locks the sell date ----

#[test]
fn test_csd026_ac_5_3_a_manual_closing_does_not_lock_the_sell_date() {
    let mut data = ProjectData::default();
    let house = house(&mut data, "Example House", None);
    let loan = new_debt(&mut data, "Example Loan", "Loan", Some(house));
    statement(&mut data, loan, "2026-03", "4000.00");
    Save::new("Loan", Some(house))
        .closing("15/05/2026", "4000.00", "Repaid")
        .run(&mut data, loan)
        .unwrap();
    save_house(&mut data, house, Some("10/06/2026"), &[], false).unwrap();
    save_house(&mut data, house, Some("11/06/2026"), &[], false).unwrap();
    assert_eq!(
        data.houses.get(house).unwrap().sale.unwrap().date.iso(),
        "2026-06-11"
    );
}

#[test]
fn test_csd026_ac_5_4_editing_the_closing_by_sale_releases_the_lock() {
    let (mut data, house, loan) = sold_house_with_settled_loan();
    assert_eq!(
        code(save_house(&mut data, house, Some("11/06/2026"), &[], false)),
        "HOUSE_SELL_DATE_LOCKED"
    );
    Save::new("Loan", Some(house))
        .closing("10/06/2026", "4000.00", "Repaid")
        .run(&mut data, loan)
        .unwrap();
    save_house(&mut data, house, Some("11/06/2026"), &[], false).unwrap();
    assert_eq!(
        data.houses.get(house).unwrap().sale.unwrap().date.iso(),
        "2026-06-11"
    );
    assert_eq!(settlement(&data, loan).unwrap().2, "2026-06-10");
}

// ---- CSD-026 REQ-6: what stays editable ----

#[test]
fn test_csd026_ac_6_4_name_opening_category_and_link_stay_editable_on_a_settled_debt() {
    let (mut data, house, loan) = sold_house_with_settled_loan();
    let mut save = Save::new("Mortgage", Some(house));
    save.name = Some("Example Home Loan");
    save.opening = Some(("01/03/2020", "210000.00"));
    save.closing = Some(("10/06/2026", "4000.00", "Cancelled"));
    save.run(&mut data, loan).unwrap();
    let debt = data.debts.get(loan).unwrap();
    assert_eq!(debt.name, "Example Home Loan");
    assert_eq!(debt.opening_date().iso(), "2020-03-01");
    assert_eq!(debt.category(), DebtCategory::Mortgage);
    assert_eq!(
        settlement(&data, loan),
        Some((
            Outcome::Cancelled,
            Origin::Sale,
            "2026-06-10".to_owned(),
            "4000.00".to_owned()
        ))
    );
}

#[test]
fn test_csd026_ac_6_1_the_statement_period_of_a_settled_debt_is_locked() {
    let (mut data, _house, loan) = sold_house_with_settled_loan();
    let result = data.debts.update_debt(
        loan,
        &DebtInput {
            name: "Example Loan".to_owned(),
            period: Some("Yearly".to_owned()),
            ..DebtInput::example()
        },
    );
    assert!(result.is_err());
    assert_eq!(
        data.debts
            .add_statement(
                loan,
                parse_period("2026-07").unwrap(),
                DebtBalance::parse("1.00").unwrap(),
                Note::default()
            )
            .unwrap_err()
            .code(),
        "DEBT_SETTLED_LOCKED"
    );
}

// ---- CSD-026 REQ-15: close by hand ----

#[test]
fn test_csd026_ac_15_2_a_debt_linked_to_a_held_house_can_be_closed_by_hand() {
    let mut data = ProjectData::default();
    let house = house(&mut data, "Example House", None);
    let loan = new_debt(&mut data, "Example Loan", "Loan", Some(house));
    statement(&mut data, loan, "2026-03", "4000.00");
    Save::new("Loan", Some(house))
        .closing("10/06/2026", "4000.00", "Repaid")
        .run(&mut data, loan)
        .unwrap();
    assert_eq!(
        settlement(&data, loan),
        Some((
            Outcome::Repaid,
            Origin::Manual,
            "2026-06-10".to_owned(),
            "4000.00".to_owned()
        ))
    );
    assert_eq!(
        settlement_statements(&data, loan),
        vec![("2026-06".to_owned(), "Repaid on closing".to_owned())]
    );
    assert_eq!(data.debts.house_of(loan), Some(house));
    assert!(data.houses.get(house).unwrap().is_held());
}

#[test]
fn test_csd026_ac_15_6_a_debt_linked_to_a_sold_house_can_be_closed_on_any_date() {
    let mut data = ProjectData::default();
    let house = house(&mut data, "Example House", Some("10/06/2026"));
    let loan = new_debt(&mut data, "Example Loan", "Loan", Some(house));
    statement(&mut data, loan, "2026-09", "3000.00");
    Save::new("Loan", Some(house))
        .closing("15/09/2026", "3000.00", "Cancelled")
        .run(&mut data, loan)
        .unwrap();
    assert_eq!(
        settlement(&data, loan),
        Some((
            Outcome::Cancelled,
            Origin::Manual,
            "2026-09-15".to_owned(),
            "3000.00".to_owned()
        ))
    );
    assert!(data.debts.validate_links(&data.houses).is_ok());
}

#[test]
fn test_csd026_ac_15_7_a_closed_debt_has_one_settlement() {
    let mut data = ProjectData::default();
    let card = new_debt(&mut data, "Example Card", "Other", None);
    Save::new("Other", None)
        .closing("10/06/2026", "800.00", "Repaid")
        .run(&mut data, card)
        .unwrap();
    Save::new("Other", None)
        .closing("11/06/2026", "800.00", "Repaid")
        .run(&mut data, card)
        .unwrap();
    assert_eq!(settlement(&data, card).unwrap().2, "2026-06-11");
    assert_eq!(settlement_statements(&data, card).len(), 1);
}

// ---- CSD-026 REQ-16: clear any closing ----

#[test]
fn test_csd026_ac_16_3_a_closing_by_sale_can_be_cleared_in_the_debt_form() {
    let (mut data, house, loan) = sold_house_with_settled_loan();
    Save::new("Loan", Some(house))
        .confirmed()
        .run(&mut data, loan)
        .unwrap();
    assert!(settlement(&data, loan).is_none());
    assert_eq!(
        regular_statements(&data, loan),
        vec![("2026-03".to_owned(), "4000.00".to_owned())]
    );
    assert_eq!(settlement_statements(&data, loan).len(), 0);
    assert_eq!(data.debts.house_of(loan), Some(house));
    assert!(!data.houses.get(house).unwrap().is_held());
    // The sell date is no longer locked by it.
    save_house(&mut data, house, Some("11/06/2026"), &[], false).unwrap();
}

#[test]
fn test_csd026_ac_16_4_clearing_a_closing_by_sale_without_confirmation() {
    let (mut data, house, loan) = sold_house_with_settled_loan();
    let result = Save::new("Loan", Some(house)).run(&mut data, loan);
    assert_eq!(code(result), "DEBT_CLOSING_UNDO_UNCONFIRMED");
    assert!(settlement(&data, loan).is_some());
}

// ---- CSD-026 REQ-17: edit a closing in place ----

#[test]
fn test_csd026_ac_17_1_edit_a_manual_closing() {
    let mut data = ProjectData::default();
    let card = new_debt(&mut data, "Example Card", "Other", None);
    statement(&mut data, card, "2026-03", "800.00");
    Save::new("Other", None)
        .closing("10/06/2026", "800.00", "Repaid")
        .run(&mut data, card)
        .unwrap();
    Save::new("Other", None)
        .closing("15/07/2026", "790.00", "Cancelled")
        .run(&mut data, card)
        .unwrap();
    assert_eq!(
        settlement(&data, card),
        Some((
            Outcome::Cancelled,
            Origin::Manual,
            "2026-07-15".to_owned(),
            "790.00".to_owned()
        ))
    );
    assert_eq!(
        settlement_statements(&data, card),
        vec![("2026-07".to_owned(), "Cancelled on closing".to_owned())]
    );
}

#[test]
fn test_csd026_ac_17_2_editing_a_closing_by_sale_makes_it_manual() {
    let (mut data, house, loan) = sold_house_with_settled_loan();
    Save::new("Loan", Some(house))
        .closing("20/05/2026", "4000.00", "Repaid")
        .run(&mut data, loan)
        .unwrap();
    assert_eq!(
        settlement(&data, loan),
        Some((
            Outcome::Repaid,
            Origin::Manual,
            "2026-05-20".to_owned(),
            "4000.00".to_owned()
        ))
    );
    assert_eq!(
        settlement_statements(&data, loan),
        vec![("2026-05".to_owned(), "Repaid on closing".to_owned())]
    );
    assert_eq!(
        data.houses.get(house).unwrap().sale.unwrap().date.iso(),
        "2026-06-10"
    );
}

#[test]
fn test_csd026_ac_17_3_an_unchanged_closing_keeps_its_origin() {
    let (mut data, house, loan) = sold_house_with_settled_loan();
    let mut save = Save::new("Loan", Some(house));
    save.name = Some("Example Loan Renamed");
    save.closing = Some(("10/06/2026", "4000.00", "Cancelled"));
    save.run(&mut data, loan).unwrap();
    assert_eq!(
        settlement(&data, loan),
        Some((
            Outcome::Cancelled,
            Origin::Sale,
            "2026-06-10".to_owned(),
            "4000.00".to_owned()
        ))
    );
    assert!(data.debts.validate_links(&data.houses).is_ok());
}

#[test]
fn test_csd026_ac_17_4_a_closing_before_a_statement_is_rejected() {
    let mut data = ProjectData::default();
    let card = new_debt(&mut data, "Example Card", "Other", None);
    statement(&mut data, card, "2026-03", "800.00");
    Save::new("Other", None)
        .closing("10/06/2026", "800.00", "Repaid")
        .run(&mut data, card)
        .unwrap();
    let result = Save::new("Other", None)
        .closing("01/02/2026", "800.00", "Repaid")
        .run(&mut data, card);
    assert_eq!(code(result), "DEBT_DATES_EXCLUDE_STATEMENTS");
    assert_eq!(settlement(&data, card).unwrap().2, "2026-06-10");
    assert_eq!(
        settlement_statements(&data, card),
        vec![("2026-06".to_owned(), "Repaid on closing".to_owned())]
    );
}

#[test]
fn test_csd026_ac_17_5_the_edit_follows_in_the_figures() {
    let mut data = ProjectData::default();
    let card = new_debt(&mut data, "Example Card", "Other", None);
    statement(&mut data, card, "2025-12", "800.00");
    Save::new("Other", None)
        .closing("10/06/2026", "800.00", "Cancelled")
        .run(&mut data, card)
        .unwrap();
    assert_eq!(data.debts.cancelled_in(2026).to_string(), "800.00");
    Save::new("Other", None)
        .closing("10/06/2026", "800.00", "Repaid")
        .run(&mut data, card)
        .unwrap();
    assert_eq!(data.debts.cancelled_in(2026).to_string(), "0.00");
}

// ---- CSD-026 REQ-18: the house link of a closed debt ----

#[test]
fn test_csd026_ac_18_1_unlink_a_debt_settled_by_a_sale() {
    let (mut data, house, loan) = sold_house_with_settled_loan();
    // The closing fields are sent as they are stored: the closing stays.
    Save::new("Loan", None)
        .closing("10/06/2026", "4000.00", "Cancelled")
        .run(&mut data, loan)
        .unwrap();
    assert_eq!(data.debts.house_of(loan), None);
    assert_eq!(
        settlement(&data, loan),
        Some((
            Outcome::Cancelled,
            Origin::Manual,
            "2026-06-10".to_owned(),
            "4000.00".to_owned()
        ))
    );
    save_house(&mut data, house, Some("11/06/2026"), &[], false).unwrap();
}

#[test]
fn test_csd026_ac_18_2_link_a_closed_debt_to_a_house() {
    let mut data = ProjectData::default();
    let house = house(&mut data, "Example House", None);
    let card = new_debt(&mut data, "Example Card", "Loan", None);
    Save::new("Loan", None)
        .closing("10/06/2026", "800.00", "Repaid")
        .run(&mut data, card)
        .unwrap();
    Save::new("Loan", Some(house))
        .closing("10/06/2026", "800.00", "Repaid")
        .run(&mut data, card)
        .unwrap();
    assert_eq!(data.debts.house_of(card), Some(house));
    assert_eq!(settlement(&data, card).unwrap().1, Origin::Manual);
}

#[test]
fn test_csd026_ac_18_3_link_a_closed_debt_to_a_sold_house() {
    let mut data = ProjectData::default();
    let guesthouse = house(&mut data, "Example Guesthouse", Some("10/09/2026"));
    let card = new_debt(&mut data, "Example Card", "Loan", None);
    statement(&mut data, card, "2026-03", "800.00");
    Save::new("Loan", None)
        .closing("10/06/2026", "800.00", "Repaid")
        .run(&mut data, card)
        .unwrap();
    // No outcome is asked and none is accepted.
    Save::new("Loan", Some(guesthouse))
        .closing("10/06/2026", "800.00", "Repaid")
        .run(&mut data, card)
        .unwrap();
    assert_eq!(data.debts.house_of(card), Some(guesthouse));
    assert_eq!(settlement_statements(&data, card).len(), 1);
    assert_eq!(
        settlement(&data, card).map(|(_, origin, ..)| origin),
        Some(Origin::Manual)
    );
    assert!(data.debts.validate_links(&data.houses).is_ok());
}

#[test]
fn test_csd026_ac_18_4_move_a_debt_settled_by_a_sale_to_another_house() {
    let (mut data, house, loan) = sold_house_with_settled_loan();
    let lodge = house_named(&mut data, "Example Lodge");
    Save::new("Loan", Some(lodge))
        .closing("10/06/2026", "4000.00", "Cancelled")
        .run(&mut data, loan)
        .unwrap();
    assert_eq!(data.debts.house_of(loan), Some(lodge));
    assert_eq!(settlement(&data, loan).unwrap().1, Origin::Manual);
    save_house(&mut data, house, None, &[], true).unwrap();
    assert!(settlement(&data, loan).is_some());
}

fn house_named(data: &mut ProjectData, name: &str) -> HouseId {
    house(data, name, None)
}

// ---- CSD-020 REQ-16, REQ-17 ----

#[test]
fn test_csd020_ac_16_2_the_link_of_a_debt_to_a_sold_house_can_be_changed() {
    let mut data = ProjectData::default();
    let guesthouse = house(&mut data, "Example Guesthouse", Some("10/06/2026"));
    let loan = new_debt(&mut data, "Example Cabin Loan", "Loan", Some(guesthouse));
    Save::new("Loan", None).run(&mut data, loan).unwrap();
    assert_eq!(data.debts.house_of(loan), None);
}

#[test]
fn test_csd020_ac_17_6_a_closed_debt_links_to_a_sold_house_without_a_prompt() {
    let mut data = ProjectData::default();
    let sold = house(&mut data, "Example House", Some("10/06/2026"));
    let mortgage = new_debt(&mut data, "Example Mortgage", "Mortgage", None);
    statement(&mut data, mortgage, "2026-03", "190000.00");
    Save::new("Mortgage", None)
        .closing("15/05/2026", "190000.00", "Repaid")
        .run(&mut data, mortgage)
        .unwrap();
    let link = |data: &mut ProjectData, outcome: Option<&str>| {
        let mut form = DebtFormInput {
            debt: DebtInput {
                name: "Example Mortgage".to_owned(),
                ..DebtInput::example()
            },
            category: Some("Mortgage".to_owned()),
            house_id: Some(sold.to_string()),
            settlement: outcome.map(str::to_owned),
            ..DebtFormInput::default()
        };
        form.closing.date = "15/05/2026".to_owned();
        form.closing.amount = "190000.00".to_owned();
        form.closing.outcome = "Repaid".to_owned();
        linking::update_debt(&data.houses, &mut data.debts, mortgage, &form).map(|_| ())
    };
    assert_eq!(
        code(link(&mut data, Some("Repaid"))),
        "HOUSE_SETTLEMENT_INVALID"
    );
    link(&mut data, None).unwrap();
    assert_eq!(data.debts.house_of(mortgage), Some(sold));
    assert_eq!(settlement(&data, mortgage).unwrap().2, "2026-05-15");
    assert!(
        linking::debt_open_balance(&data.houses, &data.debts, mortgage, sold)
            .unwrap()
            .is_none()
    );
}
