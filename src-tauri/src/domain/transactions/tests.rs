//! Tests of CSD-032 on synthetic exports ("Example" names, round amounts).

use super::*;
use crate::domain::common::YearMonth;
use crate::domain::money::Money;
use crate::domain::NumberFormat;

/// A filter as the frontend sends it, read in the default number format.
fn filter_of(json: serde_json::Value) -> TransactionFilter {
    serde_json::from_value::<TransactionFilterInput>(json)
        .unwrap()
        .resolve(NumberFormat::default())
}

const HEADER: &str = "Rekeningnummer;Transactiedatum;Valutacode;CreditDebet;Bedrag;Tegenrekeningnummer;Tegenrekeninghouder;Valutadatum;Betaalwijze;Omschrijving;Type betaling;Machtigingsnummer;Incassant ID;Referentie;Boekdatum";

/// One export row: own account, date, currency, direction, amount, name,
/// description and reference.
#[allow(
    clippy::too_many_arguments,
    reason = "one argument per column of the export keeps the test rows readable"
)]
fn row(
    own: &str,
    date: &str,
    currency: &str,
    direction: &str,
    amount: &str,
    name: &str,
    description: &str,
    reference: &str,
) -> String {
    format!(
        "{own};{date};{currency};{direction};{amount}; ;{name};{date};Betaalautomaat;{description};;;;{reference};{date}"
    )
}

fn simple(date: &str, direction: &str, amount: &str, name: &str, reference: &str) -> String {
    row(
        "NL00EXAM0000000001",
        date,
        "EUR",
        direction,
        amount,
        name,
        "Example description",
        reference,
    )
}

fn export(rows: &[String]) -> String {
    let mut lines = vec![HEADER.to_owned()];
    lines.extend(rows.iter().cloned());
    lines.join("\r\n") + "\r\n"
}

fn imported(rows: &[String]) -> Transactions {
    let mut data = Transactions::default();
    data.import(&export(rows), Currency::Eur).unwrap();
    data
}

fn money(text: &str) -> Money {
    Money::parse(text).unwrap()
}

fn category(data: &Transactions, name: &str) -> CategoryId {
    data.categories()
        .iter()
        .find(|category| category.name == name)
        .unwrap()
        .id
}

fn find<'a>(data: &'a Transactions, name: &str) -> &'a Transaction {
    data.transactions()
        .iter()
        .find(|transaction| transaction.counterparty_name == name)
        .unwrap()
}

// REQ-1

#[test]
fn test_csd032_ac_1_1_a_file_of_three_new_transactions() {
    let mut data = Transactions::default();
    let rows = [
        simple(
            "30-10-2026",
            "Afschrijvingen",
            "23,97",
            "Example Shop",
            "R1",
        ),
        simple(
            "29-10-2026",
            "Afschrijvingen",
            "10,00",
            "Example Cafe",
            "R2",
        ),
        simple(
            "28-10-2026",
            "Bijschrijvingen",
            "5,00",
            "Example Friend",
            "R3",
        ),
    ];
    let summary = data.import(&export(&rows), Currency::Eur).unwrap();
    assert_eq!(
        summary,
        ImportSummary {
            added: 3,
            skipped: 0
        }
    );
    assert_eq!(data.total_count(), 3);
}

#[test]
fn test_csd032_ac_1_4_an_empty_file_adds_nothing() {
    let mut data = Transactions::default();
    let summary = data.import(&export(&[]), Currency::Eur).unwrap();
    assert_eq!(
        summary,
        ImportSummary {
            added: 0,
            skipped: 0
        }
    );
}

// REQ-2

#[test]
fn test_csd032_ac_2_1_a_debt_is_negative_and_a_credit_positive() {
    let data = imported(&[
        simple(
            "30-10-2026",
            "Afschrijvingen",
            "23,97",
            "Example Shop",
            "R1",
        ),
        simple(
            "28-10-2026",
            "Bijschrijvingen",
            "1250,00",
            "Example Employer",
            "R2",
        ),
    ]);
    let debit = find(&data, "Example Shop");
    assert_eq!(debit.amount, money("-23.97"));
    assert_eq!(
        serde_json::to_value(debit.date).unwrap(),
        serde_json::json!("2026-10-30")
    );
    assert_eq!(find(&data, "Example Employer").amount, money("1250.00"));
}

#[test]
fn test_csd032_ac_2_2_a_row_without_counterparty_has_empty_fields() {
    let data = imported(&[simple("30-10-2026", "Afschrijvingen", "9,00", "", "R1")]);
    let only = data.transactions().first().unwrap();
    assert_eq!(only.counterparty_name, "");
    assert_eq!(only.counterparty_iban, "");
}

#[test]
fn test_csd032_ac_2_3_line_endings_leave_no_carriage_return() {
    let rows = [simple(
        "30-10-2026",
        "Afschrijvingen",
        "9,00",
        "Example Shop",
        "R1",
    )];
    for text in [export(&rows), export(&rows).replace("\r\n", "\n")] {
        let mut data = Transactions::default();
        data.import(&text, Currency::Eur).unwrap();
        let only = data.transactions().first().unwrap();
        assert!(!only.reference.contains('\r'));
        assert!(!only.description.contains('\r'));
    }
}

#[test]
fn test_csd032_ac_2_4_a_semicolon_inside_quotes_is_text() {
    let line = row(
        "NL00EXAM0000000001",
        "30-10-2026",
        "EUR",
        "Afschrijvingen",
        "9,00",
        "Example Shop",
        "\"Example; Shop\"",
        "R1",
    );
    let data = imported(&[line]);
    assert_eq!(
        data.transactions().first().unwrap().description,
        "Example; Shop"
    );
}

// REQ-3

fn import_error(rows: &[String]) -> DomainError {
    Transactions::default()
        .import(&export(rows), Currency::Eur)
        .unwrap_err()
}

fn five_rows_with_bad_fourth(bad: String) -> Vec<String> {
    let good = |n: u32| {
        simple(
            "30-10-2026",
            "Afschrijvingen",
            "9,00",
            "Example Shop",
            &format!("R{n}"),
        )
    };
    vec![good(1), good(2), bad, good(4), good(5)]
}

#[test]
fn test_csd032_ac_3_1_one_bad_row_imports_nothing() {
    let bad_rows = [
        (
            simple("31-04-2026", "Afschrijvingen", "9,00", "X", "R3"),
            "TRANSACTION_IMPORT_DATE",
        ),
        (
            simple("30-10-2026", "Anders", "9,00", "X", "R3"),
            "TRANSACTION_IMPORT_DIRECTION",
        ),
        (
            simple("30-10-2026", "Afschrijvingen", "abc", "X", "R3"),
            "TRANSACTION_IMPORT_AMOUNT",
        ),
        (
            simple("30-10-2026", "Afschrijvingen", "0,00", "X", "R3"),
            "TRANSACTION_IMPORT_AMOUNT",
        ),
        (
            simple("30-10-2026", "Afschrijvingen", "1,234", "X", "R3"),
            "TRANSACTION_IMPORT_AMOUNT",
        ),
        (
            "NL00EXAM0000000001;30-10-2026;EUR".to_owned(),
            "TRANSACTION_IMPORT_COLUMNS",
        ),
        (
            simple("30-10-2026", "Afschrijvingen", "9,00", "X", ""),
            "TRANSACTION_IMPORT_REFERENCE",
        ),
    ];
    for (bad, code) in bad_rows {
        let mut data = Transactions::default();
        let text = export(&five_rows_with_bad_fourth(bad));
        let error = data.import(&text, Currency::Eur).unwrap_err();
        assert_eq!(error.code(), code);
        // The header is line 1, so the third data row is line 4.
        assert!(error.to_string().contains("Line 4"), "{error}");
        assert_eq!(data.total_count(), 0, "{code}");
    }
}

#[test]
fn test_csd032_ac_3_2_another_header_is_not_a_knab_export() {
    let error = Transactions::default()
        .import("Datum;Bedrag\r\n30-10-2026;9,00\r\n", Currency::Eur)
        .unwrap_err();
    assert_eq!(error.code(), "TRANSACTION_IMPORT_FORMAT");
    let empty = Transactions::default()
        .import("", Currency::Eur)
        .unwrap_err();
    assert_eq!(empty.code(), "TRANSACTION_IMPORT_FORMAT");
}

#[test]
fn test_csd032_ac_3_3_the_error_never_shows_a_path() {
    let error = import_error(&[simple("31-04-2026", "Afschrijvingen", "9,00", "X", "R1")]);
    assert!(!error.to_string().contains('/'));
}

#[test]
fn test_csd032_ac_3_4_a_field_that_is_too_long_is_rejected() {
    let long = "x".repeat(1001);
    let line = simple("30-10-2026", "Afschrijvingen", "9,00", &long, "R1");
    assert_eq!(import_error(&[line]).code(), "TRANSACTION_IMPORT_FIELD");
}

