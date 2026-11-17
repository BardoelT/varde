//! Tests of the ABN AMRO TAB import (CSD-032 REQ-34).

use super::*;

/// One line of a TAB export: account, currency, day (`yyyymmdd`), balance before
/// and after, value day, amount and description.
fn abn_line(day: &str, before: &str, after: &str, amount: &str, description: &str) -> String {
    [
        "000000001",
        "EUR",
        day,
        before,
        after,
        day,
        amount,
        description,
    ]
    .join("\t")
}

fn abn_export(lines: &[String]) -> String {
    lines.join("\r\n") + "\r\n"
}

fn abn_imported(lines: &[String]) -> Transactions {
    let mut data = Transactions::default();
    data.import(&abn_export(lines), Currency::Eur).unwrap();
    data
}

const STRUCTURED: &str = "/TRTP/SEPA OVERBOEKING/IBAN/NL00EXAM0000000002/BIC/EXAMNL2A/NAME/Example Shop 1234 Town/REMI/Order 1/EREF/NOTPROVIDED          ";
const LABELLED: &str = "SEPA Overboeking                 IBAN: NL00EXAM0000000003        BIC: EXAMNL2A                    Naam: Example Employer B.V.                  Omschrijving: Salary October     Kenmerk: EXAMPLE-0001        ";
const COLLECTION: &str = "/TRTP/SEPA Incasso algemeen doorlopend/CSID/NL00ZZZ000000000000/NAME/Example Energy/MARF/MANDATE-0001/REMI/Advance October/IBAN/NL00EXAM0000000004/BIC/EXAMNL2A/EREF/EXAMPLE-0002";

#[test]
fn test_csd032_ac_34_1_a_structured_labelled_card_and_bank_description_are_read() {
    let data = abn_imported(&[
        abn_line("20261030", "1000,00", "975,50", "-24,50", STRUCTURED),
        abn_line("20261029", "975,50", "2475,50", "1500,00", LABELLED),
        abn_line(
            "20261028",
            "2475,50",
            "2465,50",
            "-10,00",
            "BEA   NR:EX0001 28.10.26/12.30 Example Cafe,PAS123 NR:00000001 Example Town          ",
        ),
        abn_line(
            "20261027",
            "2465,50",
            "2455,50",
            "-10,00",
            "GEA   NR:EX0002 27.10.26/09.15 Example Bank Pin,PAS123          ",
        ),
        abn_line(
            "20261026",
            "2455,50",
            "2450,00",
            "-5,50",
            "ABN AMRO Bank N.V.               Costs of the account                 5,50",
        ),
        abn_line("20261025", "2450,00", "2300,00", "-150,00", COLLECTION),
    ]);
    assert_eq!(data.total_count(), 6);
    let by = |name: &str| find(&data, name).clone();
    // `/TAG/value` description: name, IBAN, payment method and the remittance text.
    let shop = by("Example Shop 1234 Town");
    assert_eq!(shop.amount, money("-24.50"));
    assert_eq!(shop.date, day("30-10-2026"));
    assert_eq!(shop.counterparty_iban, "NL00EXAM0000000002");
    assert_eq!(shop.payment_method, "SEPA OVERBOEKING");
    assert_eq!(shop.description, "Order 1");
    assert_eq!(shop.own_account, "000000001");
    // `Label: value` description.
    let employer = by("Example Employer B.V.");
    assert_eq!(employer.amount, money("1500.00"));
    assert_eq!(employer.counterparty_iban, "NL00EXAM0000000003");
    assert_eq!(employer.payment_method, "SEPA Overboeking");
    assert_eq!(employer.description, "Salary October");
    // A card payment and a cash withdrawal: the name between the stamp and ",PAS".
    let cafe = by("Example Cafe");
    assert_eq!(cafe.payment_method, "Betaalautomaat");
    assert!(
        cafe.description.starts_with("BEA NR:EX0001"),
        "{}",
        cafe.description
    );
    assert_eq!(by("Example Bank Pin").payment_method, "Geldautomaat");
    // A line of the bank itself.
    let bank = by("ABN AMRO Bank N.V.");
    assert_eq!(bank.amount, money("-5.50"));
    assert!(bank.description.contains("Costs of the account"));
    // A collection: the name and the remittance, not the creditor id, mandate or reference (REQ-7).
    let energy = by("Example Energy");
    assert_eq!(energy.payment_method, "SEPA Incasso algemeen doorlopend");
    assert_eq!(energy.description, "Advance October");
    let stored = serde_json::to_string(data.transactions()).unwrap();
    for gone in [
        "NL00ZZZ000000000000",
        "MANDATE-0001",
        "EXAMPLE-0002",
        "NOTPROVIDED",
        "EXAMPLE-0001",
    ] {
        assert!(!stored.contains(gone), "{gone} is stored");
    }
}

#[test]
fn test_csd032_ac_34_2_the_amount_is_signed_and_the_day_is_yyyymmdd() {
    let data = abn_imported(&[
        abn_line("20261030", "0,00", "-23,97", "-23,97", STRUCTURED),
        abn_line("20261031", "-23,97", "1226,03", "1250,00", LABELLED),
    ]);
    let amounts: Vec<Money> = data.transactions().iter().map(|t| t.amount).collect();
    assert_eq!(amounts, vec![money("-23.97"), money("1250.00")]);
    assert_eq!(data.transactions()[1].date, day("31-10-2026"));
    // A plus sign and a whole amount are fine.
    assert!(
        abn_imported(&[abn_line("20261030", "0,00", "5,00", "+5", STRUCTURED)])
            .transactions()
            .first()
            .is_some_and(|t| t.amount == money("5.00"))
    );
}

