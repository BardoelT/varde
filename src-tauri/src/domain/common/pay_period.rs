//! How often a series is recorded: monthly or yearly.

use serde::{Deserialize, Serialize};

/// How a job is recorded (CSD-001 REQ-1): one paycheck per month, or one
/// annual statement per calendar year. Statement periods of the other areas
/// use the same two kinds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PayPeriod {
    #[default]
    Monthly,
    Yearly,
}

impl PayPeriod {
    /// Parses `"Monthly"` or `"Yearly"` (case-insensitive).
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "monthly" => Some(Self::Monthly),
            "yearly" => Some(Self::Yearly),
            _ => None,
        }
    }
}
