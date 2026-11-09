//! The undo and redo history of the open project (CSD-027).
//!
//! A step is a copy of the project data from before one confirmed change. The
//! history lives in memory only and is never written to disk or logged
//! (CSD-027 CON-4).

use std::collections::VecDeque;
use std::fmt;

use serde::{Serialize, Serializer};

use crate::domain::ProjectData;

/// The most undo steps kept per open project (CSD-027 REQ-6).
pub const MAX_STEPS: usize = 100;

/// What a step did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Add,
    Edit,
    Delete,
}

/// The kind of record a step changed (CSD-027 section 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Record {
    Job,
    Paycheck,
    Statement,
    InvestmentAccount,
    InvestmentStatement,
    BankAccount,
    BankEntry,
    Debt,
    DebtStatement,
    House,
    HouseValuation,
    Expense,
}

/// The English name of a step, such as "Delete Bank Account". It labels the
/// Edit menu items and is the key of the Dutch text in the status message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepLabel {
    action: Action,
    record: Record,
}

impl StepLabel {
    /// The label of `action` on `record`.
    #[must_use]
    pub const fn new(action: Action, record: Record) -> Self {
        Self { action, record }
    }

    /// A step that adds a record.
    #[must_use]
    pub const fn add(record: Record) -> Self {
        Self::new(Action::Add, record)
    }

    /// A step that changes a record.
    #[must_use]
    pub const fn edit(record: Record) -> Self {
        Self::new(Action::Edit, record)
    }

    /// A step that deletes a record.
    #[must_use]
    pub const fn delete(record: Record) -> Self {
        Self::new(Action::Delete, record)
    }
}

impl fmt::Display for StepLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let action = match self.action {
            Action::Add => "Add",
            Action::Edit => "Edit",
            Action::Delete => "Delete",
        };
        let record = match self.record {
            Record::Job => "Job",
            Record::Paycheck => "Paycheck",
            Record::Statement => "Statement",
            Record::InvestmentAccount => "Investment Account",
            Record::InvestmentStatement => "Investment Statement",
            Record::BankAccount => "Bank Account",
            Record::BankEntry => "Bank Entry",
            Record::Debt => "Debt",
            Record::DebtStatement => "Debt Statement",
            Record::House => "House",
            Record::HouseValuation => "House Valuation",
            Record::Expense => "Expense",
        };
        write!(f, "{action} {record}")
    }
}

impl Serialize for StepLabel {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// A step and the data to go back to when it is applied.
#[derive(Debug)]
pub(super) struct Entry {
    pub label: StepLabel,
    pub data: ProjectData,
}

/// The steps that can be undone and the steps that can be redone.
#[derive(Debug, Default)]
pub(super) struct History {
    undo: VecDeque<Entry>,
    redo: Vec<Entry>,
}

impl History {
    /// Remembers `before` as the state a new step started from. A new step ends
    /// the redo steps (REQ-5) and drops the oldest step past the limit (REQ-6).
    pub fn push(&mut self, label: StepLabel, before: ProjectData) {
        self.redo.clear();
        self.undo.push_back(Entry {
            label,
            data: before,
        });
        if self.undo.len() > MAX_STEPS {
            self.undo.pop_front();
        }
    }

    pub fn pop_undo(&mut self) -> Option<Entry> {
        self.undo.pop_back()
    }

    pub fn pop_redo(&mut self) -> Option<Entry> {
        self.redo.pop()
    }

    /// Keeps `entry` as a step that can be redone, after it was undone.
    pub fn keep_for_redo(&mut self, entry: Entry) {
        self.redo.push(entry);
    }

    /// Keeps `entry` as a step that can be undone, after it was redone. Unlike
    /// [`History::push`] this leaves the other redo steps alone.
    pub fn keep_for_undo(&mut self, entry: Entry) {
        self.undo.push_back(entry);
    }

    pub fn undo_label(&self) -> Option<StepLabel> {
        self.undo.back().map(|entry| entry.label)
    }

    pub fn redo_label(&self) -> Option<StepLabel> {
        self.redo.last().map(|entry| entry.label)
    }

    /// Forgets every step (REQ-8).
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}
