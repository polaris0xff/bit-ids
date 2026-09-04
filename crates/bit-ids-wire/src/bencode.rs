//! Bencode, decoded without losing anything the sender chose.
//!
//! BEP 3 says a dictionary's keys are sorted and an integer carries no leading
//! zero. A decoder that enforced both would be correct and useless here: a
//! build that emits `i03e`, or keys in insertion order, has told us something
//! about itself, and this project exists to record exactly that. So the shape
//! rules are **observed and reported**, never imposed.
//!
//! ⛔ **The invariant is that [`encode`] of a decoded [`Value`] reproduces the
//! input byte for byte.** That is the single property which makes a fixture
//! able to separate an observer regression from a client behaviour change: if a
//! decode drops a detail, the re-encode cannot put it back, and the round trip
//! fails in the suite rather than silently in a published record.
//!
//! One deviation is refused rather than preserved, and the asymmetry is
//! deliberate. A byte-string length prefix with a leading zero (`03:abc`) is a
//! framing artefact of the encoder rather than a value the build chose, and
//! carrying its digit text would put a second spelling on every string in the
//! tree to record a signal no real encoder emits. A length comes out of an
//! integer formatter, where a value can come out of anywhere. An integer's own
//! text is kept, because that is a value.

use crate::error::WireError;

/// How deep a decoded document may nest.
///
/// A peer under measurement is untrusted input by construction: it is a binary
/// this project just installed and pointed at a socket. `l` repeated ten
/// thousand times is a stack overflow in any recursive decoder, and an aborted
/// process loses the whole capture rather than one frame.
pub const MAX_DEPTH: usize = 32;

/// The digit text of a bencoded integer, exactly as it arrived.
///
/// Kept as text because `i-0e` and `i0e` mean the same number and are different
/// bytes, and this crate must be able to write back the ones it read.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Integer(String);

impl Integer {
    /// Accepts the text between `i` and `e`.
    ///
    /// # Errors
    ///
    /// Returns `integer-shape` when the text is empty, is a bare `-`, or holds
    /// anything but an optional leading `-` followed by ASCII digits.
    pub fn parse(text: &str) -> Result<Self, WireError> {
        let digits = text.strip_prefix('-').unwrap_or(text);
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return Err(WireError::new(
                "integer-shape",
                0,
                format!("not an optionally signed decimal: {text:?}"),
            ));
        }
        Ok(Self(text.to_owned()))
    }

    /// Builds the canonical text for a number.
    #[must_use]
    pub fn from_i64(value: i64) -> Self {
        Self(value.to_string())
    }

    /// The digit text as it arrived.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The number, or `None` when it does not fit in an `i64`.
    ///
    /// A width that does not fit is not a decode failure. The bytes are still
    /// recorded, and a peer advertising a 40-digit `reqq` is an observation
    /// about that peer rather than a frame this crate should refuse.
    #[must_use]
    pub fn to_i64(&self) -> Option<i64> {
        self.0.parse().ok()
    }

    /// Whether the text is the one spelling BEP 3 allows for its value.
    ///
    /// False for `-0`, for a leading zero, and for nothing else. A caller
    /// recording an identity field wants to know; the decoder does not care.
    #[must_use]
    pub fn is_canonical(&self) -> bool {
        let digits = self.0.strip_prefix('-').unwrap_or(&self.0);
        if self.0 == "-0" {
            return false;
        }
        digits == "0" || !digits.starts_with('0')
    }
}

/// One bencoded value, with the sender's choices intact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    /// `i<digits>e`, holding the digit text rather than a number.
    Integer(Integer),
    /// `<len>:<bytes>`.
    Bytes(Vec<u8>),
    /// `l...e`.
    List(Vec<Value>),
    /// `d...e`, as received: order preserved, duplicate keys preserved.
    ///
    /// ⛔ Not a map. Sorting on the way in, or collapsing a duplicate key,
    /// discards the two shape signals this type exists to carry.
    Dictionary(Vec<(Vec<u8>, Value)>),
}

impl Value {
    /// Wraps a byte string.
    #[must_use]
    pub fn bytes(value: impl Into<Vec<u8>>) -> Self {
        Self::Bytes(value.into())
    }

    /// Wraps a number in its canonical text.
    #[must_use]
    pub fn integer(value: i64) -> Self {
        Self::Integer(Integer::from_i64(value))
    }

