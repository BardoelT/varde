//! Yearly totals over all transactions (CSD-032 REQ-24).

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;

use crate::domain::money::Money;

use super::category::{CategoryId, UNCATEGORIZED_ENTITY};
use super::Transactions;

/// Months in a year: the length of every monthly series of the grid.
const MONTHS: usize = 12;

/// The net of the twelve months of a year, January first; `None` for a month
/// without a transaction (CSD-032 REQ-24).
pub type MonthlyNets = Vec<Option<Money>>;

/// One category row of the grid of a year (CSD-032 REQ-24). `id` is the
/// category id, or `uncategorized`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CategoryYearlyNet {
    /// The category id, or `uncategorized`.
    pub id: String,
    /// The name of the category.
    pub name: String,
    /// The signed net (credits minus debits) per month, January first.
    pub months: MonthlyNets,
    /// The net of the category in the year.
    pub total: Money,
}

/// Adds a signed amount to a month of a series.
fn add_to_month(months: &mut [Option<Money>], month: u8, amount: Money) {
    if let Some(cell) = months.get_mut(usize::from(month.saturating_sub(1))) {
        *cell = Some(cell.unwrap_or(Money::ZERO).saturating_add(amount));
    }
}

/// The grid of one year while it is built: the net per category slot and month.
struct YearGrid {
    rows: HashMap<usize, MonthlyNets>,
    months: MonthlyNets,
}

impl YearGrid {
    fn new() -> Self {
        Self {
            rows: HashMap::new(),
            months: vec![None; MONTHS],
        }
    }
}

/// The totals of one calendar year (CSD-032 REQ-24).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TransactionYearlyTotal {
    /// The calendar year.
    pub year: i32,
    /// The sum of the credits of the year.
    pub income: Money,
    /// The sum of the debits of the year, as a positive amount.
    pub spent: Money,
    /// `income` minus `spent`, negative when more was spent than came in.
    pub net: Money,
    /// How many transactions the year has.
    pub transaction_count: usize,
    /// The categories that have a transaction in the year, in the order of the
    /// category list with Uncategorized last, each with its net per month.
    pub categories: Vec<CategoryYearlyNet>,
    /// The net of all categories per month, January first.
    pub months: MonthlyNets,
    /// The net of the year; equal to `net`.
    pub total: Money,
}

impl Transactions {
    /// Totals per calendar year with at least one transaction, newest year
    /// first, over every transaction whatever the filter of the list, without
    /// extrapolating partial years, each with the net per category and month
    /// (REQ-24).
    #[must_use]
    pub fn yearly_totals(&self) -> Vec<TransactionYearlyTotal> {
        let slots: HashMap<CategoryId, usize> = self
            .categories()
            .iter()
            .enumerate()
            .map(|(slot, category)| (category.id, slot))
            .collect();
        let uncategorized = self.categories().len();
        let mut years: BTreeMap<i32, TransactionYearlyTotal> = BTreeMap::new();
        let mut grids: BTreeMap<i32, YearGrid> = BTreeMap::new();
        for transaction in self.transactions() {
            let year = transaction.date.year();
            let row = years.entry(year).or_insert_with(|| TransactionYearlyTotal {
                year,
                income: Money::ZERO,
                spent: Money::ZERO,
                net: Money::ZERO,
                transaction_count: 0,
                categories: Vec::new(),
                months: Vec::new(),
                total: Money::ZERO,
            });
            if transaction.amount.is_negative() {
                row.spent = row.spent.saturating_add(transaction.amount.abs());
            } else {
                row.income = row.income.saturating_add(transaction.amount);
            }
            row.net = row.income.saturating_sub(row.spent);
            row.transaction_count += 1;
            let slot = transaction
                .category_id
                .and_then(|id| slots.get(&id).copied())
                .unwrap_or(uncategorized);
            let month = transaction.date.year_month().month();
            let grid = grids.entry(year).or_insert_with(YearGrid::new);
            add_to_month(&mut grid.months, month, transaction.amount);
            add_to_month(
                grid.rows.entry(slot).or_insert_with(|| vec![None; MONTHS]),
                month,
                transaction.amount,
            );
        }
        for (year, total) in &mut years {
            total.total = total.net;
            if let Some(mut grid) = grids.remove(year) {
                total.months = grid.months;
                total.categories = self.grid_rows(&mut grid.rows);
            }
        }
        years.into_values().rev().collect()
    }

    /// The category rows of a year, in the order of the category list with
    /// Uncategorized last; a category without a transaction has no row.
    fn grid_rows(&self, rows: &mut HashMap<usize, MonthlyNets>) -> Vec<CategoryYearlyNet> {
        let names = self
            .categories()
            .iter()
            .map(|category| (category.id.to_string(), category.name.clone()))
            .chain([(UNCATEGORIZED_ENTITY.to_owned(), "Uncategorized".to_owned())]);
        names
            .enumerate()
            .filter_map(|(slot, (id, name))| {
                rows.remove(&slot).map(|months| CategoryYearlyNet {
                    id,
                    name,
                    total: Money::saturating_sum(months.iter().flatten().copied()),
                    months,
                })
            })
            .collect()
    }
}
