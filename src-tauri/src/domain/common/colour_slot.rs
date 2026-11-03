//! The colour slot of a series in the Insights charts.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

/// One of the eight colours of the Insights charts, 0 to 7. The project file
/// and IPC hold the plain number.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct ColourSlot(u8);

impl ColourSlot {
    /// How many colours the Insights charts have.
    pub const COUNT: u8 = 8;

    /// The slot with this number, `None` from 8 up.
    #[must_use]
    pub fn new(slot: u8) -> Option<Self> {
        (slot < Self::COUNT).then_some(Self(slot))
    }

    /// Every slot, lowest first.
    pub fn all() -> impl Iterator<Item = Self> {
        (0..Self::COUNT).filter_map(Self::new)
    }

    /// The slot as a number, 0 to 7.
    #[allow(
        dead_code,
        reason = "the number is read by the tests; the app only serialises it"
    )]
    #[must_use]
    pub fn get(self) -> u8 {
        self.0
    }
}

impl<'de> Deserialize<'de> for ColourSlot {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let slot = u8::deserialize(deserializer)?;
        Self::new(slot).ok_or_else(|| D::Error::custom("colour slot out of range"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slot_is_valid_from_0_to_7() {
        assert_eq!(ColourSlot::new(0).map(ColourSlot::get), Some(0));
        assert_eq!(ColourSlot::new(7).map(ColourSlot::get), Some(7));
        assert_eq!(ColourSlot::new(8), None);
        assert_eq!(ColourSlot::all().count(), 8);
    }

    #[test]
    fn a_slot_is_a_plain_number_on_the_wire() {
        let slot: ColourSlot = serde_json::from_str("5").expect("valid slot");
        assert_eq!(slot.get(), 5);
        assert_eq!(serde_json::to_string(&slot).expect("serialises"), "5");
    }

    #[test]
    fn a_slot_outside_the_range_is_rejected_on_reading() {
        assert!(serde_json::from_str::<ColourSlot>("8").is_err());
        assert!(serde_json::from_str::<ColourSlot>("256").is_err());
        assert!(serde_json::from_str::<ColourSlot>("-1").is_err());
    }
}
