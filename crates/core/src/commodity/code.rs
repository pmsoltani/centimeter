//! The commodity code: the natural key a user reads and types.
//!
//! A code is a short label such as `"USD"`, `"AAPL"`, `"BRK.B"`, or `"HOUR"`.
//! Centimeter attaches no meaning to it and checks it against no registry
//! (neither ISO 4217, nor any ticker list). The same code space has to hold
//! currencies, securities, crypto, and invented units side by side, so it is.
//!
//! It does, however, pin down is a shape that stays unambiguous: at most
//! `MAX_LENGTH` bytes drawn from `[A-Za-z0-9._-]`, beginning and ending
//! alphanumeric. Surrounding whitespace is trimmed rather than rejected.
//!
//! Codes are compared exactly. `"USD"` and `"usd"` are therefore two different
//! commodities, and case is preserved as entered and never folded. A ledger
//! wanting one canonical casing enforces it above the core, where it can apply
//! whatever convention its domain actually uses.

use super::CommodityError;

/// A commodity code that has passed validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CommodityCode(String);

impl CommodityCode {
    /// The maximum length of a commodity code.
    pub(super) const MAX_LENGTH: usize = 32;

    fn validate(code: &str) -> Result<String, CommodityError> {
        let code = code.trim();
        if code.is_empty() {
            return Err(CommodityError::CodeEmpty);
        }
        let len = code.len();
        if Self::MAX_LENGTH < len {
            return Err(CommodityError::CodeTooLong { max: Self::MAX_LENGTH, got: len });
        }
        if !code.starts_with(|c: char| c.is_ascii_alphanumeric()) {
            return Err(CommodityError::CodeBadFirstChar { got: code.to_string() });
        }
        if !code.ends_with(|c: char| c.is_ascii_alphanumeric()) {
            return Err(CommodityError::CodeBadLastChar { got: code.to_string() });
        }
        let char_check = |c: char| c.is_ascii_alphanumeric() || ".-_".contains(c);
        if let Some((index, _)) = code.char_indices().find(|(_, c)| !char_check(*c)) {
            return Err(CommodityError::CodeBadChar { got: code.to_string(), index });
        }

        Ok(code.to_string())
    }

