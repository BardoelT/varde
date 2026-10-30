//! A debt linked to a house is settled when the house is sold (CSD-020).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::domain::error::DomainError;
use crate::domain::houses::{HouseDate, HouseId, Houses};
use crate::domain::income::PayPeriod;
use crate::domain::investments::{Note, Period};

use super::{Category, Debt, DebtBalance, DebtId, DebtStatement, Debts};

/// How a debt ended on the sale of its house (CSD-020 section 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    /// The debt was paid off. It counts in full in the wealth accumulation.
    Repaid,
    /// The debt was written off. Its amount is no cash repayment.
    Cancelled,
}

impl Outcome {
    /// Parses `Repaid` or `Cancelled`.
    ///
    /// # Errors
    /// Returns `HouseSettlementInvalid` for any other text.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        match text.trim() {
            "Repaid" => Ok(Self::Repaid),
            "Cancelled" => Ok(Self::Cancelled),
            _ => Err(DomainError::HouseSettlementInvalid),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Repaid => "Repaid",
            Self::Cancelled => "Cancelled",
        }
    }
}

/// The statement an earlier version of the settlement replaced in the sale
/// period. Only read from old files, to turn it back into a regular statement
/// (CSD-020 REQ-16).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Replaced {
    balance: DebtBalance,
    #[serde(default)]
    note: Note,
}

/// The outcome, date and settled amount of a settled debt (CSD-020 section 3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settlement {
    pub outcome: Outcome,
    /// The sell date of the house.
    pub date: HouseDate,
    /// The open balance at the sale.
    pub amount: DebtBalance,
    /// Read from files saved before the settlement statement was added next to
    /// the regular one; never written.
    #[serde(default, skip_serializing)]
    pub(super) replaced: Option<Replaced>,
}

/// A linked debt with an open balance at a sale, for the settlement prompt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OpenDebt {
    pub debt_id: DebtId,
    pub debt_name: String,
    pub amount: Decimal,
}

/// The period of a debt that holds `date`: the month of a monthly debt, the
/// year of a yearly one (CSD-020 section 3).
pub(crate) fn sale_period(kind: PayPeriod, date: HouseDate) -> Result<Period, DomainError> {
    let text = match kind {
        PayPeriod::Monthly => format!("{:04}-{:02}", date.year(), date.month()),
        PayPeriod::Yearly => format!("{:04}", date.year()),
    };
    Period::parse(&text).map_err(|_| DomainError::DebtDataInvalid)
}

impl Debts {
    /// The house a debt is linked to.
    #[must_use]
    pub fn house_of(&self, id: DebtId) -> Option<HouseId> {
        self.debt(id).ok().and_then(|debt| debt.house_id)
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
        self.debts.iter().find(|debt| debt.id == id)
    }

    /// Whether a debt has a settlement.
    #[must_use]
    pub fn is_settled(&self, id: DebtId) -> bool {
        self.debt(id).is_ok_and(|debt| debt.settlement.is_some())
    }

    /// Sets the category of a debt (CSD-020 REQ-1). A settled debt cannot change.
    ///
    /// # Errors
    /// Returns `DebtNotFound` or `DebtSettledLocked`.
    pub(crate) fn set_category(
        &mut self,
        id: DebtId,
        category: Category,
    ) -> Result<(), DomainError> {
        let debt = self.debt_mut(id)?;
        if debt.category != category && debt.settlement.is_some() {
            return Err(DomainError::DebtSettledLocked);
        }
        debt.category = category;
        Ok(())
    }

    pub(crate) fn set_house(
        &mut self,
        id: DebtId,
        house_id: Option<HouseId>,
    ) -> Result<(), DomainError> {
        self.debt_mut(id)?.house_id = house_id;
        Ok(())
    }

