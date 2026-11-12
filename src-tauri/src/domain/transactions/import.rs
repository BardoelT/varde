//! Reading a Knab CSV export (CSD-032 REQ-2 to REQ-4).
//!
//! The whole file is checked before anything is returned, so a bad row
//! imports nothing (REQ-3).

use std::str::FromStr;

use rust_decimal::Decimal;

use crate::domain::error::DomainError;
use crate::domain::money::{Currency, Money};

use super::transaction::{TransactionDate, MAX_FIELD_CHARS};

/// The header row of a Knab export.
const HEADER: [&str; 15] = [
    "Rekeningnummer",
    "Transactiedatum",
    "Valutacode",
    "CreditDebet",
    "Bedrag",
    "Tegenrekeningnummer",
    "Tegenrekeninghouder",
    "Valutadatum",
    "Betaalwijze",
    "Omschrijving",
    "Type betaling",
    "Machtigingsnummer",
    "Incassant ID",
    "Referentie",
    "Boekdatum",
];

/// The longest amount text, a bound on untrusted input.
const MAX_AMOUNT_CHARS: usize = 20;

/// One valid row of an export, before it becomes a transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Row {
    pub own_account: String,
    pub date: TransactionDate,
    pub amount: Money,
    pub counterparty_iban: String,
    pub counterparty_name: String,
    pub payment_method: String,
    pub description: String,
    pub reference: String,
}

/// Splits a line at its semicolons. A field may be in double quotes, in which
/// a semicolon is text and `""` is a quote. `None` for an unterminated quote.
fn split_fields(line: &str) -> Option<Vec<String>> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match (in_quotes, c) {
            (true, '"') if chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            (true, '"') => in_quotes = false,
            (false, '"') if field.is_empty() => in_quotes = true,
            (false, ';') => fields.push(std::mem::take(&mut field)),
            _ => field.push(c),
        }
    }
    if in_quotes {
        return None;
    }
    fields.push(field);
    Some(fields)
}

/// Reads an amount such as `23,97` into its magnitude, which must be above
/// zero with at most two decimals and within the largest amount.
fn parse_amount(text: &str) -> Option<Money> {
    let text = text.trim();
    let (whole, fraction) = text.split_once(',').unwrap_or((text, ""));
    let digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    if text.len() > MAX_AMOUNT_CHARS
        || whole.is_empty()
        || !digits(whole)
        || !digits(fraction)
        || fraction.len() > 2
        || (text.contains(',') && fraction.is_empty())
    {
        return None;
    }
    let value = Decimal::from_str(&format!("{whole}.{fraction}0")).ok()?;
    let money = Money::from_decimal(value).ok()?;
    (!money.is_zero() && money <= Money::MAX_BALANCE).then_some(money)
}

/// Parses one data row. Checks run in the order columns, field length, date,
/// direction, amount, reference, currency.
fn parse_row(line: usize, text: &str, currency: Currency) -> Result<Row, DomainError> {
    let fields = split_fields(text)
        .filter(|fields| fields.len() == HEADER.len())
        .ok_or(DomainError::TransactionImportColumns { line })?;
    if fields
        .iter()
        .any(|field| field.chars().count() > MAX_FIELD_CHARS)
    {
        return Err(DomainError::TransactionImportField { line });
    }
    let field = |index: usize| fields.get(index).map_or("", |text| text.trim());
    let date = TransactionDate::parse_export(field(1))
        .ok_or(DomainError::TransactionImportDate { line })?;
    let debit = match field(3) {
        "Afschrijvingen" => true,
        "Bijschrijvingen" => false,
        _ => return Err(DomainError::TransactionImportDirection { line }),
    };
    let magnitude = parse_amount(field(4)).ok_or(DomainError::TransactionImportAmount { line })?;
    if field(13).is_empty() {
        return Err(DomainError::TransactionImportReference { line });
    }
    if Currency::parse(field(2)) != Some(currency) {
        return Err(DomainError::TransactionImportCurrency { line });
    }
    Ok(Row {
        own_account: field(0).to_owned(),
        date,
        amount: if debit { -magnitude } else { magnitude },
        counterparty_iban: field(5).to_owned(),
        counterparty_name: field(6).to_owned(),
        payment_method: field(8).to_owned(),
        description: field(9).to_owned(),
        reference: field(13).to_owned(),
    })
}

/// Parses a whole export in the project currency.
///
/// # Errors
/// `TransactionImportFormat` when the header is not the Knab header, and the
/// error of the first offending line otherwise.
pub(super) fn parse(text: &str, currency: Currency) -> Result<Vec<Row>, DomainError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .enumerate();
    let header = lines
        .next()
        .and_then(|(_, line)| split_fields(line))
        .ok_or(DomainError::TransactionImportFormat)?;
    let is_knab = header.len() == HEADER.len()
        && header
            .iter()
            .zip(HEADER)
            .all(|(found, expected)| found.trim() == expected);
    if !is_knab {
        return Err(DomainError::TransactionImportFormat);
    }
    lines
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| parse_row(index + 1, line, currency))
        .collect()
}
