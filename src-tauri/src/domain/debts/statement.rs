//! One statement of a debt, its change, and the form values for "copy previous" (CSD-011 REQ-7, REQ-15, REQ-25).

use crate::domain::money::Money;
use serde::{Deserialize, Serialize};

use crate::domain::common::{Note, Period};
use crate::domain::error::DomainError;

use crate::domain::series::{self, SeriesData, SeriesRecord};

use super::{debt_note, DebtBalance, DebtId, Debts};

/// Whether a statement is one the user entered or the final one a settlement
/// added. The variants are in statement order: within a period the settlement
/// statement comes after the regular one (CSD-011 REQ-8).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum StatementKind {
    /// A statement the user entered.
    #[default]
    Regular,
    /// The final statement a settlement adds (CSD-026 REQ-3); it may share its
    /// period with a regular statement.
    Settlement,
}

/// The record of one debt for one period (CSD-011 REQ-7). On the wire the kind
/// is the boolean key `settlement`, left out for a regular statement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "DebtStatementRaw", into = "DebtStatementRaw")]
pub struct DebtStatement {
    /// Identifier of the debt the statement belongs to.
    pub debt_id: DebtId,
    /// The month or year the statement is for.
    pub period: Period,
    /// Closing balance of the period.
    pub balance: DebtBalance,
    /// Free-text note, possibly empty.
    pub note: Note,
    /// Whether the statement is a regular one or part of a settlement. Only
    /// `Debts::settle` makes a settlement statement, together with the
    /// settlement of its debt.
    kind: StatementKind,
}

/// The flat shape of a statement in the project file and IPC.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DebtStatementRaw {
    debt_id: DebtId,
    period: Period,
    balance: DebtBalance,
    note: Note,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    settlement: bool,
}

impl From<DebtStatementRaw> for DebtStatement {
    fn from(raw: DebtStatementRaw) -> Self {
        Self {
            debt_id: raw.debt_id,
            period: raw.period,
            balance: raw.balance,
            note: raw.note,
            kind: if raw.settlement {
                StatementKind::Settlement
            } else {
                StatementKind::Regular
            },
        }
    }
}

impl From<DebtStatement> for DebtStatementRaw {
    fn from(statement: DebtStatement) -> Self {
        let settlement = statement.is_settlement();
        Self {
            debt_id: statement.debt_id,
            period: statement.period,
            balance: statement.balance,
            note: statement.note,
            settlement,
        }
    }
}

impl DebtStatement {
    /// A statement the user enters.
    pub(super) fn regular(
        debt_id: DebtId,
        period: Period,
        balance: DebtBalance,
        note: Note,
    ) -> Self {
        Self {
            debt_id,
            period,
            balance,
            note,
            kind: StatementKind::Regular,
        }
    }

    /// The final statement of a settlement: a balance of 0.00 in the period of
    /// the sale. Only `Debts::settle` calls this, in the same step in which it
    /// sets the settlement of the debt.
    pub(super) fn settlement(
        debt_id: DebtId,
        period: Period,
        balance: DebtBalance,
        note: Note,
    ) -> Self {
        Self {
            kind: StatementKind::Settlement,
            ..Self::regular(debt_id, period, balance, note)
        }
    }

    /// Whether this is the settlement statement of a sale.
    #[must_use]
    pub fn is_settlement(&self) -> bool {
        self.kind == StatementKind::Settlement
    }
}

impl SeriesRecord for DebtStatement {
    type AccountId = DebtId;
    /// By period, and within a period the settlement statement after the
    /// regular one (CSD-011 REQ-8).
    type Order = ((i32, u8), StatementKind);

    fn account_id(&self) -> DebtId {
        self.debt_id
    }

    fn period(&self) -> Period {
        self.period
    }

    fn set_period(&mut self, period: Period) {
        self.period = period;
    }

    fn order_key(&self) -> ((i32, u8), StatementKind) {
        (self.period.sort_key(), self.kind)
    }

    fn amount(&self) -> Money {
        self.balance.value()
    }

    fn is_regular(&self) -> bool {
        !self.is_settlement()
    }

    fn is_valid(&self) -> bool {
        DebtBalance::new(self.balance.value()).is_ok() && debt_note(self.note.as_str()).is_ok()
    }
}

/// Parses `YYYY-MM` or `YYYY` typed by the user (CSD-011 REQ-7).
///
/// # Errors
/// Returns `DebtStatementPeriodInvalid` for any other text.
pub fn parse_period(text: &str) -> Result<Period, DomainError> {
    series::parse_period(text, DomainError::DebtStatementPeriodInvalid)
}

/// Form values for "copy previous month" or "copy previous year" (CSD-011
/// REQ-15): the note is never copied.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DebtStatementPrefill {
    /// Balance to prefill, `None` when there is nothing to copy.
    pub balance: Option<Money>,
}

impl Debts {
    /// Form values for "copy previous month" or "copy previous year": the
    /// balance of the latest statement and no note (REQ-15).
    ///
    /// # Errors
    /// Returns `DebtNotFound`.
    pub fn prefill_from_latest(&self, id: DebtId) -> Result<DebtStatementPrefill, DomainError> {
        Ok(DebtStatementPrefill {
            balance: self.latest_amount(id)?,
        })
    }

    /// The change of a statement: the balance of the previous statement of the
    /// debt minus its own, and for a first statement the opening amount of the
    /// debt minus its own (REQ-25). `None` only when the debt does not exist.
    /// Derived each time, never stored.
    #[must_use]
    pub fn change(&self, statement: &DebtStatement) -> Option<Money> {
        let before = match self.previous_record(statement) {
            Some(previous) => previous.balance.value(),
            None => self.account(statement.debt_id).ok()?.opening_amount.value(),
        };
        Some(before.saturating_sub(statement.balance.value()))
    }
}
