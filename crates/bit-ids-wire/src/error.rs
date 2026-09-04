//! The one error type every codec in this crate returns.
//!
//! A wire parser reports two things a record parser does not have to: which
//! byte was wrong, and what was expected there. An evidence bundle is read by
//! somebody trying to decide whether a parser regressed or a build changed
//! behaviour, and "malformed announce" cannot settle that question while
//! "byte 74: message length 16777216 exceeds the 8192-byte cap" can.

use core::fmt;

/// A byte string this crate refused to decode.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireError {
    kind: &'static str,
    offset: usize,
    detail: String,
}

impl WireError {
    /// Records a refusal at `offset` bytes into the frame being decoded.
    #[must_use]
    pub fn new(kind: &'static str, offset: usize, detail: impl Into<String>) -> Self {
        Self {
            kind,
            offset,
            detail: detail.into(),
        }
    }

    /// The stable machine-readable name of the refusal.
    ///
    /// Tests assert on this rather than on the message, so the wording of a
    /// diagnostic can improve without rewriting the suite that proves it fires.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        self.kind
    }

    /// How many bytes into the frame the refusal happened.
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// Re-bases the offset onto an enclosing frame.
    ///
    /// A message payload is decoded from a slice, so its offsets start at zero.
    /// Reported unshifted from inside a transcript they would name the wrong
    /// byte of the capture, which is worse than reporting none.
    #[must_use]
    pub fn at_base(mut self, base: usize) -> Self {
        self.offset += base;
        self
    }
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: byte {}: {}", self.kind, self.offset, self.detail)
    }
}

impl core::error::Error for WireError {}

/// Reads a big-endian integer of `N` bytes at `offset`, or reports the shortfall.
///
/// # Errors
///
/// Returns `truncated` when fewer than `N` bytes remain.
pub(crate) fn be_bytes<const N: usize>(
    input: &[u8],
    offset: usize,
    what: &'static str,
) -> Result<[u8; N], WireError> {
    let end = offset.checked_add(N).ok_or_else(|| {
        WireError::new(
            "truncated",
            offset,
            format!("{what}: offset overflows usize"),
        )
    })?;
    let slice = input.get(offset..end).ok_or_else(|| {
        WireError::new(
            "truncated",
            offset,
            format!(
                "{what}: needs {N} bytes, {} remain",
                input.len().saturating_sub(offset)
            ),
        )
    })?;
    let mut out = [0_u8; N];
    out.copy_from_slice(slice);
    Ok(out)
}
