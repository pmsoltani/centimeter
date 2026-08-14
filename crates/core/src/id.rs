//! Typed identifiers for centimeter's records.
//!
//! This module keeps record IDs stable and self-describing at the boundary:
//! each record type defines a fixed prefix, and the underlying UUID is encoded
//! with a `TypeID` suffix.

use std::fmt;
use std::marker::PhantomData;

use uuid::Uuid;

mod error;
mod prefix;
mod suffix;

pub use error::IdError;
pub use prefix::IdPrefix;
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
            version => Err(IdError::UuidNotV7 { got: id, version }),
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
            return Err(IdError::PrefixMismatch { got: s.into(), expected: T::PREFIX.as_str() });
        }
        let suffix = decode_base32(suffix_str)?;
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

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
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
            Err(IdError::PrefixMismatch { .. })
        ));
    }

    #[test]
    fn test_record_id_rejects_bad_suffix() {
        assert!(matches!(
            "tst_not1a1typeid".parse::<Id<TestRecord>>(),
            Err(IdError::SuffixBadLength { ref got, len: 12 }) if got == "not1a1typeid"
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

    proptest! {
        /// Every accepted uuid survives the trip out to the wire format and
        /// back, which is the promise the whole TypeID scheme rests on. The
        /// width is fixed too: ids never render ragged.
        #[test]
        fn prop_id_round_trips_through_its_string_form(bytes: [u8; 16]) {
            // Arbitrary bytes are not a v7 uuid, and `from_uuid` refuses those
            // by design, so stamp the two fields that identify the version.
            let mut bytes = bytes;
            bytes[6] = (bytes[6] & 0x0F) | 0x70; // version 7
            bytes[8] = (bytes[8] & 0x3F) | 0x80; // RFC 4122 variant
            let uuid = Uuid::from_bytes(bytes);

            let id = Id::<TestRecord>::from_uuid(uuid).unwrap();
            let rendered = id.to_string();

            prop_assert!(rendered.starts_with("tst_"), "got {rendered}");
            prop_assert_eq!(rendered.len(), "tst_".len() + 26);

            let parsed: Id<TestRecord> = rendered.parse().unwrap();
            prop_assert_eq!(parsed, id);
            prop_assert_eq!(parsed.as_uuid(), uuid);
        }
    }
}
