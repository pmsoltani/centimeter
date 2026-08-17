//! Text: a display string that is validated at construction.
//!
//! Applies common rules to human-readable names: trim surrounding whitespace,
//! reject the result if it is too short or too long, and reject control chars.
//! These rules can be modified per field by implementing [`TextSpec`] on a unit
//! struct, which is then used as the type parameter.
//!
//! Length is measured in characters rather than bytes, so a name containing 256
//! kanji is treated the same as one containing 256 ASCII letters.
//!
//! The type parameter has no data. Instead, it acts as a type-level marker: a
//! value validated under one set of rules cannot accidentally be used where
//! another set of rules is required. It also makes the fact that "this string
//! has been validated" explicit in the type signature.

use std::{fmt, hash, marker::PhantomData};

/// A validated display string, parameterised by the rules it satisfies.
///
/// `Text<S>` can only be created through [`try_new`](Self::try_new), so having
/// an instance means the string satisfies `S`'s rules: it has been trimmed, its
/// length falls within [`MIN_LENGTH`](TextSpec::MIN_LENGTH) and
/// [`MAX_LENGTH`](TextSpec::MAX_LENGTH), and every character passes
/// [`is_allowed`](TextSpec::is_allowed).
pub(crate) struct Text<S: TextSpec> {
    text: String,
    _marker: PhantomData<S>,
}

/// The validation rules for a particular kind of [`Text`].
///
/// Implement this trait on a unit struct. The struct is never instantiated; it
/// exists only as the type parameter identifying which rules a `Text` follows.
///
/// The defaults are suitable for a typical display name: non-empty, no longer
/// than 256 characters, and no control characters. An implementation that uses
/// these rules only needs to provide its error type and map validation failures
/// to it. Override the defaults when a domain has different requirements.
pub(crate) trait TextSpec {
    /// The maximum length of the text field (counted in chars).
    const MAX_LENGTH: usize = 256;

    /// The minimum length of the text field (counted in chars).
    /// This is set to 1 by default, meaning that empty strings are not allowed.
    const MIN_LENGTH: usize = 1;

    /// The domain error that a failed validation is reported as.
    type Error;

    /// Translates a [`TextProblem`] into the domain's own error.
    ///
    /// `problem` returns the rejected input: an implementation must not let an
    /// untrusted character reach a terminal unescaped.
    fn map_error(problem: TextProblem) -> Self::Error;

    /// Rejects control characters. Override to narrow the alphabet further.
    fn is_allowed(c: char) -> bool {
        !c.is_control()
    }
}

impl<S: TextSpec> Text<S> {
    /// Trims `text` and validates the result.
    ///
    /// Surrounding whitespace is removed before validation. Subsequent checks
    /// will be applied to the normalized text that is ultimately stored.
    ///
    /// # Errors
    /// Whatever `S` maps [`TextProblem`] to: the text is shorter than
    /// [`MIN_LENGTH`](TextSpec::MIN_LENGTH), longer than
    /// [`MAX_LENGTH`](TextSpec::MAX_LENGTH), or has a character `S` rejects.
    pub(crate) fn try_new(text: &str) -> Result<Self, S::Error> {
        let text = text.trim();
        let len = text.chars().count();
        if len < S::MIN_LENGTH {
            return Err(S::map_error(TextProblem::TooShort { min: S::MIN_LENGTH, got: len }));
        }
        if S::MAX_LENGTH < len {
            return Err(S::map_error(TextProblem::TooLong { max: S::MAX_LENGTH, got: len }));
        }
        if let Some((index, character)) = text.char_indices().find(|&(_, c)| !S::is_allowed(c)) {
            return Err(S::map_error(TextProblem::BadChar { character, index }));
        }
        Ok(Self { text: text.to_string(), _marker: PhantomData })
    }

    /// Borrows the validated text.
    pub(crate) fn as_str(&self) -> &str {
        &self.text
    }
}

/// Why an input string was rejected, mapped to a domain error by [`TextSpec`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextProblem {
    /// The text was shorter than [`TextSpec::MIN_LENGTH`].
    TooShort {
        /// The shortest length allowed, in chars.
        min: usize,
        /// The length of the trimmed text, in chars.
        got: usize,
    },

    /// The text was longer than [`TextSpec::MAX_LENGTH`].
    TooLong {
        /// The longest length allowed, in chars.
        max: usize,
        /// The length of the trimmed text, in chars.
        got: usize,
    },

    /// The text had a character that [`TextSpec::is_allowed`] rejected.
    BadChar {
        /// The rejected character. Untrusted: escape it before display.
        character: char,
        /// The byte offset of `character` within the trimmed text.
        index: usize,
    },
}

impl<S: TextSpec> fmt::Debug for Text<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.text, f)
    }
}

impl<S: TextSpec> fmt::Display for Text<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.text, f)
    }
}

impl<S: TextSpec> AsRef<str> for Text<S> {
    fn as_ref(&self) -> &str {
        &self.text
    }
}

