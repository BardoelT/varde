//! Reading an ABN AMRO "TXT" export, the `.TAB` file (CSD-032 REQ-34).
//!
//! A line has eight tab-separated fields and the file has no header: the own
//! account, the currency, the booking day (`yyyymmdd`), the balance before and
//! after, the value day, the signed amount with a decimal comma, and one
//! description. The description is structured in one of three ways, and the
//! counterparty, its IBAN and the payment method are taken from it. Like the
//! Knab reader it checks the whole file before it returns anything.

use std::collections::HashMap;
use std::str::FromStr;

use rust_decimal::Decimal;

use crate::domain::error::DomainError;
use crate::domain::money::{Currency, Money};

use super::import::Row;
use super::transaction::{TransactionDate, MAX_FIELD_CHARS};

/// The number of fields of a line.
pub(super) const FIELDS: usize = 8;

/// The longest amount text, a bound on untrusted input.
const MAX_AMOUNT_CHARS: usize = 20;

/// The tags of a description that is written as `/TAG/value/TAG/value`.
const SLASH_TAGS: [&str; 16] = [
    "TRTP", "CSID", "NAME", "MARF", "REMI", "ISDT", "IBAN", "BIC", "EREF", "ORDP", "BENM", "CNTP",
    "ADDR", "ID", "SVCL", "PREF",
];

/// The labels of a description that is written as `Label: value`.
const LABELS: [&str; 8] = [
    "IBAN:",
    "BIC:",
    "Naam:",
    "Omschrijving:",
    "Kenmerk:",
    "Machtiging:",
    "Incassant:",
    "Valutadatum:",
];

/// What a description tells about the other side.
#[derive(Debug, Default, PartialEq, Eq)]
struct Parts {
    counterparty_name: String,
    counterparty_iban: String,
    payment_method: String,
    description: String,
}

/// Replaces every run of white space by one space and trims.
fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The positions of the `/TAG/` markers of a structured description.
fn slash_markers(text: &str) -> Vec<(usize, &'static str)> {
    let mut found = Vec::new();
    for (index, _) in text.match_indices('/') {
        let rest = text.get(index + 1..).unwrap_or_default();
        if let Some(tag) = SLASH_TAGS.iter().find(|tag| {
            rest.strip_prefix(**tag)
                .is_some_and(|after| after.starts_with('/'))
        }) {
            found.push((index, *tag));
        }
    }
    found
}

/// `/TRTP/SEPA OVERBOEKING/IBAN/NL00.../BIC/.../NAME/Example Shop/REMI/October/EREF/...`.
/// The creditor id, mandate and end-to-end reference are left out (REQ-7).
fn from_slash_tags(text: &str) -> Parts {
    let markers = slash_markers(text);
    let value_of = |wanted: &str| -> String {
        markers
            .iter()
            .enumerate()
            .find(|(_, (_, tag))| *tag == wanted)
            .map(|(position, (index, tag))| {
                let start = index + tag.len() + 2;
                let end = markers
                    .get(position + 1)
                    .map_or(text.len(), |(next, _)| *next);
                collapse(text.get(start..end).unwrap_or_default())
            })
            .unwrap_or_default()
    };
    Parts {
        counterparty_name: value_of("NAME"),
        counterparty_iban: value_of("IBAN"),
        payment_method: value_of("TRTP"),
        description: value_of("REMI"),
    }
}

/// The positions of the `Label:` markers of a labelled description; a label
/// counts at the start or after white space.
fn label_markers(text: &str) -> Vec<(usize, &'static str)> {
    let mut found = Vec::new();
    for label in LABELS {
        for (index, _) in text.match_indices(label) {
            let at_boundary = text
                .get(..index)
                .and_then(|before| before.chars().last())
                .is_none_or(char::is_whitespace);
            if at_boundary {
                found.push((index, label));
            }
        }
    }
    found.sort_by_key(|(index, _)| *index);
    found
}

/// `SEPA Overboeking   IBAN: NL00...   BIC: ...   Naam: Example Shop   Omschrijving: October`.
fn from_labels(text: &str) -> Parts {
    let markers = label_markers(text);
    let first = markers.first().map_or(text.len(), |(index, _)| *index);
    let value_of = |wanted: &str| -> String {
        markers
            .iter()
            .enumerate()
            .find(|(_, (_, label))| *label == wanted)
            .map(|(position, (index, label))| {
                let start = index + label.len();
                let end = markers
                    .get(position + 1)
                    .map_or(text.len(), |(next, _)| *next);
                collapse(text.get(start..end).unwrap_or_default())
            })
            .unwrap_or_default()
    };
    Parts {
        counterparty_name: value_of("Naam:"),
        counterparty_iban: value_of("IBAN:"),
        payment_method: collapse(text.get(..first).unwrap_or_default()),
        description: value_of("Omschrijving:"),
    }
}

