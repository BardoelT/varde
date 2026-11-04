//! Debts: their id, name, statement period and colour slot (CSD-011 REQ-1 to REQ-6).

use serde::{Deserialize, Serialize};

use crate::domain::common::{parse_wire_name, ColourSlot, PayPeriod};
use crate::domain::common::{Id, IdKind};
use crate::domain::error::DomainError;
use crate::domain::houses::HouseId;

use crate::domain::series::{AccountInput, SeriesAccount};

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
pub enum DebtCategory {
    /// A mortgage, which can be linked to a house.
    Mortgage,
    /// A loan.
    Loan,
    /// A scholarship debt.
    Scholarship,
    /// Any other kind.
    #[default]
    Other,
}

impl DebtCategory {
    /// Parses `Mortgage`, `Loan`, `Scholarship` or `Other`, the names serde
    /// writes.
    ///
    /// # Errors
    /// Returns `DebtCategoryInvalid` for any other text.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        parse_wire_name(text).ok_or(DomainError::DebtCategoryInvalid)
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
    /// Identifier of the debt.
    pub id: DebtId,
    /// Name of the debt.
    pub name: String,
    /// Whether the debt is kept per month or per year.
    pub period: PayPeriod,
    /// 0 to 7, fixed when the debt is created (CSD-011 REQ-3).
    pub colour_slot: ColourSlot,
    /// Mortgage, Loan or Other (CSD-020 REQ-15).
    pub category: DebtCategory,
    /// The house this debt is linked to (CSD-020 REQ-15).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub house_id: Option<HouseId>,
    /// How the debt was settled on the sale of its house (CSD-026 REQ-3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settlement: Option<Settlement>,
}

/// Debt fields as typed by the user, before validation.
pub type DebtInput = AccountInput;

impl SeriesAccount for Debt {
    type Id = DebtId;

    fn id(&self) -> DebtId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn period(&self) -> PayPeriod {
        self.period
    }
}
