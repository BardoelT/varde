//! A debt linked to a house is settled when the house is sold (CSD-026).

use crate::domain::money::Money;
use serde::{Deserialize, Serialize};

use crate::domain::common::PayPeriod;
use crate::domain::common::{parse_wire_name, wire_name, Period, Year, YearMonth};
use crate::domain::error::DomainError;
use crate::domain::houses::{House, HouseDate, HouseId, Houses};

use crate::domain::series::{SeriesData, SeriesRecord};

use super::{Debt, DebtBalance, DebtCategory, DebtId, DebtStatement, Debts};

/// How a debt ended on the sale of its house (CSD-026 section 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    /// The debt was paid off. It counts in full in the wealth accumulation.
    Repaid,
    /// The debt was written off. Its amount is no cash repayment.
    Cancelled,
}

impl Outcome {
    /// Parses `Repaid` or `Cancelled`, the names serde writes.
    ///
    /// # Errors
    /// Returns `HouseSettlementInvalid` for any other text.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        parse_wire_name(text).ok_or(DomainError::HouseSettlementInvalid)
    }
}

/// The outcome, date and settled amount of a settled debt (CSD-026 section 3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settlement {
    /// How the debt is settled.
    pub outcome: Outcome,
    /// The sell date of the house.
    pub date: HouseDate,
    /// The open balance at the sale.
    pub amount: DebtBalance,
}

/// A linked debt with an open balance at a sale, for the settlement prompt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OpenDebt {
    /// Identifier of the debt.
    pub debt_id: DebtId,
    /// Name of the debt.
    pub debt_name: String,
    /// The balance that was settled.
    pub amount: Money,
}

/// The period of a debt that holds `date`: the month of a monthly debt, the
/// year of a yearly one (CSD-026 section 3).
pub(crate) fn sale_period(kind: PayPeriod, date: HouseDate) -> Result<Period, DomainError> {
    let invalid = || DomainError::DebtDataInvalid;
    match kind {
        PayPeriod::Monthly => YearMonth::new(date.year(), date.month())
            .map(Period::Month)
            .ok_or_else(invalid),
        PayPeriod::Yearly => Year::new(date.year()).map(Period::Year).ok_or_else(invalid),
    }
}

impl Debts {
    /// The house a debt is linked to.
    #[must_use]
    pub fn house_of(&self, id: DebtId) -> Option<HouseId> {
        self.account(id).ok().and_then(|debt| debt.house_id)
    }

    /// The ids of the debts linked to `house_id`.
    #[must_use]
    pub fn linked_to(&self, house_id: HouseId) -> Vec<DebtId> {
        self.debts
            .iter()
            .filter(|debt| debt.house_id == Some(house_id))
            .map(|debt| debt.id)
            .collect()
    }

    /// A debt by id.
    #[must_use]
    pub fn get(&self, id: DebtId) -> Option<&Debt> {
        self.account(id).ok()
    }

    /// Whether a debt has a settlement.
    #[must_use]
    pub fn is_settled(&self, id: DebtId) -> bool {
        self.account(id).is_ok_and(|debt| debt.settlement.is_some())
    }

    /// Sets the category of a debt (CSD-020 REQ-15). A settled debt cannot change.
    ///
    /// # Errors
    /// Returns `DebtNotFound` or `DebtSettledLocked`.
    pub(crate) fn set_category(
        &mut self,
        id: DebtId,
        category: DebtCategory,
    ) -> Result<(), DomainError> {
        let debt = self.account_mut(id)?;
        if debt.category != category && debt.settlement.is_some() {
            return Err(DomainError::DebtSettledLocked);
        }
        debt.category = category;
        Ok(())
    }

    /// Links a debt to a house, or unlinks it. A settled debt keeps its house,
    /// and only a Mortgage or a Loan can be linked (CSD-020 REQ-15, CSD-026
    /// REQ-6).
    ///
    /// # Errors
    /// Returns `DebtNotFound`, `DebtSettledLocked` or `DebtLinkCategoryInvalid`.
    pub(crate) fn set_house(
        &mut self,
        id: DebtId,
        house_id: Option<HouseId>,
    ) -> Result<(), DomainError> {
        let debt = self.account_mut(id)?;
        if debt.house_id != house_id && debt.settlement.is_some() {
            return Err(DomainError::DebtSettledLocked);
        }
        if house_id.is_some() && !debt.category.can_link() {
            return Err(DomainError::DebtLinkCategoryInvalid);
        }
        debt.house_id = house_id;
        Ok(())
    }