/// The name in `BEA   NR:AB1234 17.11.26/12.30 Example Shop,PAS123`: what lies
/// between the day and time and the `,PAS`.
fn card_name(text: &str) -> String {
    let Some(pas) = text.find(",PAS") else {
        return String::new();
    };
    let before = text.get(..pas).unwrap_or_default();
    let is_stamp = |word: &str| {
        word.len() == 14
            && word.chars().enumerate().all(|(at, c)| {
                if matches!(at, 2 | 5 | 8 | 11) {
                    matches!(c, '.' | '/')
                } else {
                    c.is_ascii_digit()
                }
            })
    };
    let words: Vec<&str> = before.split_whitespace().collect();
    words
        .iter()
        .position(|word| is_stamp(word))
        .map(|at| words.get(at + 1..).unwrap_or_default().join(" "))
        .unwrap_or_default()
}

/// Everything else: the bank's own lines such as the costs of the account, and
/// the card payments and cash withdrawals.
fn from_free_text(text: &str) -> Parts {
    let collapsed = collapse(text);
    let (payment_method, counterparty_name) = if collapsed.starts_with("BEA") {
        ("Betaalautomaat", card_name(text))
    } else if collapsed.starts_with("GEA") {
        ("Geldautomaat", card_name(text))
    } else if collapsed.starts_with("ABN AMRO Bank") {
        ("", "ABN AMRO Bank N.V.".to_owned())
    } else {
        ("", String::new())
    };
    Parts {
        counterparty_name,
        counterparty_iban: String::new(),
        payment_method: payment_method.to_owned(),
        description: collapsed,
    }
}

/// Splits a description into what the transaction stores.
fn read_description(text: &str) -> Parts {
    let trimmed = text.trim();
    if trimmed.starts_with("/TRTP/") {
        from_slash_tags(trimmed)
    } else if !label_markers(trimmed).is_empty() {
        from_labels(trimmed)
    } else {
        from_free_text(trimmed)
    }
}

/// Reads a signed amount such as `-23,97` or `1250,00`: digits and at most
/// one decimal comma with up to two decimals, within the largest amount and
/// not zero. A thousands separator is not written.
fn parse_signed(text: &str) -> Option<Money> {
    let text = text.trim();
    let (negative, unsigned) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let (whole, fraction) = unsigned.split_once(',').unwrap_or((unsigned, ""));
    let digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    if text.len() > MAX_AMOUNT_CHARS
        || whole.is_empty()
        || !digits(whole)
        || !digits(fraction)
        || fraction.len() > 2
        || (unsigned.contains(',') && fraction.is_empty())
    {
        return None;
    }
    let value = Decimal::from_str(&format!("{whole}.{fraction}0")).ok()?;
    let value = if negative { -value } else { value };
    let money = Money::from_decimal(value).ok()?;
    (!money.is_zero() && money.abs() <= Money::MAX_BALANCE).then_some(money)
}

/// Whether the first line of a file is an ABN AMRO line: eight tab-separated
/// fields are not needed to say so, one tab is enough to choose this reader,
/// which then checks the columns of every line.
pub(super) fn looks_like(first_line: &str) -> bool {
    first_line.contains('\t')
}

/// Parses one line. Checks run in the order columns, field length, date,
/// amount, currency.
fn parse_line(line: usize, text: &str, currency: Currency) -> Result<(Row, String), DomainError> {
    let fields: Vec<&str> = text.split('\t').collect();
    if fields.len() != FIELDS {
        return Err(DomainError::TransactionImportColumns { line });
    }
    if fields
        .iter()
        .any(|field| field.chars().count() > MAX_FIELD_CHARS)
    {
        return Err(DomainError::TransactionImportField { line });
    }
    let field = |index: usize| fields.get(index).map_or("", |text| text.trim());
    let date = TransactionDate::parse_compact(field(2))
        .ok_or(DomainError::TransactionImportDate { line })?;
    let amount = parse_signed(field(6)).ok_or(DomainError::TransactionImportAmount { line })?;
    if Currency::parse(field(1)) != Some(currency) {
        return Err(DomainError::TransactionImportCurrency { line });
    }
    let parts = read_description(field(7));
    // The bank gives no reference. The day, the amount and the balance after
    // the booking say the same for the same booking in every export.
    let key = format!("{}|{}|{}", field(2), field(6), field(4));
    Ok((
        Row {
            own_account: field(0).to_owned(),
            date,
            amount,
            counterparty_iban: parts.counterparty_iban,
            counterparty_name: parts.counterparty_name,
            payment_method: parts.payment_method,
            description: parts.description,
            reference: String::new(),
        },
        key,
    ))
}

/// Parses a whole ABN AMRO export in the project currency.
///
/// # Errors
/// The error of the first offending line.
pub(super) fn parse(text: &str, currency: Currency) -> Result<Vec<Row>, DomainError> {
    let mut occurrences: HashMap<String, usize> = HashMap::new();
    text.split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            let (mut row, key) = parse_line(index + 1, line, currency)?;
            // Two bookings with the same day, amount and balance (a payment and its
            // refund in a row) get a number, so both are kept.
            let seen = occurrences.entry(key.clone()).or_insert(0);
            *seen += 1;
            row.reference = if *seen == 1 {
                key
            } else {
                format!("{key}|{seen}")
            };
            Ok(row)
        })
        .collect()
}