impl<S: TextSpec> Clone for Text<S> {
    fn clone(&self) -> Self {
        Self { text: self.text.clone(), _marker: PhantomData }
    }
}

impl<S: TextSpec> PartialEq for Text<S> {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

impl<S: TextSpec> Eq for Text<S> {}

impl<S: TextSpec> hash::Hash for Text<S> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.text.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use proptest::prelude::*;

    use super::*;

    /// Every default, reporting problems verbatim so a test can assert on the
    /// `TextProblem` itself rather than on some domain error's rendering.
    struct Name;
    impl TextSpec for Name {
        type Error = TextProblem;
        fn map_error(problem: TextProblem) -> Self::Error {
            problem
        }
    }

    /// Narrow bounds, to prove the lengths are read from the spec.
    struct Tight;
    impl TextSpec for Tight {
        const MAX_LENGTH: usize = 5;
        const MIN_LENGTH: usize = 3;

        type Error = TextProblem;
        fn map_error(problem: TextProblem) -> Self::Error {
            problem
        }
    }

    /// A narrowed alphabet, to prove `is_allowed` is consulted.
    struct Lower;
    impl TextSpec for Lower {
        type Error = TextProblem;
        fn map_error(problem: TextProblem) -> Self::Error {
            problem
        }
        fn is_allowed(c: char) -> bool {
            c.is_ascii_lowercase()
        }
    }

    /// A distinct error type, to prove `map_error` is what reaches the caller.
    #[derive(Debug, PartialEq, Eq)]
    enum Mapped {
        Short,
        Long,
        Bad(char),
    }

    struct Custom;
    impl TextSpec for Custom {
        type Error = Mapped;
        fn map_error(problem: TextProblem) -> Self::Error {
            match problem {
                TextProblem::TooShort { .. } => Mapped::Short,
                TextProblem::TooLong { .. } => Mapped::Long,
                TextProblem::BadChar { character, .. } => Mapped::Bad(character),
            }
        }
    }

    /// A `Text<Name>` that must be accepted.
    fn name(text: &str) -> Text<Name> {
        Text::try_new(text).unwrap_or_else(|e| panic!("{text:?} should be accepted, got {e:?}"))
    }

    // Construction and trimming

    #[test]
    fn test_accepts_a_plain_name() {
        let text = "US Dollar";
        assert_eq!(name(text).as_str(), text);
    }

    #[test]
    fn test_trims_surrounding_whitespace() {
        for text in [" US Dollar", "US Dollar ", "  US Dollar  ", " US Dollar \n"] {
            assert_eq!(name(text).as_str(), "US Dollar");
        }
    }

    #[test]
    fn test_allows_non_control_unicode() {
        for text in ["Café 🍰 Voucher", "日本円", "Brent Crude (bbl)", "£ sterling"] {
            assert_eq!(name(text).as_str(), text);
        }
    }

    // length

    #[test]
    fn test_length_is_measured_in_chars_not_bytes() {
        let text = "💵".repeat(256);
        assert_eq!(text.chars().count(), 256);
        assert!(256 < text.len());
        assert_eq!(name(&text).as_str(), text);
    }

    #[test]
    fn test_accepts_exactly_the_maximum() {
        let text = "a".repeat(Name::MAX_LENGTH);
        assert_eq!(name(&text).as_str(), text);
        let text = "💵".repeat(Name::MAX_LENGTH);
        assert_eq!(name(&text).as_str(), text);
    }

    #[test]
    fn test_rejects_one_char_over_the_maximum() {
        let text = "💵".repeat(Name::MAX_LENGTH + 1);
        assert!(matches!(Text::<Name>::try_new(&text), Err(TextProblem::TooLong { got: 257, .. })));
    }

    #[test]
    fn test_rejects_empty_and_whitespace_only() {
        for text in ["", " ", "\n", "\t", " \n\t "] {
            assert!(matches!(
                Text::<Name>::try_new(text),
                Err(TextProblem::TooShort { got: 0, min: 1 })
            ));
        }
    }

    #[test]
    fn test_bounds_come_from_the_spec() {
        assert!(matches!(
            Text::<Tight>::try_new("ab"),
            Err(TextProblem::TooShort { got: 2, min: 3 })
        ));
        assert!(matches!(
            Text::<Tight>::try_new("abcdef"),
            Err(TextProblem::TooLong { got: 6, max: 5 })
        ));
    }

    #[test]
    fn test_length_is_measured_after_trimming() {
        assert!(matches!(
            Text::<Tight>::try_new("  ab  "),
            Err(TextProblem::TooShort { got: 2, min: 3 })
        ));
        assert!(matches!(
            Text::<Tight>::try_new("  abcdef  "),
            Err(TextProblem::TooLong { got: 6, max: 5 })
        ));
    }

    // Alphabet

