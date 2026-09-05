//! What a run's transcript becomes on disk.
//!
//! `OBS-09`. [`crate::journal`] keeps what the lab observed in memory, which is
//! what the lab's own tests assert against and is not what a capture publishes.
//! A run has to leave content-addressed artifacts a manifest can cite, and this
//! is what writes them.
//!
//! ⭐ **The shape was already specified, so this writes to a contract rather
//! than inventing one.** `bit_ids::manifest::EvidenceRecord` requires per
//! artifact a kind, a readable path, a size, a digest, the tool that produced
//! it, the phase it came out of, and whether anything was scrubbed, and it
//! derives the store path from the digest rather than recording it.
//!
//! # The digest is of the file, and the file is compared against the buffer
//!
//! ⛔ **Two checks, not one.** The digest is computed over bytes read back off
//! the disk, because a writer that digests what it *meant* to write cannot
//! detect a short write: the manifest would then be internally consistent and
//! describe an artifact nobody has. Reading it back closes half of that, and
//! only half: a truncated file digests to a value that matches itself. So the
//! bytes read back are also compared against the bytes intended, and a
//! difference is [`BundleError::ShortWrite`] rather than a smaller number in a
//! record.
//!
//! # A transcript is never scrubbed
//!
//! ⛔ **The bytes a build put on the wire are the measurement.** Scrubbing one
//! changes the identity being measured, and a peer ID is exactly the sort of
//! high-entropy token a scrubber reaches for. So [`Bundle::transcript`] cannot
//! scrub and its records are never `redacted`; the type has no argument for it
//! rather than a flag somebody could pass. Scrubbing belongs to text a *host*
//! produced, which is what [`Bundle::scrubbed_text`] is for, and a scrub there
//! is declared with its count so that `raw` cannot quietly mean `edited`.
//!
//! ⚠ **A scrub replaces what the caller names, and guesses at nothing beyond an
//! address.** A capture harness knows its own hostname, account and variables;
//! a scrubber written here would have to guess at them and would miss one. The
//! caller passes literals with the rule each serves, and the only pattern this
//! module recognises by itself is an IPv4 address, which has a shape and no
//! false negative worth the name.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use bit_ids::canonical::{RelPath, Sha256Digest, Slug};
use bit_ids::manifest::{EvidenceRecord, PhaseName, Redaction, RedactionRule};
use bit_ids::record::EvidenceKind;
use bit_ids_wire::tracker_udp::Direction;

use crate::journal::Journal;

/// The document a transcript artifact is written as.
pub const TRANSCRIPT_SCHEMA: &str = "bit-ids/transcript/1";

/// The placeholder a scrubbed value is replaced with.
///
/// ⚠ Fixed rather than per-occurrence. A placeholder carrying an index would
/// leak how many distinct values there were, and the count is declared in the
/// manifest where a reader can see it beside the rule.
pub const REDACTED: &str = "[redacted]";

/// Why a bundle could not be written.
#[derive(Debug)]
pub enum BundleError {
    /// The artifact identifier or path was not canonical.
    Name(String),
    /// Two artifacts claimed one identifier or one path.
    Duplicate(String),
    /// An artifact would have had no bytes, which the manifest refuses as a
    /// measurement with nothing behind it.
    Empty(Slug),
    /// The journal carries an endpoint the transcript plan does not name.
    Unplanned(Slug),
    /// The path resolves outside the bundle root.
    Outside(RelPath),
    /// Something is already at the path the artifact would take.
    Occupied(RelPath),
    /// The file on disk is not what was written to it.
    ShortWrite {
        /// Which artifact.
        id: Slug,
        /// How many bytes were handed to the filesystem.
        intended: usize,
        /// How many came back.
        found: usize,
    },
    /// The filesystem refused.
    Io(std::io::Error),
}

