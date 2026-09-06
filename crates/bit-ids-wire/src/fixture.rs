//! The fixture document, and the rule that a fixture must round-trip.
//!
//! `FOUND-03` exists because a live capture alone cannot separate an observer
//! regression from a client behaviour change: both arrive as "the parse looks
//! different this week". A fixture holds bytes that provably did not change, so
//! a parse that changed against one is the parser's doing and nothing else.
//!
//! ⛔ **Nothing here is a measurement.** Every fixture is written by hand from a
//! published specification and carries `origin: synthetic` to say so. A fixture
//! is exactly the kind of input `docs/capture-methodology.md` lists as allowed
//! to seed a parser and forbidden to populate the catalogue, and there is no
//! `captured` origin to blur that: a transcript that came off a real build is
//! evidence, belongs in a run's bundle under its manifest, and is not a fixture.
//!
//! ⭐ **Every frame is bytes the target emitted.** The observer's own replies
//! prove nothing about a build, so they are not fixture material and there is no
//! direction field to get wrong.
//!
//! ## The bytes are hexadecimal, and that is deliberate
//!
//! A `.bin` beside the document would be the obvious shape and the wrong one.
//! `scripts/common/check-control-bytes.sh` documents what a literal control byte
//! costs: `grep` skips the file and `git diff` renders no diff at all, so the
//! one artefact a reviewer most needs to read becomes the one they cannot. Hex
//! is lossless, so nothing is given up for it.

use std::path::{Path, PathBuf};

use bit_ids::canonical::{HexBytes, Instant, Label, Sha256Digest, Slug};
use bit_ids::observation::Surface;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::bencode;
use crate::dht;
use crate::error::WireError;
use crate::peer_wire::Transcript;
use crate::tracker_http::HttpRequest;
use crate::tracker_udp::{Datagram, Direction};

/// Identifier carried by every first-generation fixture.
pub const FIXTURE_SCHEMA: &str = "bit-ids/wire-fixture/1";
/// Identifier carried by the corpus index.
pub const INDEX_SCHEMA: &str = "bit-ids/wire-fixture-index/1";

/// Where a fixture's bytes came from.
///
/// One variant, and the absence of a second is the point. See the module
/// documentation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// Written by hand from a published specification. Never evidence.
    Synthetic,
}

/// Why these bytes are what they are.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    /// How the bytes were produced.
    pub origin: Origin,
    /// When the fixture was written.
    pub authored: Instant,
    /// The specifications the bytes were written from, such as `BEP 15`.
    pub specifications: Vec<Label>,
}

/// One write the target made, with the observer's monotonic clock.
///
/// A stream surface is framed by the writes the target chose to make, and that
/// segmentation is itself a difference between builds: a handshake and a
/// bitfield in one write is not the same observation as the same bytes in two.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    /// Milliseconds since the first frame of this fixture.
    pub offset_ms: u64,
    /// The bytes, lowercase hexadecimal.
    pub bytes: HexBytes,
}

/// One byte-exact protocol fixture.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Fixture {
    /// The schema identifier, read before anything else.
    pub schema: String,
    /// The fixture's identifier, which is also its file stem.
    pub id: Slug,
    /// The surface these bytes belong to.
    pub surface: Surface,
    /// One line saying what this fixture is for.
    pub summary: Label,
    /// Why these bytes are what they are.
    pub provenance: Provenance,
    /// The target's writes, in order.
    pub frames: Vec<Frame>,
}

/// The same shape with the derive on it, so the public type's own
/// `Deserialize` can validate. Same construction as `bit_ids::record`: a
/// document that reached a caller without being checked is the defect.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureFields {
    schema: String,
    id: Slug,
    surface: Surface,
    summary: Label,
    provenance: Provenance,
    frames: Vec<Frame>,
}

