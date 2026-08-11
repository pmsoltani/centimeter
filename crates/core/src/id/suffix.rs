//! Crockford's Base32 encoding and decoding for record IDs.

use super::IdError;

const ALPHABET: [u8; 32] = *b"0123456789abcdefghjkmnpqrstvwxyz";

const INVALID: u8 = 0xFF;
const LOOKUP: [u8; 256] = {
    let mut lookup = [INVALID; 256];
    let mut i = 0u8;
    while i < 32 {
        lookup[ALPHABET[i as usize] as usize] = i;
        i += 1;
    }
    lookup
};

/// Encodes a `u128` value into a 26-character Crockford's Base32 string.
pub(super) fn encode_base32(value: u128) -> [u8; 26] {
    let mut n = value;
    let mut result = [0u8; 26];
    for byte in result.iter_mut().rev() {
        *byte = ALPHABET[(n & 0x1F) as usize];
        n >>= 5;
    }
    result
}

/// Decodes a 26-character Crockford's Base32 string into a `u128` value.
///
/// # Errors
/// Returns an error if the provided string is not a valid 26-character
/// Crockford's Base32 string or if the first character is not in '0..=7'.
pub(super) fn decode_base32(s: &str) -> Result<u128, IdError> {
    let encoded = s.as_bytes();
    if encoded.len() != 26 {
        // NOTE: Do not use `s.len()` or `s.chars().count()` here, as it may be
        // different from `bytes.len()` if the string contains non-ASCII chars.
        return Err(IdError::SuffixBadLength { got: s.into(), len: encoded.len() });
    }
    let first = LOOKUP[encoded[0] as usize];
    match first {
        INVALID => return Err(IdError::SuffixBadChar { got: s.into(), index: 0 }),
        8..=31 => return Err(IdError::SuffixOverflow { got: encoded[0] }),
        _ => {}
    }

    let mut result = u128::from(first);
    for (index, &byte) in encoded[1..].iter().enumerate() {
        let value = LOOKUP[byte as usize];
        let index = index + 1; // First character already processed.
        if value == INVALID {
            return Err(IdError::SuffixBadChar { got: s.into(), index });
        }
        result = (result << 5) | u128::from(value);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::str;

    use proptest::prelude::*;
    use uuid::Uuid;

    use super::*;

    const TEST_CASES: &[(&str, &str, &str)] = &[
        // (name, uuid, expected_suffix)
        ("nil", "00000000-0000-0000-0000-000000000000", "00000000000000000000000000"),
        ("one", "00000000-0000-0000-0000-000000000001", "00000000000000000000000001"),
        ("ten", "00000000-0000-0000-0000-00000000000a", "0000000000000000000000000a"),
        ("sixteen", "00000000-0000-0000-0000-000000000010", "0000000000000000000000000g"),
        ("thirty-two", "00000000-0000-0000-0000-000000000020", "00000000000000000000000010"),
        ("max", "ffffffff-ffff-ffff-ffff-ffffffffffff", "7zzzzzzzzzzzzzzzzzzzzzzzzz"),
        ("alphabet", "01890a5d-ac96-774b-bcce-b302099a8057", "01h455vb4pex5vsknk084sn02q"),
    ];

    #[test]
    fn test_alphabet_has_32_distinct_characters() {
        assert_eq!(ALPHABET.len(), 32);
        let mut seen = HashSet::new();
        for c in ALPHABET {
            assert!(seen.insert(c), "Duplicate character found in alphabet: {}", c as char);
        }
    }

    #[test]
    fn test_encode() {
        for &(name, uuid, expected) in TEST_CASES {
            let u = Uuid::parse_str(uuid).expect("Invalid UUID string");
            let encoded = encode_base32(u.as_u128());
            let encoded_str = str::from_utf8(&encoded).expect("Invalid UTF-8 in encoded string");
            assert_eq!(
                encoded_str, expected,
                "Test {name} failed: expected {expected}, got {encoded_str}"
            );
        }
    }

    #[test]
    fn test_decode() {
        for &(name, uuid, encoded) in TEST_CASES {
            let u = Uuid::parse_str(uuid).expect("Invalid UUID string");
            let decoded = decode_base32(encoded).expect("Expected valid base32 suffix");
            assert_eq!(decoded, u.as_u128(), "Test {name} failed: expected {uuid}, got {encoded}");
        }

        assert!(matches!(
            decode_base32(""),
            Err(IdError::SuffixBadLength { ref got, len: 0 }) if got.is_empty(),
        ));
        let mut invalid_suffix = "01h455vb4pex5vsknk084sn02";
        assert!(matches!(
            decode_base32(invalid_suffix),
            Err(IdError::SuffixBadLength { ref got, len: 25 }) if got == invalid_suffix
        ));

        invalid_suffix = "01h455vb4pex5vsknk084sn02qq";
        assert!(matches!(
            decode_base32(invalid_suffix),
            Err(IdError::SuffixBadLength { ref got, len: 27 }) if got == invalid_suffix
        ));

        invalid_suffix = "01h455vb4pex5vsknk084sn02i";
        assert!(matches!(
            decode_base32(invalid_suffix),
            Err(IdError::SuffixBadChar { ref got, index: 25 }) if got == invalid_suffix
        ));

        invalid_suffix = "01h455vb4pex5vsknk084sn02-";
        assert!(matches!(
            decode_base32(invalid_suffix),
            Err(IdError::SuffixBadChar { ref got, index: 25 }) if got == invalid_suffix
        ));

        invalid_suffix = "01H455VB4PEX5VSKNK084SN02Q";
        assert!(matches!(
            decode_base32(invalid_suffix),
            Err(IdError::SuffixBadChar { ref got, index: 2 }) if got == invalid_suffix
        ));

        // 25 characters but 26 bytes: `é` is U+00E9, which UTF-8 encodes as
        // two bytes. This is the case the byte-length note above exists for.
        // It must reach the character check rather than the length check.
        invalid_suffix = "01h455vb4pex5vsknk084sn0é";
        assert!(matches!(
            decode_base32(invalid_suffix),
            Err(IdError::SuffixBadChar { ref got, index: 24 }) if got == invalid_suffix
        ));

        invalid_suffix = "0000000000000000000000000?";
        assert!(matches!(
            decode_base32(invalid_suffix),
            Err(IdError::SuffixBadChar { ref got, index: 25 }) if got == invalid_suffix
        ));
        assert!(matches!(
            decode_base32("80000000000000000000000000"),
            Err(IdError::SuffixOverflow { got: b'8' })
        ));
        assert!(matches!(
            decode_base32("z0000000000000000000000000"),
            Err(IdError::SuffixOverflow { got: b'z' })
        ));
    }

    // proptests
    fn suffix_shaped() -> impl Strategy<Value = String> {
        prop_oneof![
            "[0-9abcdefghjkmnpqrstvwxyz]{0,40}".boxed(),
            "[0-9a-zA-Z_.-]{26}".boxed(),
            any::<String>().boxed(),
        ]
    }

    proptest! {
        /// Every `u128` survives a round trip through the codec.
        #[test]
        fn prop_round_trip(n: u128) {
            let encoded = encode_base32(n);
            let as_str = str::from_utf8(&encoded).expect("encode must emit ASCII");
            let decoded = decode_base32(as_str).expect("a freshly encoded suffix must decode");
            prop_assert_eq!(decoded, n);
        }

        /// Every encoding has the shape the `TypeID` spec requires.
        #[test]
        fn prop_encoding_shape(n: u128) {
            let encoded = encode_base32(n);
            for (at, byte) in encoded.iter().enumerate() {
                prop_assert!(
                    ALPHABET.contains(byte),
                    "byte {at} is {byte:#04x}, which is outside the alphabet"
                );
            }
            // 26 digits hold 130 bits while a UUID only has 128, so the leading
            // digit holds only 3 significant bits and can never exceed 7.
            prop_assert!(
                (b'0'..=b'7').contains(&encoded[0]),
                "leading byte is {:?}, which is above 7",
                encoded[0] as char
            );
        }

        /// A leading character above `7` is always rejected.
        #[test]
        fn prop_rejects_high_leading_character(n: u128, c in "[89abcdefghjkmnpqrstvwxyz]") {
            let mut encoded = encode_base32(n);
            encoded[0] = c.as_bytes()[0];
            let as_str = str::from_utf8(&encoded).expect("the substituted byte is ASCII");
            let decoded = decode_base32(as_str);
            prop_assert!(
                matches!(decoded, Err(IdError::SuffixOverflow { .. })),
                "leading {c:?} should be rejected, got {decoded:?}"
            );
        }

        /// An excluded letter is rejected wherever in the suffix it appears.
        ///
        /// `i`, `l`, `o` and `u` are the four Crockford dropped as confusable.
        #[test]
        fn prop_rejects_excluded_letters(n: u128, at in 0usize..26, c in "[ilou]") {
            let mut encoded = encode_base32(n);
            encoded[at] = c.as_bytes()[0];
            let as_str = str::from_utf8(&encoded).expect("the substituted byte is ASCII");
            let decoded = decode_base32(as_str);
            prop_assert!(
                matches!(decoded, Err(IdError::SuffixBadChar { .. })),
                "{c:?} at index {at} should be rejected, got {decoded:?}"
            );
        }

        /// Decoding never panics, and only ever accepts the canonical form.
        #[test]
        fn prop_decode_is_total_and_canonical(s in suffix_shaped()) {
            if let Ok(n) = decode_base32(&s) {
                let re_encoded = encode_base32(n);
                prop_assert_eq!(&re_encoded[..], s.as_bytes());
            }
        }
    }
}