// REQ-4

#[test]
fn test_csd032_ac_4_1_another_currency_imports_nothing() {
    let rows = [
        simple("30-10-2026", "Afschrijvingen", "9,00", "A", "R1"),
        simple("30-10-2026", "Afschrijvingen", "9,00", "B", "R2"),
        row(
            "NL00EXAM0000000001",
            "30-10-2026",
            "USD",
            "Afschrijvingen",
            "9,00",
            "C",
            "d",
            "R3",
        ),
    ];
    let mut data = Transactions::default();
    let error = data.import(&export(&rows), Currency::Eur).unwrap_err();
    assert_eq!(error.code(), "TRANSACTION_IMPORT_CURRENCY");
    assert!(error.to_string().contains("Line 4"));
    assert_eq!(data.total_count(), 0);
}

#[test]
fn test_csd032_ac_4_2_the_project_currency_is_compared() {
    let rows = [simple("30-10-2026", "Afschrijvingen", "9,00", "A", "R1")];
    let error = Transactions::default()
        .import(&export(&rows), Currency::Usd)
        .unwrap_err();
    assert_eq!(error.code(), "TRANSACTION_IMPORT_CURRENCY");
}

// REQ-5

#[test]
fn test_csd032_ac_5_1_an_overlapping_export_skips_what_is_known() {
    let first = [
        simple("30-10-2026", "Afschrijvingen", "9,00", "A", "R1"),
        simple("29-10-2026", "Afschrijvingen", "9,00", "B", "R2"),
        simple("28-10-2026", "Afschrijvingen", "9,00", "C", "R3"),
    ];
    let mut data = imported(&first);
    let mut full = first.to_vec();
    full.push(simple("27-10-2026", "Afschrijvingen", "9,00", "D", "R4"));
    full.push(simple("26-10-2026", "Afschrijvingen", "9,00", "E", "R5"));
    let summary = data.import(&export(&full), Currency::Eur).unwrap();
    assert_eq!(
        summary,
        ImportSummary {
            added: 2,
            skipped: 3
        }
    );
    assert_eq!(data.total_count(), 5);
}

#[test]
fn test_csd032_ac_5_2_the_same_reference_on_another_account_is_added() {
    let mut data = imported(&[simple(
        "30-10-2026",
        "Afschrijvingen",
        "9,00",
        "A",
        "REF0001",
    )]);
    let other = row(
        "NL00EXAM0000000002",
        "30-10-2026",
        "EUR",
        "Afschrijvingen",
        "9,00",
        "A",
        "d",
        "REF0001",
    );
    let summary = data.import(&export(&[other]), Currency::Eur).unwrap();
    assert_eq!(summary.added, 1);
}

#[test]
fn test_csd032_ac_5_3_a_repeated_row_in_one_file_counts_once() {
    let line = simple("30-10-2026", "Afschrijvingen", "9,00", "A", "R1");
    let mut data = Transactions::default();
    let summary = data
        .import(&export(&[line.clone(), line]), Currency::Eur)
        .unwrap();
    assert_eq!(
        summary,
        ImportSummary {
            added: 1,
            skipped: 1
        }
    );
}

#[test]
fn test_csd032_ac_5_4_a_deleted_transaction_can_be_imported_again() {
    let rows = [simple("30-10-2026", "Afschrijvingen", "9,00", "A", "R1")];
    let mut data = imported(&rows);
    let id = data.transactions().first().unwrap().id;
    data.delete_transaction(id).unwrap();
    let summary = data.import(&export(&rows), Currency::Eur).unwrap();
    assert_eq!(summary.added, 1);
}

// REQ-7

#[test]
fn test_csd032_ac_7_2_text_fields_are_stored_trimmed() {
    let line = row(
        "NL00EXAM0000000001",
        "30-10-2026",
        "EUR",
        "Afschrijvingen",
        "9,00",
        " Example Shop ",
        "Pas 0000 Example Town",
        "R1",
    );
    let data = imported(&[line]);
    let only = data.transactions().first().unwrap();
    assert_eq!(only.counterparty_name, "Example Shop");
    assert_eq!(only.description, "Pas 0000 Example Town");
}

// REQ-8

#[test]
fn test_csd032_ac_8_1_a_new_project_has_the_default_categories() {
    let data = Transactions::default();
    let names: Vec<&str> = data.categories().iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Salary",
            "Groceries",
            "Housing",
            "Utilities",
            "Insurance",
            "Transport",
            "Eating out",
            "Shopping",
            "Health",
            "Leisure",
            "Subscriptions",
            "Savings & transfers",
            "Taxes",
            "Other"
        ]
    );
    assert_eq!(data.total_count(), 0);
}

#[test]
fn test_csd032_ac_8_2_add_and_rename_a_category() {
    let mut data = Transactions::default();
    let added = data.add_category("Example Pets").unwrap();
    let renamed = data.rename_category(added.id, "Pets").unwrap();
    assert_eq!(renamed.name, "Pets");
    assert_eq!(category(&data, "Pets"), added.id);
}

#[test]
fn test_csd032_ac_8_3_invalid_category_names_are_rejected() {
    let mut data = Transactions::default();
    let cases = [
        (String::new(), "TRANSACTION_CATEGORY_NAME_REQUIRED"),
        ("   ".to_owned(), "TRANSACTION_CATEGORY_NAME_REQUIRED"),
        ("x".repeat(51), "TRANSACTION_CATEGORY_NAME_TOO_LONG"),
        ("groceries".to_owned(), "TRANSACTION_CATEGORY_NAME_TAKEN"),
        (
            "uncategorized".to_owned(),
            "TRANSACTION_CATEGORY_NAME_TAKEN",
        ),
    ];
    for (name, code) in cases {
        assert_eq!(
            data.add_category(&name).unwrap_err().code(),
            code,
            "{name:?}"
        );
    }
    assert!(data.add_category(&"x".repeat(50)).is_ok());
}

#[test]
fn test_csd032_ac_8_5_a_category_may_change_only_its_own_case() {
    let mut data = Transactions::default();
    let id = category(&data, "Groceries");
    assert_eq!(
        data.rename_category(id, "GROCERIES").unwrap().name,
        "GROCERIES"
    );
    let other = category(&data, "Taxes");
    let error = data.rename_category(other, "groceries").unwrap_err();
    assert_eq!(error.code(), "TRANSACTION_CATEGORY_NAME_TAKEN");
}

// REQ-9

#[test]
fn test_csd032_ac_9_1_add_and_reorder_rules() {
    let mut data = Transactions::default();
    let groceries = category(&data, "Groceries");
    let before = data.rules().len();
    let added = data.add_rule("Example Market", groceries).unwrap();
    assert_eq!(data.rules().last().unwrap().text, "Example Market");
    assert_eq!(data.rules().len(), before + 1);
    data.move_rule(added.id, 0).unwrap();
    assert_eq!(data.rules().first().unwrap().id, added.id);
    data.move_rule(added.id, 9999).unwrap();
    assert_eq!(data.rules().last().unwrap().id, added.id);
}

#[test]
fn test_csd032_ac_9_3_invalid_rules_are_rejected() {
    let mut data = Transactions::default();
    let groceries = category(&data, "Groceries");
    assert_eq!(
        data.add_rule("  ", groceries).unwrap_err().code(),
        "TRANSACTION_RULE_TEXT_REQUIRED"
    );
    assert_eq!(
        data.add_rule(&"x".repeat(101), groceries)
            .unwrap_err()
            .code(),
        "TRANSACTION_RULE_TEXT_TOO_LONG"
    );
    let unknown = CategoryId::new();
    assert_eq!(
        data.add_rule("Example", unknown).unwrap_err().code(),
        "TRANSACTION_RULE_CATEGORY_INVALID"
    );
}

#[test]
fn test_csd032_ac_9_4_the_seed_rules_name_existing_categories() {
    let data = Transactions::default();
    assert!(data.rules().len() >= 10);
    for rule in data.rules() {
        assert!(data.categories().iter().any(|c| c.id == rule.category_id));
    }
}

#[test]
fn test_csd032_ac_9_5_edit_and_delete_a_rule() {
    let mut data = Transactions::default();
    let leisure = category(&data, "Leisure");
    let rule = data.add_rule("Example Gym", leisure).unwrap();
    let edited = data.edit_rule(rule.id, "Example Club", leisure).unwrap();
    assert_eq!(edited.text, "Example Club");
    data.delete_rule(rule.id).unwrap();
    assert_eq!(
        data.delete_rule(rule.id).unwrap_err().code(),
        "TRANSACTION_RULE_NOT_FOUND"
    );
}

