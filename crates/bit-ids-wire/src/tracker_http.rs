//! The HTTP tracker request, kept as the bytes that arrived.
//!
//! `docs/architecture.md` section 5 lists what the parsed view must retain:
//! query and header order, duplicate fields, percent-encoding hex case, the 20
//! peer-ID bytes, key shape, `numwant`, the compact and no-peer-ID flags, event
//! behaviour and address-family extensions. Every one of those is destroyed by
//! the obvious implementation, which is a map of decoded strings.
//!
//! ⛔ **So nothing here is a map and nothing is decoded on the way in.** A
//! header is its raw line with the colon located; a query pair is its raw key
//! and raw value. Decoding is something a caller asks for on one field, never
//! something the parser did to the whole request before anyone could look.
//!
//! ⚠ `+` is **not** decoded to a space. That is an HTML form convention, and an
//! `info_hash` containing byte `0x2b` is sent as a literal `+` by a client that
//! percent-encodes only what it must. Folding it would silently corrupt one
//! byte in 256 of the field this catalogue is mostly about.

use crate::error::WireError;

/// One line of a message head, with the terminator it actually used.
///
/// A bare `\n` where the grammar requires `\r\n` is tolerated by most trackers
/// and is a difference between client implementations, so it is recorded rather
/// than normalised.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Line {
    text: Vec<u8>,
    terminator: Vec<u8>,
}

impl Line {
    /// The line without its terminator.
    #[must_use]
    pub fn text(&self) -> &[u8] {
        &self.text
    }

    /// The terminator bytes, `\r\n` or `\n`.
    #[must_use]
    pub fn terminator(&self) -> &[u8] {
        &self.terminator
    }

    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.text);
        out.extend_from_slice(&self.terminator);
    }
}

/// Which hexadecimal case a value's percent escapes used.
///
/// `%1a` and `%1A` are the same byte and different evidence. Clients are
/// consistent within a build, so the case is an identity signal in its own
/// right, and one that a decode-on-arrival parser can never report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PercentCase {
    /// The value carried no percent escape at all.
    None,
    /// Every escape used `a-f`.
    Lower,
    /// Every escape used `A-F`.
    Upper,
    /// Escapes disagreed with each other inside one value.
    Mixed,
    /// Both cases appeared, but only because no escape used a letter digit.
    Digits,
}

/// One `key=value` of the query string, still encoded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryPair {
    key: Vec<u8>,
    value: Option<Vec<u8>>,
}

impl QueryPair {
    /// The key as it arrived.
    #[must_use]
    pub fn key(&self) -> &[u8] {
        &self.key
    }

    /// The value as it arrived, or `None` for a bare key with no `=`.
    ///
    /// `None` and `Some(b"")` are different requests and stay different here.
    #[must_use]
    pub fn raw_value(&self) -> Option<&[u8]> {
        self.value.as_deref()
    }

    /// The value with `%xx` escapes resolved.
    ///
    /// # Errors
    ///
    /// Returns `percent-escape` for a truncated or non-hexadecimal escape. A
    /// tracker that guessed here would publish bytes no client sent.
    pub fn decoded_value(&self) -> Result<Option<Vec<u8>>, WireError> {
        self.value.as_deref().map(percent_decode).transpose()
    }

    /// Which hexadecimal case this value's escapes used.
    #[must_use]
    pub fn percent_case(&self) -> PercentCase {
        self.value
            .as_deref()
            .map_or(PercentCase::None, percent_case)
    }
}

/// Resolves `%xx` escapes and nothing else.
///
/// # Errors
///
/// Returns `percent-escape` when an escape is truncated or not hexadecimal.
pub fn percent_decode(input: &[u8]) -> Result<Vec<u8>, WireError> {
    let mut out = Vec::with_capacity(input.len());
    let mut index = 0;
    while index < input.len() {
        if input[index] == b'%' {
            let hex = input.get(index + 1..index + 3).ok_or_else(|| {
                WireError::new("percent-escape", index, "escape is cut off by the end")
            })?;
            let (Some(high), Some(low)) = (hex_digit(hex[0]), hex_digit(hex[1])) else {
                return Err(WireError::new(
                    "percent-escape",
                    index,
                    "escape is not two hexadecimal digits",
                ));
            };
            out.push((high << 4) | low);
            index += 3;
        } else {
            out.push(input[index]);
            index += 1;
        }
    }
    Ok(out)
}

const fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn percent_case(input: &[u8]) -> PercentCase {
    let mut lower = false;
    let mut upper = false;
    let mut escapes = 0_usize;
    let mut index = 0;
    while index < input.len() {
        if input[index] == b'%' {
            escapes += 1;
            for &digit in input.get(index + 1..index + 3).unwrap_or_default() {
                if digit.is_ascii_lowercase() {
                    lower = true;
                } else if digit.is_ascii_uppercase() {
                    upper = true;
                }
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    match (escapes, lower, upper) {
        (0, _, _) => PercentCase::None,
        (_, false, false) => PercentCase::Digits,
        (_, true, false) => PercentCase::Lower,
        (_, false, true) => PercentCase::Upper,
        (_, true, true) => PercentCase::Mixed,
    }
}

/// One header, located inside its raw line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Header {
    line: Line,
    colon: usize,
}

impl Header {
    /// The field name, in the case the sender used.
    #[must_use]
    pub fn name(&self) -> &[u8] {
        &self.line.text[..self.colon]
    }

    /// The field value with surrounding spaces and tabs removed.
    ///
    /// The raw line is still available through [`Header::line`], so the
    /// whitespace a sender chose is not lost by offering the trimmed view.
    #[must_use]
    pub fn value(&self) -> &[u8] {
        let raw = &self.line.text[self.colon + 1..];
        let start = raw
            .iter()
            .position(|b| !matches!(b, b' ' | b'\t'))
            .unwrap_or(raw.len());
        let end = raw
            .iter()
            .rposition(|b| !matches!(b, b' ' | b'\t'))
            .map_or(start, |index| index + 1);
        &raw[start..end]
    }

    /// The whole line, terminator included.
    #[must_use]
    pub const fn line(&self) -> &Line {
        &self.line
    }
}

/// An HTTP request from a target under measurement, decoded losslessly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpRequest {
    request_line: Line,
    method: core::ops::Range<usize>,
    target: core::ops::Range<usize>,
    version: core::ops::Range<usize>,
    headers: Vec<Header>,
    blank: Vec<u8>,
    body: Vec<u8>,
}

impl HttpRequest {
    /// The longest head this will decode, in bytes.
    ///
    /// An announce is a few hundred bytes. The cap exists because the target is
    /// untrusted input by construction: it is a binary this project just
    /// installed and pointed at a socket, and a head it never terminates is a
    /// memory leak with a socket attached.
    pub const MAX_HEAD: usize = 64 * 1024;

    /// Decodes a request.
    ///
    /// # Errors
    ///
    /// Returns a [`WireError`] when the head is unterminated or over-long, the
    /// request line does not have three space-separated parts, or a header line
    /// carries no colon.
    pub fn parse(input: &[u8]) -> Result<Self, WireError> {
        // ⚠ The cap is on where the head ENDS, not on whether one exists. The
        // first version only fired when there was no blank line at all, so a
        // hundred megabytes of headers with a blank line at the end sailed
        // past it: the bound was written and did not bind.
        match find_blank(input) {
            None if input.len() > Self::MAX_HEAD => {
                return Err(WireError::new(
                    "head-too-long",
                    0,
                    format!("no blank line in the first {} bytes", Self::MAX_HEAD),
                ));
            }
            Some(at) if at > Self::MAX_HEAD => {
                return Err(WireError::new(
                    "head-too-long",
                    at,
                    format!("head is {at} bytes, over the {} cap", Self::MAX_HEAD),
                ));
            }
            _ => {}
        }
        let mut cursor = 0;
        let request_line = take_line(input, &mut cursor)?;
        let (method, target, version) = split_request_line(&request_line.text)?;
        let mut headers = Vec::new();
        let blank = loop {
            let at = cursor;
            let line = take_line(input, &mut cursor)?;
            if line.text.is_empty() {
                break line.terminator;
            }
            let Some(colon) = line.text.iter().position(|&b| b == b':') else {
                return Err(WireError::new(
                    "header-shape",
                    at,
                    "header line has no colon",
                ));
            };
            if colon == 0 {
                return Err(WireError::new("header-shape", at, "header name is empty"));
            }
            headers.push(Header { line, colon });
        };
        Ok(Self {
            request_line,
            method,
            target,
            version,
            headers,
            blank,
            body: input[cursor..].to_vec(),
        })
    }

    /// The request method.
    #[must_use]
    pub fn method(&self) -> &[u8] {
        &self.request_line.text[self.method.clone()]
    }

    /// The request target, still percent-encoded.
    #[must_use]
    pub fn target(&self) -> &[u8] {
        &self.request_line.text[self.target.clone()]
    }

    /// The protocol version token, such as `HTTP/1.1`.
    #[must_use]
    pub fn version(&self) -> &[u8] {
        &self.request_line.text[self.version.clone()]
    }

    /// The target up to the first `?`.
    #[must_use]
    pub fn path(&self) -> &[u8] {
        let target = self.target();
        target
            .iter()
            .position(|&b| b == b'?')
            .map_or(target, |index| &target[..index])
    }

    /// The target after the first `?`, or `None` when there is none.
    #[must_use]
    pub fn query(&self) -> Option<&[u8]> {
        let target = self.target();
        target
            .iter()
            .position(|&b| b == b'?')
            .map(|index| &target[index + 1..])
    }

    /// The query pairs in the order they were sent, duplicates included.
    ///
    /// An empty query yields no pairs; `?a=1&&b=2` yields two, because an empty
    /// segment carries nothing to record.
    #[must_use]
    pub fn query_pairs(&self) -> Vec<QueryPair> {
        let Some(query) = self.query() else {
            return Vec::new();
        };
        query
            .split(|&b| b == b'&')
            .filter(|segment| !segment.is_empty())
            .map(|segment| {
                let (key, value) = match segment.iter().position(|&b| b == b'=') {
                    Some(index) => (&segment[..index], Some(segment[index + 1..].to_vec())),
                    None => (segment, None),
                };
                QueryPair {
                    key: key.to_vec(),
                    value,
                }
            })
            .collect()
    }

    /// Every value sent under `key`, in order.
    ///
    /// A list rather than an option: `numwant` twice is a real request and
    /// answering with one of them would hide it.
    #[must_use]
    pub fn query_values(&self, key: &[u8]) -> Vec<QueryPair> {
        self.query_pairs()
            .into_iter()
            .filter(|pair| pair.key() == key)
            .collect()
    }

    /// The headers in the order they were sent.
    #[must_use]
    pub fn headers(&self) -> &[Header] {
        &self.headers
    }

    /// Every header whose name matches `name`, compared without case.
    #[must_use]
    pub fn header_values(&self, name: &[u8]) -> Vec<&Header> {
        self.headers
            .iter()
            .filter(|header| header.name().eq_ignore_ascii_case(name))
            .collect()
    }

    /// The message body, empty for an announce.
    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Writes the request back.
    ///
    /// ⛔ Byte for byte what [`HttpRequest::parse`] read.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.request_line.write(&mut out);
        for header in &self.headers {
            header.line.write(&mut out);
        }
        out.extend_from_slice(&self.blank);
        out.extend_from_slice(&self.body);
        out
    }
}

