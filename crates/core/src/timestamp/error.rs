//! Errors for the timestamp domain.

/// Errors related to timestamps.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TimestampError {
    /// The timestamp is out of range for the supported years.
    #[error("{got} milliseconds since the epoch is out of range (years 1-9999)")]
    OutOfRange {
        /// The provided timestamp.
        got: i64,
    },
}