    /// The open balance of a debt at a sale on `date`: the balance of its
    /// latest statement up to and including the sale period, `None` when there
    /// is none or it is 0.00 (CSD-020 REQ-4).
    ///
    /// # Errors
    /// Returns `DebtNotFound`.
    pub fn open_balance(
        &self,
        id: DebtId,
        date: HouseDate,
    ) -> Result<Option<Decimal>, DomainError> {
        let period = sale_period(self.debt(id)?.statement_period, date)?;
        Ok(self
            .statements
            .iter()
            .filter(|s| s.debt_id == id && s.period.sort_key() <= period.sort_key())
            .max_by_key(|s| s.order_key())
            .map(|s| s.balance.value())
            .filter(|balance| !balance.is_zero()))
    }

    /// The linked debts with an open balance at a sale on `date`, in debt
    /// order, for the settlement prompt (CSD-020 REQ-3, REQ-15).
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

    /// Settles a debt on the sale of the house `house_name` on `date`: stores
    /// the settlement and adds the settlement statement of 0.00 in the sale
    /// period, next to any regular statement of that period (CSD-020 REQ-5).
    /// The debt must have an open balance at `date`.
    ///
    /// # Errors
    /// Returns `DebtNotFound`, `HouseSettlementInvalid` for a debt without an
    /// open balance, or `DebtStatementAfterSale` when a statement follows the
    /// sale period.
    pub fn settle(
        &mut self,
        id: DebtId,
        outcome: Outcome,
        house_name: &str,
        date: HouseDate,
    ) -> Result<(), DomainError> {
        let period = sale_period(self.debt(id)?.statement_period, date)?;
        let amount = self
            .open_balance(id, date)?
            .ok_or(DomainError::HouseSettlementInvalid)?;
        if self
            .statements
            .iter()
            .any(|s| s.debt_id == id && s.period.sort_key() > period.sort_key())
        {
            return Err(DomainError::DebtStatementAfterSale);
        }
        let note = crate::domain::debts::debt_note(&format!(
            "{} on sale of {house_name}",
            outcome.label()
        ))?;
        self.statements.push(DebtStatement {
            debt_id: id,
            period,
            balance: DebtBalance::new(Decimal::ZERO)?,
            note,
            settlement: true,
        });
        self.debt_mut(id)?.settlement = Some(Settlement {
            outcome,
            date,
            amount: DebtBalance::new(amount)?,
            replaced: None,
        });
        Ok(())
    }

    /// Undoes the settlement of a debt: removes its settlement statement and
    /// leaves the regular statements as they are (CSD-020 REQ-6).
    ///
    /// # Errors
    /// Returns `DebtNotFound`.
    pub fn unsettle(&mut self, id: DebtId) -> Result<(), DomainError> {
        let debt = self.debt(id)?;
        let Some(settlement) = debt.settlement.clone() else {
            return Ok(());
        };
        let period = sale_period(debt.statement_period, settlement.date)?;
        self.statements
            .retain(|s| !(s.debt_id == id && s.period == period && s.settlement));
        self.debt_mut(id)?.settlement = None;
        Ok(())
    }

    /// Rejects a change to the statement of `period` when the debt is settled
    /// and `period` is the settlement period or later (CSD-020 REQ-8).
    pub(super) fn check_unsettled_at(&self, id: DebtId, period: Period) -> Result<(), DomainError> {
        let debt = self.debt(id)?;
        let Some(settlement) = &debt.settlement else {
            return Ok(());
        };
        let locked_from = sale_period(debt.statement_period, settlement.date)?;
        if period.sort_key() >= locked_from.sort_key() {
            return Err(DomainError::DebtSettledLocked);
        }
        Ok(())
    }

