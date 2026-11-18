//! One bank transaction and its date (CSD-032 REQ-7).

use serde::de::Error as _;
use serde::ser::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use time::macros::format_description;
use time::Date;

use crate::domain::common::{Id, IdKind, YearMonth};
use crate::domain::error::DomainError;
use crate::domain::money::Money;

use super::{AccountId, Category};

/// Identifies a transaction.
pub type TransactionId = Id<Transaction>;

impl IdKind for Transaction {
    fn not_found() -> DomainError {
        DomainError::TransactionNotFound
    }
}

/// The longest stored text field of a transaction, in characters. A bound on
/// untrusted input (CSD-032 CON-6).
pub const MAX_FIELD_CHARS: usize = 1000;

/// The calendar day of a transaction. The export writes `dd-mm-yyyy`, the user
/// sees `dd/mm/yyyy`, and the project file and IPC hold `YYYY-MM-DD`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransactionDate(Date);

impl TransactionDate {
    /// Parses the `dd-mm-yyyy` of a Knab export.
    #[must_use]
    pub fn parse_export(text: &str) -> Option<Self> {
        Date::parse(text.trim(), format_description!("[day]-[month]-[year]"))
            .ok()
            .map(Self)
    }

    /// Parses the `yyyymmdd` of an ABN AMRO export.
    #[must_use]
    pub fn parse_compact(text: &str) -> Option<Self> {
        Date::parse(text.trim(), format_description!("[year][month][day]"))
            .ok()
            .map(Self)
    }

    /// Parses the `dd/mm/yyyy` the user types.
    #[must_use]
    pub fn parse_typed(text: &str) -> Option<Self> {
        Date::parse(text.trim(), format_description!("[day]/[month]/[year]"))
            .ok()
            .map(Self)
    }

    /// The calendar year.
    #[must_use]
    pub fn year(self) -> i32 {
        self.0.year()
    }

    /// The month the date falls in (CSD-028 REQ-3).
    #[must_use]
    pub fn year_month(self) -> YearMonth {
        YearMonth::from_date(self.0)
    }
}

impl Serialize for TransactionDate {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0
            .format(format_description!("[year]-[month]-[day]"))
            .map_err(S::Error::custom)
            .and_then(|text| serializer.serialize_str(&text))
    }
}

impl<'de> Deserialize<'de> for TransactionDate {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Date::parse(&text, format_description!("[year]-[month]-[day]"))
            .map(Self)
            .map_err(D::Error::custom)
    }
}

/// One booked bank line (CSD-032 section 3). `category_id` is `None` for the
/// built-in "Uncategorized".
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transaction {
    /// Identifier of the transaction.
    pub id: TransactionId,
    /// The account the transaction belongs to (CSD-034).
    pub account_id: AccountId,
    /// The day the transaction was made.
    pub date: TransactionDate,
    /// Signed amount: a debit is negative, a credit positive.
    pub amount: Money,
    /// Name of the other side, possibly empty.
    pub counterparty_name: String,
    /// IBAN of the other side, possibly empty.
    pub counterparty_iban: String,
    /// The description of the booking, possibly empty.
    pub description: String,
    /// The account the export belongs to, kept as text.
    pub own_account: String,
    /// The bank's own id of the booking.
    pub reference: String,
    /// How the payment was made, possibly empty.
    pub payment_method: String,
    /// The category, or `None` for Uncategorized.
    pub category_id: Option<Id<Category>>,
    /// Whether the user chose the category by hand (CSD-032 REQ-11).
    pub manual: bool,
}

impl Transaction {
    /// Whether the stored fields follow the rules of CSD-032, for data read
    /// from a file.
    pub(super) fn is_consistent(&self) -> bool {
        let fits = |text: &str| text.chars().count() <= MAX_FIELD_CHARS;
        fits(&self.counterparty_name)
            && fits(&self.counterparty_iban)
            && fits(&self.description)
            && fits(&self.own_account)
            && fits(&self.reference)
            && fits(&self.payment_method)
            && !self.reference.is_empty()
    }
}