// REQ-10

fn no_rules(data: &mut Transactions) {
    let ids: Vec<RuleId> = data.rules().iter().map(|rule| rule.id).collect();
    for id in ids {
        data.delete_rule(id).unwrap();
    }
}

fn with_rules(rules: &[(&str, &str)]) -> Transactions {
    let mut data = Transactions::default();
    no_rules(&mut data);
    for (text, name) in rules {
        let id = category(&data, name);
        data.add_rule(text, id).unwrap();
    }
    data
}

#[test]
fn test_csd032_ac_10_1_the_first_matching_rule_wins() {
    let mut data = with_rules(&[("Example Market", "Groceries"), ("Example", "Shopping")]);
    let line = simple(
        "30-10-2026",
        "Afschrijvingen",
        "9,00",
        "EXAMPLE MARKET Town",
        "R1",
    );
    data.import(&export(&[line]), Currency::Eur).unwrap();
    let groceries = category(&data, "Groceries");
    assert_eq!(
        data.transactions().first().unwrap().category_id,
        Some(groceries)
    );
}

#[test]
fn test_csd032_ac_10_2_the_description_is_matched_too() {
    let mut data = with_rules(&[("Rent October", "Housing")]);
    let line = row(
        "NL00EXAM0000000001",
        "30-10-2026",
        "EUR",
        "Afschrijvingen",
        "1000,00",
        "",
        "Rent October 2026",
        "R1",
    );
    data.import(&export(&[line]), Currency::Eur).unwrap();
    let housing = category(&data, "Housing");
    assert_eq!(
        data.transactions().first().unwrap().category_id,
        Some(housing)
    );
}

#[test]
fn test_csd032_ac_10_3_no_match_is_uncategorized() {
    let mut data = with_rules(&[]);
    let line = simple("30-10-2026", "Afschrijvingen", "9,00", "Example Shop", "R1");
    data.import(&export(&[line]), Currency::Eur).unwrap();
    let only = data.transactions().first().unwrap();
    assert_eq!(only.category_id, None);
    assert!(!only.manual);
}

#[test]
fn test_csd032_ac_10_4_matching_ignores_case_but_not_the_text() {
    let mut data = with_rules(&[("example market", "Groceries")]);
    let line = simple("30-10-2026", "Afschrijvingen", "9,00", "Example Mkt", "R1");
    data.import(&export(&[line]), Currency::Eur).unwrap();
    assert_eq!(data.transactions().first().unwrap().category_id, None);
}

// REQ-11, REQ-12

#[test]
fn test_csd032_ac_11_1_a_manual_choice_sticks() {
    let mut data = with_rules(&[]);
    let line = simple("30-10-2026", "Afschrijvingen", "9,00", "Example Shop", "R1");
    data.import(&export(&[line]), Currency::Eur).unwrap();
    let id = data.transactions().first().unwrap().id;
    let shopping = category(&data, "Shopping");
    data.set_category(id, Some(shopping)).unwrap();
    let leisure = category(&data, "Leisure");
    data.add_rule("Example Shop", leisure).unwrap();
    data.reapply_rules(ReapplyScope::AllNonManual);
    let only = data.transactions().first().unwrap();
    assert_eq!(only.category_id, Some(shopping));
    assert!(only.manual);
}

#[test]
fn test_csd032_ac_11_2_back_to_uncategorized_by_hand_stays_manual() {
    let mut data = with_rules(&[("Example", "Shopping")]);
    let line = simple("30-10-2026", "Afschrijvingen", "9,00", "Example Shop", "R1");
    data.import(&export(&[line]), Currency::Eur).unwrap();
    let id = data.transactions().first().unwrap().id;
    data.set_category(id, None).unwrap();
    let only = data.transactions().first().unwrap();
    assert_eq!(only.category_id, None);
    assert!(only.manual);
}

#[test]
fn test_csd032_ac_11_3_a_stale_screen_is_reported() {
    let mut data = Transactions::default();
    let error = data.set_category(TransactionId::new(), None).unwrap_err();
    assert_eq!(error.code(), "TRANSACTION_NOT_FOUND");
    let line = simple("30-10-2026", "Afschrijvingen", "9,00", "Example Shop", "R1");
    data.import(&export(&[line]), Currency::Eur).unwrap();
    let id = data.transactions().first().unwrap().id;
    let error = data.set_category(id, Some(CategoryId::new())).unwrap_err();
    assert_eq!(error.code(), "TRANSACTION_CATEGORY_NOT_FOUND");
}

fn reapply_fixture() -> Transactions {
    let mut data = with_rules(&[]);
    let rows = [
        simple("30-10-2026", "Afschrijvingen", "9,00", "Example One", "R1"),
        simple("29-10-2026", "Afschrijvingen", "9,00", "Example Two", "R2"),
        simple(
            "28-10-2026",
            "Afschrijvingen",
            "9,00",
            "Example Three",
            "R3",
        ),
    ];
    data.import(&export(&rows), Currency::Eur).unwrap();
    let shopping = category(&data, "Shopping");
    let id = find(&data, "Example Three").id;
    // Three is categorised by an old rule, not by hand.
    let rule = data.add_rule("Three", shopping).unwrap();
    data.reapply_rules(ReapplyScope::AllNonManual);
    data.delete_rule(rule.id).unwrap();
    assert_eq!(
        data.transactions()
            .iter()
            .find(|t| t.id == id)
            .unwrap()
            .category_id,
        Some(shopping)
    );
    let leisure = category(&data, "Leisure");
    data.add_rule("Example", leisure).unwrap();
    data
}

#[test]
fn test_csd032_ac_12_1_including_manual_also_changes_hand_set_categories() {
    let mut data = reapply_fixture();
    let leisure = category(&data, "Leisure");
    let shopping = category(&data, "Shopping");
    let id = find(&data, "Example One").id;
    data.set_category(id, Some(shopping)).unwrap();
    // Apply rules leaves the hand-set category alone: only Two and Three are covered.
    assert_eq!(data.reapply_rules(ReapplyScope::AllNonManual), 2);
    assert_eq!(find(&data, "Example One").category_id, Some(shopping));
    assert!(find(&data, "Example One").manual);
    // Including manual changes it too, and the transaction is no longer manual.
    assert_eq!(data.reapply_rules(ReapplyScope::All), 1);
    assert_eq!(find(&data, "Example One").category_id, Some(leisure));
    assert!(!find(&data, "Example One").manual);
    assert_eq!(data.reapply_rules(ReapplyScope::All), 0);
}

#[test]
fn test_csd032_ac_12_2_reapply_to_all_non_manual() {
    let mut data = reapply_fixture();
    let leisure = category(&data, "Leisure");
    assert_eq!(data.reapply_rules(ReapplyScope::AllNonManual), 3);
    assert_eq!(find(&data, "Example One").category_id, Some(leisure));
    assert_eq!(find(&data, "Example Three").category_id, Some(leisure));
}

#[test]
fn test_csd032_ac_12_3_nothing_to_change_reports_zero() {
    let mut data = reapply_fixture();
    data.reapply_rules(ReapplyScope::AllNonManual);
    let before = data.clone();
    assert_eq!(data.reapply_rules(ReapplyScope::AllNonManual), 0);
    assert_eq!(data, before);
}

// REQ-13, REQ-14

#[test]
fn test_csd032_ac_13_1_deleting_a_category_clears_transactions_and_rules() {
    let mut data = Transactions::default();
    let leisure = category(&data, "Leisure");
    data.add_rule("Example Gym", leisure).unwrap();
    data.add_rule("Example Club", leisure).unwrap();
    let line = simple("30-10-2026", "Afschrijvingen", "9,00", "Example Gym", "R1");
    data.import(&export(&[line]), Currency::Eur).unwrap();
    let id = data.transactions().first().unwrap().id;
    data.set_category(id, Some(leisure)).unwrap();
    data.delete_category(leisure).unwrap();
    let only = data.transactions().first().unwrap();
    assert_eq!(only.category_id, None);
    assert!(only.manual);
    assert!(data.rules().iter().all(|rule| rule.category_id != leisure));
    assert!(data.categories().iter().all(|c| c.id != leisure));
}

#[test]
fn test_csd032_ac_14_1_deleting_a_transaction_removes_only_that_one() {
    let mut data = imported(&[
        simple("30-10-2026", "Afschrijvingen", "9,00", "Example Shop", "R1"),
        simple("29-10-2026", "Afschrijvingen", "9,00", "Example Cafe", "R2"),
    ]);
    let id = find(&data, "Example Shop").id;
    data.delete_transaction(id).unwrap();
    assert_eq!(data.total_count(), 1);
    assert_eq!(
        data.transactions().first().unwrap().counterparty_name,
        "Example Cafe"
    );
    assert_eq!(
        data.delete_transaction(id).unwrap_err().code(),
        "TRANSACTION_NOT_FOUND"
    );
}