    /// The name of the bencode type, for a diagnostic.
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Integer(_) => "integer",
            Self::Bytes(_) => "bytes",
            Self::List(_) => "list",
            Self::Dictionary(_) => "dictionary",
        }
    }

    /// The first value stored under `key`, when this is a dictionary.
    ///
    /// First rather than last, and only when this is a dictionary: a lookup
    /// that silently answered from a list would make a shape defect look like a
    /// missing field.
    #[must_use]
    pub fn get(&self, key: &[u8]) -> Option<&Self> {
        let Self::Dictionary(entries) = self else {
            return None;
        };
        entries
            .iter()
            .find(|(name, _)| name.as_slice() == key)
            .map(|(_, value)| value)
    }

    /// Whether a dictionary's keys arrived in the ascending order BEP 3 requires.
    ///
    /// `None` for every other type. Unsorted keys are legible bencode and a
    /// real difference between implementations, so this reports rather than
    /// refuses.
    #[must_use]
    pub fn keys_are_sorted(&self) -> Option<bool> {
        let Self::Dictionary(entries) = self else {
            return None;
        };
        Some(entries.windows(2).all(|pair| pair[0].0 < pair[1].0))
    }

    /// Whether a dictionary carried the same key twice.
    ///
    /// `None` for every other type. A duplicate key has no defined meaning, so
    /// a decoder that kept one of them would be choosing the record's contents.
    #[must_use]
    pub fn has_duplicate_keys(&self) -> Option<bool> {
        let Self::Dictionary(entries) = self else {
            return None;
        };
        let mut keys: Vec<&[u8]> = entries.iter().map(|(name, _)| name.as_slice()).collect();
        keys.sort_unstable();
        let total = keys.len();
        keys.dedup();
        Some(keys.len() != total)
    }
}

/// Decodes one value and requires it to be the whole input.
///
/// # Errors
///
/// Returns a [`WireError`] for any malformed frame, and `trailing-bytes` when
/// the input holds more than one value. A tracker response with a second
/// document glued to it is a finding, not something to read past.
pub fn decode(input: &[u8]) -> Result<Value, WireError> {
    let (value, used) = decode_prefix(input)?;
    if used != input.len() {
        return Err(WireError::new(
            "trailing-bytes",
            used,
            format!("{} bytes after the value", input.len() - used),
        ));
    }
    Ok(value)
}

/// Decodes one value from the front of `input`, returning it and its width.
///
/// # Errors
///
/// Returns a [`WireError`] naming the byte that could not be decoded.
pub fn decode_prefix(input: &[u8]) -> Result<(Value, usize), WireError> {
    let mut cursor = 0;
    let value = decode_at(input, &mut cursor, 0)?;
    Ok((value, cursor))
}

fn decode_at(input: &[u8], cursor: &mut usize, depth: usize) -> Result<Value, WireError> {
    if depth > MAX_DEPTH {
        return Err(WireError::new(
            "nesting-depth",
            *cursor,
            format!("deeper than {MAX_DEPTH} containers"),
        ));
    }
    let Some(&lead) = input.get(*cursor) else {
        return Err(WireError::new("truncated", *cursor, "expected a value"));
    };
    match lead {
        b'i' => decode_integer(input, cursor),
        b'l' => decode_list(input, cursor, depth),
        b'd' => decode_dictionary(input, cursor, depth),
        b'0'..=b'9' => decode_bytes(input, cursor),
        other => Err(WireError::new(
            "value-lead",
            *cursor,
            format!("no bencode value starts with {:?}", char::from(other)),
        )),
    }
}

fn decode_integer(input: &[u8], cursor: &mut usize) -> Result<Value, WireError> {
    let start = *cursor + 1;
    let Some(offset) = input[start..].iter().position(|&b| b == b'e') else {
        return Err(WireError::new("truncated", *cursor, "integer has no e"));
    };
    let text = core::str::from_utf8(&input[start..start + offset])
        .map_err(|_| WireError::new("integer-shape", start, "integer text is not ASCII digits"))?;
    let integer = Integer::parse(text)
        .map_err(|error| WireError::new(error.kind(), start, "integer text"))?;
    *cursor = start + offset + 1;
    Ok(Value::Integer(integer))
}