impl core::fmt::Display for BundleError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Name(detail) => write!(f, "not a canonical name: {detail}"),
            Self::Duplicate(detail) => write!(f, "claimed twice: {detail}"),
            Self::Empty(id) => write!(f, "{id} would have no bytes"),
            Self::Unplanned(endpoint) => write!(
                f,
                "the run recorded {endpoint} and the plan does not say what its evidence is"
            ),
            Self::Outside(path) => write!(f, "{path} resolves outside the bundle"),
            Self::Occupied(path) => write!(f, "{path} is already taken"),
            Self::ShortWrite {
                id,
                intended,
                found,
            } => write!(
                f,
                "{id} was written as {intended} bytes and reads back as {found}"
            ),
            Self::Io(error) => write!(f, "{error}"),
        }
    }
}

impl core::error::Error for BundleError {}

impl From<std::io::Error> for BundleError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// One transformation to apply before writing text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Scrub {
    /// Every IPv4 address, by shape.
    ///
    /// The one rule this module recognises without being told the value, and
    /// only because a dotted quad has a shape that a hostname and an account
    /// name do not.
    Ipv4Addresses,
    /// A literal the caller knows is in the text, declared under the rule it
    /// serves.
    Literal {
        /// What class of value it is.
        rule: RedactionRule,
        /// The exact text to replace.
        value: String,
    },
}

impl Scrub {
    const fn rule(&self) -> RedactionRule {
        match self {
            Self::Ipv4Addresses => RedactionRule::IpAddress,
            Self::Literal { rule, .. } => *rule,
        }
    }
}

/// What one endpoint's bytes become in the manifest.
///
/// ⚠ The identifier is declared rather than derived from the endpoint name. A
/// profile cites evidence by identifier, so the run that writes the artifact
/// and the record that cites it have to agree on one, and deriving it here
/// would make that agreement a convention nothing checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranscriptOf {
    /// The evidence identifier a profile will cite.
    pub id: Slug,
    /// What the bytes are.
    pub kind: EvidenceKind,
}

/// The artifacts of one run, and what was taken out of them.
///
/// ⚠ Written as it is built rather than buffered and flushed. A bundle that
/// held every artifact in memory to write at the end would put a run's whole
/// transcript in the heap twice, and would lose everything to the failure that
/// interrupted it.
#[derive(Debug)]
pub struct Bundle {
    root: PathBuf,
    produced_by: Slug,
    phase: PhaseName,
    evidence: Vec<EvidenceRecord>,
    redactions: Vec<Redaction>,
}

