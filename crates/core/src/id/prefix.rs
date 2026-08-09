/// A record ID prefix, which is a static string that must follow `TypeID` rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdPrefix(&'static str);

impl IdPrefix {
    /// Creates a new prefix, enforcing `TypeID` rules at compile time.
    ///
    /// # Panics
    /// Panics if the prefix is empty, exceeds 63 characters, starts or ends
    /// with `_`, or contains characters other than `a`..=`z` and `_`.
    #[must_use]
    pub const fn new(prefix: &'static str) -> Self {
        let bytes = prefix.as_bytes();
        let len = bytes.len();

        assert!(0 < len, "prefix cannot be empty");
        assert!(len <= 63, "prefix cannot exceed 63 characters");
        assert!(bytes[0] != b'_' && bytes[len - 1] != b'_', "prefix cannot start or end with '_'");

        let mut i = 0;
        while i < len {
            assert!(
                matches!(bytes[i], b'a'..=b'z' | b'_'),
                "prefix must only contain 'a-z' or '_'"
            );
            i += 1;
        }

        Self(prefix)
    }

    /// Expose the underlying static string slice
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_prefix_validation() {
        // Valid prefixes should compile fine
        let _p1 = IdPrefix::new("a");
        let _p2 = IdPrefix::new("abc_xyz");
    }

    #[test]
    #[should_panic(expected = "prefix cannot be empty")]
    fn test_prefix_empty() {
        let _ = IdPrefix::new("");
    }

    #[test]
    #[should_panic(expected = "prefix cannot exceed 63 characters")]
    fn test_prefix_too_long() {
        // 64 characters long string
        let long = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let _ = IdPrefix::new(long);
    }

    #[test]
    #[should_panic(expected = "prefix cannot start or end with '_'")]
    fn test_prefix_starts_with_underscore() {
        let _ = IdPrefix::new("_abc");
    }

    #[test]
    #[should_panic(expected = "prefix cannot start or end with '_'")]
    fn test_prefix_ends_with_underscore() {
        let _ = IdPrefix::new("abc_");
    }

    #[test]
    #[should_panic(expected = "prefix must only contain 'a-z' or '_'")]
    fn test_prefix_invalid_character_uppercase() {
        // Uppercase letters are not allowed
        let _ = IdPrefix::new("Usr");
    }

    #[test]
    #[should_panic(expected = "prefix must only contain 'a-z' or '_'")]
    fn test_prefix_invalid_character_symbol() {
        let _ = IdPrefix::new("user-id");
    }
}
