//! The rules of a series written once: finding, adding, moving and deleting
//! records, the name and period rules of the accounts, the checks of a file,
//! and the yearly closing figures the totals and the insights are built on.
//!
//! An area (`BankAccounts`, `Investments`, `Debts`, `Houses`) implements
//! [`SeriesData`] for its data struct, [`SeriesAccount`] for its account and
//! [`SeriesRecord`] for its record. The data struct keeps its own fields (they
//! are the project file), the traits only say where the two lists are and what
//! differs: the errors, and the few hooks a house (holding window) or a debt
//! (settlement) adds. The public methods of an area stay as they were and call
//! the methods of [`SeriesData`].

use std::cmp::Reverse;
use std::collections::{BTreeMap, HashSet};
use std::fmt::Display;
use std::hash::Hash;

use rust_decimal::Decimal;

use crate::domain::charts::{stack_from_zero, Marker, Piece};
use crate::domain::common::{series_years, PayPeriod, Period, YearMonth, MAX_NAME_CHARS};
use crate::domain::error::DomainError;
use crate::domain::money::zero;

use super::{name_order, name_taken};

/// The errors an area gives for the rules every series has. The area maps each
/// kind to its own [`DomainError`], so codes and messages stay per area.
#[derive(Clone, Copy, Debug)]
pub(crate) enum SeriesError {
    AccountNotFound,
    NameRequired,
    NameTooLong,
    NameDuplicate,
    PeriodUnsupported,
    /// The period of an account with records cannot change.
    PeriodLocked,
    /// The period of a record is not of the kind of its account.
    PeriodMismatch,
    RecordNotFound,
    RecordDuplicate,
    /// Data read from a file breaks a rule.
    DataInvalid,
}

/// What the rules need to know of an account (a bank account, an investment
/// account, a debt or a house).
pub(crate) trait SeriesAccount {
    type Id: Copy + Eq + Hash + Display;

    fn id(&self) -> Self::Id;
    fn name(&self) -> &str;
    /// Whether its records are monthly or yearly.
    fn period(&self) -> PayPeriod;
}

/// What the rules need to know of a record (an entry, a statement or a
/// valuation).
pub(crate) trait SeriesRecord: Clone {
    type AccountId: Copy + Eq + Hash;
    /// What orders the records of one account: the sort key of the period, and
    /// for a debt also whether the record is the settlement statement.
    type Order: Ord + Copy;

    fn account_id(&self) -> Self::AccountId;
    fn period(&self) -> Period;
    fn set_period(&mut self, period: Period);
    fn order_key(&self) -> Self::Order;
    /// The balance or value, the figure that is summed.
    fn amount(&self) -> Decimal;
    /// Whether the user entered the record. A record that is not regular (the
    /// settlement statement of a debt) is not found by period, not edited and
    /// not deleted by the rules here, and may share its period with a regular
    /// one.
    fn is_regular(&self) -> bool {
        true
    }
    /// Whether the amounts and the note follow their rules, for data read from
    /// a file.
    fn is_valid(&self) -> bool;
}

/// The id of the accounts of a series.
pub(crate) type AccountId<S> = <<S as SeriesData>::Account as SeriesAccount>::Id;

/// The figures of one year of a series, from which an area builds its
/// insights year.
pub(crate) struct YearFigures<'a, A> {
    pub(crate) year: i32,
    /// The accounts with a record in the year, in creation order, with the
    /// amount of their latest record that year.
    pub(crate) segments: Vec<(&'a A, Decimal)>,
    /// The sum of the segments.
    pub(crate) closing: Decimal,
    /// How many records count in the year.
    pub(crate) count: usize,
    /// Whether the year is the current one.
    pub(crate) is_partial: bool,
    /// The segments stacked from zero for the chart.
    pub(crate) pieces: Vec<Piece>,
    /// The closing figure, for a year with records.
    pub(crate) marker: Option<Marker>,
}

/// The data of one series and the rules on it. Everything but the accessors,
/// the errors and the hooks has a default that all areas share.
pub(crate) trait SeriesData {
    type Account: SeriesAccount + Clone;
    type Record: SeriesRecord<AccountId = AccountId<Self>>;

    /// Starts the chart key of a segment: `bank` gives `bank-<account id>`.
    const PIECE_PREFIX: &'static str;