#[test]
fn test_csd032_ac_25_1_deleting_all_transactions_keeps_categories_and_rules() {
    let mut data = imported(&[
        simple("30-10-2026", "Afschrijvingen", "9,00", "Example Shop", "R1"),
        simple("29-10-2026", "Afschrijvingen", "9,00", "Example Cafe", "R2"),
    ]);
    let categories = data.categories().to_vec();
    let rules = data.rules().to_vec();
    assert_eq!(data.delete_all_transactions(), 2);
    assert_eq!(data.total_count(), 0);
    assert_eq!(data.categories(), categories.as_slice());
    assert_eq!(data.rules(), rules.as_slice());
}

// REQ-15

fn many(count: u32) -> Transactions {
    let mut data = with_rules(&[]);
    let rows: Vec<String> = (0..count)
        .map(|n| {
            let day = 1 + n % 28;
            let month = 1 + (n / 28) % 12;
            simple(
                &format!("{day:02}-{month:02}-2026"),
                "Afschrijvingen",
                "1,00",
                "Example Shop",
                &format!("R{n}"),
            )
        })
        .collect();
    data.import(&export(&rows), Currency::Eur).unwrap();
    data
}

#[test]
fn test_csd032_ac_15_1_newest_first_and_cut_to_the_limit() {
    let data = many(450);
    let page = data.list_filtered(None, false, 200);
    assert_eq!(page.rows.len(), 200);
    assert_eq!(page.count, 450);
    let dates: Vec<_> = page.rows.iter().map(|row| row.date).collect();
    assert!(dates.windows(2).all(|pair| pair[0] >= pair[1]));
    assert_eq!(data.list_filtered(None, false, 400).rows.len(), 400);
    assert_eq!(data.list_filtered(None, false, 600).rows.len(), 450);
}

#[test]
fn test_csd032_ac_15_2_the_total_covers_all_matches() {
    let mut data = many(450);
    let groceries = category(&data, "Groceries");
    let ids: Vec<TransactionId> = data.transactions().iter().map(|t| t.id).collect();
    for id in ids {
        data.set_category(id, Some(groceries)).unwrap();
    }
    let filter = filter_of(serde_json::json!({ "ids": [groceries.to_string()] }));
    let page = data.list_filtered(Some(&filter), false, 10);
    assert_eq!(page.rows.len(), 10);
    assert_eq!(page.count, 450);
    assert_eq!(page.total, money("-450.00"));
}

#[test]
fn test_csd032_ac_15_5_equal_dates_list_the_one_added_last_first() {
    let data = imported(&[
        simple("30-10-2026", "Afschrijvingen", "9,00", "First added", "R1"),
        simple("30-10-2026", "Afschrijvingen", "9,00", "Second added", "R2"),
    ]);
    let page = data.list_filtered(None, false, 10);
    assert_eq!(page.rows.first().unwrap().counterparty_name, "Second added");
}

#[test]
fn test_csd032_ac_26_1_manual_only_lists_the_hand_set_transactions() {
    let mut data = imported(&[
        simple("30-10-2026", "Afschrijvingen", "9,00", "Example Shop", "R1"),
        simple("29-10-2026", "Afschrijvingen", "5,00", "Example Cafe", "R2"),
        simple(
            "28-10-2026",
            "Afschrijvingen",
            "1,00",
            "Example Bakery",
            "R3",
        ),
    ]);
    let shopping = category(&data, "Shopping");
    let shop = find(&data, "Example Shop").id;
    data.set_category(shop, Some(shopping)).unwrap();
    let page = data.list_filtered(None, true, 10);
    assert_eq!(page.count, 1);
    assert_eq!(page.rows.first().unwrap().counterparty_name, "Example Shop");
    assert_eq!(page.total, money("-9.00"));
    // Without the filter every transaction is listed.
    assert_eq!(data.list_filtered(None, false, 10).count, 3);
    // It combines with the filter of CSD-028: manual and not in "Shopping" is nothing.
    let leisure = category(&data, "Leisure");
    let other = filter_of(serde_json::json!({ "ids": [leisure.to_string()] }));
    assert_eq!(data.list_filtered(Some(&other), true, 10).count, 0);
}

#[test]
fn test_csd032_ac_15_6_uncategorized_can_be_filtered() {
    let mut data = with_rules(&[("Example Shop", "Shopping")]);
    let rows = [
        simple("30-10-2026", "Afschrijvingen", "9,00", "Example Shop", "R1"),
        simple("30-10-2026", "Afschrijvingen", "9,00", "Example Cafe", "R2"),
    ];
    data.import(&export(&rows), Currency::Eur).unwrap();
    let filter = filter_of(serde_json::json!({ "ids": ["uncategorized"] }));
    let page = data.list_filtered(Some(&filter), false, 10);
    assert_eq!(page.count, 1);
    assert_eq!(page.rows.first().unwrap().counterparty_name, "Example Cafe");
}

// REQ-17

#[test]
fn test_csd032_ac_17_4_a_transaction_with_an_unknown_category_is_invalid() {
    let data = imported(&[simple(
        "30-10-2026",
        "Afschrijvingen",
        "9,00",
        "Example Shop",
        "R1",
    )]);
    let mut list = data.transactions().to_vec();
    list.first_mut().unwrap().category_id = Some(CategoryId::new());
    let error = Transactions::from_parts(list, data.categories().to_vec(), data.rules().to_vec())
        .unwrap_err();
    assert_eq!(error.code(), "TRANSACTION_DATA_INVALID");
}

#[test]
fn test_csd032_ac_17_6_duplicate_ids_and_dangling_rules_are_invalid() {
    let data = imported(&[simple(
        "30-10-2026",
        "Afschrijvingen",
        "9,00",
        "Example Shop",
        "R1",
    )]);
    let mut doubled = data.transactions().to_vec();
    doubled.push(doubled.first().unwrap().clone());
    assert!(
        Transactions::from_parts(doubled, data.categories().to_vec(), data.rules().to_vec())
            .is_err()
    );
    let mut rules = data.rules().to_vec();
    rules.first_mut().unwrap().category_id = CategoryId::new();
    assert!(Transactions::from_parts(Vec::new(), data.categories().to_vec(), rules).is_err());
}

// REQ-19 (the sharing the budget relies on)

#[test]
fn test_csd032_ac_19_1_a_clone_shares_the_list_until_it_changes() {
    let mut data = many(10);
    let before = data.clone();
    assert_eq!(data.shared_weight().0, before.shared_weight().0);
    let id = data.transactions().first().unwrap().id;
    data.set_category(id, None).unwrap();
    assert_ne!(data.shared_weight().0, before.shared_weight().0);
    assert!(!before.transactions().first().unwrap().manual);
}

#[test]
fn test_csd032_ac_9_6_a_rule_can_create_its_category_with_it() {
    let mut data = Transactions::default();
    let (categories, rules) = (data.categories().len(), data.rules().len());
    let (category, rule) = data
        .add_rule_with_new_category("Example Gym", "Example Sport")
        .unwrap();
    assert_eq!(data.categories().len(), categories + 1);
    assert_eq!(data.categories().last().unwrap().name, "Example Sport");
    assert_eq!(data.rules().len(), rules + 1);
    assert_eq!(data.rules().last().unwrap().category_id, category.id);
    assert_eq!(rule.text, "Example Gym");
}

#[test]
fn test_csd032_ac_10_4_a_star_in_a_rule_matches_any_text() {
    let rules = |text: &str| {
        // The seed rules would match too, so start from none.
        let mut data = Transactions::default();
        let seeds: Vec<RuleId> = data.rules().iter().map(|rule| rule.id).collect();
        for id in seeds {
            data.delete_rule(id).unwrap();
        }
        let groceries = category(&data, "Groceries");
        data.add_rule(text, groceries).unwrap();
        data
    };
    let hit = |data: &Transactions, counterparty: &str, description: &str| {
        categorise(data.rules(), counterparty, description).is_some()
    };
    let trailing = rules("Albert Heijn*");
    assert!(hit(&trailing, "Albert Heijn B.V.", ""));
    assert!(hit(&trailing, "ALBERT HEIJN 1157", "Pinbetaling"));
    assert!(hit(&trailing, "Albert Heijn", ""));
    assert!(!hit(&trailing, "Jumbo 1157", "Albert Hein"));
    // A star inside: the parts must come in that order.
    let inner = rules("Albert*1157");
    assert!(hit(&inner, "Albert Heijn 1157", ""));
    assert!(hit(&inner, "Albert", "Heijn 1157"));
    assert!(!hit(&inner, "1157 Albert Heijn", ""));
    // Two stars, a leading star, and a text without a star is still "contains".
    assert!(hit(&rules("*heijn*1157*"), "Albert Heijn 1157 Utrecht", ""));
    assert!(hit(&rules("Heijn"), "Albert Heijn 1157", ""));
    assert!(!hit(&rules("Albert*Jumbo"), "Albert Heijn 1157", ""));
    // A star alone matches everything.
    assert!(hit(&rules("*"), "Anything", ""));
}