fn find_blank(input: &[u8]) -> Option<usize> {
    input
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .or_else(|| input.windows(2).position(|window| window == b"\n\n"))
}

fn take_line(input: &[u8], cursor: &mut usize) -> Result<Line, WireError> {
    let start = *cursor;
    let Some(offset) = input[start..].iter().position(|&b| b == b'\n') else {
        return Err(WireError::new("truncated", start, "line has no newline"));
    };
    let end = start + offset;
    let (text_end, terminator) = if end > start && input[end - 1] == b'\r' {
        (end - 1, b"\r\n".to_vec())
    } else {
        (end, b"\n".to_vec())
    };
    *cursor = end + 1;
    Ok(Line {
        text: input[start..text_end].to_vec(),
        terminator,
    })
}

type RequestLineParts = (
    core::ops::Range<usize>,
    core::ops::Range<usize>,
    core::ops::Range<usize>,
);

fn split_request_line(text: &[u8]) -> Result<RequestLineParts, WireError> {
    let Some(first) = text.iter().position(|&b| b == b' ') else {
        return Err(WireError::new(
            "request-line",
            0,
            "no space after the method",
        ));
    };
    let Some(last) = text.iter().rposition(|&b| b == b' ') else {
        unreachable!("a first space is also a last space");
    };
    if last == first {
        return Err(WireError::new(
            "request-line",
            first,
            "no space before the version",
        ));
    }
    if first == 0 || last + 1 >= text.len() {
        return Err(WireError::new("request-line", 0, "empty method or version"));
    }
    Ok((0..first, first + 1..last, last + 1..text.len()))
}

#[cfg(test)]
mod tests {
    use super::{HttpRequest, PercentCase, QueryPair, percent_decode};

    const ANNOUNCE: &[u8] =
        b"GET /announce?info_hash=%01%02%2b&peer_id=%2Dtest&numwant=200 HTTP/1.1\r\n\
Host: 127.0.0.1:6969\r\n\
User-Agent: fixture/0\r\n\
Accept-Encoding: gzip\r\n\
\r\n";

    #[test]
    fn a_request_re_encodes_to_the_bytes_it_was_parsed_from() {
        let request = HttpRequest::parse(ANNOUNCE).expect("decodes");
        assert_eq!(request.encode(), ANNOUNCE);
    }