    /// The error of the area for a kind of error.
    fn error(kind: SeriesError) -> DomainError;
    fn accounts_list(&self) -> &[Self::Account];
    fn accounts_list_mut(&mut self) -> &mut Vec<Self::Account>;
    fn records_list(&self) -> &[Self::Record];
    fn records_list_mut(&mut self) -> &mut Vec<Self::Record>;

    // Hooks: what only some areas add. The defaults do nothing.

    /// Whether an account read from a file follows the area's own rules.
    fn account_is_consistent(&self, _account: &Self::Account) -> bool {
        true
    }

    /// Whether `period` may hold a record of the account (a house: inside its
    /// holding window), for data read from a file.
    fn period_fits(&self, _account: &Self::Account, _period: Period) -> bool {
        true
    }

    /// Extra rule for a new period of a record, checked after the kind and
    /// the duplicate (a house: the holding window; a debt: not after its
    /// settlement).
    fn check_period_allowed(
        &self,
        _id: AccountId<Self>,
        _period: Period,
    ) -> Result<(), DomainError> {
        Ok(())
    }

    /// Whether the record of `period` may be changed or deleted at all (a
    /// debt: not once settled).
    fn check_record_editable(
        &self,
        _id: AccountId<Self>,
        _period: Period,
    ) -> Result<(), DomainError> {
        Ok(())
    }

    /// Whether the account counts in a year (a house: held at its end).
    fn counts_in_year(&self, _account: &Self::Account, _year: i32) -> bool {
        true
    }

    // Accounts

    /// An account by id.
    fn account(&self, id: AccountId<Self>) -> Result<&Self::Account, DomainError> {
        self.accounts_list()
            .iter()
            .find(|account| account.id() == id)
            .ok_or_else(|| Self::error(SeriesError::AccountNotFound))
    }

    /// An account by id, for changing.
    fn account_mut(&mut self, id: AccountId<Self>) -> Result<&mut Self::Account, DomainError> {
        self.accounts_list_mut()
            .iter_mut()
            .find(|account| account.id() == id)
            .ok_or_else(|| Self::error(SeriesError::AccountNotFound))
    }

    /// The accounts alphabetically by name, ignoring case.
    fn sorted_accounts(&self) -> Vec<&Self::Account> {
        let mut accounts: Vec<&Self::Account> = self.accounts_list().iter().collect();
        accounts.sort_by_cached_key(|account| name_order(account.name(), &account.id()));
        accounts
    }

    /// Checks that no other account has the name, ignoring case.
    fn check_name_free(
        &self,
        name: &str,
        except: Option<AccountId<Self>>,
    ) -> Result<(), DomainError> {
        let taken = name_taken(
            self.accounts_list()
                .iter()
                .map(|account| (account.id(), account.name())),
            name,
            except,
        );
        if taken {
            return Err(Self::error(SeriesError::NameDuplicate));
        }
        Ok(())
    }

    /// Checks that the period of an account may change from `current` to
    /// `new`: not once the account has a record.
    fn check_period_unlocked(
        &self,
        id: AccountId<Self>,
        current: PayPeriod,
        new: PayPeriod,
    ) -> Result<(), DomainError> {
        if new != current && self.record_count(id) > 0 {
            return Err(Self::error(SeriesError::PeriodLocked));
        }
        Ok(())
    }

    /// Removes an account and all its records.
    fn remove_account(&mut self, id: AccountId<Self>) -> Result<(), DomainError> {
        self.account(id)?;
        self.accounts_list_mut()
            .retain(|account| account.id() != id);
        self.records_list_mut()
            .retain(|record| record.account_id() != id);
        Ok(())
    }

    // Records

    /// The regular record of a period of an account.
    fn record(&self, id: AccountId<Self>, period: Period) -> Option<&Self::Record> {
        self.records_list().iter().find(|record| {
            record.account_id() == id && record.period() == period && record.is_regular()
        })
    }

    /// The record with the most recent period of an account.
    fn latest(&self, id: AccountId<Self>) -> Option<&Self::Record> {
        self.records_list()
            .iter()
            .filter(|record| record.account_id() == id)
            .max_by_key(|record| record.order_key())
    }

    /// The amount of the latest record of an account, for "copy previous".
    fn latest_amount(&self, id: AccountId<Self>) -> Result<Option<Decimal>, DomainError> {
        self.account(id)?;
        Ok(self.latest(id).map(SeriesRecord::amount))
    }

