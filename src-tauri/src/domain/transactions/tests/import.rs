use super::*;

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
