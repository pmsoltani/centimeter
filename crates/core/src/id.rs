//! Typed identifiers for centimeter's records.
//!
//! This module keeps record IDs stable and self-describing at the boundary:
//! each record type defines a fixed prefix, and the underlying UUID is encoded
//! with a `TypeID` suffix.

use std::fmt;
use std::marker::PhantomData;

use uuid::Uuid;

mod prefix;
mod suffix;

pub use prefix::IdPrefix;
pub use suffix::SuffixError;
use suffix::{decode_base32, encode_base32};

/// Represents a domain record that has a unique identifier.
///
/// Each type chooses a prefix that becomes part of the [`Id`] format.
pub trait Identifiable {
    /// The validated unique prefix used to identify this specific record type
    const PREFIX: IdPrefix;
}

/// A typed identifier for a record type `T`.
///
/// The string representation is `"<prefix>_<typeid-suffix>"`, where the prefix
/// is defined by the record type and the suffix is derived from the underlying
/// UUID. This keeps IDs human-readable while still being globally unique.
pub struct Id<T: Identifiable> {
    id: Uuid,
    _marker: PhantomData<T>,
}

impl<T: Identifiable> Id<T> {
    // NOTE: Minting new IDs is not core's responsibility (i.e., no `new` constructor).

    /// Creates a typed record ID from an existing UUID.
    ///
    /// # Errors
    /// Returns an error if the provided UUID is not a version 7 UUID.
    pub fn from_uuid(id: Uuid) -> Result<Self, IdError> {
        match id.get_version_num() {
            7 => Ok(Self { id, _marker: PhantomData }),
            version => Err(IdError::NotV7 { got: id, version }),
        }
    }

    /// Returns the UUID associated with this record ID.
    #[must_use]
    pub fn as_uuid(&self) -> Uuid {
        self.id
    }
}

impl<T: Identifiable> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = encode_base32(self.id.as_u128());
        let suffix = std::str::from_utf8(&bytes).map_err(|_| fmt::Error)?;
        write!(f, "{}_{}", T::PREFIX.as_str(), suffix)
    }
}

impl<T: Identifiable> std::str::FromStr for Id<T> {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((prefix_str, suffix_str)) = s.rsplit_once('_') else {
            return Err(IdError::BadFormat { got: s.into() });
        };
        if prefix_str != T::PREFIX.as_str() {
            return Err(IdError::BadPrefix { got: s.into(), expected: T::PREFIX.as_str() });
        }
        let suffix = decode_base32(suffix_str)
            .map_err(|e| IdError::BadSuffix { got: s.into(), source: e })?;
        Self::from_uuid(Uuid::from_u128(suffix))
    }
}

impl<T: Identifiable> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id<{}>({})", std::any::type_name::<T>(), self)
    }
}

impl<T: Identifiable> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Identifiable> Copy for Id<T> {}

impl<T: Identifiable> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: Identifiable> Eq for Id<T> {}

impl<T: Identifiable> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// A stable total order over the underlying UUID bytes, so that an `Id` can key
/// a `BTreeMap` or be sorted deterministically.
impl<T: Identifiable> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl<T: Identifiable> std::hash::Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// Errors related to record ids.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum IdError {
    /// The UUID provided is not a version 7 UUID.
    #[error("uuid is not version 7: {got} has version {version}")]
    NotV7 {
        /// The provided UUID.
        got: Uuid,
        /// The provided UUID's version.
        version: usize,
    },

    /// A record id string is not in the correct format.
    #[error("record id string is not in the correct format, got {got}")]
    BadFormat {
        /// The provided ID.
        got: String,
    },

    /// A record id string has an invalid prefix.
    #[error("record id string has an invalid prefix, got {got}, expected {expected}")]
    BadPrefix {
        /// The provided ID.
        got: String,
        /// The expected prefix for the record type.
        expected: &'static str,
    },

    /// A record id string could not be parsed into a valid `Id`.
    #[error("record id string has an invalid suffix, got {got}")]
    BadSuffix {
        /// The provided ID.
        got: String,
        /// The underlying error that occurred while parsing the ID.
        #[source]
        source: SuffixError,
    },
}

#[cfg(test)]
mod tests {
    use uuid::Timestamp;

    use super::*;

    #[derive(Debug, PartialEq)]
    struct TestRecord;
    impl Identifiable for TestRecord {
        const PREFIX: IdPrefix = IdPrefix::new("tst");
    }

    #[test]
    fn test_record_id_display_and_parse() {
        let ts = Timestamp::from_unix(uuid::NoContext, 1_692_345_600, 0);

        let uuid = Uuid::new_v7(ts);
        let record_id = Id::<TestRecord>::from_uuid(uuid).unwrap();
        let record_id_str = record_id.to_string();

        // Check that the string representation is correct
        assert!(record_id_str.starts_with("tst_"));

        // Parse the string back into an Id
        let parsed_record_id: Id<TestRecord> = record_id_str.parse().unwrap();
        assert_eq!(parsed_record_id, record_id);
    }

    #[test]
    fn test_record_id_rejects_bad_prefix_and_format() {
        assert!(matches!(
            "no-underscore".parse::<Id<TestRecord>>(),
            Err(IdError::BadFormat { .. })
        ));
        assert!(matches!(
            "bad_01h4559h7xgk9z5j1m2q3r4s5t".parse::<Id<TestRecord>>(),
            Err(IdError::BadPrefix { .. })
        ));
    }

    #[test]
    fn test_record_id_rejects_bad_suffix() {
        assert!(matches!(
            "tst_not1a1typeid".parse::<Id<TestRecord>>(),
            Err(IdError::BadSuffix { .. })
        ));
    }

    #[test]
    #[allow(clippy::clone_on_copy)]
    fn test_id_equality_and_cloning() {
        let ts = Timestamp::from_unix(uuid::NoContext, 1_692_345_600, 0);
        let uuid = Uuid::new_v7(ts);

        let id1 = Id::<TestRecord>::from_uuid(uuid).unwrap();
        let id2 = id1; // Tests Copy
        let id3 = id1.clone(); // Tests Clone

        assert_eq!(id1, id2);
        assert_eq!(id1, id3);
    }
}