impl Bundle {
    /// Opens a bundle rooted at `root`, whose artifacts `produced_by` made
    /// during `phase`.
    ///
    /// # Errors
    ///
    /// Returns [`BundleError::Io`] when the root cannot be created.
    pub fn create(
        root: impl Into<PathBuf>,
        produced_by: Slug,
        phase: PhaseName,
    ) -> Result<Self, BundleError> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        // ⛔ Resolved once, here, so every write can be checked against where
        // the root actually is rather than against how it was spelled.
        let root = fs::canonicalize(&root)?;
        Ok(Self {
            root,
            produced_by,
            phase,
            evidence: Vec::new(),
            redactions: Vec::new(),
        })
    }

    /// Writes one artifact per endpoint the journal recorded, in endpoint order.
    ///
    /// ⭐ One per endpoint rather than one per run: the surfaces are separate
    /// measurements and a manifest cites them separately, and a single blob
    /// would make a peer transcript unciteable without the tracker's bytes
    /// beside it.
    ///
    /// ⛔ **An endpoint the plan does not name is refused, not defaulted.** A
    /// derived identifier and a guessed kind would give a run that grew a
    /// surface an artifact nobody planned, filed under whatever this function
    /// assumed, and a manifest carries the kind as a claim about what the bytes
    /// are. Failing closed costs one line in the plan; the default costs a
    /// mis-described artifact that validates.
    ///
    /// # Errors
    ///
    /// Returns [`BundleError::Unplanned`] for an endpoint the plan does not
    /// name, and otherwise for a name that is not canonical, an identifier or
    /// path already claimed, or a write that did not survive the round trip.
    pub fn transcripts(
        &mut self,
        journal: &Journal,
        plan: &BTreeMap<Slug, TranscriptOf>,
    ) -> Result<(), BundleError> {
        // Sorted and deduplicated, so two runs over the same journal produce
        // the same bundle: a set built from iteration order would depend on
        // which endpoint thread appended first.
        let mut endpoints: Vec<&Slug> = journal
            .segments()
            .iter()
            .map(crate::journal::Segment::endpoint)
            .collect();
        endpoints.sort_unstable();
        endpoints.dedup();

        for endpoint in endpoints {
            let Some(planned) = plan.get(endpoint) else {
                return Err(BundleError::Unplanned(endpoint.clone()));
            };
            let document = transcript_document(endpoint, journal);
            self.write(
                planned.id.clone(),
                planned.kind,
                &format!("{endpoint}.transcript.json"),
                document.as_bytes(),
                Vec::new(),
            )?;
        }
        Ok(())
    }

    /// Writes bytes that are the measurement, unaltered.
    ///
    /// # Errors
    ///
    /// As [`Bundle::transcripts`].
    pub fn transcript(
        &mut self,
        id: Slug,
        kind: EvidenceKind,
        name: &str,
        bytes: &[u8],
    ) -> Result<(), BundleError> {
        self.write(id, kind, name, bytes, Vec::new())
    }

    /// Writes text a host produced, with `scrubs` applied and declared.
    ///
    /// ⚠ A rule that replaced nothing is not declared, because `E-MAN-61`
    /// refuses one and is right to: a declaration saying zero were removed
    /// reads as a scrub that ran, and it is a scrub that found nothing.
    ///
    /// # Errors
    ///
    /// As [`Bundle::transcripts`].
    pub fn scrubbed_text(
        &mut self,
        id: Slug,
        kind: EvidenceKind,
        name: &str,
        text: &str,
        scrubs: &[Scrub],
    ) -> Result<(), BundleError> {
        let mut cleaned = text.to_owned();
        // ⛔ Aggregated by rule rather than by scrub, because `E-MAN-64` refuses
        // two declarations of one rule against one artifact and two literals
        // can serve the same rule. A list rather than a map: `RedactionRule` is
        // not ordered, six is the whole vocabulary, and first-appearance order
        // is the caller's declared order and so is reproducible.
        let mut declared: Vec<(RedactionRule, u32)> = Vec::new();
        for scrub in scrubs {
            let (next, replaced) = apply(&cleaned, scrub);
            cleaned = next;
            if replaced == 0 {
                continue;
            }
            let rule = scrub.rule();
            if let Some(entry) = declared.iter_mut().find(|(seen, _)| *seen == rule) {
                entry.1 = entry.1.saturating_add(replaced);
            } else {
                declared.push((rule, replaced));
            }
        }
        self.write(id, kind, name, cleaned.as_bytes(), declared)
    }

    /// Every artifact written, ready for a manifest.
    #[must_use]
    pub fn evidence(&self) -> &[EvidenceRecord] {
        &self.evidence
    }

    /// Every declaration of what was taken out, ready for a manifest.
    #[must_use]
    pub fn redactions(&self) -> &[Redaction] {
        &self.redactions
    }

    /// Where the bundle was written.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Re-reads every artifact and checks it against the record that names it.
    ///
    /// ⭐ **What a publisher runs before it appends.** The write path proves the
    /// bytes reached the disk; this proves they are still there and unchanged,
    /// which is a different question and the one that matters between a capture
    /// and a publication. `PUB-01` assembles the tree and this is what it can
    /// call to find out that an artifact was truncated after the run that made
    /// it.
    ///
    /// # Errors
    ///
    /// Returns [`BundleError::ShortWrite`] naming the first artifact whose
    /// bytes no longer match its record, and [`BundleError::Io`] for one that
    /// can no longer be read at all.
    pub fn verify(&self) -> Result<(), BundleError> {
        for record in &self.evidence {
            read_back(
                &self.root.join(record.path.as_str()),
                &record.id,
                record.bytes,
                &record.sha256,
            )?;
        }
        Ok(())
    }

    fn write(
        &mut self,
        id: Slug,
        kind: EvidenceKind,
        name: &str,
        bytes: &[u8],
        declared: Vec<(RedactionRule, u32)>,
    ) -> Result<(), BundleError> {
        if bytes.is_empty() {
            return Err(BundleError::Empty(id));
        }
        let path = RelPath::parse(name).map_err(|error| BundleError::Name(error.to_string()))?;
        if self.evidence.iter().any(|record| record.id == id) {
            return Err(BundleError::Duplicate(id.to_string()));
        }
        if self.evidence.iter().any(|record| record.path == path) {
            return Err(BundleError::Duplicate(path.to_string()));
        }

        let on_disk = self.root.join(path.as_str());
        if let Some(parent) = on_disk.parent() {
            fs::create_dir_all(parent)?;
            // ⛔ **Where the path resolves, not how it is spelled.** `RelPath`
            // already refuses `..`, a leading separator and a backslash, and
            // that is a rule about the text. A symlink sitting in a reused
            // bundle root satisfies every one of those and still lands the
            // artifact somewhere else, with the manifest citing a path that
            // reads as inside the bundle. Two gates on one action is the shape
            // `docs/methodology/reviews.md` names, and this is the second.
            //
            // ⚠ The root is resolved in `create` for the other half of this:
            // compared against an unresolved root, the test below refuses every
            // write when the root is itself reached through a symlink. The
            // acceptance suite builds exactly that case rather than arguing
            // about which platforms have one.
            if !fs::canonicalize(parent)?.starts_with(&self.root) {
                return Err(BundleError::Outside(path));
            }
        }
        // ⛔ A bundle writes new artifacts into its own root. A symlink, a file
        // or a directory already at the path means a rerun into a dirty root or
        // something planted there, and writing anyway would follow the link or
        // overwrite the artifact.
        //
        // ⭐ Anything else is left to the read-back below, and the split is the
        // point: this refuses what would be **followed or overwritten**, and the
        // read-back catches what would be **swallowed**. A character device
        // planted at an artifact path accepts every byte and returns none, which
        // is the one case a check on existence cannot tell from a healthy write
        // and a check on the bytes can. Refusing both here would leave the
        // read-back with no reachable failure and so no way to know it works.
        if fs::symlink_metadata(&on_disk).is_ok_and(|found| {
            let kind = found.file_type();
            kind.is_symlink() || kind.is_file() || kind.is_dir()
        }) {
            return Err(BundleError::Occupied(path));
        }
        fs::write(&on_disk, bytes)?;

        // ⛔ Read back, and compared against what was written, through the same
        // function `verify` uses. The digest of a truncated file matches that
        // file, so digesting the read alone would record a consistent
        // description of the wrong artifact. ⭐ One comparison with two callers
        // rather than two comparisons: the write path's failure needs a
        // filesystem that misbehaves and cannot be provoked in a test, so it is
        // proved by the caller that can be.
        let found = read_back(
            &on_disk,
            &id,
            u64::try_from(bytes.len()).unwrap_or(u64::MAX),
            &Sha256Digest::of(bytes),
        )?;

        let redacted = !declared.is_empty();
        for (rule, occurrences) in declared {
            self.redactions.push(Redaction {
                evidence: id.clone(),
                rule,
                occurrences,
            });
        }
        self.evidence.push(EvidenceRecord {
            id,
            kind,
            path,
            // Of the file, both of them: `read_back` has just proved the file
            // is the buffer, so these describe the artifact on disk.
            bytes: u64::try_from(found.len()).unwrap_or(u64::MAX),
            sha256: Sha256Digest::of(&found),
            produced_by: self.produced_by.clone(),
            phase: self.phase,
            redacted,
        });
        // `E-MAN-50` requires ascending ids, and a caller writing the peer
        // transcript before the tracker one is writing them in the order the
        // run happened. Sorting here rather than refusing keeps the record's
        // order a property of the record.
        self.evidence.sort_by(|left, right| left.id.cmp(&right.id));
        self.redactions
            .sort_by(|left, right| left.evidence.cmp(&right.evidence));
        Ok(())
    }
}

