//! Capital growth of a statement and the balance-or-growth calculation of the form (CSD-005 REQ-23 to REQ-25).

use serde::{Deserialize, Serialize};

use crate::domain::error::DomainError;
use crate::domain::money::Money;

use super::amounts::parse_growth;
use super::{Balance, InvestmentAccountId, InvestmentStatement, Investments, NetDeposits};
use crate::domain::common::Period;
use crate::domain::series::SeriesData;

/// Which of balance and capital growth the user typed in the statement form
/// (CSD-005 REQ-24).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CalculationInput {
    /// The balance is entered and the capital growth is calculated.
    Balance,
    /// The capital growth is entered and the balance is calculated.
    Growth,
}

/// The balance and capital growth the statement form shows after a
/// calculation (CSD-005 REQ-24).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StatementCalculation {
    /// The balance.
    pub balance: Money,
    /// Capital growth.
    pub capital_growth: Money,
}

impl Investments {
    /// The capital growth of a statement: its balance minus the balance of the
    /// previous statement of the account minus its net deposits. An account is
    /// taken to start at 0.00, so its first statement has no previous balance to
    /// subtract (CSD-005 REQ-23).
    #[must_use]
    pub fn capital_growth(&self, statement: &InvestmentStatement) -> Money {
        let previous = self
            .previous_balance(statement.account_id, statement.period, None)
            .unwrap_or(Money::ZERO);
        growth_of(
            statement.balance.value(),
            previous,
            statement.net_deposits.value(),
        )
    }

    /// Calculates the other one of balance and capital growth for the
    /// statement form, from the value the user typed in and the net deposits.
    /// `ignore` is the current period of the statement being edited, which
    /// never counts as a previous statement. Nothing is stored (CSD-005
    /// REQ-24, REQ-25).
    ///
    /// # Errors
    /// Returns `InvestmentAccountNotFound`, `InvestmentPeriodMismatch`, or the
    /// net deposits and balance or growth errors of REQ-22, REQ-9 and REQ-25.
    pub fn calculate(
        &self,
        account_id: InvestmentAccountId,
        period: Period,
        net_deposits: &str,
        input: CalculationInput,
        value: &str,
        ignore: Option<Period>,
    ) -> Result<StatementCalculation, DomainError> {
        if self.account(account_id)?.period != period.kind() {
            return Err(DomainError::InvestmentPeriodMismatch);
        }
        let net_deposits = NetDeposits::parse(net_deposits)?.value();
        let previous = self
            .previous_balance(account_id, period, ignore)
            .unwrap_or(Money::ZERO);
        match input {
            CalculationInput::Balance => {
                let balance = Balance::parse(value)?.value();
                Ok(StatementCalculation {
                    balance,
                    capital_growth: growth_of(balance, previous, net_deposits),
                })
            }
            CalculationInput::Growth => {
                let growth = parse_growth(value)?;
                let total = previous
                    .checked_add(net_deposits)
                    .and_then(|sum| sum.checked_add(growth))
                    .ok_or(DomainError::InvestmentBalanceTooLarge)?;
                let balance = Balance::new(total)?;
                Ok(StatementCalculation {
                    balance: balance.value(),
                    capital_growth: growth,
                })
            }
        }
    }
}

/// Balance minus the previous balance minus the net deposits, as one amount.
/// Every operand is a stored amount of at most 1,000,000,000.00, so the
/// bounded subtraction never reaches its bound.
fn growth_of(balance: Money, previous: Money, net_deposits: Money) -> Money {
    balance
        .saturating_sub(previous)
        .saturating_sub(net_deposits)
}