#[test]
fn test_csd032_ac_27_1_a_rule_counts_the_transactions_it_categorises() {
    let mut data = Transactions::default();
    let seeds: Vec<RuleId> = data.rules().iter().map(|rule| rule.id).collect();
    for id in seeds {
        data.delete_rule(id).unwrap();
    }
    let groceries = category(&data, "Groceries");
    let leisure = category(&data, "Leisure");
    data.add_rule("Example Market", groceries).unwrap();
    data.add_rule("Example C*", leisure).unwrap();
    data.add_rule("Nothing Matches This", leisure).unwrap();
    let rows = [
        simple(
            "30-10-2026",
            "Afschrijvingen",
            "9,00",
            "Example Market One",
            "R1",
        ),
        simple(
            "29-10-2026",
            "Afschrijvingen",
            "9,00",
            "Example Market Two",
            "R2",
        ),
        simple("28-10-2026", "Afschrijvingen", "9,00", "Example Cafe", "R3"),
        simple(
            "27-10-2026",
            "Afschrijvingen",
            "9,00",
            "Example Bakery",
            "R4",
        ),
        simple("26-10-2026", "Afschrijvingen", "9,00", "Other Place", "R5"),
    ];
    data.import(&export(&rows), Currency::Eur).unwrap();
    // The first match counts: "Example Market One" and "Two" belong to the first rule.
    assert_eq!(data.rule_counts(), vec![2, 1, 0]);
    // A hand-set transaction is not counted.
    let cafe = find(&data, "Example Cafe").id;
    data.set_category(cafe, Some(groceries)).unwrap();
    assert_eq!(data.rule_counts(), vec![2, 0, 0]);
}

#[test]
fn test_csd032_ac_30_1_a_category_counts_its_transactions_by_whoever_set_them() {
    let mut data = imported(&[
        simple("30-10-2026", "Afschrijvingen", "9,00", "Example One", "R1"),
        simple("29-10-2026", "Afschrijvingen", "9,00", "Example Two", "R2"),
        simple(
            "28-10-2026",
            "Afschrijvingen",
            "9,00",
            "Example Three",
            "R3",
        ),
        simple("27-10-2026", "Afschrijvingen", "9,00", "Example Four", "R4"),
    ]);
    let groceries = category(&data, "Groceries");
    let leisure = category(&data, "Leisure");
    let counts = |data: &Transactions| -> (usize, usize) {
        let all = data.category_counts();
        let at = |id| {
            data.categories()
                .iter()
                .position(|each| each.id == id)
                .and_then(|index| all.get(index).copied())
                .unwrap()
        };
        (at(groceries), at(leisure))
    };
    assert_eq!(data.category_counts().len(), data.categories().len());
    for name in ["Example One", "Example Two"] {
        let id = find(&data, name).id;
        data.set_category(id, Some(groceries)).unwrap();
    }
    let three = find(&data, "Example Three").id;
    data.set_category(three, Some(leisure)).unwrap();
    // A hand-set transaction counts too; the uncategorized one does not.
    assert_eq!(counts(&data), (2, 1));
    let four = find(&data, "Example Four").id;
    data.set_category(four, Some(leisure)).unwrap();
    assert_eq!(counts(&data), (2, 2));
}

#[test]
fn test_csd032_ac_15_7_the_period_filter_goes_down_to_the_day() {
    let data = imported(&[
        simple("31-10-2026", "Afschrijvingen", "9,00", "Example Late", "R1"),
        simple(
            "15-10-2026",
            "Afschrijvingen",
            "5,00",
            "Example Middle",
            "R2",
        ),
        simple(
            "01-10-2026",
            "Afschrijvingen",
            "1,00",
            "Example Early",
            "R3",
        ),
        simple(
            "30-09-2026",
            "Afschrijvingen",
            "2,00",
            "Example Before",
            "R4",
        ),
    ]);
    let filter = |json: serde_json::Value| -> TransactionFilter { filter_of(json) };
    let count = |filter: &TransactionFilter| data.list_filtered(Some(filter), false, 10).count;
    // Both ends are included, to the day.
    assert_eq!(
        count(&filter(
            serde_json::json!({ "from": "2026-10-01", "to": "2026-10-15" })
        )),
        2
    );
    assert_eq!(
        count(&filter(
            serde_json::json!({ "from": "2026-10-02", "to": "2026-10-30" })
        )),
        1
    );
    assert_eq!(
        count(&filter(serde_json::json!({ "from": "2026-10-15" }))),
        2
    );
    assert_eq!(count(&filter(serde_json::json!({ "to": "2026-09-30" }))), 1);
    // An inverted range shows nothing, and the ids of the same filter agree.
    let inverted = filter(serde_json::json!({ "from": "2026-10-20", "to": "2026-10-10" }));
    assert_eq!(count(&inverted), 0);
    let range = filter(serde_json::json!({ "from": "2026-10-01", "to": "2026-10-15" }));
    assert_eq!(data.matching_ids(Some(&range), false).len(), 2);
    // A month is no longer a valid bound for this list.
    assert!(serde_json::from_value::<TransactionFilterInput>(
        serde_json::json!({ "from": "2026-10" })
    )
    .is_err());
}

#[test]
fn test_csd032_ac_15_8_the_amount_filter_is_a_range_of_signed_amounts() {
    let data = imported(&[
        simple(
            "30-10-2026",
            "Afschrijvingen",
            "100,00",
            "Example Big",
            "R1",
        ),
        simple(
            "29-10-2026",
            "Afschrijvingen",
            "10,00",
            "Example Small",
            "R2",
        ),
        simple(
            "28-10-2026",
            "Afschrijvingen",
            "10,00",
            "Example Small Too",
            "R3",
        ),
        simple(
            "27-10-2026",
            "Bijschrijvingen",
            "2500,00",
            "Example Salary",
            "R4",
        ),
    ]);
    let count =
        |json: serde_json::Value| data.list_filtered(Some(&filter_of(json)), false, 10).count;
    // A debit is negative; both ends are included.
    assert_eq!(
        count(serde_json::json!({ "min_amount": "-100", "max_amount": "-10" })),
        3
    );
    assert_eq!(
        count(serde_json::json!({ "min_amount": "-99.99", "max_amount": "-10.00" })),
        2
    );
    assert_eq!(count(serde_json::json!({ "min_amount": "0" })), 1);
    assert_eq!(count(serde_json::json!({ "max_amount": "-100" })), 1);
    // Typed amounts are rounded like every typed amount, text that is not an amount leaves the end open.
    assert_eq!(
        count(serde_json::json!({ "min_amount": "-10.004", "max_amount": "-9.996" })),
        2
    );
    assert_eq!(
        count(serde_json::json!({ "min_amount": "abc", "max_amount": "" })),
        4
    );
    // An inverted range shows nothing; the ids of the same filter agree; it combines with the days.
    assert_eq!(
        count(serde_json::json!({ "min_amount": "-10", "max_amount": "-100" })),
        0
    );
    let range = filter_of(
        serde_json::json!({ "min_amount": "-100", "max_amount": "-10", "from": "2026-10-29" }),
    );
    assert_eq!(data.matching_ids(Some(&range), false).len(), 2);
    // The number format decides how the text is read: "1.000,50" in the comma-point format.
    let input: TransactionFilterInput =
        serde_json::from_value(serde_json::json!({ "min_amount": "1.000,50" })).unwrap();
    let thousands = input.resolve(NumberFormat::CommaPoint);
    assert_eq!(data.list_filtered(Some(&thousands), false, 10).count, 1);
}

