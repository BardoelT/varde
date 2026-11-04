//! The yearly ratios between income, wealth, growth and expenses (CSD-018).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::Serialize;

use crate::domain::bank_accounts::BankAccounts;
use crate::domain::charts::percent_label;
use crate::domain::common::{series_years, YearMonth};
use crate::domain::debts::Debts;
use crate::domain::expenses::Expenses;
use crate::domain::houses::Houses;
use crate::domain::income::Income;
use crate::domain::investments::Investments;
use crate::domain::money::{percent_of, Money};
use crate::domain::net_worth::net_worth_insights;

#[cfg(test)]
mod tests;

/// One column of the ratios table (CSD-018 REQ-3). A figure that cannot be
/// computed is `None`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RatiosYear {
    /// The calendar year.
    pub year: i32,
    /// Whether the year is still running, so its figures cover only part of it.
    pub is_partial: bool,
    /// The total gross of the year, 0.00 without income (REQ-4).
    pub total_income: Money,
    /// Bank increase plus net deposits plus repaid (REQ-5).
    pub wealth_accumulation: Option<Money>,
    /// The wealth accumulation in percent of the total income (REQ-6).
    pub wealth_percent: Option<Decimal>,
    /// [`Self::wealth_percent`] as the table shows it: "+6.09%", "-2.50%",
    /// "0.00%" or a dash (CSD-018 REQ-10).
    pub wealth_percent_label: String,
    /// The growth percentage of the Investments chart (REQ-7).
    pub investment_growth_percent: Option<Decimal>,
    /// [`Self::investment_growth_percent`] as the table shows it.
    pub investment_growth_percent_label: String,
    /// The net worth against that of the year before, in percent (REQ-8).
    pub wealth_growth_percent: Option<Decimal>,
    /// [`Self::wealth_growth_percent`] as the table shows it.
    pub wealth_growth_percent_label: String,
    /// Wealth accumulation plus expenses (REQ-9).
    pub disposable_income: Option<Money>,
    /// The disposable income in percent of the total income (REQ-9).
    pub disposable_percent: Option<Decimal>,
    /// [`Self::disposable_percent`] as the table shows it.
    pub disposable_percent_label: String,
}

/// The payload of `get_ratios` (CSD-018 section 5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Ratios {
    /// One entry per year.
    pub years: Vec<RatiosYear>,
}

/// What the ratios are computed from: the data of every area.
pub struct RatioSources<'a> {
    /// The income included in the figures.
    pub income: &'a Income,
    /// The expenses included in the figures.
    pub expenses: &'a Expenses,
    /// The houses included in the figures.
    pub houses: &'a Houses,
    /// The investments included in the figures.
    pub investments: &'a Investments,
    /// The bank accounts included in the figures.
    pub bank_accounts: &'a BankAccounts,
    /// The debts included in the figures.
    pub debts: &'a Debts,
}

/// One column per calendar year from the earliest year in which any area has a
/// record up to and including the later of the current year and the latest year
/// with a record, at most 50 years back from it, oldest first (REQ-2). A record
/// older than that has no column but still counts in every figure that spans
/// all years. Every figure comes from the code that gives it to its own page
/// (CON-2). The first column has no year before it to compare with.
#[must_use]
pub fn ratios(sources: &RatioSources<'_>, today: YearMonth) -> Ratios {
    let income: BTreeMap<i32, Money> = sources
        .income
        .yearly_totals()
        .into_iter()
        .map(|total| (total.year, total.total_gross))
        .collect();
    let expenses: BTreeMap<i32, Money> = sources
        .expenses
        .yearly_totals()
        .into_iter()
        .map(|total| (total.year, total.total))
        .collect();
    let deposits: BTreeMap<i32, Money> = sources
        .investments
        .yearly_totals()
        .into_iter()
        .map(|total| (total.year, total.net_deposits))
        .collect();
    let repaid: BTreeMap<i32, Option<Money>> = sources
        .debts
        .yearly_totals()
        .into_iter()
        .map(|total| {
            // A Cancelled debt was not paid with cash (CSD-026 REQ-10).
            let cancelled = sources.debts.cancelled_in(total.year);
            (
                total.year,
                total.repaid.map(|repaid| repaid.saturating_sub(cancelled)),
            )
        })
        .collect();
    let investment_growth: BTreeMap<i32, Option<Decimal>> = sources
        .investments
        .insights(today)
        .into_iter()
        .map(|row| (row.year, row.growth_percent))
        .collect();
    let net_worth: BTreeMap<i32, (bool, Money)> = net_worth_insights(
        sources.houses,
        sources.investments,
        sources.bank_accounts,
        sources.debts,
        today,
    )
    .years
    .into_iter()
    .map(|row| (row.year, (row.has_records, row.net_worth)))
    .collect();

    // The net worth years already span every record of the wealth areas.
    let record_years = income
        .keys()
        .chain(expenses.keys())
        .chain(net_worth.keys())
        .copied();
    let years = series_years(record_years, today.year())
        .into_iter()
        .map(|year| {
            let total_income = income.get(&year).copied().unwrap_or_default();
            let wealth_accumulation = wealth_accumulation(
                sources.bank_accounts.increase(year),
                deposits.get(&year).copied(),
                repaid.get(&year).copied().flatten(),
            );
            let disposable_income = wealth_accumulation.map(|wealth| {
                wealth.saturating_add(expenses.get(&year).copied().unwrap_or_default())
            });
            let wealth_percent =
                wealth_accumulation.and_then(|wealth| percent_of(wealth, total_income));
            let investment_growth_percent = investment_growth.get(&year).copied().flatten();
            let wealth_growth_percent = wealth_growth(&net_worth, year);
            let disposable_percent =
                disposable_income.and_then(|disposable| percent_of(disposable, total_income));
            RatiosYear {
                year,
                is_partial: year == today.year(),
                total_income,
                wealth_accumulation,
                wealth_percent,
                wealth_percent_label: percent_label(wealth_percent),
                investment_growth_percent,
                investment_growth_percent_label: percent_label(investment_growth_percent),
                wealth_growth_percent,
                wealth_growth_percent_label: percent_label(wealth_growth_percent),
                disposable_income,
                disposable_percent,
                disposable_percent_label: percent_label(disposable_percent),
            }
        })
        .collect();
    Ratios { years }
}

/// The three parts added up, a part that is none counting as 0.00, or none
/// when every part is none (REQ-5).
fn wealth_accumulation(
    bank_increase: Option<Money>,
    net_deposits: Option<Money>,
    repaid: Option<Money>,
) -> Option<Money> {
    if bank_increase.is_none() && net_deposits.is_none() && repaid.is_none() {
        return None;
    }
    Some(Money::saturating_sum([
        bank_increase.unwrap_or_default(),
        net_deposits.unwrap_or_default(),
        repaid.unwrap_or_default(),
    ]))
}

/// The net worth of `year` against that of the year before, in percent. None
/// unless both years have records and the earlier net worth is positive
/// (REQ-8).
fn wealth_growth(net_worth: &BTreeMap<i32, (bool, Money)>, year: i32) -> Option<Decimal> {
    let (has_records, now) = *net_worth.get(&year)?;
    let (earlier_has_records, before) = *net_worth.get(&(year - 1))?;
    if !has_records || !earlier_has_records || before <= Money::ZERO {
        return None;
    }
    percent_of(now.saturating_sub(before), before)
}