    /// The open balance of a debt at a sale on `date`: the balance of its
    /// latest statement up to and including the sale period, `None` when there
    /// is none or it is 0.00 (CSD-026 REQ-2).
    ///
    /// # Errors
    /// Returns `DebtNotFound`.
    pub fn open_balance(&self, id: DebtId, date: HouseDate) -> Result<Option<Money>, DomainError> {
        let period = sale_period(self.account(id)?.period, date)?;
        Ok(self
            .statements
            .iter()
            .filter(|s| s.debt_id == id && s.period.sort_key() <= period.sort_key())
            .max_by_key(|s| s.order_key())
            .map(|s| s.balance.value())
            .filter(|balance| !balance.is_zero()))
    }

    /// The linked debts with an open balance at a sale on `date`, in debt
    /// order, for the settlement prompt (CSD-026 REQ-1, REQ-13).
    ///
    /// # Errors
    /// Returns `DebtNotFound` for an unknown id.
    pub fn open_debts(
        &self,
        ids: &[DebtId],
        date: HouseDate,
    ) -> Result<Vec<OpenDebt>, DomainError> {
        let mut open = Vec::new();
        for debt in self.debts() {
            if !ids.contains(&debt.id) {
                continue;
            }
            if let Some(amount) = self.open_balance(debt.id, date)? {
                open.push(OpenDebt {
                    debt_id: debt.id,
                    debt_name: debt.name.clone(),
                    amount,
                });
            }
        }
        Ok(open)
    }

    /// Settles a debt on the sale of the house it is linked to: stores the
    /// settlement and adds the settlement statement of 0.00 in the sale period,
    /// next to any regular statement of that period (CSD-026 REQ-3). This is
    /// the only way a settlement or a settlement statement comes to be, so the
    /// two always exist together and agree: the settlement carries the sell
    /// date of `house`, the statement stands in the period of that date, and
    /// no statement follows it. The debt must be linked to `house`, which must
    /// be sold, and have an open balance at the sell date.
    ///
    /// # Errors
    /// Returns `DebtNotFound`, `HouseSettlementInvalid` for a debt that is not
    /// linked to a sold `house`, is settled already or has no open balance,
    /// or `DebtStatementAfterSale` when a statement follows the sale period.
    pub fn settle(
        &mut self,
        id: DebtId,
        outcome: Outcome,
        house: &House,
    ) -> Result<(), DomainError> {
        let date = house.sale.ok_or(DomainError::HouseSettlementInvalid)?.date;
        let debt = self.account(id)?;
        if debt.house_id != Some(house.id) || debt.settlement.is_some() {
            return Err(DomainError::HouseSettlementInvalid);
        }
        let period = sale_period(debt.period, date)?;
        let amount = self
            .open_balance(id, date)?
            .ok_or(DomainError::HouseSettlementInvalid)?;
        if self.has_statement_after(id, period) {
            return Err(DomainError::DebtStatementAfterSale);
        }
        let note = crate::domain::debts::debt_note(&format!(
            "{} on sale of {}",
            wire_name(&outcome),
            house.name
        ))?;
        let statement = DebtStatement::settlement(id, period, DebtBalance::new(Money::ZERO)?, note);
        let settlement = Settlement {
            outcome,
            date,
            amount: DebtBalance::new(amount)?,
        };
        // Everything that can fail is done: the two changes go in together.
        self.account_mut(id)?.settlement = Some(settlement);
        self.statements.push(statement);
        Ok(())
    }

    /// Undoes the settlement of a debt: removes its settlement statement and
    /// leaves the regular statements as they are (CSD-026 REQ-4).
    ///
    /// # Errors
    /// Returns `DebtNotFound`.
    pub fn unsettle(&mut self, id: DebtId) -> Result<(), DomainError> {
        let debt = self.account(id)?;
        let Some(settlement) = debt.settlement.clone() else {
            return Ok(());
        };
        let period = sale_period(debt.period, settlement.date)?;
        self.statements
            .retain(|s| !(s.debt_id == id && s.period == period && s.is_settlement()));
        self.account_mut(id)?.settlement = None;
        Ok(())
    }

    /// Rejects a regular statement for `period` when the debt is settled and
    /// `period` is after the settlement period (CSD-026 REQ-6). The
    /// settlement period itself stays open for a regular statement.
    pub(super) fn check_unsettled_at(&self, id: DebtId, period: Period) -> Result<(), DomainError> {
        if self.is_after_settlement(id, period)? {
            return Err(DomainError::DebtSettledLocked);
        }
        Ok(())
    }

