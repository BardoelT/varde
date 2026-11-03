//! Investment accounts: their id, name and statement period (CSD-005 REQ-1 to REQ-5).

use serde::{Deserialize, Serialize};

use crate::domain::common::PayPeriod;
use crate::domain::common::{Id, IdKind};
use crate::domain::error::DomainError;
use crate::domain::series::{AccountInput, SeriesAccount};

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
pub type InvestmentAccountInput = AccountInput;

impl SeriesAccount for InvestmentAccount {
    type Id = InvestmentAccountId;

    fn id(&self) -> InvestmentAccountId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn period(&self) -> PayPeriod {
        self.period
    }
}
