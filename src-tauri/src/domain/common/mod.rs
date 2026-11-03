//! Types and limits every area shares: typed ids, periods, notes and the
//! shared limits. No area owns them.

mod id;
mod limits;
mod note;
mod pay_period;
mod period;
mod series_years;
mod year_month;

pub use id::{Id, IdKind};
pub use note::Note;
pub use pay_period::PayPeriod;
pub use period::Period;
pub use series_years::series_years;
pub use year_month::{Year, YearMonth};

pub(crate) use limits::{COLOUR_SLOTS, MAX_NAME_CHARS};