    /// The latest record of the same account before `record`.
    fn previous_record(&self, record: &Self::Record) -> Option<&Self::Record> {
        self.records_list()
            .iter()
            .filter(|other| {
                other.account_id() == record.account_id() && other.order_key() < record.order_key()
            })
            .max_by_key(|other| other.order_key())
    }

    /// How many records an account has.
    fn record_count(&self, id: AccountId<Self>) -> usize {
        self.records_list()
            .iter()
            .filter(|record| record.account_id() == id)
            .count()
    }

    /// The rules a period must follow for an account to hold a record for it:
    /// the kind of the account, not taken, and what the area adds.
    fn check_new_period(&self, id: AccountId<Self>, period: Period) -> Result<(), DomainError> {
        if self.account(id)?.period() != period.kind() {
            return Err(Self::error(SeriesError::PeriodMismatch));
        }
        if self.record(id, period).is_some() {
            return Err(Self::error(SeriesError::RecordDuplicate));
        }
        self.check_period_allowed(id, period)
    }

    /// Adds a record after checking its period.
    fn add_record(&mut self, record: Self::Record) -> Result<Self::Record, DomainError> {
        self.check_new_period(record.account_id(), record.period())?;
        self.records_list_mut().push(record.clone());
        Ok(record)
    }

    /// Changes a record with `change` and, when `new_period` is a different
    /// period, moves it there. The new period follows the rules of adding a
    /// record. The edit is applied as a whole or not at all.
    fn update_record(
        &mut self,
        id: AccountId<Self>,
        period: Period,
        new_period: Period,
        change: impl FnOnce(&mut Self::Record),
    ) -> Result<Self::Record, DomainError> {
        self.account(id)?;
        self.check_record_editable(id, period)?;
        if self.record(id, period).is_none() {
            return Err(Self::error(SeriesError::RecordNotFound));
        }
        if new_period != period {
            self.check_new_period(id, new_period)?;
        }
        let record = self
            .records_list_mut()
            .iter_mut()
            .find(|record| {
                record.account_id() == id && record.period() == period && record.is_regular()
            })
            .ok_or_else(|| Self::error(SeriesError::RecordNotFound))?;
        record.set_period(new_period);
        change(record);
        Ok(record.clone())
    }

    /// Removes one record and nothing else.
    fn delete_record(&mut self, id: AccountId<Self>, period: Period) -> Result<(), DomainError> {
        self.account(id)?;
        self.check_record_editable(id, period)?;
        let records = self.records_list_mut();
        let before = records.len();
        records.retain(|record| {
            !(record.account_id() == id && record.period() == period && record.is_regular())
        });
        if records.len() == before {
            return Err(Self::error(SeriesError::RecordNotFound));
        }
        Ok(())
    }

    /// The records of all accounts with their account, newest period first;
    /// within a period the order is that of the accounts.
    fn all_records(&self) -> Vec<(&Self::Account, &Self::Record)> {
        let mut rows: Vec<(&Self::Account, &Self::Record)> = self
            .records_list()
            .iter()
            .filter_map(|record| {
                self.accounts_list()
                    .iter()
                    .find(|account| account.id() == record.account_id())
                    .map(|account| (account, record))
            })
            .collect();
        rows.sort_by_cached_key(|(account, record)| {
            (
                Reverse(record.order_key()),
                name_order(account.name(), &account.id()),
            )
        });
        rows
    }

    /// Checks that data read from a file follows the rules, so a hand-edited
    /// or damaged file cannot break them: names, unique ids, records of a
    /// known account with a period of its kind, no two records for one period.
    fn validate(&self) -> Result<(), DomainError> {
        let invalid = || Self::error(SeriesError::DataInvalid);
        let mut ids = HashSet::new();
        let mut names = HashSet::new();
        for account in self.accounts_list() {
            let name = account.name().trim();
            if name.is_empty()
                || name.chars().count() > MAX_NAME_CHARS
                || !ids.insert(account.id())
                || !names.insert(name.to_lowercase())
                || !self.account_is_consistent(account)
            {
                return Err(invalid());
            }
        }
        let mut seen = HashSet::new();
        for record in self.records_list() {
            let fits = self.account(record.account_id()).is_ok_and(|account| {
                account.period() == record.period().kind()
                    && self.period_fits(account, record.period())
            });
            if !fits
                || !seen.insert((record.account_id(), record.period(), record.is_regular()))
                || !record.is_valid()
            {
                return Err(invalid());
            }
        }
        Ok(())
    }

