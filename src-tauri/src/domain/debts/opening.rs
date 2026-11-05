//! The opening and the closing of a debt, the window they make for its
//! statements, and the Opening row of the Statements table (CSD-011 REQ-30 to
//! REQ-33, CSD-026 REQ-15 to REQ-17).

use std::cmp::Reverse;

use serde::Deserialize;

use crate::domain::common::{wire_name, Period};
use crate::domain::error::DomainError;
use crate::domain::houses::HouseDate;
use crate::domain::money::Money;
use crate::domain::series::{name_order, SeriesData};

use super::settlement::{sale_period, Origin, Outcome, Settlement};
use super::{debt_note, Debt, DebtBalance, DebtId, DebtStatement, Debts};

/// The closing fields of the debt form as typed (CSD-011 REQ-34). All three
/// empty means no closing.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ClosingInput {
    /// The closing date, `dd/mm/yyyy`.
    #[serde(default)]
    pub date: String,
    /// The amount still open at the closing.
    #[serde(default)]
    pub amount: String,
    /// `Repaid` or `Cancelled`.
    #[serde(default)]
    pub outcome: String,
}

impl ClosingInput {
    /// Whether nothing is typed in the closing fields.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.date.trim().is_empty()
            && self.amount.trim().is_empty()
            && self.outcome.trim().is_empty()
    }
}

/// A validated closing: the settlement a manual closing stores.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Closing {
    /// How the debt ended.
    pub outcome: Outcome,
    /// The closing date.
    pub date: HouseDate,
    /// The amount still open at the closing.
    pub amount: DebtBalance,
}

/// One row of the Statements table: a statement, or the Opening row of a debt
/// (CSD-011 REQ-33).
#[derive(Clone, Copy, Debug)]
pub enum StatementRow<'a> {
    /// A statement of a debt.
    Statement(&'a Debt, &'a DebtStatement),
    /// The opening of a debt, shown as a read-only row.
    Opening(&'a Debt),
}

/// Parses the opening amount typed by the user (CSD-011 REQ-30, REQ-31).
///
/// # Errors
/// Returns the `DebtOpeningAmount*` errors.
fn parse_opening_amount(text: &str) -> Result<DebtBalance, DomainError> {
    if text.trim().is_empty() {
        return Err(DomainError::DebtOpeningAmountRequired);
    }
    DebtBalance::parse(text).map_err(|error| match error {
        DomainError::DebtStatementBalanceNegative => DomainError::DebtOpeningAmountNegative,
        DomainError::DebtStatementBalanceTooLarge => DomainError::DebtOpeningAmountTooLarge,
        _ => DomainError::DebtOpeningAmountInvalid,
    })
}

/// Parses the opening date and amount typed in the debt form.
///
/// # Errors
/// Returns the `DebtOpening*` errors of CSD-011 REQ-30 and REQ-31.
pub(super) fn parse_opening(
    date: &str,
    amount: &str,
) -> Result<(HouseDate, DebtBalance), DomainError> {
    if date.trim().is_empty() {
        return Err(DomainError::DebtOpeningDateRequired);
    }
    let date = HouseDate::parse(date).map_err(|_| DomainError::DebtOpeningDateInvalid)?;
    Ok((date, parse_opening_amount(amount)?))
}

impl ClosingInput {
    /// Validates the closing fields (CSD-011 REQ-31); `None` when all three
    /// are empty.
    ///
    /// # Errors
    /// Returns the `DebtClosing*` errors.
    pub fn parse(&self) -> Result<Option<Closing>, DomainError> {
        if self.is_empty() {
            return Ok(None);
        }
        let (date, amount, outcome) = (self.date.trim(), self.amount.trim(), self.outcome.trim());
        if date.is_empty() != amount.is_empty() {
            return Err(DomainError::DebtClosingIncomplete);
        }
        if date.is_empty() {
            return Err(DomainError::DebtClosingIncomplete);
        }
        let date = HouseDate::parse(date).map_err(|_| DomainError::DebtClosingDateInvalid)?;
        let amount = DebtBalance::parse(amount).map_err(|error| match error {
            DomainError::DebtStatementBalanceNegative => DomainError::DebtClosingAmountNegative,
            DomainError::DebtStatementBalanceTooLarge => DomainError::DebtClosingAmountTooLarge,
            _ => DomainError::DebtClosingAmountInvalid,
        })?;
        if outcome.is_empty() {
            return Err(DomainError::DebtClosingOutcomeRequired);
        }
        let outcome =
            Outcome::parse(outcome).map_err(|_| DomainError::DebtClosingOutcomeRequired)?;
        Ok(Some(Closing {
            outcome,
            date,
            amount,
        }))
    }
}