/// Reads `path` and refuses it unless it is exactly `bytes` long and digests to
/// `sha256`.
///
/// ⛔ **The size AND the digest.** A comparison on size alone passes over an
/// edit that kept the length, which is the tampering that matters most.
///
/// ⚠ **The size half is not refuted by any test and is kept deliberately.** For
/// an artifact this bundle wrote, the digest subsumes it: any change to the
/// length changes the digest, so dropping `size != bytes` fails nothing. What it
/// catches is a record whose declared length disagrees with the bytes its own
/// digest names, and that needs a `Bundle` reconstructed from a manifest read
/// off disk rather than one this process just wrote. `PUB-01` is where that
/// arrives. Until then it is a guard against a state nothing can reach, kept
/// because the state becomes reachable and removing it would make the addition
/// somebody else's to remember.
fn read_back(
    path: &Path,
    id: &Slug,
    bytes: u64,
    sha256: &Sha256Digest,
) -> Result<Vec<u8>, BundleError> {
    let found = fs::read(path)?;
    let size = u64::try_from(found.len()).unwrap_or(u64::MAX);
    if size != bytes || Sha256Digest::of(&found) != *sha256 {
        return Err(BundleError::ShortWrite {
            id: id.clone(),
            intended: usize::try_from(bytes).unwrap_or(usize::MAX),
            found: found.len(),
        });
    }
    Ok(found)
}

