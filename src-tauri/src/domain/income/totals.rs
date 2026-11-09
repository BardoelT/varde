//! Yearly totals over all jobs (CSD-001 REQ-11) and the gross up to a month (CSD-004).

use std::collections::BTreeMap;

use crate::domain::money::Money;
use serde::Serialize;

use super::{Entry, Income, Job};
use crate::domain::common::YearMonth;

/// Totals over all jobs for one calendar year (CSD-001 REQ-11).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IncomeYearlyTotal {
    /// The calendar year.
    pub year: i32,
    /// Gross salary.
    pub gross_salary: Money,
    /// Holiday pay.
    pub holiday_pay: Money,
    /// Bonus.
    pub bonus: Money,
    /// Gross salary, holiday pay and bonus together.
    pub total_gross: Money,
    /// Number of entries in the year.
    pub entry_count: usize,
}

impl Income {
    /// The total gross of `year` over all jobs, counting the paychecks of the
    /// months up to and including `up_to` and the annual statement of the year
    /// in full (CSD-004 REQ-9, REQ-10). 0.00 when there is none. Months after
    /// `up_to`, which only a project file can contain, are not counted.
    #[must_use]
    pub fn gross_in_year_up_to(&self, year: i32, up_to: YearMonth) -> Money {
        let paychecks = self
            .paychecks
            .iter()
            .filter(|p| p.month.year() == year && p.month <= up_to)
            .map(|p| p.amounts.total_gross());
        let statements = self
            .statements
            .iter()
            .filter(|s| s.year.value() == year)
            .map(|s| s.amounts.total_gross());
        Money::saturating_sum(paychecks.chain(statements))
    }

    /// The entries of all jobs with their job, newest period first (an annual
    /// statement counts as December of its year) and, within a period, in job
    /// order (REQ-12).
    #[must_use]
    pub fn all_entries(&self) -> Vec<(&Job, Entry<'_>)> {
        let mut rows: Vec<(usize, &Job, Entry<'_>)> = Vec::new();
        for (position, job) in self.jobs().into_iter().enumerate() {
            rows.extend(
                self.entries()
                    .filter(|entry| entry.job_id() == job.id)
                    .map(|entry| (position, job, entry)),
            );
        }
        rows.sort_by(|a, b| b.2.sort_key().cmp(&a.2.sort_key()).then(a.0.cmp(&b.0)));
        rows.into_iter()
            .map(|(_, job, entry)| (job, entry))
            .collect()
    }

    /// Sums per calendar year over all jobs, newest year first, without
    /// extrapolating partial years (REQ-11). An annual statement counts in its
    /// own year only.
    #[must_use]
    pub fn yearly_totals(&self) -> Vec<IncomeYearlyTotal> {
        let mut by_year: BTreeMap<i32, IncomeYearlyTotal> = BTreeMap::new();
        for entry in self.entries() {
            let year = entry.year();
            let amounts = entry.amounts();
            let total = by_year.entry(year).or_insert_with(|| IncomeYearlyTotal {
                year,
                gross_salary: Money::ZERO,
                holiday_pay: Money::ZERO,
                bonus: Money::ZERO,
                total_gross: Money::ZERO,
                entry_count: 0,
            });
            total.gross_salary = total.gross_salary.saturating_add(amounts.gross_salary());
            total.holiday_pay = total.holiday_pay.saturating_add(amounts.holiday_pay());
            total.bonus = total.bonus.saturating_add(amounts.bonus());
            total.total_gross = total.total_gross.saturating_add(amounts.total_gross());
            total.entry_count += 1;
        }
        by_year.into_values().rev().collect()
    }
}