#[test]
fn test_csd032_ac_34_3_one_bad_line_imports_nothing_and_names_its_line() {
    let good = |n: u32| {
        abn_line(
            "20261030",
            "0,00",
            &format!("{n},00"),
            &format!("{n},00"),
            STRUCTURED,
        )
    };
    let cases: Vec<(String, &str)> = vec![
        (
            abn_line("20261331", "0,00", "1,00", "1,00", STRUCTURED),
            "TRANSACTION_IMPORT_DATE",
        ),
        (
            abn_line("30-10-2026", "0,00", "1,00", "1,00", STRUCTURED),
            "TRANSACTION_IMPORT_DATE",
        ),
        (
            abn_line("20261030", "0,00", "1,00", "abc", STRUCTURED),
            "TRANSACTION_IMPORT_AMOUNT",
        ),
        (
            abn_line("20261030", "0,00", "0,00", "0,00", STRUCTURED),
            "TRANSACTION_IMPORT_AMOUNT",
        ),
        (
            abn_line("20261030", "0,00", "1,23", "1,234", STRUCTURED),
            "TRANSACTION_IMPORT_AMOUNT",
        ),
        (
            abn_line("20261030", "0,00", "1,00", "1.000,00", STRUCTURED),
            "TRANSACTION_IMPORT_AMOUNT",
        ),
        (
            "000000001\tEUR\t20261030\t0,00\t1,00\t20261030\t1,00".to_owned(),
            "TRANSACTION_IMPORT_COLUMNS",
        ),
        (
            abn_line("20261030", "0,00", "1,00", "1,00", &"x".repeat(1001)),
            "TRANSACTION_IMPORT_FIELD",
        ),
        (
            abn_line("20261030", "0,00", "1,00", "1,00", STRUCTURED).replace("EUR", "USD"),
            "TRANSACTION_IMPORT_CURRENCY",
        ),
    ];
    for (bad, code) in cases {
        let mut data = Transactions::default();
        let text = abn_export(&[good(1), good(2), bad, good(4)]);
        let error = data.import(&text, Currency::Eur).unwrap_err();
        assert_eq!(error.code(), code);
        // There is no header, so the third line is line 3.
        assert!(error.to_string().contains("Line 3"), "{error}");
        assert_eq!(data.total_count(), 0, "{code}");
    }
}

#[test]
fn test_csd032_ac_34_4_the_same_booking_is_skipped_and_a_repeat_in_a_row_is_kept() {
    let first = [
        abn_line("20261029", "100,00", "110,00", "10,00", LABELLED),
        abn_line("20261030", "110,00", "86,03", "-23,97", STRUCTURED),
    ];
    let mut data = Transactions::default();
    assert_eq!(
        data.import(&abn_export(&first), Currency::Eur)
            .unwrap()
            .added,
        2
    );
    // An overlapping export holds the same two bookings and a new one.
    let overlapping = [
        first[0].clone(),
        first[1].clone(),
        abn_line("20261031", "86,03", "76,03", "-10,00", COLLECTION),
    ];
    let summary = data
        .import(&abn_export(&overlapping), Currency::Eur)
        .unwrap();
    assert_eq!((summary.added, summary.skipped), (1, 2));
    // A payment and its refund of the same amount leave the same balance twice: both are kept.
    let mut other = Transactions::default();
    let same_balance = [
        abn_line("20261030", "100,00", "105,00", "5,00", LABELLED),
        abn_line("20261030", "105,00", "100,00", "-5,00", STRUCTURED),
        abn_line("20261030", "100,00", "105,00", "5,00", LABELLED),
    ];
    assert_eq!(
        other
            .import(&abn_export(&same_balance), Currency::Eur)
            .unwrap()
            .added,
        3
    );
    // Importing that file again adds nothing.
    assert_eq!(
        other
            .import(&abn_export(&same_balance), Currency::Eur)
            .unwrap()
            .added,
        0
    );
}

#[test]
fn test_csd032_ac_34_5_the_format_is_chosen_by_the_file_and_other_files_are_refused() {
    // A Knab file still works, and a mixed project holds both.
    let mut data = imported(&[simple(
        "30-10-2026",
        "Afschrijvingen",
        "9,00",
        "Example Shop",
        "R1",
    )]);
    let summary = data
        .import(
            &abn_export(&[abn_line("20261030", "0,00", "1,00", "1,00", STRUCTURED)]),
            Currency::Eur,
        )
        .unwrap();
    assert_eq!(summary.added, 1);
    assert_eq!(data.total_count(), 2);
    // Text with neither semicolons nor tabs is neither format.
    for text in ["", "just some text\r\n", "30-10-2026,9.00\r\n"] {
        let error = Transactions::default()
            .import(text, Currency::Eur)
            .unwrap_err();
        assert_eq!(error.code(), "TRANSACTION_IMPORT_FORMAT", "{text:?}");
    }
}

#[test]
fn test_csd032_ac_34_6_an_abn_amro_line_is_categorised_by_the_rules() {
    let mut data = with_rules(&[("Example Shop*", "Groceries"), ("*Salary*", "Salary")]);
    data.import(
        &abn_export(&[
            abn_line("20261030", "1000,00", "975,50", "-24,50", STRUCTURED),
            abn_line("20261029", "975,50", "2475,50", "1500,00", LABELLED),
        ]),
        Currency::Eur,
    )
    .unwrap();
    assert_eq!(
        find(&data, "Example Shop 1234 Town").category_id,
        Some(category(&data, "Groceries"))
    );
    assert_eq!(
        find(&data, "Example Employer B.V.").category_id,
        Some(category(&data, "Salary"))
    );
}