impl From<FixtureFields> for Fixture {
    fn from(fields: FixtureFields) -> Self {
        Self {
            schema: fields.schema,
            id: fields.id,
            surface: fields.surface,
            summary: fields.summary,
            provenance: fields.provenance,
            frames: fields.frames,
        }
    }
}

impl<'de> Deserialize<'de> for Fixture {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let fixture = Self::from(FixtureFields::deserialize(deserializer)?);
        // ⚠ Joined into one string because serde's error carries text only.
        // That is why `from_json` goes the long way round through the mirror:
        // it is the path that hands a caller the codes.
        fixture.validate().map_err(|violations| {
            let joined: Vec<String> = violations.iter().map(ToString::to_string).collect();
            D::Error::custom(joined.join("; "))
        })?;
        Ok(fixture)
    }
}

/// One refused fixture invariant.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureViolation {
    code: &'static str,
    detail: String,
}

impl FixtureViolation {
    /// The stable diagnostic code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }

    /// What is wrong, in one line.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl core::fmt::Display for FixtureViolation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

/// Why a fixture could not be read.
#[derive(Debug)]
pub enum FixtureError {
    /// The document declares a schema this build does not read, reported before
    /// any other parsing.
    UnsupportedSchema {
        /// The identifier the document declared.
        found: String,
        /// The identifier this build reads.
        expected: &'static str,
    },
    /// The document is not JSON, or not this schema's shape.
    Malformed(serde_json::Error),
    /// The document parsed and then refused an invariant.
    Invalid(Vec<FixtureViolation>),
    /// The file could not be read from disk.
    Unreadable(std::io::Error),
}

impl core::fmt::Display for FixtureError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnsupportedSchema { found, expected } => write!(
                f,
                "unsupported schema {found:?}, this build reads {expected:?}"
            ),
            Self::Malformed(error) => write!(f, "malformed fixture: {error}"),
            Self::Invalid(violations) => {
                writeln!(f, "invalid fixture:")?;
                for violation in violations {
                    writeln!(f, "  {violation}")?;
                }
                Ok(())
            }
            Self::Unreadable(error) => write!(f, "unreadable fixture: {error}"),
        }
    }
}

impl core::error::Error for FixtureError {}

fn violation(code: &'static str, detail: impl Into<String>) -> FixtureViolation {
    FixtureViolation {
        code,
        detail: detail.into(),
    }
}

impl Fixture {
    /// Reads and validates a fixture document.
    ///
    /// # Errors
    ///
    /// Returns [`FixtureError::UnsupportedSchema`] when the document was written
    /// against another generation, [`FixtureError::Malformed`] when it is not
    /// this schema's shape, and [`FixtureError::Invalid`] when it parsed and
    /// then refused an invariant.
    pub fn from_json(document: &str) -> Result<Self, FixtureError> {
        #[derive(Deserialize)]
        struct VersionProbe {
            schema: String,
        }

        let probe: VersionProbe =
            serde_json::from_str(document).map_err(FixtureError::Malformed)?;
        if probe.schema != FIXTURE_SCHEMA {
            return Err(FixtureError::UnsupportedSchema {
                found: probe.schema,
                expected: FIXTURE_SCHEMA,
            });
        }
        // Through the mirror, so the codes reach the caller rather than the one
        // string serde would let `Deserialize` report.
        let fields: FixtureFields =
            serde_json::from_str(document).map_err(FixtureError::Malformed)?;
        let fixture = Self::from(fields);
        fixture.validate().map_err(FixtureError::Invalid)?;
        Ok(fixture)
    }

    /// Reads and validates a fixture from a file.
    ///
    /// # Errors
    ///
    /// Returns [`FixtureError::Unreadable`] when the file cannot be read, plus
    /// everything [`Fixture::from_json`] returns, and
    /// [`FixtureError::Invalid`] with `E-FIX-02` when the identifier inside
    /// disagrees with the file stem.
    pub fn from_path(path: &Path) -> Result<Self, FixtureError> {
        let document = std::fs::read_to_string(path).map_err(FixtureError::Unreadable)?;
        let fixture = Self::from_json(&document)?;
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("");
        if stem != fixture.id.as_str() {
            return Err(FixtureError::Invalid(vec![violation(
                "E-FIX-02",
                format!("file stem {stem:?} is not the id {:?}", fixture.id),
            )]));
        }
        Ok(fixture)
    }

