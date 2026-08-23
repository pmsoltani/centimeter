//! The single integration-test binary for `centimeter-core`.
//!
//! Everything here exercises the crate exactly as a consumer sees it, through
//! the public API only. Private invariants are tested inline beside the code.

// `allow-expect-in-tests` only reaches functions carrying `#[test]`, so shared
// helpers in an integration binary need the exemption spelled out.
#![allow(clippy::expect_used)]

mod account;
mod commodity;
mod date;
mod fixtures;
mod posting;
mod quantity;
mod rate;
mod timestamp;
