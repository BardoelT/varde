//! Investment accounts: their id, name and statement period (CSD-005 REQ-1 to REQ-5).

use serde::{Deserialize, Serialize};

use crate::domain::common::PayPeriod;
use crate::domain::common::MAX_NAME_CHARS;
use crate::domain::common::{Id, IdKind};
use crate::domain::error::DomainError;

/// Identifies a `InvestmentAccount`.
pub type InvestmentAccountId = Id<InvestmentAccount>;

impl IdKind for InvestmentAccount {
    fn not_found() -> DomainError {
        DomainError::InvestmentAccountNotFound
    }
}

/// One place where the user holds investments (CSD-005 REQ-1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvestmentAccount {
    pub id: InvestmentAccountId,
    pub name: String,
    pub period: PayPeriod,
}

/// Account fields as typed by the user, before validation.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct InvestmentAccountInput {
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

impl InvestmentAccountInput {
    pub(super) fn validate(&self) -> Result<ValidAccount, DomainError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(DomainError::InvestmentAccountNameRequired);
        }
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(DomainError::InvestmentAccountNameTooLong);
        }
        let period = self
            .period
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| PayPeriod::parse(text).ok_or(DomainError::InvestmentPeriodUnsupported))
            .transpose()?;
        Ok(ValidAccount {
            name: name.to_owned(),
            period,
        })
    }
}
