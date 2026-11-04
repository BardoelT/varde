//! The optional plain-text note of a statement, entry or valuation.

use serde::{Deserialize, Serialize};

use super::limits::MAX_NOTE_CHARS;
use crate::domain::error::DomainError;

/// The optional plain-text note of a statement, at most 2,000 characters
/// (CSD-005 REQ-10). It is kept exactly as typed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Note(String);

impl Note {
    /// Builds a note.
    ///
    /// # Errors
    /// Returns `InvestmentNoteTooLong` above 2,000 characters (CON-8); the
    /// areas map it to their own error code.
    pub fn new(text: &str) -> Result<Self, DomainError> {
        if text.chars().count() > MAX_NOTE_CHARS {
            return Err(DomainError::InvestmentNoteTooLong);
        }
        Ok(Self(text.to_owned()))
    }

    /// The note text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