    /// Writes the fixture in the canonical form.
    ///
    /// # Errors
    ///
    /// Returns [`FixtureError::Invalid`] with the refused invariants; an invalid
    /// fixture has no canonical form and is not written.
    pub fn to_json(&self) -> Result<String, FixtureError> {
        self.validate().map_err(FixtureError::Invalid)?;
        let mut out = serde_json::to_string_pretty(self).map_err(FixtureError::Malformed)?;
        out.push('\n');
        Ok(out)
    }

    /// The bytes of every frame, joined.
    ///
    /// A stream surface is one transcript split across writes, so the codec
    /// reads the join and the frames record the segmentation.
    #[must_use]
    pub fn joined_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for frame in &self.frames {
            out.extend_from_slice(frame.bytes.as_slice());
        }
        out
    }

    /// The digest of the fixture's canonical document form.
    ///
    /// # Errors
    ///
    /// Returns whatever [`Fixture::to_json`] refused.
    pub fn digest(&self) -> Result<Sha256Digest, FixtureError> {
        Ok(Sha256Digest::of(self.to_json()?.as_bytes()))
    }

    /// Every invariant this fixture refuses.
    ///
    /// # Errors
    ///
    /// Returns the refusals, each with a stable code:
    ///
    /// - `E-FIX-01` the schema identifier is not [`FIXTURE_SCHEMA`];
    /// - `E-FIX-03` there are no frames;
    /// - `E-FIX-04` the first frame is not at offset zero;
    /// - `E-FIX-05` an offset is earlier than the one before it;
    /// - `E-FIX-06` no specification is named;
    /// - `E-FIX-07` this crate has no codec for the surface;
    /// - `E-FIX-08` the bytes do not decode;
    /// - `E-FIX-09` the bytes decode and re-encode to something else;
    /// - `E-FIX-10` an extension dictionary decodes and re-encodes to something
    ///   else, which the transcript round trip cannot see.
    ///
    /// Two codes are not raised here because one fixture cannot answer them.
    /// `E-FIX-02`, the identifier disagreeing with the file name, belongs to
    /// [`Fixture::from_path`]; `E-FIX-11`, an entry in the corpus directory that
    /// is not a fixture, belongs to [`load_directory`].
    pub fn validate(&self) -> Result<(), Vec<FixtureViolation>> {
        let mut found = Vec::new();
        if self.schema != FIXTURE_SCHEMA {
            found.push(violation(
                "E-FIX-01",
                format!("schema {:?} is not {FIXTURE_SCHEMA:?}", self.schema),
            ));
        }
        if self.frames.is_empty() {
            found.push(violation(
                "E-FIX-03",
                "a fixture with no bytes proves nothing",
            ));
        } else if self.frames[0].offset_ms != 0 {
            found.push(violation(
                "E-FIX-04",
                format!(
                    "the first frame is at {}ms; offsets are relative to it",
                    self.frames[0].offset_ms
                ),
            ));
        }
        for (index, pair) in self.frames.windows(2).enumerate() {
            if pair[1].offset_ms < pair[0].offset_ms {
                found.push(violation(
                    "E-FIX-05",
                    format!(
                        "frame {} is at {}ms, after {}ms",
                        index + 1,
                        pair[1].offset_ms,
                        pair[0].offset_ms
                    ),
                ));
            }
        }
        if self.provenance.specifications.is_empty() {
            found.push(violation(
                "E-FIX-06",
                "a synthetic fixture must name what it was written from",
            ));
        }
        // ⚠ Skipped when there are no frames: `E-FIX-03` already says so, and a
        // codec asked to decode nothing would report a truncation that names a
        // byte the fixture does not have.
        if !self.frames.is_empty()
            && let Err(error) = self.round_trip()
        {
            found.push(error);
        }
        if found.is_empty() { Ok(()) } else { Err(found) }
    }

    /// Decodes the fixture with its surface's codec and re-encodes it.
    ///
    /// ⛔ **This is the invariant the whole entry rests on.** A decoder that
    /// drops a detail cannot put it back, so a lossy parse fails here rather
    /// than silently in a published record.
    fn round_trip(&self) -> Result<(), FixtureViolation> {
        let bytes = self.joined_bytes();
        let re_encoded = match self.surface {
            // ⭐ Local discovery shares the HTTP reader on purpose. A BEP 14
            // announce is an HTTP request with a different method, and a
            // second head parser for it would be two readings of one grammar
            // that disagree first about header case and line terminators,
            // which is what an announce is identifying by. `OBS-06`.
            Surface::TrackerHttp | Surface::LocalDiscovery => {
                HttpRequest::parse(&bytes).map(|request| request.encode())
            }
            Surface::PeerWire => Transcript::parse(&bytes).map(|transcript| transcript.encode()),
            Surface::TrackerUdp => self.round_trip_datagrams(),
            // ⚠ Framed per datagram for the reason the UDP tracker is: one KRPC
            // message is one packet, so joining them would read the second as
            // the first one's trailing bytes, which the DHT codec would then
            // report as a departure rather than as a message.
            Surface::Dht => self.round_trip_krpc(),
            other => {
                return Err(violation(
                    "E-FIX-07",
                    format!("no codec in this crate reads {other}"),
                ));
            }
        };
        let re_encoded = re_encoded.map_err(|error: WireError| {
            violation("E-FIX-08", format!("does not decode: {error}"))
        })?;
        if re_encoded != bytes {
            return Err(violation(
                "E-FIX-09",
                format!(
                    "re-encoded to {} bytes from {}; the decode lost something",
                    re_encoded.len(),
                    bytes.len()
                ),
            ));
        }
        if self.surface == Surface::PeerWire {
            Self::round_trip_extension_dictionaries(&bytes)?;
        }
        Ok(())
    }

    /// The bencode inside an extended message must re-encode too.
    ///
    /// ⭐ The transcript round trip does **not** cover this and cannot. A
    /// message re-encodes from its payload bytes, held verbatim, so the bencode
    /// encoder is nowhere on that path: a mutation that canonicalised integer
    /// text on the way out was planted and the corpus passed. That is the
    /// "grep the callers before believing it is load-bearing" finding, and this
    /// is the check that closes it.
    fn round_trip_extension_dictionaries(bytes: &[u8]) -> Result<(), FixtureViolation> {
        let transcript = Transcript::parse(bytes)
            .map_err(|error| violation("E-FIX-08", format!("does not decode: {error}")))?;
        for message in transcript.messages() {
            let Some(extended) = message.as_extended() else {
                continue;
            };
            let extended = extended
                .map_err(|error| violation("E-FIX-08", format!("does not decode: {error}")))?;
            let re_encoded = bencode::encode(extended.document());
            if re_encoded != extended.raw() {
                return Err(violation(
                    "E-FIX-10",
                    format!(
                        "extension dictionary {} re-encoded to {} bytes from {}",
                        extended.extended_id(),
                        re_encoded.len(),
                        extended.raw().len()
                    ),
                ));
            }
        }
        Ok(())
    }

    /// UDP is framed by datagram, so each frame is decoded on its own.
    ///
    /// Joining them and decoding once would read the second datagram as the
    /// first one's trailing bytes, which is the whole reason a datagram surface
    /// cannot share the stream path.
    fn round_trip_datagrams(&self) -> Result<Vec<u8>, WireError> {
        let mut out = Vec::new();
        for frame in &self.frames {
            let datagram = Datagram::parse(Direction::FromTarget, frame.bytes.as_slice())?;
            out.extend_from_slice(&datagram.encode());
        }
        Ok(out)
    }

    /// KRPC is framed by datagram too, so each frame decodes on its own.
    ///
    /// ⚠ Separate from [`Fixture::round_trip_datagrams`] rather than generic
    /// over a codec: the two decode different grammars and share only the loop,
    /// and a helper taking a function pointer would read as though the surfaces
    /// were interchangeable.
    fn round_trip_krpc(&self) -> Result<Vec<u8>, WireError> {
        let mut out = Vec::new();
        for frame in &self.frames {
            let message = dht::Message::parse(frame.bytes.as_slice())?;
            out.extend_from_slice(&message.encode());
        }
        Ok(out)
    }
}