    /// Rejects changing or deleting the statement of `period` when the debt is
    /// settled and that statement is after the settlement period, or is the
    /// settlement statement itself, the only one in the settlement period
    /// (CSD-026 REQ-6).
    pub(super) fn check_target_editable(
        &self,
        id: DebtId,
        period: Period,
    ) -> Result<(), DomainError> {
        self.check_unsettled_at(id, period)?;
        let debt = self.account(id)?;
        let Some(settlement) = &debt.settlement else {
            return Ok(());
        };
        let settled_period = sale_period(debt.period, settlement.date)?;
        if period == settled_period && self.record(id, period).is_none() {
            return Err(DomainError::DebtSettledLocked);
        }
        Ok(())
    }

    fn is_after_settlement(&self, id: DebtId, period: Period) -> Result<bool, DomainError> {
        let debt = self.account(id)?;
        let Some(settlement) = &debt.settlement else {
            return Ok(false);
        };
        let settled_period = sale_period(debt.period, settlement.date)?;
        Ok(period.sort_key() > settled_period.sort_key())
    }

    /// Whether a statement can no longer change: the settlement statement, and
    /// the statements after the settlement period of a settled debt (CSD-026
    /// REQ-6, REQ-14).
    #[must_use]
    pub fn statement_locked(&self, statement: &DebtStatement) -> bool {
        statement.is_settlement()
            || self
                .is_after_settlement(statement.debt_id, statement.period)
                .unwrap_or(false)
    }

    /// Whether a statement of the debt stands in a period after `period`.
    fn has_statement_after(&self, id: DebtId, period: Period) -> bool {
        self.statements
            .iter()
            .any(|s| s.debt_id == id && s.period.sort_key() > period.sort_key())
    }

    /// Checks that the links and settlements read from a file follow CSD-020
    /// and CSD-026, so a damaged file cannot break them. This is the only
    /// place that checks a file for them: what the rules of `settle`,
    /// `unsettle`, `set_house` and `set_category` make impossible in memory
    /// (a settlement without its statement or the other way round, a
    /// settlement date that is not the sell date of the house, a statement
    /// after the sale, a house that is not sold or does not exist) can only
    /// come from a file whose two lists were edited apart.
    ///
    /// # Errors
    /// Returns `DebtDataInvalid` for the first inconsistency found.
    pub fn validate_links(&self, houses: &Houses) -> Result<(), DomainError> {
        self.check_settlement_statements()?;
        for debt in &self.debts {
            let house = match debt.house_id {
                Some(house_id) => Some(houses.get(house_id).ok_or(DomainError::DebtDataInvalid)?),
                None => None,
            };
            if house.is_some() && !debt.category.can_link() {
                return Err(DomainError::DebtDataInvalid);
            }
            if let Some(settlement) = &debt.settlement {
                self.check_settled(debt, settlement, house)?;
            }
        }
        Ok(())
    }

    /// A settlement statement belongs to a settled debt, in its settlement
    /// period (CSD-020 REQ-33).
    fn check_settlement_statements(&self) -> Result<(), DomainError> {
        for statement in self.statements.iter().filter(|s| s.is_settlement()) {
            let debt = self.account(statement.debt_id)?;
            let belongs = match &debt.settlement {
                Some(settlement) => sale_period(debt.period, settlement.date)? == statement.period,
                None => false,
            };
            if !belongs {
                return Err(DomainError::DebtDataInvalid);
            }
        }
        Ok(())
    }

    /// A settled debt is linked to a house sold on the settlement date, has its
    /// settlement statement of 0.00 in the sale period and no statement after
    /// it (CSD-026 REQ-3, REQ-6).
    fn check_settled(
        &self,
        debt: &Debt,
        settlement: &Settlement,
        house: Option<&House>,
    ) -> Result<(), DomainError> {
        let period = sale_period(debt.period, settlement.date)?;
        let sold_on_date =
            house.is_some_and(|house| house.sale.map(|sale| sale.date) == Some(settlement.date));
        let zero_statement = self.statements.iter().any(|s| {
            s.debt_id == debt.id
                && s.is_settlement()
                && s.period == period
                && s.balance.value().is_zero()
        });
        if !sold_on_date || !zero_statement || self.has_statement_after(debt.id, period) {
            return Err(DomainError::DebtDataInvalid);
        }
        Ok(())
    }

    /// The settled amount of the debts Cancelled in `year` that have an opening
    /// balance in it, which the wealth accumulation takes out of the repaid
    /// amount (CSD-026 REQ-10).
    #[must_use]
    pub fn cancelled_in(&self, year: i32) -> Money {
        Money::saturating_sum(self.debts.iter().filter_map(|debt| {
            let settlement = debt.settlement.as_ref()?;
            let has_opening = self
                .statements
                .iter()
                .any(|s| s.debt_id == debt.id && s.period.year() < year);
            (settlement.outcome == Outcome::Cancelled
                && settlement.date.year() == year
                && has_opening)
                .then(|| settlement.amount.value())
        }))
    }
}