    // Yearly figures

    /// All records of a calendar year, whatever their account.
    fn records_in_year(&self, year: i32) -> Vec<&Self::Record> {
        self.records_list()
            .iter()
            .filter(|record| record.period().year() == year)
            .collect()
    }

    /// How many records count in each calendar year: a record counts in the
    /// year of its own period, when its account counts in that year.
    fn records_per_year(&self) -> BTreeMap<i32, usize> {
        let mut counts: BTreeMap<i32, usize> = BTreeMap::new();
        for record in self.records_list() {
            let year = record.period().year();
            let counted = self
                .account(record.account_id())
                .is_ok_and(|account| self.counts_in_year(account, year));
            if counted {
                *counts.entry(year).or_default() += 1;
            }
        }
        counts
    }

    /// For each account that counts in `year` and has a record in it, in
    /// creation order, the amount of its latest record in that year.
    fn year_segments(&self, year: i32) -> Vec<(&Self::Account, Decimal)> {
        self.accounts_list()
            .iter()
            .filter(|account| self.counts_in_year(account, year))
            .filter_map(|account| {
                self.records_list()
                    .iter()
                    .filter(|record| {
                        record.account_id() == account.id() && record.period().year() == year
                    })
                    .max_by_key(|record| record.order_key())
                    .map(|record| (account, record.amount()))
            })
            .collect()
    }

    /// The latest record of an account in a year before `year`.
    fn latest_before_year(&self, id: AccountId<Self>, year: i32) -> Option<&Self::Record> {
        self.records_list()
            .iter()
            .filter(|record| record.account_id() == id && record.period().year() < year)
            .max_by_key(|record| record.order_key())
    }

    /// The change in `year`: for each account with a record in the year and an
    /// earlier one, the amount of its latest record in the year minus that of
    /// its latest earlier record. `None` when no account has both.
    fn year_change(&self, year: i32) -> Option<Decimal> {
        let mut change: Option<Decimal> = None;
        for (account, closing) in self.year_segments(year) {
            let opening = self
                .latest_before_year(account.id(), year)
                .map(SeriesRecord::amount);
            if let Some(opening) = opening {
                change = Some(change.unwrap_or_else(zero) + closing - opening);
            }
        }
        change
    }

    /// For each calendar year with records, newest first: the year, how many
    /// records count and the sum of the amounts of the segments.
    fn yearly_closings(&self) -> Vec<(i32, usize, Decimal)> {
        self.records_per_year()
            .into_iter()
            .rev()
            .map(|(year, count)| {
                let closing = self
                    .year_segments(year)
                    .into_iter()
                    .fold(zero(), |sum, (_, amount)| sum + amount);
                (year, count, closing)
            })
            .collect()
    }

    /// One set of figures per year of the series range (see `series_years`),
    /// oldest first. Years without records are zero and have no segments.
    fn year_figures(&self, today: YearMonth) -> Vec<YearFigures<'_, Self::Account>> {
        let counts = self.records_per_year();
        series_years(counts.keys().copied(), today.year())
            .into_iter()
            .map(|year| {
                let segments = self.year_segments(year);
                let closing = segments
                    .iter()
                    .fold(zero(), |sum, (_, amount)| sum + *amount);
                let count = counts.get(&year).copied().unwrap_or(0);
                let pieces = stack_from_zero(segments.iter().map(|(account, amount)| {
                    (format!("{}-{}", Self::PIECE_PREFIX, account.id()), *amount)
                }));
                YearFigures {
                    year,
                    segments,
                    closing,
                    count,
                    is_partial: year == today.year(),
                    pieces,
                    marker: (count > 0).then(|| Marker::at(closing)),
                }
            })
            .collect()
    }

    /// The accounts that have a segment in any of the years, in creation
    /// order: the legend of the chart.
    fn legend<'a>(&'a self, years: &[YearFigures<'_, Self::Account>]) -> Vec<&'a Self::Account> {
        self.accounts_list()
            .iter()
            .filter(|account| {
                years.iter().any(|year| {
                    year.segments
                        .iter()
                        .any(|(segment, _)| segment.id() == account.id())
                })
            })
            .collect()
    }
}
