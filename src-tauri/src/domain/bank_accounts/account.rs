//! Bank accounts: their id, name, entry period and colour slot (CSD-009 REQ-1 to REQ-6).

use serde::{Deserialize, Serialize};

use crate::domain::common::{ColourSlot, PayPeriod};
use crate::domain::common::{Id, IdKind};
use crate::domain::error::DomainError;
use crate::domain::series::{AccountInput, SeriesAccount};

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
pub type BankAccountInput = AccountInput;

impl SeriesAccount for BankAccount {
    type Id = BankAccountId;

    fn id(&self) -> BankAccountId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn period(&self) -> PayPeriod {
        self.period
    }
}
