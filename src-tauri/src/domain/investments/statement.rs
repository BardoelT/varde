//! One statement of an account, and the form values for "copy previous" (CSD-005 REQ-6, REQ-13).

use serde::{Deserialize, Serialize};

use crate::domain::error::DomainError;
use crate::domain::money::Money;

use super::{Balance, InvestmentAccountId, Investments, NetDeposits};
use crate::domain::common::{Note, Period};
use crate::domain::series::{SeriesData, SeriesRecord};

/// The record of one account for one period (CSD-005 REQ-6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvestmentStatement {
    /// Identifier of the investment account the statement belongs to.
    pub account_id: InvestmentAccountId,
    /// The month or year the statement is for.
    pub period: Period,
    /// Closing balance of the period.
    pub balance: Balance,
    /// Net deposits of the period: money put in minus money taken out.
    pub net_deposits: NetDeposits,
    /// Free-text note, possibly empty.
    pub note: Note,
}

impl SeriesRecord for InvestmentStatement {
    type AccountId = InvestmentAccountId;
    type Order = (i32, u8);

    fn account_id(&self) -> InvestmentAccountId {
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
        Balance::new(self.balance.value()).is_ok()
            && NetDeposits::new(self.net_deposits.value()).is_ok()
            && Note::new(self.note.as_str()).is_ok()
    }
}

/// Form values for "copy previous month" or "copy previous year" (CSD-005
/// REQ-13): the note is never copied and the net deposits start at `0.00`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InvestmentStatementPrefill {
    /// Balance to prefill, `None` when there is nothing to copy.
    pub balance: Option<Money>,
    /// Net deposits to prefill.
    pub net_deposits: Money,
}

impl Investments {
    /// Form values for "copy previous month" or "copy previous year": the
    /// balance of the latest statement and no note (REQ-13).
    ///
    /// # Errors
    /// Returns `InvestmentAccountNotFound`.
    pub fn prefill_from_latest(
        &self,
        id: InvestmentAccountId,
    ) -> Result<InvestmentStatementPrefill, DomainError> {
        Ok(InvestmentStatementPrefill {
            balance: self.latest_amount(id)?,
            net_deposits: Money::ZERO,
        })
    }
}