impl Debts {
    /// The period of the opening date of a debt, in the kind of the debt.
    fn opening_period(debt: &Debt) -> Result<Period, DomainError> {
        sale_period(debt.period, debt.opening_date)
    }

    /// Whether a statement period is before the opening of the debt
    /// (CSD-011 REQ-32).
    pub(super) fn is_before_opening(debt: &Debt, period: Period) -> Result<bool, DomainError> {
        Ok(period.sort_key() < Self::opening_period(debt)?.sort_key())
    }

    /// Changes the opening of a debt. A date after a statement, or after the
    /// closing, is rejected (CSD-011 REQ-31, REQ-32).
    ///
    /// # Errors
    /// Returns `DebtNotFound`, `DebtClosingBeforeOpening` or
    /// `DebtDatesExcludeStatements`.
    pub(super) fn set_opening(
        &mut self,
        id: DebtId,
        date: HouseDate,
        amount: DebtBalance,
    ) -> Result<(), DomainError> {
        let debt = self.account(id)?;
        if debt
            .settlement
            .as_ref()
            .is_some_and(|settlement| settlement.date < date)
        {
            return Err(DomainError::DebtClosingBeforeOpening);
        }
        let first = sale_period(debt.period, date)?;
        if self
            .statements
            .iter()
            .any(|s| s.debt_id == id && s.period.sort_key() < first.sort_key())
        {
            return Err(DomainError::DebtDatesExcludeStatements);
        }
        let debt = self.account_mut(id)?;
        debt.opening_date = date;
        debt.opening_amount = amount;
        Ok(())
    }

    /// Closes a debt by hand, or changes its closing in place: stores the
    /// settlement with the origin Manual and puts its settlement statement of
    /// 0.00 in the period of the closing date, next to any regular statement
    /// of that period, replacing the settlement statement of an earlier closing
    /// (CSD-026 REQ-15, REQ-17). The debt may be linked to a house, held or
    /// sold, and keeps its link.
    ///
    /// # Errors
    /// Returns `DebtNotFound`, `DebtClosingBeforeOpening` or
    /// `DebtDatesExcludeStatements`.
    pub fn set_closing(&mut self, id: DebtId, closing: &Closing) -> Result<(), DomainError> {
        let debt = self.account(id)?;
        if closing.date < debt.opening_date {
            return Err(DomainError::DebtClosingBeforeOpening);
        }
        let period = sale_period(debt.period, closing.date)?;
        let old_period = match &debt.settlement {
            Some(old) => Some(sale_period(debt.period, old.date)?),
            None => None,
        };
        // A regular statement after the new closing would lie outside the
        // window; the settlement statement of the old closing moves with it.
        if self.statements.iter().any(|s| {
            s.debt_id == id && !s.is_settlement() && s.period.sort_key() > period.sort_key()
        }) {
            return Err(DomainError::DebtDatesExcludeStatements);
        }
        let note = debt_note(&format!("{} on closing", wire_name(&closing.outcome)))?;
        let statement = DebtStatement::settlement(id, period, DebtBalance::new(Money::ZERO)?, note);
        let settlement = Settlement {
            outcome: closing.outcome,
            origin: Origin::Manual,
            date: closing.date,
            amount: closing.amount,
        };
        // Everything that can fail is done: the changes go in together.
        if let Some(old_period) = old_period {
            self.statements
                .retain(|s| !(s.debt_id == id && s.period == old_period && s.is_settlement()));
        }
        self.account_mut(id)?.settlement = Some(settlement);
        self.statements.push(statement);
        Ok(())
    }

    /// The rows of the Statements table: the statements of all debts and the
    /// Opening row of each debt, newest period first. A yearly debt counts as
    /// December of its year, within a period the settlement statement comes
    /// first, then the regular one, then the Opening row, and then the order
    /// of the debts (CSD-011 REQ-18, REQ-33).
    #[must_use]
    pub fn statement_rows(&self) -> Vec<StatementRow<'_>> {
        let mut rows: Vec<StatementRow<'_>> = self
            .all_statements()
            .into_iter()
            .map(|(debt, statement)| StatementRow::Statement(debt, statement))
            .collect();
        rows.extend(self.debts.iter().map(StatementRow::Opening));
        rows.sort_by_cached_key(|row| match row {
            StatementRow::Statement(debt, statement) => (
                Reverse((
                    statement.period.sort_key(),
                    1 + u8::from(statement.is_settlement()),
                )),
                name_order(&debt.name, &debt.id),
            ),
            StatementRow::Opening(debt) => (
                Reverse((
                    Self::opening_period(debt).map_or((i32::MIN, 0), Period::sort_key),
                    0,
                )),
                name_order(&debt.name, &debt.id),
            ),
        });
        rows
    }
}