    pub(super) fn try_new(code: &str) -> Result<Self, CommodityError> {
        Self::validate(code).map(Self)
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// Parses `code` and returns the stored string, panicking if it was rejected.
    fn accept(code: &str) -> String {
        CommodityCode::try_new(code)
            .unwrap_or_else(|e| panic!("{code:?} should be accepted, got {e}"))
            .as_str()
            .to_string()
    }

    #[test]
    fn test_code_accepts_realistic_codes() {
        // Currencies, tickers, crypto, and non-currency units all share one type.
        for code in ["USD", "JPY", "AAPL", "BRK.B", "BTC", "HOUR", "X", "0", "a-b_c.d"] {
            assert_eq!(accept(code), code);
        }

        // Exactly at the byte limit.
        let max = "a".repeat(CommodityCode::MAX_LENGTH);
        assert_eq!(accept(&max), max);
    }

    #[test]
    fn test_code_is_trimmed_before_validation() {
        // Trimming happens first, so surrounding whitespace is not a bad char.
        assert_eq!(accept("  USD\t\n"), "USD");
    }

    #[test]
    fn test_code_rejects_empty() {
        assert!(matches!(CommodityCode::try_new(""), Err(CommodityError::CodeEmpty)));
        // Whitespace-only trims down to empty rather than tripping the char check.
        assert!(matches!(CommodityCode::try_new("   \t "), Err(CommodityError::CodeEmpty)));
    }

    #[test]
    fn test_code_rejects_too_long() {
        let over = "a".repeat(CommodityCode::MAX_LENGTH + 1);
        assert!(matches!(
            CommodityCode::try_new(&over),
            Err(CommodityError::CodeTooLong { max: CommodityCode::MAX_LENGTH, got: 33 })
        ));
    }

    #[test]
    fn test_code_length_is_measured_in_bytes() {
        // 17 * 2 bytes = 34 > 32, even though it is only 17 chars. The length
        // check runs before the ASCII char check, so this is `CodeTooLong`.
        let multibyte = "é".repeat(17);
        assert_eq!(multibyte.chars().count(), 17);
        assert!(matches!(
            CommodityCode::try_new(&multibyte),
            Err(CommodityError::CodeTooLong { got: 34, .. })
        ));
    }

    #[test]
    fn test_code_rejects_bad_first_char() {
        for code in ["-USD", ".USD", "_USD", "€UR"] {
            assert!(
                matches!(
                    CommodityCode::try_new(code),
                    Err(CommodityError::CodeBadFirstChar { .. })
                ),
                "{code:?} should be rejected for its first character"
            );
        }
    }

    #[test]
    fn test_code_rejects_bad_last_char() {
        for code in ["USD-", "USD.", "USD_"] {
            assert!(
                matches!(CommodityCode::try_new(code), Err(CommodityError::CodeBadLastChar { .. })),
                "{code:?} should be rejected for its last character"
            );
        }
    }

    #[test]
    fn test_code_rejects_bad_interior_char() {
        assert!(matches!(
            CommodityCode::try_new("US D"),
            Err(CommodityError::CodeBadChar { ref got, index: 2 }) if got == "US D"
        ));
        assert!(matches!(
            CommodityCode::try_new("US$D"),
            Err(CommodityError::CodeBadChar { index: 2, .. })
        ));
        // The index is a byte offset, and it points at the *first* offender:
        // `é` is itself rejected at byte 1, before `$` at byte 3 is reached.
        assert!(matches!(
            CommodityCode::try_new("aé$b"),
            Err(CommodityError::CodeBadChar { index: 1, .. })
        ));
    }

    #[test]
    fn test_code_error_escapes_untrusted_input() {
        // A rejected code is echoed back into the message, so it must not be
        // able to smuggle control characters into a log or a terminal.
        let err = CommodityCode::try_new("US\u{1b}[2JD").unwrap_err();
        let message = err.to_string();
        assert!(!message.contains('\u{1b}'), "message leaked a raw escape: {message:?}");
        assert!(message.contains("\\u{1b}"), "message should escape the control char: {message:?}");
    }

    // proptests
    /// Any string built only from the accepted alphabet, with alphanumeric ends.
    fn valid_code() -> impl Strategy<Value = String> {
        prop_oneof!["[A-Za-z0-9]".boxed(), "[A-Za-z0-9][A-Za-z0-9._-]{0,30}[A-Za-z0-9]".boxed(),]
    }

    proptest! {
        /// Every well-formed code is accepted and stored verbatim.
        #[test]
        fn prop_accepts_well_formed_codes(code in valid_code()) {
            let parsed = CommodityCode::try_new(&code);
            prop_assert!(parsed.is_ok(), "{code:?} should be accepted, got {parsed:?}");
            let parsed = parsed.unwrap();
            prop_assert_eq!(parsed.as_str(), &code);
        }

        /// Validation never panics, and acceptance implies the documented shape.
        #[test]
        fn prop_validation_is_total(s: String) {
            if let Ok(code) = CommodityCode::try_new(&s) {
                let code = code.as_str();
                prop_assert_eq!(code, s.trim());
                prop_assert!(!code.is_empty());
                prop_assert!(code.len() <= CommodityCode::MAX_LENGTH);
                prop_assert!(code.chars().all(|c| c.is_ascii_alphanumeric() || ".-_".contains(c)));
                prop_assert!(code.starts_with(|c: char| c.is_ascii_alphanumeric()));
                prop_assert!(code.ends_with(|c: char| c.is_ascii_alphanumeric()));
            }
        }

        /// Validation is idempotent: a stored code re-validates to itself.
        #[test]
        fn prop_validation_is_idempotent(code in valid_code()) {
            let once = CommodityCode::try_new(&code).unwrap();
            let twice = CommodityCode::try_new(once.as_str()).unwrap();
            prop_assert_eq!(once, twice);
        }
    }
}
