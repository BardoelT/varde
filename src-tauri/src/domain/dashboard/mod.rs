//! The dashboard: one tile per area with a headline figure and its change
//! against the same moment last year (CSD-004).
//!
//! Every figure comes from the code that gives it to its own page (CSD-004
//! CON-5): the totals of the areas and the net worth. This module only picks
//! the moment and the comparison point and compares.

use rust_decimal::Decimal;
use serde::Serialize;
use time::Date;

use crate::domain::charts::percent_label;
use crate::domain::common::{Period, YearMonth};
use crate::domain::houses::House;
use crate::domain::money::{percent_of, Money};
use crate::domain::series::{SeriesAccount, SeriesData, SeriesRecord};
use crate::domain::ProjectData;

#[cfg(test)]
mod tests;

/// Where a figure went compared with the same moment last year (CSD-004 REQ-11).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// The change rounds to above 0.00%.
    Up,
    /// The change rounds to below 0.00%.
    Down,
    /// The change rounds to 0.00%.
    Stable,
}

/// Whether a change is good or bad for the owner (CSD-004 REQ-12).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Sentiment {
    /// Shown green.
    Good,
    /// Shown red.
    Bad,
    /// Shown grey.
    Neutral,
}

/// Which direction is good for a tile.
#[derive(Clone, Copy)]
enum Polarity {
    UpIsGood,
    DownIsGood,
}

/// The change of a tile against last year (CSD-004 REQ-11).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Change {
    /// Up, down or stable.
    pub direction: Direction,
    /// Good, bad or neutral.
    pub sentiment: Sentiment,
    /// The signed percentage as shown: "+6.09%", "-2.50%", "0.00%".
    pub percent_label: String,
}

/// One tile of the dashboard (CSD-004 section 5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Tile {
    /// Whether the area has any record; without one the tile says "No data yet".
    pub has_data: bool,
    /// The headline figure, none without data.
    pub value: Option<Money>,
    /// For a balance tile, the latest period the figure counts.
    pub as_of: Option<Period>,
    /// The change against last year, none without a usable comparison.
    pub change: Option<Change>,
}

impl Tile {
    fn empty() -> Self {
        Self {
            has_data: false,
            value: None,
            as_of: None,
            change: None,
        }
    }

    fn of(
        value: Option<Money>,
        as_of: Option<Period>,
        comparison: Option<Money>,
        polarity: Polarity,
    ) -> Self {
        Self {
            has_data: true,
            value,
            as_of,
            change: change(value, comparison, polarity),
        }
    }
}

/// The payload of `get_dashboard` (CSD-004 section 5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Dashboard {
    /// Houses plus investments plus bank accounts minus debts.
    pub net_worth: Tile,
    /// The gross income of the year to date.
    pub income: Tile,
    /// The expenses of the year to date.
    pub expenses: Tile,
    /// The total of the latest bank balances.
    pub bank_accounts: Tile,
    /// The total of the latest investment balances.
    pub investments: Tile,
    /// The total value of the held houses.
    pub houses: Tile,
    /// The total of the latest debt balances.
    pub debts: Tile,
}

/// The total of a series at a moment and the latest period it counts.
#[derive(Clone, Copy)]
struct Snapshot {
    total: Money,
    as_of: Period,
}

/// The same day one year before `today`; 29/02 becomes 28/02 (CSD-004 section 3).
fn comparison_date(today: Date) -> Option<Date> {
    let year = today.year().checked_sub(1)?;
    Date::from_calendar_date(year, today.month(), today.day())
        .or_else(|_| Date::from_calendar_date(year, today.month(), 28))
        .ok()
}

/// Whether the period ends on or before the last day of `month`: a month if it
/// is not after it, a year (which ends on 31/12) if it is before its year.
fn ends_by(period: Period, month: YearMonth) -> bool {
    match period {
        Period::Month(own) => own <= month,
        Period::Year(year) => year.value() < month.year(),
    }
}

/// Per account that `counts`, the amount of its latest record whose period
/// ends by `up_to` (any record without a limit), added up. `None` when no
/// account has such a record.
fn snapshot<S: SeriesData>(
    data: &S,
    up_to: Option<YearMonth>,
    counts: impl Fn(&S::Account) -> bool,
) -> Option<Snapshot> {
    let latest: Vec<(Money, Period)> = data
        .accounts_list()
        .iter()
        .filter(|account| counts(account))
        .filter_map(|account| {
            data.records_list()
                .iter()
                .filter(|record| record.account_id() == account.id())
                .filter(|record| up_to.is_none_or(|month| ends_by(record.period(), month)))
                .max_by_key(|record| record.order_key())
                .map(|record| (record.amount(), record.period()))
        })
        .collect();
    let as_of = latest
        .iter()
        .map(|(_, period)| *period)
        .max_by_key(|period| period.sort_key())?;
    Some(Snapshot {
        total: Money::saturating_sum(latest.iter().map(|(amount, _)| *amount)),
        as_of,
    })
}

/// Whether the house was purchased on or before `date` and not sold on or
/// before it (CSD-004 section 3, as CSD-019 defines held).
fn held_on(house: &House, date: Date) -> bool {
    house.purchase_date.date() <= date && house.sale.is_none_or(|sale| sale.date.date() > date)
}