    /// Brings a debt settled by an earlier version into the current form: its
    /// unflagged statement of 0.00 in the settlement period becomes the
    /// settlement statement, and the statement it replaced comes back as a
    /// regular one (CSD-020 REQ-16). Files saved by this version change nothing.
    ///
    /// # Errors
    /// Returns `DebtDataInvalid` when the settlement date has no valid period.
    pub fn migrate_legacy_settlements(&mut self) -> Result<(), DomainError> {
        for index in 0..self.debts.len() {
            let Some(debt) = self.debts.get(index) else {
                continue;
            };
            let Some(settlement) = debt.settlement.clone() else {
                continue;
            };
            let (id, period) = (
                debt.id,
                sale_period(debt.statement_period, settlement.date)?,
            );
            let has_flagged = self
                .statements
                .iter()
                .any(|s| s.debt_id == id && s.period == period && s.settlement);
            if !has_flagged {
                if let Some(zero) = self.statements.iter_mut().find(|s| {
                    s.debt_id == id
                        && s.period == period
                        && !s.settlement
                        && s.balance.value().is_zero()
                }) {
                    zero.settlement = true;
                }
            }
            if let Some(replaced) = settlement.replaced {
                if self.statement(id, period).is_none() {
                    self.statements.push(DebtStatement {
                        debt_id: id,
                        period,
                        balance: replaced.balance,
                        note: replaced.note,
                        settlement: false,
                    });
                }
                if let Some(settled) = self
                    .debts
                    .get_mut(index)
                    .and_then(|d| d.settlement.as_mut())
                {
                    settled.replaced = None;
                }
            }
        }
        Ok(())
    }

    /// Whether a statement can no longer change because its debt is settled and
    /// the statement lies in or after the settlement period (CSD-020 REQ-8,
    /// REQ-18).
    #[must_use]
    pub fn statement_locked(&self, statement: &DebtStatement) -> bool {
        self.check_unsettled_at(statement.debt_id, statement.period)
            .is_err()
    }

    /// Checks that the links and settlements read from a file follow CSD-020, so
    /// a damaged file cannot break them.
    ///
    /// # Errors
    /// Returns `DebtDataInvalid` for the first inconsistency found.
    pub fn validate_links(&self, houses: &Houses) -> Result<(), DomainError> {
        let invalid = DomainError::DebtDataInvalid;
        // A settlement statement belongs to a settled debt, in its settlement
        // period (CSD-020 REQ-16).
        for statement in self.statements.iter().filter(|s| s.settlement) {
            let debt = self.debt(statement.debt_id)?;
            let belongs = match &debt.settlement {
                Some(settlement) => {
                    sale_period(debt.statement_period, settlement.date)? == statement.period
                }
                None => false,
            };
            if !belongs {
                return Err(invalid);
            }
        }
        for debt in &self.debts {
            let house = match debt.house_id {
                Some(house_id) => Some(houses.get(house_id).ok_or(invalid.clone())?),
                None => None,
            };
            if house.is_some() && !debt.category.can_link() {
                return Err(invalid);
            }
            let Some(settlement) = &debt.settlement else {
                continue;
            };
            let period = sale_period(debt.statement_period, settlement.date)?;
            let zero_statement = self.statements.iter().any(|s| {
                s.debt_id == debt.id
                    && s.settlement
                    && s.period == period
                    && s.balance.value().is_zero()
            });
            let later = self
                .statements
                .iter()
                .any(|s| s.debt_id == debt.id && s.period.sort_key() > period.sort_key());
            if house.is_none_or(|house| house.sell_date != Some(settlement.date))
                || !zero_statement
                || later
            {
                return Err(invalid);
            }
        }
        Ok(())
    }

    /// The settled amount of the debts Cancelled in `year` that have an opening
    /// balance in it, which the wealth accumulation takes out of the repaid
    /// amount (CSD-020 REQ-12).
    #[must_use]
    pub fn cancelled_in(&self, year: i32) -> Decimal {
        self.debts
            .iter()
            .filter_map(|debt| {
                let settlement = debt.settlement.as_ref()?;
                let has_opening = self
                    .statements
                    .iter()
                    .any(|s| s.debt_id == debt.id && s.period.year() < year);
                (settlement.outcome == Outcome::Cancelled
                    && settlement.date.year() == year
                    && has_opening)
                    .then(|| settlement.amount.value())
            })
            .fold(Decimal::ZERO, |sum, amount| sum + amount)
    }
}
