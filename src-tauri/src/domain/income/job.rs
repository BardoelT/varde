//! Jobs: their id, pay period and the rules for the fields the user types (CSD-001 REQ-1 to REQ-4).

use serde::{Deserialize, Serialize};
use time::macros::format_description;
use time::Date;

use crate::domain::common::{Id, IdKind, PayPeriod};
use crate::domain::error::DomainError;

const MAX_TEXT_CHARS: usize = 200;

time::serde::format_description!(iso_date, Date, "[year]-[month]-[day]");

/// Parses a date as the user types it: `dd/mm/yyyy` with a two-digit day and
/// month (CSD-001 REQ-22). Other formats and impossible dates are rejected.
fn parse_date(text: &str) -> Result<Date, DomainError> {
    Date::parse(text.trim(), format_description!("[day]/[month]/[year]"))
        .map_err(|_| DomainError::JobDateInvalid)
}

/// Identifies a Job.
pub type JobId = Id<Job>;

impl IdKind for Job {
    fn not_found() -> DomainError {
        DomainError::JobNotFound
    }
}

/// One employment relationship (CSD-001 REQ-1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Job {
    /// Identifier of the job.
    pub id: JobId,
    /// Employer name.
    pub employer: String,
    /// Job title.
    pub title: Option<String>,
    /// First day of the job.
    #[serde(with = "iso_date")]
    pub start_date: Date,
    /// Last day of the job, `None` while it continues.
    #[serde(with = "iso_date::option")]
    pub end_date: Option<Date>,
    /// A job without a pay period (an older file) is monthly.
    pub period: PayPeriod,
}

/// Job fields as typed by the user, before validation.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct JobInput {
    /// Employer name.
    #[serde(default)]
    pub employer: String,
    /// Job title.
    #[serde(default)]
    pub title: Option<String>,
    /// First day of the job.
    #[serde(default)]
    pub start_date: Option<String>,
    /// Last day of the job, `None` while it continues.
    #[serde(default)]
    pub end_date: Option<String>,
    /// `None` means Monthly when creating and "unchanged" when editing.
    #[serde(default)]
    pub period: Option<String>,
}

pub(super) struct ValidJob {
    pub(super) employer: String,
    pub(super) title: Option<String>,
    pub(super) start_date: Date,
    pub(super) end_date: Option<Date>,
    pub(super) period: Option<PayPeriod>,
}

impl JobInput {
    pub(super) fn validate(&self) -> Result<ValidJob, DomainError> {
        let employer = self.employer.trim();
        if employer.is_empty() {
            return Err(DomainError::JobEmployerRequired);
        }
        let title = non_blank(self.title.as_deref());
        if employer.chars().count() > MAX_TEXT_CHARS
            || title.is_some_and(|title| title.chars().count() > MAX_TEXT_CHARS)
        {
            return Err(DomainError::JobFieldTooLong);
        }
        let start_date = non_blank(self.start_date.as_deref())
            .ok_or(DomainError::JobStartRequired)
            .and_then(parse_date)?;
        let end_date = non_blank(self.end_date.as_deref())
            .map(parse_date)
            .transpose()?;
        let period = non_blank(self.period.as_deref())
            .map(|text| PayPeriod::parse(text).ok_or(DomainError::JobPeriodUnsupported))
            .transpose()?;
        if end_date.is_some_and(|end| end < start_date) {
            return Err(DomainError::JobEndBeforeStart);
        }
        Ok(ValidJob {
            employer: employer.to_owned(),
            title: title.map(str::to_owned),
            start_date,
            end_date,
            period,
        })
    }
}

fn non_blank(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|text| !text.is_empty())
}
