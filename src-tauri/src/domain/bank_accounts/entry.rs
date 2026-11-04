//! One entry of an account, and the form values for "copy previous" (CSD-009 REQ-7, REQ-15).

use crate::domain::money::Money;
use serde::{Deserialize, Serialize};

use crate::domain::common::{Note, Period};
use crate::domain::error::DomainError;
use crate::domain::series::{self, SeriesData, SeriesRecord};

use super::{bank_note, BankAccountId, BankAccounts, BankBalance};

/// The record of one account for one period (CSD-009 REQ-7).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BankEntry {
    /// Identifier of the bank account the entry belongs to.
    pub account_id: BankAccountId,
    /// The month or year the entry is for.
    pub period: Period,
    /// Closing balance of the period.
    pub balance: BankBalance,
    /// Free-text note, possibly empty.
    pub note: Note,
}

impl SeriesRecord for BankEntry {
    type AccountId = BankAccountId;
    type Order = (i32, u8);

    fn account_id(&self) -> BankAccountId {
        self.account_id
    }

    fn period(&self) -> Period {
        self.period
    }

    fn set_period(&mut self, period: Period) {
        self.period = period;
    }

    fn order_key(&self) -> (i32, u8) {
        self.period.sort_key()
    }

    fn amount(&self) -> Money {
        self.balance.value()
    }

    fn is_valid(&self) -> bool {
        BankBalance::new(self.balance.value()).is_ok() && bank_note(self.note.as_str()).is_ok()
    }
}

/// Parses `YYYY-MM` or `YYYY` typed by the user (CSD-009 REQ-7).
///
/// # Errors
/// Returns `BankEntryPeriodInvalid` for any other text.
pub fn parse_period(text: &str) -> Result<Period, DomainError> {
    series::parse_period(text, DomainError::BankEntryPeriodInvalid)
}

/// Form values for "copy previous month" or "copy previous year" (CSD-009
/// REQ-15): the note is never copied.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EntryPrefill {
    /// Balance to prefill, `None` when there is nothing to copy.
    pub balance: Option<Money>,
}

impl BankAccounts {
    /// Form values for "copy previous month" or "copy previous year": the
    /// balance of the latest entry and no note (REQ-15).
    ///
    /// # Errors
    /// Returns `BankAccountNotFound`.
    pub fn prefill_from_latest(&self, id: BankAccountId) -> Result<EntryPrefill, DomainError> {
        Ok(EntryPrefill {
            balance: self.latest_amount(id)?,
        })
    }
}