/// One row of the corpus index.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexEntry {
    /// The fixture's identifier, which is also its file stem.
    pub id: Slug,
    /// The digest of the fixture's canonical document form.
    pub digest: Sha256Digest,
}

/// The committed digest of every fixture in the corpus.
///
/// ⭐ This is what makes "the digests are identical" something a suite asserts
/// rather than something a person compares by eye across two runs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FixtureIndex {
    /// The schema identifier.
    pub schema: String,
    /// The digest over every row, derived by [`FixtureIndex::derive_corpus`].
    pub corpus: Sha256Digest,
    /// The rows, ascending by identifier.
    pub entries: Vec<IndexEntry>,
}

/// The same shape with the derive on it.
///
/// ⛔ The index does **not** derive `Deserialize` either, for the reason
/// `Profile` does not: `serde_json::from_str::<FixtureIndex>` would otherwise be
/// a second, looser door that skips the corpus-digest check, and an index nobody
/// checked certifies whatever it happens to say. The door sweep found this one;
/// `from_json` was correct and was not the only way in.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureIndexFields {
    schema: String,
    corpus: Sha256Digest,
    entries: Vec<IndexEntry>,
}

impl From<FixtureIndexFields> for FixtureIndex {
    fn from(fields: FixtureIndexFields) -> Self {
        Self {
            schema: fields.schema,
            corpus: fields.corpus,
            entries: fields.entries,
        }
    }
}