    #[test]
    fn the_query_keeps_its_order_and_its_escape_case() {
        let request = HttpRequest::parse(ANNOUNCE).expect("decodes");
        let pairs = request.query_pairs();
        let keys: Vec<&[u8]> = pairs.iter().map(QueryPair::key).collect();
        assert_eq!(keys, [&b"info_hash"[..], b"peer_id", b"numwant"]);
        // `%2b` is lowercase, `%2D` is uppercase, and `%01%02` constrains
        // nothing because neither escape uses a letter digit. The three answers
        // are different questions and a two-valued report would merge them.
        assert_eq!(
            request.query_values(b"info_hash")[0].percent_case(),
            PercentCase::Lower
        );
        assert_eq!(
            request.query_values(b"peer_id")[0].percent_case(),
            PercentCase::Upper
        );
        let digits_only = HttpRequest::parse(b"GET /a?k=%01%02 HTTP/1.1\r\n\r\n").expect("decodes");
        assert_eq!(
            digits_only.query_values(b"k")[0].percent_case(),
            PercentCase::Digits
        );
    }

    #[test]
    fn a_literal_plus_stays_a_plus_rather_than_becoming_a_space() {
        let request = HttpRequest::parse(b"GET /a?k=a+b%2Bc HTTP/1.1\r\n\r\n").expect("decodes");
        let value = request.query_values(b"k")[0]
            .decoded_value()
            .expect("escapes are well formed")
            .expect("the key has a value");
        assert_eq!(value, b"a+b+c");
    }

    #[test]
    fn a_bare_key_and_an_empty_value_stay_different_requests() {
        let request = HttpRequest::parse(b"GET /a?x&y= HTTP/1.1\r\n\r\n").expect("decodes");
        let pairs = request.query_pairs();
        assert_eq!(pairs[0].raw_value(), None);
        assert_eq!(pairs[1].raw_value(), Some(&b""[..]));
    }

    #[test]
    fn header_order_case_and_duplicates_all_survive() {
        let raw = b"GET / HTTP/1.1\r\nhost: a\r\nX-Try: 1\r\nX-Try: 2\r\n\r\n";
        let request = HttpRequest::parse(raw).expect("decodes");
        assert_eq!(request.headers()[0].name(), b"host");
        assert_eq!(request.header_values(b"Host").len(), 1);
        assert_eq!(request.header_values(b"x-try").len(), 2);
        assert_eq!(request.encode(), raw);
    }

    #[test]
    fn a_bare_newline_terminator_is_recorded_rather_than_repaired() {
        let raw = b"GET / HTTP/1.1\nHost: a\n\n";
        let request = HttpRequest::parse(raw).expect("decodes");
        assert_eq!(request.headers()[0].line().terminator(), b"\n");
        assert_eq!(request.encode(), raw);
    }

    #[test]
    fn a_truncated_escape_is_refused_rather_than_guessed_at() {
        assert_eq!(
            percent_decode(b"%2").expect_err("cut off").kind(),
            "percent-escape"
        );
        assert_eq!(
            percent_decode(b"%zz").expect_err("not hex").kind(),
            "percent-escape"
        );
    }

    #[test]
    fn an_over_long_head_is_refused_even_when_it_is_terminated() {
        let mut raw = b"GET / HTTP/1.1\r\n".to_vec();
        while raw.len() <= HttpRequest::MAX_HEAD {
            // ⚠ Not hex padding. A 40-character run of `0-9a-f` in a tracked
            // file is a credential shape whatever it means, and the public
            // secret scan reported this line when it was one.
            raw.extend_from_slice(b"X-Pad: padding-padding-padding-padding-padding\r\n");
        }
        raw.extend_from_slice(b"\r\n");
        assert_eq!(
            HttpRequest::parse(&raw)
                .expect_err("a terminated head is still capped")
                .kind(),
            "head-too-long"
        );
        let mut unterminated = vec![b'a'; HttpRequest::MAX_HEAD + 1];
        unterminated.splice(0..0, *b"GET / HTTP/1.1\r\n");
        assert_eq!(
            HttpRequest::parse(&unterminated)
                .expect_err("no blank line at all")
                .kind(),
            "head-too-long"
        );
    }

    #[test]
    fn a_body_after_the_blank_line_survives_the_round_trip() {
        let raw = b"POST /announce HTTP/1.1\r\nHost: a\r\n\r\nd1:ai1ee";
        let request = HttpRequest::parse(raw).expect("decodes");
        assert_eq!(request.body(), b"d1:ai1ee");
        assert_eq!(request.method(), b"POST");
        assert_eq!(request.encode(), raw);
    }

    #[test]
    fn a_malformed_head_is_refused_with_the_part_that_was_wrong() {
        assert_eq!(
            HttpRequest::parse(b"GET /only-two-parts\r\n\r\n")
                .expect_err("three parts required")
                .kind(),
            "request-line"
        );
        assert_eq!(
            HttpRequest::parse(b"GET / HTTP/1.1\r\nnocolon\r\n\r\n")
                .expect_err("a header needs a colon")
                .kind(),
            "header-shape"
        );
        assert_eq!(
            HttpRequest::parse(b"GET / HTTP/1.1\r\nHost: a\r\n")
                .expect_err("no blank line")
                .kind(),
            "truncated"
        );
    }
}
