//! The fixed categories of an expense (CSD-013 REQ-3, CSD-014 section 3).

use serde::{Deserialize, Serialize};

use crate::domain::common::{parse_wire_name, ColourSlot};
use crate::domain::error::DomainError;

/// What an expense was for. The list is fixed and its order is the order of
/// the segments and the legend of the Insights chart (CSD-014 REQ-4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExpenseCategory {
    Home,
    Vehicle,
    Holiday,
    Sport,
    Study,
    Appliances,
    Hobby,
    Other,
}

impl ExpenseCategory {
    /// Every category, in the fixed order.
    pub const ALL: [Self; 8] = [
        Self::Home,
        Self::Vehicle,
        Self::Holiday,
        Self::Sport,
        Self::Study,
        Self::Appliances,
        Self::Hobby,
        Self::Other,
    ];

    /// Parses the exact name serde writes, so "home" is not accepted.
    ///
    /// # Errors
    /// Returns `ExpenseCategoryUnsupported`.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        parse_wire_name(text).ok_or(DomainError::ExpenseCategoryUnsupported)
    }

    /// The colour slot 0 to 7 of the chart, fixed by the place in the list
    /// (CSD-014 REQ-6).
    #[must_use]
    pub fn colour_slot(self) -> ColourSlot {
        Self::ALL
            .iter()
            .position(|category| *category == self)
            .and_then(|index| u8::try_from(index).ok())
            .and_then(ColourSlot::new)
            .unwrap_or_default()
    }
}