impl<'de> Deserialize<'de> for FixtureIndex {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let index = Self::from(FixtureIndexFields::deserialize(deserializer)?);
        index.check_corpus().map_err(D::Error::custom)?;
        Ok(index)
    }
}

impl FixtureIndex {
    /// The corpus digest a set of rows produces.
    ///
    /// Domain-separated and length-prefixed, the same construction
    /// `bit_ids::identity` uses for a record identifier: without the lengths,
    /// two different row sets can serialise to one byte string and collide.
    #[must_use]
    pub fn derive_corpus(entries: &[IndexEntry]) -> Sha256Digest {
        let mut buffer = Vec::new();
        buffer.extend_from_slice(INDEX_SCHEMA.as_bytes());
        buffer.push(0x1f);
        for entry in entries {
            let id = entry.id.as_str().as_bytes();
            buffer.extend_from_slice(&(id.len() as u64).to_be_bytes());
            buffer.extend_from_slice(id);
            buffer.extend_from_slice(entry.digest.as_bytes());
        }
        Sha256Digest::of(&buffer)
    }

    /// Builds an index from fixtures, sorted by identifier.
    ///
    /// # Errors
    ///
    /// Returns whatever [`Fixture::digest`] refused.
    pub fn of(fixtures: &[Fixture]) -> Result<Self, FixtureError> {
        let mut entries = fixtures
            .iter()
            .map(|fixture| {
                Ok(IndexEntry {
                    id: fixture.id.clone(),
                    digest: fixture.digest()?,
                })
            })
            .collect::<Result<Vec<_>, FixtureError>>()?;
        entries.sort_by(|left, right| left.id.as_str().cmp(right.id.as_str()));
        Ok(Self {
            schema: INDEX_SCHEMA.to_owned(),
            corpus: Self::derive_corpus(&entries),
            entries,
        })
    }

