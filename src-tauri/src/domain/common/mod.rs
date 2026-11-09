//! Types and limits every area shares: typed ids, periods, notes and the
//! shared limits. No area owns them.

mod colour_slot;
mod id;
mod limits;
mod note;
mod pay_period;
mod period;
mod series_years;
mod statement_filter;
mod wire_name;
mod year_month;

pub use colour_slot::ColourSlot;
pub use id::{Id, IdKind};
pub use note::Note;
pub use pay_period::PayPeriod;
pub use period::Period;
pub use series_years::series_years;
pub use statement_filter::{apply as apply_filter, StatementFilter};
pub use year_month::{Year, YearMonth};

pub(crate) use wire_name::{parse_wire_name, wire_name};

pub(crate) use limits::MAX_NAME_CHARS;