#[test]
fn test_csd032_ac_15_9_the_text_filters_look_in_counterparty_and_description() {
    let own = "NL00EXAM0000000001";
    let data = imported(&[
        row(
            own,
            "30-10-2026",
            "EUR",
            "Afschrijvingen",
            "9,00",
            "Example Shop",
            "Weekly groceries",
            "R1",
        ),
        row(
            own,
            "29-10-2026",
            "EUR",
            "Afschrijvingen",
            "5,00",
            "Example Cafe",
            "Coffee at the shop",
            "R2",
        ),
        row(
            own,
            "28-10-2026",
            "EUR",
            "Afschrijvingen",
            "7,00",
            "Other Place",
            "",
            "R3",
        ),
    ]);
    let count =
        |json: serde_json::Value| data.list_filtered(Some(&filter_of(json)), false, 10).count;
    // Contains, anywhere in the text, ignoring case and surrounding spaces.
    assert_eq!(count(serde_json::json!({ "counterparty": "EXAMPLE" })), 2);
    assert_eq!(count(serde_json::json!({ "counterparty": " shop " })), 1);
    assert_eq!(count(serde_json::json!({ "description": "SHOP" })), 1);
    // Each field looks only in its own column; together they both have to match.
    assert_eq!(count(serde_json::json!({ "counterparty": "coffee" })), 0);
    assert_eq!(
        count(serde_json::json!({ "counterparty": "example", "description": "groceries" })),
        1
    );
    // An empty field leaves the filter open; `*` is no wildcard here.
    assert_eq!(
        count(serde_json::json!({ "counterparty": "  ", "description": "" })),
        3
    );
    assert_eq!(count(serde_json::json!({ "counterparty": "Example*" })), 0);
    // The ids, count and total of the same filter agree, and it combines with the others.
    let narrow = filter_of(serde_json::json!({ "counterparty": "example", "from": "2026-10-30" }));
    assert_eq!(data.matching_ids(Some(&narrow), false).len(), 1);
    assert_eq!(
        data.list_filtered(Some(&narrow), false, 10)
            .total
            .to_string(),
        "-9.00"
    );
}

#[test]
fn test_csd032_ac_28_6_bulk_changes_are_all_or_nothing() {
    let mut data = imported(&[
        simple("30-10-2026", "Afschrijvingen", "9,00", "Example Shop", "R1"),
        simple("29-10-2026", "Afschrijvingen", "5,00", "Example Cafe", "R2"),
        simple(
            "28-10-2026",
            "Afschrijvingen",
            "1,00",
            "Example Bakery",
            "R3",
        ),
    ]);
    let leisure = category(&data, "Leisure");
    let ids: Vec<TransactionId> = ["Example Shop", "Example Cafe"]
        .iter()
        .map(|name| find(&data, name).id)
        .collect();
    // Every id of a filter: all of them, or only the manual ones.
    assert_eq!(data.matching_ids(None, false).len(), 3);
    assert_eq!(data.matching_ids(None, true).len(), 0);
    // Set the category: both are in Leisure and manual, the third is untouched.
    assert_eq!(data.set_category_of_many(&ids, Some(leisure)).unwrap(), 2);
    assert_eq!(data.matching_ids(None, true).len(), 2);
    assert_eq!(find(&data, "Example Cafe").category_id, Some(leisure));
    assert_eq!(find(&data, "Example Bakery").category_id, None);
    assert!(!find(&data, "Example Bakery").manual);
    // An unknown id or category changes nothing at all.
    let before = data.clone();
    let mut with_unknown = ids.clone();
    with_unknown.push(TransactionId::new());
    assert_eq!(
        data.set_category_of_many(&with_unknown, None)
            .unwrap_err()
            .code(),
        "TRANSACTION_NOT_FOUND"
    );
    assert_eq!(
        data.set_category_of_many(&ids, Some(CategoryId::new()))
            .unwrap_err()
            .code(),
        "TRANSACTION_CATEGORY_NOT_FOUND"
    );
    assert_eq!(
        data.delete_many(&with_unknown).unwrap_err().code(),
        "TRANSACTION_NOT_FOUND"
    );
    assert_eq!(data, before);
    // Delete removes exactly those two; no ids is no change.
    assert_eq!(data.delete_many(&[]).unwrap(), 0);
    assert_eq!(data.delete_many(&ids).unwrap(), 2);
    assert_eq!(data.total_count(), 1);
    assert_eq!(
        data.transactions().first().unwrap().counterparty_name,
        "Example Bakery"
    );
}

// REQ-29

/// Imports `count` transactions for each name, with distinct references.
fn many_of(names: &[(&str, usize)]) -> Transactions {
    let mut lines = Vec::new();
    for (name, count) in names {
        for n in 0..*count {
            lines.push(simple(
                "30-10-2026",
                "Afschrijvingen",
                "9,00",
                name,
                &format!("{name}-{n}"),
            ));
        }
    }
    imported(&lines)
}

fn texts(found: &RuleSuggestions) -> Vec<(&str, usize)> {
    found
        .suggestions
        .iter()
        .map(|suggestion| (suggestion.text.as_str(), suggestion.count))
        .collect()
}

#[test]
fn test_csd032_ac_29_2_store_numbers_and_legal_forms_are_dropped() {
    let data = many_of(&[
        ("Albert Heijn 1157", 3),
        ("Albert Heijn 2210", 2),
        ("Albert Heijn B.V.", 1),
        ("Example Cafe", 4),
    ]);
    assert_eq!(
        texts(&data.suggest_rules(5).unwrap()),
        vec![("Albert Heijn*", 6)]
    );
    assert_eq!(
        texts(&data.suggest_rules(4).unwrap()),
        vec![("Albert Heijn*", 6), ("Example Cafe", 4)]
    );
    assert_eq!(data.suggest_rules(7).unwrap().suggestions.len(), 0);
    // Case does not split a group, and the most frequent spelling is the text.
    let cased = many_of(&[("EXAMPLE CAFE 12", 1), ("Example Cafe 3", 2)]);
    assert_eq!(
        texts(&cased.suggest_rules(1).unwrap()),
        vec![("Example Cafe*", 3)]
    );
}

#[test]
fn test_csd032_ac_29_3_the_count_is_what_the_rule_matches() {
    let mut lines = vec![
        simple("30-10-2026", "Afschrijvingen", "9,00", "Example Cafe", "R1"),
        simple(
            "29-10-2026",
            "Afschrijvingen",
            "9,00",
            "Example Cafe Amsterdam",
            "R2",
        ),
        simple(
            "28-10-2026",
            "Afschrijvingen",
            "9,00",
            "Example Cafe Amsterdam",
            "R3",
        ),
    ];
    // No counterparty, but the description holds the text.
    lines.push(row(
        "NL00EXAM0000000001",
        "27-10-2026",
        "EUR",
        "Afschrijvingen",
        "9,00",
        "",
        "Pin Example Cafe Utrecht",
        "R4",
    ));
    let mut data = imported(&lines);
    let shopping = category(&data, "Shopping");
    let manual = find(&data, "Example Cafe Amsterdam").id;
    data.set_category(manual, Some(shopping)).unwrap();
    // The group "Example Cafe" has one member, the rule "Example Cafe" matches all four, manual included.
    let found = data.suggest_rules(1).unwrap();
    assert_eq!(texts(&found).first(), Some(&("Example Cafe", 4)));
    assert_eq!(data.count_rule_matches("example cafe").unwrap(), 4);
    assert_eq!(data.count_rule_matches("Example Cafe A*").unwrap(), 2);
    assert_eq!(
        data.count_rule_matches("  ").unwrap_err().code(),
        "TRANSACTION_RULE_TEXT_REQUIRED"
    );
}

#[test]
fn test_csd032_ac_29_4_nothing_to_suggest_and_existing_rules_are_hidden() {
    let mut lines = vec![
        row(
            "NL00EXAM0000000001",
            "30-10-2026",
            "EUR",
            "Afschrijvingen",
            "9,00",
            "",
            "Example no counterparty",
            "R1",
        ),
        simple("29-10-2026", "Afschrijvingen", "9,00", "12345", "R2"),
        simple("28-10-2026", "Afschrijvingen", "9,00", "Example Cafe", "R3"),
        simple(
            "27-10-2026",
            "Afschrijvingen",
            "9,00",
            "Example Cafe 12",
            "R4",
        ),
    ];
    let mut data = imported(&lines);
    assert_eq!(
        texts(&data.suggest_rules(1).unwrap()),
        vec![("Example Cafe*", 2)]
    );
    // A rule with the same text hides the suggestion; its text with a star is another text.
    lines.clear();
    let leisure = category(&data, "Leisure");
    data.add_rule("example cafe*", leisure).unwrap();
    assert_eq!(data.suggest_rules(1).unwrap().suggestions.len(), 0);
}

#[test]
fn test_csd032_ac_29_5_sorted_and_cut_at_100() {
    let names: Vec<String> = (0..130_u8)
        .map(|n| {
            let first = char::from(b'a' + n / 26);
            let second = char::from(b'a' + n % 26);
            format!("Example Shop q{first}{second}")
        })
        .collect();
    let list: Vec<(&str, usize)> = names.iter().map(|name| (name.as_str(), 1)).collect();
    let mut data = many_of(&list);
    let found = data.suggest_rules(1).unwrap();
    // Every rule text "Example Shop xx" matches only its own transaction.
    assert_eq!(found.suggestions.len(), 100);
    assert_eq!(found.not_shown, 30);
    let listed: Vec<&str> = found.suggestions.iter().map(|s| s.text.as_str()).collect();
    let mut sorted = listed.clone();
    sorted.sort_unstable();
    assert_eq!(listed, sorted, "equal counts are ordered by text");
    // A higher count comes first.
    let more = many_of(&[("Example Big", 3), ("Example Small", 1)]);
    assert_eq!(
        texts(&more.suggest_rules(1).unwrap()),
        vec![("Example Big", 3), ("Example Small", 1)]
    );
    data.delete_all_transactions();
    assert_eq!(data.suggest_rules(1).unwrap().not_shown, 0);
}

