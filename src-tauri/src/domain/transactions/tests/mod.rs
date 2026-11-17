//! Tests of CSD-032 and CSD-033 on synthetic exports ("Example" names, round amounts). The helpers are here;
//! the tests are in a file per area.
use super::*;
use crate::domain::common::YearMonth;
use crate::domain::money::Money;
use crate::domain::NumberFormat;

mod categories;
mod data;
mod import;
mod insights;
mod list;
mod rules;
mod suggest;

/// A day written as in the export, `dd-mm-yyyy`.
fn day(text: &str) -> TransactionDate {
    TransactionDate::parse_export(text).unwrap()
}

/// The days from `from` to `to`, `dd-mm-yyyy`, an empty text leaving the end open.
fn period(from: &str, to: &str) -> RulePeriod {
    RulePeriod {
        from: (!from.is_empty()).then(|| day(from)),
        to: (!to.is_empty()).then(|| day(to)),
    }
}

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

// REQ-2

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

// REQ-4

// REQ-5

// REQ-7

// REQ-8

// REQ-9

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

// REQ-11, REQ-12

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
    let rule = data.add_rule("*Three", shopping).unwrap();
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
    data.add_rule("Example*", leisure).unwrap();
    data
}

// REQ-13, REQ-14

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

// REQ-17

// REQ-19 (the sharing the budget relies on)

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

// REQ-24

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

/// A project without the seed rules, with transactions on 30/06, 01/07, 15/07
/// and 16/07/2026 (counterparty "Example Shop", then "Example Hotel" on 15/07).
fn holiday_fixture() -> Transactions {
    let mut data = with_rules(&[]);
    let rows = [
        simple("30-06-2026", "Afschrijvingen", "9,00", "Example Shop", "R1"),
        simple("01-07-2026", "Afschrijvingen", "9,00", "Example Shop", "R2"),
        simple(
            "15-07-2026",
            "Afschrijvingen",
            "9,00",
            "Example Hotel",
            "R3",
        ),
        simple("16-07-2026", "Afschrijvingen", "9,00", "Example Shop", "R4"),
    ];
    data.import(&export(&rows), Currency::Eur).unwrap();
    data
}

fn category_on(data: &Transactions, reference: &str) -> Option<CategoryId> {
    data.transactions()
        .iter()
        .find(|transaction| transaction.reference == reference)
        .unwrap()
        .category_id
}