/// The `bit-ids/transcript/1` document for one endpoint.
///
/// ⛔ Serialised by hand rather than through `serde_json`, for the reason the
/// codecs exist: the field order, the hex case and the exact spacing are the
/// bytes a digest names, and a derive that reordered a field on a dependency
/// bump would change every artifact's digest with nothing saying why.
fn transcript_document(endpoint: &Slug, journal: &Journal) -> String {
    use core::fmt::Write as _;

    let mut out = String::new();
    out.push_str("{\n");
    writeln!(out, "  \"schema\": \"{TRANSCRIPT_SCHEMA}\",").expect("a String cannot fail");
    writeln!(out, "  \"endpoint\": \"{endpoint}\",").expect("a String cannot fail");
    out.push_str("  \"segments\": [\n");
    let segments = journal.for_endpoint(endpoint);
    for (index, segment) in segments.iter().enumerate() {
        out.push_str("    {\n");
        if let Some(connection) = segment.connection() {
            writeln!(out, "      \"connection\": {connection},").expect("a String cannot fail");
        }
        writeln!(out, "      \"offset_ms\": {},", segment.offset_ms())
            .expect("a String cannot fail");
        let direction = match segment.direction() {
            Direction::FromTarget => "from_target",
            Direction::ToTarget => "to_target",
        };
        writeln!(out, "      \"direction\": \"{direction}\",").expect("a String cannot fail");
        writeln!(out, "      \"bytes\": \"{}\"", hex(segment.bytes()))
            .expect("a String cannot fail");
        out.push_str(if index + 1 == segments.len() {
            "    }\n"
        } else {
            "    },\n"
        });
    }
    out.push_str("  ]\n}\n");
    out
}

fn hex(bytes: &[u8]) -> String {
    use core::fmt::Write as _;
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(out, "{byte:02x}").expect("writing to a String cannot fail");
    }
    out
}

/// Applies one scrub, answering the text and how many values it replaced.
fn apply(text: &str, scrub: &Scrub) -> (String, u32) {
    match scrub {
        Scrub::Ipv4Addresses => replace_ipv4(text),
        Scrub::Literal { value, .. } => {
            if value.is_empty() {
                // ⛔ An empty needle matches between every character. Replacing
                // it would rewrite the whole artifact and declare a count that
                // is the length of the text.
                return (text.to_owned(), 0);
            }
            let count = u32::try_from(text.matches(value.as_str()).count()).unwrap_or(u32::MAX);
            (text.replace(value.as_str(), REDACTED), count)
        }
    }
}