#[test]
fn test_csd032_ac_29_6_the_most_used_category_is_suggested() {
    let mut data = many_of(&[("Example Cafe", 6), ("Example Bakery", 2)]);
    let leisure = category(&data, "Leisure");
    let shopping = category(&data, "Shopping");
    let ids: Vec<TransactionId> = data
        .transactions()
        .iter()
        .filter(|t| t.counterparty_name == "Example Cafe")
        .map(|t| t.id)
        .collect();
    for (position, id) in ids.iter().enumerate() {
        match position {
            0..=2 => data.set_category(*id, Some(leisure)).unwrap(),
            3 => data.set_category(*id, Some(shopping)).unwrap(),
            _ => {}
        }
    }
    let found = data.suggest_rules(1).unwrap();
    let cafe = found
        .suggestions
        .iter()
        .find(|s| s.text == "Example Cafe")
        .unwrap();
    assert_eq!(
        cafe.category_id,
        Some(leisure),
        "3 of the categorised 4, 2 uncategorized do not count"
    );
    let bakery = found
        .suggestions
        .iter()
        .find(|s| s.text == "Example Bakery")
        .unwrap();
    assert_eq!(bakery.category_id, None);
    // A tie goes to the category listed first.
    let tied = {
        let mut tied = many_of(&[("Example Tie", 2)]);
        let ids: Vec<TransactionId> = tied.transactions().iter().map(|t| t.id).collect();
        let (first, second) = (category(&tied, "Leisure"), category(&tied, "Shopping"));
        tied.set_category(*ids.first().unwrap(), Some(second))
            .unwrap();
        tied.set_category(*ids.last().unwrap(), Some(first))
            .unwrap();
        tied
    };
    let first_listed = tied
        .categories()
        .iter()
        .find(|c| c.id == category(&tied, "Leisure") || c.id == category(&tied, "Shopping"))
        .unwrap()
        .id;
    assert_eq!(
        tied.suggest_rules(1)
            .unwrap()
            .suggestions
            .first()
            .unwrap()
            .category_id,
        Some(first_listed)
    );
}

#[test]
fn test_csd032_ac_29_7_a_minimal_count_of_zero_is_rejected() {
    let data = Transactions::default();
    assert_eq!(
        data.suggest_rules(0).unwrap_err().code(),
        "TRANSACTION_SUGGEST_MIN_COUNT"
    );
}

#[test]
fn test_csd032_ac_29_10_adding_rules_is_all_or_nothing() {
    let mut data = with_rules(&[("Existing", "Groceries")]);
    let leisure = category(&data, "Leisure");
    let before = data.clone();
    for (rules, position, code) in [
        (
            vec![("Example Cafe", leisure), ("example cafe", leisure)],
            1,
            "TRANSACTION_RULE_TEXT_TAKEN",
        ),
        (
            vec![("Example Cafe", leisure), ("EXISTING", leisure)],
            1,
            "TRANSACTION_RULE_TEXT_TAKEN",
        ),
        (vec![("", leisure)], 0, "TRANSACTION_RULE_TEXT_REQUIRED"),
        (
            vec![("Example Cafe", CategoryId::new())],
            0,
            "TRANSACTION_RULE_CATEGORY_INVALID",
        ),
    ] {
        let rules: Vec<(String, CategoryId)> = rules
            .into_iter()
            .map(|(text, id)| (text.to_owned(), id))
            .collect();
        let (found_at, error) = data.add_rules(&rules).unwrap_err();
        assert_eq!((found_at, error.code()), (position, code));
        assert_eq!(data, before);
    }
    let rules = vec![
        ("Example Cafe".to_owned(), leisure),
        ("Example Bakery".to_owned(), leisure),
    ];
    assert_eq!(data.add_rules(&rules).unwrap().len(), 2);
    let added: Vec<&str> = data.rules().iter().map(|r| r.text.as_str()).collect();
    assert_eq!(added, vec!["Existing", "Example Cafe", "Example Bakery"]);
}

#[test]
fn test_csd032_ac_9_10_a_rule_with_the_same_text_is_rejected() {
    let mut data = Transactions::default();
    let groceries = category(&data, "Groceries");
    let leisure = category(&data, "Leisure");
    let first = data.add_rule("Example Market", groceries).unwrap();
    let before = data.clone();
    // The same text in another case or with spaces around it, whatever the category.
    for text in ["Example Market", "  example MARKET ", "EXAMPLE MARKET"] {
        for target in [groceries, leisure] {
            assert_eq!(
                data.add_rule(text, target).unwrap_err().code(),
                "TRANSACTION_RULE_TEXT_TAKEN"
            );
        }
    }
    assert_eq!(
        data.add_rule_with_new_category("example market", "Example Sport")
            .unwrap_err()
            .code(),
        "TRANSACTION_RULE_TEXT_TAKEN"
    );
    assert_eq!(
        data, before,
        "a rejected rule adds nothing, not even a category"
    );
    // Another text is fine, and editing a rule to keep its own text is too.
    let second = data.add_rule("Example Cafe", groceries).unwrap();
    assert!(data.edit_rule(first.id, "EXAMPLE MARKET", leisure).is_ok());
    assert_eq!(
        data.edit_rule(second.id, "example market", groceries)
            .unwrap_err()
            .code(),
        "TRANSACTION_RULE_TEXT_TAKEN"
    );
}

#[test]
fn test_csd032_ac_9_7_a_rejected_rule_or_name_adds_neither() {
    let mut data = Transactions::default();
    let before = data.clone();
    for (text, name, code) in [
        ("", "Example Sport", "TRANSACTION_RULE_TEXT_REQUIRED"),
        ("Example Gym", "", "TRANSACTION_CATEGORY_NAME_REQUIRED"),
        (
            "Example Gym",
            "groceries",
            "TRANSACTION_CATEGORY_NAME_TAKEN",
        ),
    ] {
        let error = data.add_rule_with_new_category(text, name).unwrap_err();
        assert_eq!(error.code(), code);
        assert_eq!(data, before, "{code}");
    }
}

// REQ-24

#[test]
fn test_csd032_ac_24_1_yearly_totals_split_income_and_spending_newest_year_first() {
    let data = imported(&[
        simple(
            "30-10-2026",
            "Afschrijvingen",
            "23,97",
            "Example Shop",
            "R1",
        ),
        simple(
            "28-10-2026",
            "Bijschrijvingen",
            "2500,00",
            "Example Employer",
            "R2",
        ),
        simple(
            "30-12-2025",
            "Afschrijvingen",
            "100,00",
            "Example Shop",
            "R3",
        ),
        simple(
            "15-12-2025",
            "Afschrijvingen",
            "50,00",
            "Example Cafe",
            "R4",
        ),
    ]);
    let totals = data.yearly_totals();
    assert_eq!(
        totals.iter().map(|row| row.year).collect::<Vec<_>>(),
        [2026, 2025]
    );
    let newest = totals.first().unwrap();
    assert_eq!(newest.income, money("2500.00"));
    assert_eq!(newest.spent, money("23.97"));
    assert_eq!(newest.net, money("2476.03"));
    assert_eq!(newest.transaction_count, 2);
    let older = totals.last().unwrap();
    assert_eq!(older.income, money("0.00"));
    assert_eq!(older.spent, money("150.00"));
    assert_eq!(older.net, money("-150.00"));
}

#[test]
fn test_csd032_ac_24_2_no_transactions_no_totals_and_a_deleted_year_is_gone() {
    assert_eq!(Transactions::default().yearly_totals(), Vec::new());
    let mut data = imported(&[simple("30-12-2025", "Afschrijvingen", "10,00", "A", "R1")]);
    let id = data.transactions().first().unwrap().id;
    data.delete_transaction(id).unwrap();
    assert_eq!(data.yearly_totals(), Vec::new());
}

