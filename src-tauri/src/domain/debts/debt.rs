//! Debts: their id, name, statement period and colour slot (CSD-011 REQ-1 to REQ-6).

use serde::{Deserialize, Serialize};

use crate::domain::common::{parse_wire_name, ColourSlot, PayPeriod};
use crate::domain::common::{Id, IdKind};
use crate::domain::error::DomainError;
use crate::domain::houses::{HouseDate, HouseId};

use crate::domain::series::SeriesAccount;

use super::settlement::Settlement;
use super::DebtBalance;

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
    /// Whether the debt is kept per month or per year. Read through
    /// [`SeriesAccount::period`]; only the debts module changes it, so a
    /// settled debt keeps the period its settlement was made in.
    pub(super) period: PayPeriod,
    /// 0 to 7, fixed when the debt is created (CSD-011 REQ-3).
    pub colour_slot: ColourSlot,
    /// The day the debt started (CSD-011 REQ-30). Required in the project file.
    pub(super) opening_date: HouseDate,
    /// The amount the debt started with (CSD-011 REQ-30). Required in the
    /// project file; it feeds no figure (CSD-011 NG-15).
    pub(super) opening_amount: DebtBalance,
    /// Mortgage, Loan or Other (CSD-020 REQ-15). Only `Debts::set_category`
    /// changes it.
    pub(super) category: DebtCategory,
    /// The house this debt is linked to (CSD-020 REQ-15). Only
    /// `Debts::set_house` changes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) house_id: Option<HouseId>,
    /// How the debt was settled on the sale of its house (CSD-026 REQ-3). Only
    /// `Debts::settle` and `Debts::unsettle` change it, and they change it
    /// together with the settlement statement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) settlement: Option<Settlement>,
}

impl Debt {
    /// A new debt: unlinked, unsettled and of the default category.
    pub(super) fn new(
        id: DebtId,
        name: String,
        period: PayPeriod,
        colour_slot: ColourSlot,
        opening_date: HouseDate,
        opening_amount: DebtBalance,
    ) -> Self {
        Self {
            id,
            name,
            period,
            colour_slot,
            opening_date,
            opening_amount,
            category: DebtCategory::default(),
            house_id: None,
            settlement: None,
        }
    }

    /// The day the debt started.
    #[must_use]
    pub fn opening_date(&self) -> HouseDate {
        self.opening_date
    }

    /// The amount the debt started with.
    #[must_use]
    pub fn opening_amount(&self) -> DebtBalance {
        self.opening_amount
    }

    /// The kind of the debt.
    #[must_use]
    pub fn category(&self) -> DebtCategory {
        self.category
    }

    /// The house the debt is linked to.
    #[must_use]
    pub fn house_id(&self) -> Option<HouseId> {
        self.house_id
    }

    /// How the debt was settled on the sale of its house, `None` while open.
    #[must_use]
    pub fn settlement(&self) -> Option<&Settlement> {
        self.settlement.as_ref()
    }
}

#[cfg(test)]
impl DebtInput {
    /// The shorthand opening of the scenarios of CSD-011: 01/01/2000 and
    /// 200000.00, for tests that do not care about it.
    pub(crate) fn example() -> Self {
        Self {
            name: String::new(),
            period: None,
            opening_date: "01/01/2000".to_owned(),
            opening_amount: "200000.00".to_owned(),
        }
    }
}

/// Debt fields as typed by the user, before validation (CSD-011 REQ-1, REQ-30).
/// The dates are typed `dd/mm/yyyy`.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct DebtInput {
    /// The name of the debt.
    #[serde(default)]
    pub name: String,
    /// `None` means Monthly when creating and "unchanged" when editing.
    #[serde(default)]
    pub period: Option<String>,
    /// The day the debt started; required.
    #[serde(default)]
    pub opening_date: String,
    /// The amount the debt started with; required.
    #[serde(default)]
    pub opening_amount: String,
}

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