/// The change of `value` against `comparison` in percent of the absolute
/// comparison (CSD-004 REQ-11). None without a comparison or when it is 0.00.
fn change(value: Option<Money>, comparison: Option<Money>, polarity: Polarity) -> Option<Change> {
    let (value, comparison) = (value?, comparison?);
    let percent: Decimal = percent_of(value.saturating_sub(comparison), comparison.abs())?;
    let direction = if percent.is_zero() {
        Direction::Stable
    } else if percent.is_sign_positive() {
        Direction::Up
    } else {
        Direction::Down
    };
    let sentiment = match (direction, polarity) {
        (Direction::Stable, _) => Sentiment::Neutral,
        (Direction::Up, Polarity::UpIsGood) | (Direction::Down, Polarity::DownIsGood) => {
            Sentiment::Good
        }
        _ => Sentiment::Bad,
    };
    Some(Change {
        direction,
        sentiment,
        percent_label: percent_label(Some(percent)),
    })
}

/// A balance tile from its snapshot now and at the comparison point.
fn balance_tile(now: Option<Snapshot>, then: Option<Snapshot>, polarity: Polarity) -> Tile {
    match now {
        Some(now) => Tile::of(
            Some(now.total),
            Some(now.as_of),
            then.map(|then| then.total),
            polarity,
        ),
        None => Tile::empty(),
    }
}

/// The seven tiles of the dashboard for `today` (CSD-004 REQ-8 to REQ-14).
#[must_use]
pub fn dashboard(data: &ProjectData, today: Date) -> Dashboard {
    let month = YearMonth::from_date(today);
    let then = comparison_date(today);
    let then_month = then.map(YearMonth::from_date);

    let bank_now = snapshot(&data.bank_accounts, None, |_| true);
    let bank_then = then_month.and_then(|m| snapshot(&data.bank_accounts, Some(m), |_| true));
    let investments_now = snapshot(&data.investments, None, |_| true);
    let investments_then = then_month.and_then(|m| snapshot(&data.investments, Some(m), |_| true));
    let debts_now = snapshot(&data.debts, None, |_| true);
    let debts_then = then_month.and_then(|m| snapshot(&data.debts, Some(m), |_| true));
    let houses_now = snapshot(&data.houses, None, House::is_held);
    let houses_then = then
        .zip(then_month)
        .and_then(|(date, m)| snapshot(&data.houses, Some(m), |house| held_on(house, date)));

    Dashboard {
        net_worth: net_worth_tile(
            data,
            [bank_now, investments_now, debts_now, houses_now],
            [bank_then, investments_then, debts_then, houses_then],
        ),
        income: flow_tile(
            data.income.has_entries(),
            data.income.gross_in_year_up_to(today.year(), month),
            then_month.map(|m| data.income.gross_in_year_up_to(m.year(), m)),
            Polarity::UpIsGood,
        ),
        expenses: flow_tile(
            data.expenses.total_count() > 0,
            expenses_between(data, today.year(), today),
            then.map(|date| expenses_between(data, date.year(), date)),
            Polarity::DownIsGood,
        ),
        bank_accounts: balance_tile(bank_now, bank_then, Polarity::UpIsGood),
        investments: balance_tile(investments_now, investments_then, Polarity::UpIsGood),
        houses: houses_tile(data, houses_now, houses_then),
        debts: balance_tile(debts_now, debts_then, Polarity::DownIsGood),
    }
}

/// The expenses from 01/01 of `year` up to and including `to`.
fn expenses_between(data: &ProjectData, year: i32, to: Date) -> Money {
    Date::from_calendar_date(year, time::Month::January, 1)
        .map_or(Money::ZERO, |from| data.expenses.total_between(from, to))
}

fn flow_tile(has_data: bool, value: Money, comparison: Option<Money>, polarity: Polarity) -> Tile {
    if !has_data {
        return Tile::empty();
    }
    Tile::of(Some(value), None, comparison, polarity)
}

/// The houses tile: without a valuation there is no data, and when every house
/// is sold the held total is 0.00.
fn houses_tile(data: &ProjectData, now: Option<Snapshot>, then: Option<Snapshot>) -> Tile {
    if data.houses.total_valuation_count() == 0 {
        return Tile::empty();
    }
    Tile::of(
        Some(now.map_or(Money::ZERO, |snapshot| snapshot.total)),
        now.map(|snapshot| snapshot.as_of),
        then.map(|snapshot| snapshot.total),
        Polarity::UpIsGood,
    )
}

/// Houses plus investments plus bank accounts minus debts, now and at the
/// comparison point (CSD-004 REQ-9, REQ-10). Snapshots are ordered bank,
/// investments, debts, houses.
fn net_worth_tile(
    data: &ProjectData,
    now: [Option<Snapshot>; 4],
    then: [Option<Snapshot>; 4],
) -> Tile {
    let has_data = data.bank_accounts.total_entry_count() > 0
        || data.investments.total_statement_count() > 0
        || data.debts.total_statement_count() > 0
        || data.houses.total_valuation_count() > 0;
    if !has_data {
        return Tile::empty();
    }
    let worth = |snapshots: &[Option<Snapshot>; 4]| -> Option<Money> {
        if snapshots.iter().all(Option::is_none) {
            return None;
        }
        let amount = |index: usize| {
            snapshots
                .get(index)
                .copied()
                .flatten()
                .map_or(Money::ZERO, |snapshot| snapshot.total)
        };
        Some(Money::saturating_sum([amount(0), amount(1), amount(3)]).saturating_sub(amount(2)))
    };
    let as_of = now
        .iter()
        .flatten()
        .map(|snapshot| snapshot.as_of)
        .max_by_key(|period| period.sort_key());
    Tile::of(worth(&now), as_of, worth(&then), Polarity::UpIsGood)
}