#[test]
fn test_csd032_ac_11_5_unsetting_manual_keeps_the_category_and_lets_rules_in_again() {
    let mut data = with_rules(&[]);
    let line = simple("30-10-2026", "Afschrijvingen", "9,00", "Example Shop", "R1");
    data.import(&export(&[line]), Currency::Eur).unwrap();
    let id = data.transactions().first().unwrap().id;
    let shopping = category(&data, "Shopping");
    data.set_category(id, Some(shopping)).unwrap();
    data.unset_manual(id).unwrap();
    let only = data.transactions().first().unwrap();
    assert_eq!(only.category_id, Some(shopping));
    assert!(!only.manual);
    let leisure = category(&data, "Leisure");
    data.add_rule("Example Shop", leisure).unwrap();
    assert_eq!(data.reapply_rules(ReapplyScope::AllNonManual), 1);
    assert_eq!(
        data.transactions().first().unwrap().category_id,
        Some(leisure)
    );
}

#[test]
fn test_csd032_ac_11_6_unsetting_manual_on_a_missing_transaction_is_reported() {
    let error = Transactions::default()
        .unset_manual(TransactionId::new())
        .unwrap_err();
    assert_eq!(error.code(), "TRANSACTION_NOT_FOUND");
}

// CSD-033

fn month(year: i32, month: u8) -> YearMonth {
    YearMonth::new(year, month).unwrap()
}

/// Spending in two years: Groceries and an uncategorized debit, and a credit that the chart ignores.
fn insights_fixture() -> Transactions {
    let mut data = with_rules(&[("Example Market", "Groceries")]);
    let rows = [
        simple(
            "30-10-2026",
            "Afschrijvingen",
            "10,00",
            "Example Market",
            "R1",
        ),
        simple(
            "15-10-2026",
            "Afschrijvingen",
            "5,00",
            "Example Market",
            "R2",
        ),
        simple(
            "02-10-2026",
            "Afschrijvingen",
            "7,50",
            "Example Other",
            "R3",
        ),
        simple(
            "28-10-2026",
            "Bijschrijvingen",
            "2500,00",
            "Example Employer",
            "R4",
        ),
        simple(
            "30-12-2024",
            "Afschrijvingen",
            "20,00",
            "Example Market",
            "R5",
        ),
    ];
    data.import(&export(&rows), Currency::Eur).unwrap();
    data
}

#[test]
fn test_csd033_ac_1_1_yearly_bars_stack_the_spending_per_category() {
    let data = insights_fixture();
    let insights = data.insights(Granularity::Yearly, month(2026, 10));
    let years: Vec<i32> = insights.years.iter().map(|bar| bar.year).collect();
    assert_eq!(
        years,
        [2024, 2025, 2026],
        "a year without spending is a zero bar"
    );
    let groceries = category(&data, "Groceries").to_string();
    let last = insights.years.last().unwrap();
    assert!(last.is_partial);
    assert_eq!(last.month, None);
    assert_eq!(
        last.total,
        money("22.50"),
        "the credit of 2,500.00 is not spending"
    );
    assert_eq!(last.income, money("2500.00"));
    assert_eq!(last.marker.as_ref().unwrap().value, money("2500.00"));
    assert_eq!(last.transaction_count, 4, "three debits and a credit");
    let amounts: Vec<(&str, Money)> = last
        .segments
        .iter()
        .map(|segment| (segment.category.as_str(), segment.amount))
        .collect();
    assert_eq!(
        amounts,
        [
            (groceries.as_str(), money("15.00")),
            ("uncategorized", money("7.50"))
        ]
    );
    let middle = insights.years.get(1).unwrap();
    assert!(middle.segments.is_empty() && middle.pieces.is_empty());
    assert_eq!(middle.total, money("0.00"));
    assert!(!insights.years.first().unwrap().is_partial);
}

#[test]
fn test_csd033_ac_1_2_monthly_bars_run_from_the_first_month_with_spending_to_the_current_month() {
    let data = insights_fixture();
    let insights = data.insights(Granularity::Monthly, month(2026, 12));
    let first = insights.years.first().unwrap();
    assert_eq!((first.year, first.month), (2024, Some(12)));
    assert_eq!(insights.years.len(), 25);
    let last = insights.years.last().unwrap();
    assert_eq!(
        (last.year, last.month, last.is_partial),
        (2026, Some(12), true)
    );
    let october = insights
        .years
        .iter()
        .find(|bar| (bar.year, bar.month) == (2026, Some(10)))
        .unwrap();
    assert_eq!(october.total, money("22.50"));
    assert!(!october.is_partial);
    assert_eq!(first.total, money("20.00"));
    let january = insights
        .years
        .iter()
        .find(|bar| (bar.year, bar.month) == (2025, Some(1)))
        .unwrap();
    assert_eq!(january.total, money("0.00"));
}

#[test]
fn test_csd033_ac_2_1_the_monthly_series_holds_at_most_36_months_and_older_ones_have_no_slot() {
    let mut data = insights_fixture();
    let old = simple("15-01-2020", "Afschrijvingen", "99,00", "Example Old", "R9");
    data.import(&export(&[old]), Currency::Eur).unwrap();
    let insights = data.insights(Granularity::Monthly, month(2026, 6));
    // The last month is 2026-10 (the latest debit), so the series starts 35 months before it.
    assert_eq!(insights.years.len(), 36);
    let first = insights.years.first().unwrap();
    assert_eq!((first.year, first.month), (2023, Some(11)));
    let last = insights.years.last().unwrap();
    assert_eq!((last.year, last.month), (2026, Some(10)));
}

#[test]
fn test_csd033_ac_3_1_a_month_after_the_current_one_gets_a_slot() {
    let mut data = with_rules(&[]);
    let line = simple("15-03-2027", "Afschrijvingen", "9,00", "Example", "R1");
    data.import(&export(&[line]), Currency::Eur).unwrap();
    let insights = data.insights(Granularity::Monthly, month(2026, 12));
    let last = insights.years.last().unwrap();
    assert_eq!(
        (last.year, last.month, last.is_partial),
        (2027, Some(3), false)
    );
}

#[test]
fn test_csd033_ac_4_2_no_transactions_no_bars() {
    for granularity in [Granularity::Yearly, Granularity::Monthly] {
        assert_eq!(
            Transactions::default()
                .insights(granularity, month(2026, 6))
                .years,
            Vec::new()
        );
    }
}

#[test]
fn test_csd033_ac_4_4_a_credit_alone_gives_an_income_marker_and_no_segments() {
    let mut data = with_rules(&[]);
    let credit = simple("15-03-2026", "Bijschrijvingen", "9,00", "Example", "R1");
    data.import(&export(&[credit]), Currency::Eur).unwrap();
    for granularity in [Granularity::Yearly, Granularity::Monthly] {
        let insights = data.insights(granularity, month(2026, 6));
        let bar = insights
            .years
            .iter()
            .find(|bar| bar.income == money("9.00"))
            .unwrap();
        assert!(bar.segments.is_empty() && bar.pieces.is_empty());
        assert_eq!(bar.total, money("0.00"));
        assert_eq!(bar.marker.as_ref().unwrap().value, money("9.00"));
        assert_eq!(bar.transaction_count, 1);
    }
}

#[test]
fn test_csd033_ac_4_5_a_bar_without_credits_has_no_marker() {
    let data = insights_fixture();
    let insights = data.insights(Granularity::Yearly, month(2026, 10));
    let older = insights.years.first().unwrap();
    assert_eq!(older.income, money("0.00"));
    assert_eq!(older.marker, None);
}

#[test]
fn test_csd033_ac_4_6_the_axis_reaches_the_income_marker() {
    let data = insights_fixture();
    let insights = data.insights(Granularity::Yearly, month(2026, 10));
    let top = insights.axis.ticks.last().unwrap().value;
    assert!(top >= money("2500.00"), "{top}");
}

#[test]
fn test_csd033_ac_6_1_the_legend_has_the_categories_in_order_and_uncategorized_last() {
    let data = Transactions::default();
    let legend = data
        .insights(Granularity::Yearly, month(2026, 6))
        .categories;
    assert_eq!(legend.len(), 15);
    assert_eq!(legend.first().unwrap().name, "Salary");
    let last = legend.last().unwrap();
    assert_eq!(
        (last.id.as_str(), last.name.as_str(), last.colour_slot.get()),
        ("uncategorized", "Uncategorized", 7)
    );
    let slots: Vec<u8> = legend
        .iter()
        .map(|category| category.colour_slot.get())
        .collect();
    assert_eq!(
        &slots[..9],
        &[0, 1, 2, 3, 4, 5, 6, 7, 0],
        "the colours repeat after eight categories"
    );
}

#[test]
fn test_csd033_ac_6_2_a_category_keeps_its_colour_while_others_are_added() {
    let mut data = Transactions::default();
    let before = data
        .insights(Granularity::Yearly, month(2026, 6))
        .categories;
    data.add_category("Example Pets").unwrap();
    let after = data
        .insights(Granularity::Yearly, month(2026, 6))
        .categories;
    assert_eq!(&after[..14], &before[..14]);
}