fn decode_bytes(input: &[u8], cursor: &mut usize) -> Result<Value, WireError> {
    let start = *cursor;
    let Some(offset) = input[start..].iter().position(|&b| b == b':') else {
        return Err(WireError::new(
            "truncated",
            start,
            "byte string has no colon",
        ));
    };
    let digits = &input[start..start + offset];
    if digits.len() > 1 && digits[0] == b'0' {
        return Err(WireError::new(
            "length-shape",
            start,
            "byte-string length has a leading zero",
        ));
    }
    let text = core::str::from_utf8(digits)
        .map_err(|_| WireError::new("length-shape", start, "length is not ASCII digits"))?;
    let length: usize = text
        .parse()
        .map_err(|_| WireError::new("length-shape", start, format!("length {text:?}")))?;
    let body = start + offset + 1;
    // ⛔ Checked against what is actually there before allocating. A hostile or
    // broken peer sending `999999999999:` must cost one error, not an attempt
    // to reserve a terabyte.
    let end = body
        .checked_add(length)
        .ok_or_else(|| WireError::new("length-shape", start, "length overflows usize"))?;
    if end > input.len() {
        return Err(WireError::new(
            "truncated",
            body,
            format!(
                "byte string claims {length} bytes, {} remain",
                input.len().saturating_sub(body)
            ),
        ));
    }
    *cursor = end;
    Ok(Value::Bytes(input[body..end].to_vec()))
}

fn decode_list(input: &[u8], cursor: &mut usize, depth: usize) -> Result<Value, WireError> {
    let start = *cursor;
    *cursor += 1;
    let mut items = Vec::new();
    loop {
        match input.get(*cursor) {
            None => return Err(WireError::new("truncated", start, "list has no e")),
            Some(b'e') => {
                *cursor += 1;
                return Ok(Value::List(items));
            }
            Some(_) => items.push(decode_at(input, cursor, depth + 1)?),
        }
    }
}

fn decode_dictionary(input: &[u8], cursor: &mut usize, depth: usize) -> Result<Value, WireError> {
    let start = *cursor;
    *cursor += 1;
    let mut entries: Vec<(Vec<u8>, Value)> = Vec::new();
    loop {
        match input.get(*cursor) {
            None => return Err(WireError::new("truncated", start, "dictionary has no e")),
            Some(b'e') => {
                *cursor += 1;
                return Ok(Value::Dictionary(entries));
            }
            Some(b'0'..=b'9') => {
                let key_at = *cursor;
                let Value::Bytes(key) = decode_bytes(input, cursor)? else {
                    unreachable!("decode_bytes returns Value::Bytes");
                };
                // ⚠ `e` counts as absent, not as a value. No bencode value
                // starts with `e`, so `d1:ae` is a key whose value is missing.
                // Falling through to `decode_at` reported "no bencode value
                // starts with 'e'", which sends a reader looking for a value
                // that was never written instead of at the key that lost one.
                if !matches!(input.get(*cursor), Some(&byte) if byte != b'e') {
                    return Err(WireError::new(
                        "truncated",
                        key_at,
                        "dictionary key has no value",
                    ));
                }
                let value = decode_at(input, cursor, depth + 1)?;
                entries.push((key, value));
            }
            Some(other) => {
                return Err(WireError::new(
                    "dictionary-key",
                    *cursor,
                    format!("key is a {:?}, not a byte string", char::from(*other)),
                ));
            }
        }
    }
}

/// Writes a value back to bytes.
///
/// ⛔ For anything [`decode`] produced this reproduces the input exactly. The
/// suite asserts it over every fixture; see `tests/fixtures.rs`.
#[must_use]
pub fn encode(value: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    write_value(value, &mut out);
    out
}

