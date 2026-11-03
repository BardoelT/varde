//! Bank accounts: their id, name, entry period and colour slot (CSD-009 REQ-1 to REQ-6).

use serde::{Deserialize, Serialize};

use crate::domain::common::MAX_NAME_CHARS;
use crate::domain::common::{ColourSlot, PayPeriod};
use crate::domain::common::{Id, IdKind};
use crate::domain::error::DomainError;

/// Identifies a `BankAccount`.
pub type BankAccountId = Id<BankAccount>;

impl IdKind for BankAccount {
    fn not_found() -> DomainError {
        DomainError::BankAccountNotFound
    }
}

/// One place where the user keeps money at a bank (CSD-009 REQ-1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BankAccount {
    pub id: BankAccountId,
    pub name: String,
    pub period: PayPeriod,
    /// 0 to 7, fixed when the account is created (CSD-009 REQ-3).
    pub colour_slot: ColourSlot,
}

/// Account fields as typed by the user, before validation.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct BankAccountInput {
    #[serde(default)]
    pub name: String,
    /// `None` means Monthly when creating and "unchanged" when editing.
    #[serde(default)]
    pub period: Option<String>,
}

pub(super) struct ValidAccount {
    pub(super) name: String,
    pub(super) period: Option<PayPeriod>,
}

impl BankAccountInput {
    pub(super) fn validate(&self) -> Result<ValidAccount, DomainError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(DomainError::BankAccountNameRequired);
        }
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(DomainError::BankAccountNameTooLong);
        }
        let period = self
            .period
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| PayPeriod::parse(text).ok_or(DomainError::BankAccountPeriodUnsupported))
            .transpose()?;
        Ok(ValidAccount {
            name: name.to_owned(),
            period,
        })
    }
}
