//! One typed id for every area: a random UUID tagged with what it identifies.

use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use crate::domain::error::DomainError;

/// What an [`Id`] identifies. The marker gives the error a bad id text maps to.
///
/// Two ids of different kinds are different types, so a `JobId` can never be
/// passed where a `DebtId` is expected.
pub trait IdKind {
    /// The error for text that is not an id of this kind.
    fn not_found() -> DomainError;
}

/// The id of one `T`. It is written as the plain UUID string, so the project
/// file and the IPC see no difference between kinds.
pub struct Id<T> {
    uuid: Uuid,
    kind: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    /// A new random id.
    #[must_use]
    pub fn new() -> Self {
        Self::from_uuid(Uuid::new_v4())
    }

    fn from_uuid(uuid: Uuid) -> Self {
        Self {
            uuid,
            kind: PhantomData,
        }
    }
}

impl<T: IdKind> Id<T> {
    /// Parses the text form returned to the frontend.
    ///
    /// # Errors
    /// Returns the kind's not-found error for text that is not an id.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        Uuid::parse_str(text.trim())
            .map(Self::from_uuid)
            .map_err(|_| T::not_found())
    }
}

// The impls below are written by hand: a derive would require `T` itself to
// be `Clone`, `Eq`, ... although the marker is never stored.

impl<T> Default for Id<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Id<T> {}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl<T> Eq for Id<T> {}

impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.uuid.cmp(&other.uuid)
    }
}

impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.uuid.hash(state);
    }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Id").field(&self.uuid).finish()
    }
}

impl<T> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.uuid, f)
    }
}

impl<T> Serialize for Id<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.uuid.serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for Id<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Uuid::deserialize(deserializer).map(Self::from_uuid)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    struct Apple;
    struct Pear;

    impl IdKind for Apple {
        fn not_found() -> DomainError {
            DomainError::JobNotFound
        }
    }

    impl IdKind for Pear {
        fn not_found() -> DomainError {
            DomainError::DebtNotFound
        }
    }

    #[test]
    fn new_ids_differ_and_round_trip_through_text() {
        let a = Id::<Apple>::new();
        assert_ne!(a, Id::<Apple>::new());
        let text = a.to_string();
        assert_eq!(text.len(), 36);
        assert_eq!(Id::<Apple>::parse(&text), Ok(a));
        assert_eq!(Id::<Apple>::parse(&format!("  {text}\n")), Ok(a));
    }

    #[test]
    fn a_bad_text_gives_the_error_of_its_kind() {
        assert_eq!(Id::<Apple>::parse("nope"), Err(DomainError::JobNotFound));
        assert_eq!(Id::<Pear>::parse(""), Err(DomainError::DebtNotFound));
    }

    #[test]
    fn serde_is_the_plain_uuid_string() {
        let id = Id::<Apple>::parse("67e55044-10b1-426f-9247-bb680e5fe0c8").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"67e55044-10b1-426f-9247-bb680e5fe0c8\"");
        assert_eq!(serde_json::from_str::<Id<Apple>>(&json).unwrap(), id);
        assert!(serde_json::from_str::<Id<Apple>>("\"nope\"").is_err());
    }

    #[test]
    fn eq_hash_and_order_follow_the_uuid() {
        let a = Id::<Apple>::new();
        let copy = a;
        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&copy));
        assert!(set.insert(Id::new()));
        assert_eq!(a.cmp(&copy), std::cmp::Ordering::Equal);
        assert_eq!(a.partial_cmp(&copy), Some(std::cmp::Ordering::Equal));
    }

    // Ids of different kinds are different types: `Id::<Apple>` and
    // `Id::<Pear>` do not unify, so mixing them does not compile. A
    // compile-fail test is not kept for that.
}