fn write_value(value: &Value, out: &mut Vec<u8>) {
    match value {
        Value::Integer(integer) => {
            out.push(b'i');
            out.extend_from_slice(integer.as_str().as_bytes());
            out.push(b'e');
        }
        Value::Bytes(bytes) => {
            out.extend_from_slice(bytes.len().to_string().as_bytes());
            out.push(b':');
            out.extend_from_slice(bytes);
        }
        Value::List(items) => {
            out.push(b'l');
            for item in items {
                write_value(item, out);
            }
            out.push(b'e');
        }
        Value::Dictionary(entries) => {
            out.push(b'd');
            for (key, item) in entries {
                out.extend_from_slice(key.len().to_string().as_bytes());
                out.push(b':');
                out.extend_from_slice(key);
                write_value(item, out);
            }
            out.push(b'e');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Integer, MAX_DEPTH, Value, decode, decode_prefix, encode};

    fn round_trips(input: &[u8]) {
        let value = decode(input).expect("decodes");
        assert_eq!(encode(&value), input, "re-encode differs from the input");
    }

    #[test]
    fn an_unsorted_dictionary_is_recorded_rather_than_refused() {
        let value = decode(b"d1:bi1e1:ai2ee").expect("unsorted keys are legible bencode");
        assert_eq!(value.keys_are_sorted(), Some(false));
        round_trips(b"d1:bi1e1:ai2ee");
    }

    #[test]
    fn a_duplicate_key_survives_the_round_trip() {
        let value = decode(b"d1:ai1e1:ai2ee").expect("a duplicate key is legible bencode");
        assert_eq!(value.has_duplicate_keys(), Some(true));
        assert_eq!(value.get(b"a"), Some(&Value::integer(1)));
        round_trips(b"d1:ai1e1:ai2ee");
    }

    #[test]
    fn a_non_canonical_integer_keeps_its_digits() {
        for text in [&b"i-0e"[..], b"i007e", b"i0e", b"i-42e"] {
            round_trips(text);
        }
        assert!(!Integer::parse("-0").expect("shape ok").is_canonical());
        assert!(!Integer::parse("007").expect("shape ok").is_canonical());
        assert!(Integer::parse("0").expect("shape ok").is_canonical());
        assert!(Integer::parse("-42").expect("shape ok").is_canonical());
    }

    #[test]
    fn an_integer_too_wide_for_i64_is_kept_rather_than_refused() {
        let value = decode(b"i99999999999999999999e").expect("width is not a framing error");
        let Value::Integer(integer) = value else {
            panic!("expected an integer");
        };
        assert_eq!(integer.to_i64(), None);
        round_trips(b"i99999999999999999999e");
    }

    #[test]
    fn an_empty_byte_string_is_legal_and_a_padded_length_is_not() {
        round_trips(b"0:");
        assert_eq!(
            decode(b"03:abc")
                .expect_err("a padded length is refused")
                .kind(),
            "length-shape"
        );
    }

    #[test]
    fn an_oversized_length_is_refused_before_it_is_allocated() {
        let error = decode(b"999999999999:ab").expect_err("nothing that long is there");
        assert_eq!(error.kind(), "truncated");
    }

    #[test]
    fn nesting_past_the_cap_is_refused_rather_than_overflowing_the_stack() {
        let mut deep = vec![b'l'; MAX_DEPTH + 2];
        deep.extend(core::iter::repeat_n(b'e', MAX_DEPTH + 2));
        assert_eq!(
            decode(&deep).expect_err("deeper than the cap").kind(),
            "nesting-depth"
        );
        let mut allowed = vec![b'l'; MAX_DEPTH];
        allowed.extend(core::iter::repeat_n(b'e', MAX_DEPTH));
        decode(&allowed).expect("exactly at the cap still decodes");
    }

    #[test]
    fn a_second_document_after_the_first_is_a_finding() {
        assert_eq!(
            decode(b"i1ei2e")
                .expect_err("two values is not one value")
                .kind(),
            "trailing-bytes"
        );
        let (value, used) = decode_prefix(b"i1ei2e").expect("the prefix alone decodes");
        assert_eq!((value, used), (Value::integer(1), 3));
    }

    #[test]
    fn a_dictionary_key_that_is_not_a_byte_string_is_refused() {
        assert_eq!(
            decode(b"di1ei2ee")
                .expect_err("keys are byte strings")
                .kind(),
            "dictionary-key"
        );
        assert_eq!(
            decode(b"d1:ae").expect_err("a key with no value").kind(),
            "truncated"
        );
    }

    #[test]
    fn a_lookup_on_a_list_answers_nothing_rather_than_guessing() {
        let value = decode(b"l1:a1:be").expect("decodes");
        assert_eq!(value.get(b"a"), None);
        assert_eq!(value.keys_are_sorted(), None);
        assert_eq!(value.has_duplicate_keys(), None);
    }
}
