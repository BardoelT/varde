//! Debts: their id, name, statement period and colour slot (CSD-011 REQ-1 to REQ-6).

use serde::{Deserialize, Serialize};

use crate::domain::common::PayPeriod;
use crate::domain::common::MAX_NAME_CHARS;
use crate::domain::common::{Id, IdKind};
use crate::domain::error::DomainError;
use crate::domain::houses::HouseId;

use super::settlement::Settlement;

/// Identifies a Debt.
pub type DebtId = Id<Debt>;

impl IdKind for Debt {
    fn not_found() -> DomainError {
        DomainError::DebtNotFound
    }
}

/// The kind of a debt: Mortgage, Loan, Scholarship or Other. Only a Mortgage
/// or a Loan can be linked to a house (CSD-020 section 3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Category {
    Mortgage,
    Loan,
    Scholarship,
    #[default]
    Other,
}

impl Category {
    /// Parses `Mortgage`, `Loan`, `Scholarship` or `Other`.
    ///
    /// # Errors
    /// Returns `DebtCategoryInvalid` for any other text.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        match text.trim() {
            "Mortgage" => Ok(Self::Mortgage),
            "Loan" => Ok(Self::Loan),
            "Scholarship" => Ok(Self::Scholarship),
            "Other" => Ok(Self::Other),
            _ => Err(DomainError::DebtCategoryInvalid),
        }
    }

    /// Whether a debt of this category may be linked to a house.
    #[must_use]
    pub fn can_link(self) -> bool {
        matches!(self, Self::Mortgage | Self::Loan)
    }
}

/// One amount the user owes to one party (CSD-011 REQ-1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Debt {
    pub id: DebtId,
    pub name: String,
    pub period: PayPeriod,
    /// 0 to 7, fixed when the debt is created (CSD-011 REQ-3).
    pub colour_slot: u8,
    /// Mortgage, Loan or Other (CSD-020 REQ-15).
    pub category: Category,
    /// The house this debt is linked to (CSD-020 REQ-15).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub house_id: Option<HouseId>,
    /// How the debt was settled on the sale of its house (CSD-020 REQ-20).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settlement: Option<Settlement>,
}

/// Debt fields as typed by the user, before validation.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct DebtInput {
    #[serde(default)]
    pub name: String,
    /// `None` means Monthly when creating and "unchanged" when editing.
    #[serde(default)]
    pub period: Option<String>,
}

pub(super) struct ValidDebt {
    pub(super) name: String,
    pub(super) period: Option<PayPeriod>,
}

impl DebtInput {
    pub(super) fn validate(&self) -> Result<ValidDebt, DomainError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(DomainError::DebtNameRequired);
        }
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(DomainError::DebtNameTooLong);
        }
        let period = self
            .period
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| PayPeriod::parse(text).ok_or(DomainError::DebtPeriodUnsupported))
            .transpose()?;
        Ok(ValidDebt {
            name: name.to_owned(),
            period,
        })
    }
}