    #[test]
    fn test_rejects_control_chars() {
        for text in ["\x00", "\x1f", "\x7f"] {
            assert!(matches!(
                Text::<Name>::try_new(text),
                Err(TextProblem::BadChar { character: c, .. }) if c == text.chars().next().unwrap()
            ));
        }
    }

    #[test]
    fn test_index_is_a_byte_offset() {
        let text = "é\u{7}";
        assert!(matches!(
            Text::<Name>::try_new(text),
            Err(TextProblem::BadChar { character: '\u{7}', index: 2 })
        ));
    }

    #[test]
    fn test_index_is_relative_to_the_trimmed_text() {
        let text = " a\u{7}b";
        assert!(matches!(
            Text::<Name>::try_new(text),
            Err(TextProblem::BadChar { character: '\u{7}', index: 1 })
        ));
    }

    #[test]
    fn test_alphabet_comes_from_the_spec() {
        assert_eq!(Text::<Lower>::try_new("abc").unwrap().as_str(), "abc");
        assert!(matches!(
            Text::<Lower>::try_new("abcD"),
            Err(TextProblem::BadChar { character: 'D', index: 3 })
        ));
    }

    // Check order

    #[test]
    fn test_length_is_checked_before_the_alphabet() {
        let text = "a".repeat(Name::MAX_LENGTH) + "\u{7}";
        assert!(matches!(
            Text::<Name>::try_new(&text),
            Err(TextProblem::TooLong { got: 257, max: 256 })
        ));
    }

    #[test]
    fn test_shortness_is_checked_before_the_alphabet() {
        let text = " \u{7} ";
        assert!(matches!(
            Text::<Tight>::try_new(text),
            Err(TextProblem::TooShort { got: 1, min: 3 })
        ));
    }

    // Mapping

    #[test]
    fn test_the_spec_maps_the_error() {
        assert_eq!(Text::<Custom>::try_new(""), Err(Mapped::Short));
        assert_eq!(Text::<Custom>::try_new(&"a".repeat(300)), Err(Mapped::Long));
        assert_eq!(Text::<Custom>::try_new("abc\u{7}def"), Err(Mapped::Bad('\u{7}')));
    }

    // Hand-written impls

    #[test]
    fn test_equal_when_the_trimmed_text_is_equal() {
        let a = Text::<Name>::try_new(" a ").unwrap();
        let b = Text::<Name>::try_new("a").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn test_hash_agrees_with_eq() {
        let a = Text::<Name>::try_new(" a ").unwrap();
        let b = Text::<Name>::try_new("a").unwrap();
        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
    }

    #[test]
    fn test_clone_is_independent() {
        let a = Text::<Name>::try_new("a").unwrap();
        let b = a.clone();
        assert_eq!(a, b);
        assert_eq!(a.as_str(), b.as_str());
    }

    #[test]
    fn test_debug_is_quoted() {
        assert_eq!(format!("{:?}", name("US Dollar")), "\"US Dollar\"");
    }

    #[test]
    fn test_display_is_the_bare_text() {
        assert_eq!(format!("{}", name("US Dollar")), "US Dollar");
    }

    // Proptests

    /// Covers the whole `char` range, including control characters, at lengths
    /// that can exceed `MAX_LENGTH`.
    fn any_text() -> impl Strategy<Value = String> {
        prop::collection::vec(any::<char>(), 0..300).prop_map(String::from_iter)
    }

    proptest! {
        /// Validation never panics, and acceptance implies the documented shape.
        #[test]
        fn prop_validation_is_total(raw in any_text()) {
            if let Ok(validated) = Text::<Name>::try_new(&raw) {
                prop_assert_eq!(&validated.as_str(), &raw.trim());
                prop_assert!(!validated.as_str().is_empty());
                prop_assert!(validated.as_str().chars().count() <= Name::MAX_LENGTH);
                prop_assert!(!validated.as_str().chars().any(char::is_control));
            }
        }

        /// Validation is idempotent: feeding an accepted name back in returns
        /// it unchanged. The chart relies on this when it compares a validated
        /// name against names already stored.
        #[test]
        fn prop_validation_is_idempotent(raw in any_text()) {
            if let Ok(once) = Text::<Name>::try_new(&raw) {
                let twice = Text::<Name>::try_new(once.as_str()).ok();
                prop_assert_eq!(twice, Some(once));
            }
        }

        #[test]
        fn prop_rejection_names_a_real_violation(raw in any_text()) {
            if let Err(problem) = Text::<Name>::try_new(&raw) {
                match problem {
                    TextProblem::TooShort { got, min } => {
                        prop_assert!(got < min);
                    }
                    TextProblem::TooLong { got, max } => {
                        prop_assert!(max < got);
                    }
                    TextProblem::BadChar { character, index } => {
                        let trimmed = raw.trim();
                        let c = trimmed[index..].chars().next().expect("index is a char boundary");
                        prop_assert_eq!(c, character);
                        prop_assert!(!Name::is_allowed(character));
                    }
                }
            }
        }
    }
}