    /// Reads an index document.
    ///
    /// # Errors
    ///
    /// Returns [`FixtureError::Malformed`] when it is not this schema's shape,
    /// [`FixtureError::UnsupportedSchema`] for another generation, and
    /// [`FixtureError::Invalid`] with `E-IDX-01` when the stored corpus digest
    /// is not the one the rows derive.
    pub fn from_json(document: &str) -> Result<Self, FixtureError> {
        // Through the mirror, so the schema is answered before the digest and
        // the caller gets the code rather than the string serde would report.
        let fields: FixtureIndexFields =
            serde_json::from_str(document).map_err(FixtureError::Malformed)?;
        let index = Self::from(fields);
        if index.schema != INDEX_SCHEMA {
            return Err(FixtureError::UnsupportedSchema {
                found: index.schema,
                expected: INDEX_SCHEMA,
            });
        }
        index
            .check_corpus()
            .map_err(|error| FixtureError::Invalid(vec![error]))?;
        Ok(index)
    }

    /// Whether the stored corpus digest is the one the rows derive.
    ///
    /// # Errors
    ///
    /// Returns `E-IDX-01` when it is not. A row edited by hand moves the derived
    /// digest and is refused rather than believed.
    fn check_corpus(&self) -> Result<(), FixtureViolation> {
        let derived = Self::derive_corpus(&self.entries);
        if derived == self.corpus {
            Ok(())
        } else {
            Err(violation(
                "E-IDX-01",
                format!(
                    "stored corpus {} but the rows derive {derived}",
                    self.corpus
                ),
            ))
        }
    }

    /// Writes the index in the canonical form.
    ///
    /// # Errors
    ///
    /// Returns [`FixtureError::Malformed`] if serialisation fails, which would
    /// mean a `Serialize` implementation in this crate refused its own value.
    pub fn to_json(&self) -> Result<String, FixtureError> {
        let mut out = serde_json::to_string_pretty(self).map_err(FixtureError::Malformed)?;
        out.push('\n');
        Ok(out)
    }
}

/// Every fixture in `directory`, ascending by file name.
///
/// ⛔ **Everything in the directory is either loaded or refused, never
/// ignored.** The index and the README are the two names skipped; anything else
/// that is not a `*.json` fixture is `E-FIX-11`, and so is a subdirectory.
///
/// The door sweep found why that matters: this used to filter for `*.json` and
/// listing is not recursive, so a fixture added under `fixtures/peer/` would
/// have been silently skipped, and nothing would have said so. The index
/// compares against what was loaded, so a fixture that never loads is a fixture
/// that never runs and never fails.
///
/// # Errors
///
/// Returns [`FixtureError::Unreadable`] when the directory cannot be listed,
/// [`FixtureError::Invalid`] with `E-FIX-11` for an entry that is neither, and
/// whatever [`Fixture::from_path`] refused for the first bad fixture.
pub fn load_directory(directory: &Path) -> Result<Vec<(PathBuf, Fixture)>, FixtureError> {
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(directory).map_err(FixtureError::Unreadable)? {
        let path = entry.map_err(FixtureError::Unreadable)?.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if name == INDEX_FILE || name == "README.md" {
            continue;
        }
        if !path.is_file() || path.extension().is_none_or(|suffix| suffix != "json") {
            return Err(FixtureError::Invalid(vec![violation(
                "E-FIX-11",
                format!("{name:?} is not a fixture, and nothing here is ignored"),
            )]));
        }
        paths.push(path);
    }
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let fixture = Fixture::from_path(&path)?;
            Ok((path, fixture))
        })
        .collect()
}

/// The file name the corpus index is stored under.
pub const INDEX_FILE: &str = "index.json";