/// Replaces every IPv4 dotted quad.
///
/// Hand-written because a regular-expression crate for one pattern is a
/// dependency `docs/supply-chain.md` would ask this entry to argue for, and the
/// argument does not hold: the grammar is four decimal octets separated by
/// dots.
fn replace_ipv4(text: &str) -> (String, u32) {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut count = 0_u32;
    let mut at = 0;
    while at < bytes.len() {
        // Only at a boundary, so the `1.2.3.4` inside `v1.2.3.4` is a version.
        let boundary = at == 0 || !is_address_char(bytes[at - 1]);
        if boundary && let Some(end) = ipv4_end(bytes, at) {
            out.push_str(REDACTED);
            count = count.saturating_add(1);
            at = end;
            continue;
        }
        // The slice is valid UTF-8 and an address character is ASCII, so this
        // never splits a multi-byte character: the push below advances by one
        // whole character.
        let character = text[at..].chars().next().unwrap_or('\u{fffd}');
        out.push(character);
        at += character.len_utf8();
    }
    (out, count)
}

const fn is_address_char(byte: u8) -> bool {
    byte.is_ascii_digit() || byte == b'.'
}

/// Where an IPv4 address starting at `at` ends, if one does.
fn ipv4_end(bytes: &[u8], at: usize) -> Option<usize> {
    let mut cursor = at;
    for octet in 0..4 {
        if octet > 0 {
            if bytes.get(cursor) != Some(&b'.') {
                return None;
            }
            cursor += 1;
        }
        let start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() && cursor - start < 3 {
            cursor += 1;
        }
        if cursor == start {
            return None;
        }
        let digits = &bytes[start..cursor];
        // ⚠ Range-checked, so `999.1.1.1` is not an address and is left alone.
        // A scrubber that removed it would be editing an artifact over text
        // that is not what it claims to be looking for.
        let value: u32 = digits
            .iter()
            .fold(0, |total, digit| total * 10 + u32::from(digit - b'0'));
        if value > 255 {
            return None;
        }
    }
    // A fifth dotted group means this is not an address either.
    if bytes.get(cursor) == Some(&b'.') && bytes.get(cursor + 1).is_some_and(u8::is_ascii_digit) {
        return None;
    }
    Some(cursor)
}

#[cfg(test)]
mod tests {
    use super::{REDACTED, Scrub, apply, replace_ipv4};
    use bit_ids::manifest::RedactionRule;

    #[test]
    fn an_address_is_replaced_and_a_version_string_is_not() {
        assert_eq!(
            replace_ipv4("peer 127.0.0.1:6881 said"),
            ("peer [redacted]:6881 said".to_owned(), 1)
        );
        assert_eq!(replace_ipv4("qBittorrent v4.6.2").1, 0);
        // Out of range, so it is not an address and editing it would be
        // rewriting an artifact over text that is not what it looks like.
        assert_eq!(replace_ipv4("999.1.1.1").1, 0);
        // A fifth group.
        assert_eq!(replace_ipv4("1.2.3.4.5").1, 0);
        // ⛔ Starting mid-number: without the boundary check the scan finds
        // `927.0.0.1` one character in and edits the middle of a longer value.
        assert_eq!(replace_ipv4("1927.0.0.1").1, 0);
        assert_eq!(replace_ipv4("10.0.0.1 and 10.0.0.2").1, 2);
    }

    #[test]
    fn an_empty_literal_replaces_nothing_rather_than_everything() {
        let scrub = Scrub::Literal {
            rule: RedactionRule::UserName,
            value: String::new(),
        };
        assert_eq!(apply("untouched", &scrub), ("untouched".to_owned(), 0));
    }

    #[test]
    fn a_literal_is_counted_once_per_occurrence() {
        let scrub = Scrub::Literal {
            rule: RedactionRule::Hostname,
            value: "runner-7".to_owned(),
        };
        let (text, count) = apply("runner-7 and runner-7", &scrub);
        assert_eq!(count, 2);
        assert_eq!(text, format!("{REDACTED} and {REDACTED}"));
    }
}
