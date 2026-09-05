//! Deterministic indexes and latest views, derived from canonical records only.
//!
//! `CORPUS-03` owns this. A consumer wants to go from something it saw on the
//! wire to the build that produced it, and from a target to the newest measured
//! release, without opening every record in the store.
//!
//! ⛔ **These are derived files and never authoritative.** Every row names the
//! record it came from, so a reader who doubts a row can open the record and
//! decide from the measurement. An index that answered a question the records
//! could not is an index that invented one.
//!
//! ⛔ **Nothing here maps a peer-ID prefix to a client name from a table.** The
//! key of a peer-prefix row is the fixed span of a peer ID *this project
//! measured*, and the row resolves to the record that measured it.
//! `docs/architecture.md` section 5 forbids the decoder table, and this is its
//! opposite: the lookup exists because the measurement does.
//!
//! ⚠ **Only publishable records enter a view.** A provisional record belongs in
//! the store and its disagreement is the evidence, but a lookup that answered
//! with one would publish the claim the record model refused to publish.
//! [`build`] reports how many it left out, because a count with no denominator
//! is what makes an omission invisible.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::agreement::publishable;
use crate::canonical::{Instant, RelPath, Sha256Digest, Slug, Version};
use crate::corpus::Corpus;
use crate::identity::RecordId;
use crate::observation::{FieldPath, FieldState, PatternRun};
use crate::record::Profile;
use crate::resolution::VersionScheme;
use crate::validate::{SchemaError, Violations};

/// Identifier carried by every first-generation index document.
pub const INDEX_SCHEMA: &str = "bit-ids/index/1";

/// The field a peer-prefix row is keyed on.
pub const PEER_ID_FIELD: &str = "peer_wire/peer_id";

/// The field a client-string row is keyed on.
pub const BEP10_CLIENT_FIELD: &str = "peer_wire/bep10.client";

/// What one index is a lookup by.
///
/// ⚠ The names are the published spellings and are pinned to their literals by
/// `index_kinds_have_one_spelling`. They appear in a document a consumer parses,
/// so renaming one is a schema change rather than a rename.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum IndexKind {
    /// By catalogue target identifier.
    Target,
    /// By the fixed leading span of a measured peer ID.
    PeerPrefix,
    /// By the measured BEP 10 client string.
    Bep10Client,
    /// By host family.
    Platform,
    /// By the version the installed build reported.
    Version,
    /// By capture instant.
    CapturedAt,
}

impl IndexKind {
    /// Every kind, in the order a document lists them.
    pub const ALL: &'static [Self] = &[
        Self::Target,
        Self::PeerPrefix,
        Self::Bep10Client,
        Self::Platform,
        Self::Version,
        Self::CapturedAt,
    ];

    /// The published spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Target => "target",
            Self::PeerPrefix => "peer_prefix",
            Self::Bep10Client => "bep10_client",
            Self::Platform => "platform",
            Self::Version => "version",
            Self::CapturedAt => "captured_at",
        }
    }
}

/// One lookup row: a key, and the record it resolves to.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct IndexRow {
    /// What a consumer looked up.
    pub key: String,
    /// The record that answers it.
    pub record: RecordId,
    /// Where that record is filed.
    pub path: RelPath,
}

/// One index.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Index {
    /// What it is a lookup by.
    pub kind: IndexKind,
    /// The rows, ascending by key then by record.
    pub rows: Vec<IndexRow>,
}

/// The newest publishable record for one build line.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct LatestRow {
    /// The catalogue target.
    pub target: Slug,
    /// Host family.
    pub platform: Slug,
    /// Machine architecture.
    pub arch: Slug,
    /// Package format.
    pub package: Slug,
    /// The version it selects.
    pub version: Version,
    /// The record that answers.
    pub record: RecordId,
    /// Where that record is filed.
    pub path: RelPath,
}

/// The build line a latest row is the newest of: target, platform, arch and
/// package. It is the identity tuple minus the two things that vary between
/// records of one line, which are the version and the capture.
type BuildLine = (Slug, Slug, Slug, Slug);

/// What decides which record of a build line is the latest: the scheme's
/// numeric ordering first, then the capture instant, then the record
/// identifier. Total by construction, so the answer does not depend on the
/// order the store was read in.
type Ranking = (Vec<u64>, Instant, RecordId);

/// Everything a build produces, and what it left out.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Indexes {
    /// One per [`IndexKind`], in `IndexKind::ALL` order.
    pub indexes: Vec<Index>,
    /// The latest view, ascending by build line.
    pub latest: Vec<LatestRow>,
    /// How many records were in the store and not in any view.
    ///
    /// ⚠ Reported rather than inferred. A consumer comparing a row count with a
    /// store's record count would otherwise read an exclusion as a defect.
    pub excluded: usize,
}

/// The leading fixed span of a measured peer ID, as a lookup key.
///
/// ⚠ A patterned value whose first run varies has no usable prefix and produces
/// no row, rather than a row keyed on nothing. `unknown`, `not_observed`,
/// `not_supported` and `variable` produce none either: a lookup key has to be
/// something a build was measured emitting.
fn peer_prefix(state: &FieldState) -> Option<String> {
    match state {
        FieldState::Constant(value) => Some(value.value.to_hex()),
        FieldState::Patterned(value) => match value.pattern.runs.first()? {
            PatternRun::Fixed { bytes } => Some(bytes.to_hex()),
            PatternRun::Varying { .. } => None,
        },
        FieldState::Unknown
        | FieldState::NotObserved
        | FieldState::NotSupported
        | FieldState::Variable(_) => None,
    }
}

/// The measured client string, as a lookup key.
///
/// ⚠ Constant only. A BEP 10 `v` string that varies between samples carries a
/// build number or a session token, and a prefix of one is a guess about which
/// half is which. `OBS-07`'s stock-client controls are what would settle that.
fn client_string(state: &FieldState) -> Option<String> {
    match state {
        FieldState::Constant(value) => Some(value.value.to_hex()),
        FieldState::Unknown
        | FieldState::NotObserved
        | FieldState::NotSupported
        | FieldState::Patterned(_)
        | FieldState::Variable(_) => None,
    }
}

fn field<'a>(profile: &'a Profile, path: &str) -> Option<&'a FieldState> {
    let path = FieldPath::parse(path).ok()?;
    profile.field(&path).map(|observed| &observed.state)
}

/// Builds every index and the latest view over a store.
///
/// `schemes` says how each target spells its versions, which is what makes a
/// latest view orderable at all. A target with no scheme, or whose selected
/// records carry a version the scheme cannot order, is **left out of the latest
/// view with a refusal** rather than ordered by a guess.
///
/// # Errors
///
/// | code | refused |
/// | --- | --- |
/// | `E-VIW-01` | a publishable record whose target declares no version scheme |
/// | `E-VIW-02` | a publishable record whose version its own scheme cannot order |
///
/// ⚠ Both are refusals about the **latest view only**. The lookup indexes carry
/// the record regardless, because finding a measurement does not require
/// ordering it.
pub fn build(
    corpus: &Corpus,
    schemes: &BTreeMap<Slug, VersionScheme>,
) -> Result<Indexes, Violations> {
    let mut errors = Vec::new();
    let mut rows: BTreeMap<IndexKind, Vec<IndexRow>> = BTreeMap::new();
    let mut best: BTreeMap<BuildLine, (Ranking, LatestRow)> = BTreeMap::new();
    let mut excluded = 0_usize;

    for stored in corpus.profiles() {
        let profile = &stored.profile;
        if publishable(profile).is_err() {
            excluded += 1;
            continue;
        }
        let mut push = |kind: IndexKind, key: String| {
            rows.entry(kind).or_default().push(IndexRow {
                key,
                record: profile.id,
                path: stored.path.clone(),
            });
        };

        push(IndexKind::Target, profile.target.id.to_string());
        push(IndexKind::Platform, profile.build.platform.to_string());
        push(IndexKind::Version, profile.build.version.to_string());
        push(
            IndexKind::CapturedAt,
            profile.capture.captured_at.to_string(),
        );
        if let Some(key) = field(profile, PEER_ID_FIELD).and_then(peer_prefix) {
            push(IndexKind::PeerPrefix, key);
        }
        if let Some(key) = field(profile, BEP10_CLIENT_FIELD).and_then(client_string) {
            push(IndexKind::Bep10Client, key);
        }

        // ⛔ The latest view fails closed. An unorderable version blocks the
        // build line rather than being skipped, because a skip yields an older
        // record selected confidently, which is `ACQ-02`'s rule about
        // resolution applied to publication.
        let Some(scheme) = schemes.get(&profile.target.id) else {
            errors.push(SchemaError::new(
                "E-VIW-01",
                stored.path.as_str(),
                format!(
                    "target {} declares no version scheme, so no latest view can order it",
                    profile.target.id
                ),
            ));
            continue;
        };
        let Some(order) = scheme.components(profile.build.version.as_str()) else {
            errors.push(SchemaError::new(
                "E-VIW-02",
                stored.path.as_str(),
                format!(
                    "version {} is not orderable under target {}'s scheme",
                    profile.build.version, profile.target.id
                ),
            ));
            continue;
        };

        let line: BuildLine = (
            profile.target.id.clone(),
            profile.build.platform.clone(),
            profile.build.arch.clone(),
            profile.build.package.clone(),
        );
        let row = LatestRow {
            target: profile.target.id.clone(),
            platform: profile.build.platform.clone(),
            arch: profile.build.arch.clone(),
            package: profile.build.package.clone(),
            version: profile.build.version.clone(),
            record: profile.id,
            path: stored.path.clone(),
        };
        // ⛔ ONE TOTAL KEY, and the version is not in it. `Version` is `Ord` as
        // text, so `1.2.9` compares greater than `1.2.10` and a comparison that
        // mentions it at all silently loses the numeric ordering the scheme
        // computed. That is not hypothetical: it was written that way here and
        // `the_latest_view_names_the_newest_record_per_build_line` caught the
        // view answering `1.2.9`.
        //
        // ⚠ Two captures of one build tie on the ordering, so the instant and
        // then the record identifier break it. Neither is a preference: the rule
        // has to be total, or two runs of this over one store produce two
        // different documents.
        let ranking = (order, profile.capture.captured_at.clone(), profile.id);
        match best.get(&line) {
            Some((have, _)) if *have >= ranking => {}
            _ => {
                best.insert(line, (ranking, row));
            }
        }
    }

    if !errors.is_empty() {
        return Violations::from_errors(errors).map(|()| unreachable!());
    }

    let mut indexes = Vec::with_capacity(IndexKind::ALL.len());
    for kind in IndexKind::ALL {
        let mut sorted = rows.remove(kind).unwrap_or_default();
        // ⛔ Sorted here and never relied on from the store's iteration order.
        // Two clean builds have to produce identical bytes, and a map's order is
        // not a contract a consumer can check.
        sorted.sort();
        sorted.dedup();
        indexes.push(Index {
            kind: *kind,
            rows: sorted,
        });
    }
    let mut latest: Vec<LatestRow> = best.into_values().map(|(_, row)| row).collect();
    // ⚠ NOT REFUTED, AND KEPT. `best` is a `BTreeMap` keyed by the build line,
    // and `LatestRow` orders on those same four fields first, so removing this
    // changes no output that can be produced today. It would matter the moment
    // `best` stopped being an ordered map or the row's ordering stopped
    // agreeing with the key's, and a reader should not take an unreached guard
    // for a proven one.
    latest.sort();

    Ok(Indexes {
        indexes,
        latest,
        excluded,
    })
}

impl Indexes {
    /// The published document.
    ///
    /// ⛔ **Written by hand, and no clock is read.** The field order, the
    /// spacing and the escaping are the bytes a digest names, so a derive that
    /// changed any of them between versions would move every digest a consumer
    /// had recorded. `two_builds_are_byte_identical` is the check.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push_str("{\n  \"schema\": \"");
        out.push_str(INDEX_SCHEMA);
        out.push_str("\",\n  \"excluded\": ");
        let _ = write!(out, "{}", self.excluded);
        out.push_str(",\n  \"indexes\": [\n");
        for (index, entry) in self.indexes.iter().enumerate() {
            out.push_str("    {\n      \"kind\": \"");
            out.push_str(entry.kind.as_str());
            out.push_str("\",\n      \"rows\": [\n");
            for (row_index, row) in entry.rows.iter().enumerate() {
                out.push_str("        {\"key\": \"");
                out.push_str(&row.key);
                out.push_str("\", \"record\": \"");
                let _ = write!(out, "{}", row.record);
                out.push_str("\", \"path\": \"");
                out.push_str(row.path.as_str());
                out.push_str("\"}");
                if row_index + 1 < entry.rows.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str("      ]\n    }");
            if index + 1 < self.indexes.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  ],\n  \"latest\": [\n");
        for (index, row) in self.latest.iter().enumerate() {
            out.push_str("    {\"target\": \"");
            out.push_str(row.target.as_str());
            out.push_str("\", \"platform\": \"");
            out.push_str(row.platform.as_str());
            out.push_str("\", \"arch\": \"");
            out.push_str(row.arch.as_str());
            out.push_str("\", \"package\": \"");
            out.push_str(row.package.as_str());
            out.push_str("\", \"version\": \"");
            out.push_str(row.version.as_str());
            out.push_str("\", \"record\": \"");
            let _ = write!(out, "{}", row.record);
            out.push_str("\", \"path\": \"");
            out.push_str(row.path.as_str());
            out.push_str("\"}");
            if index + 1 < self.latest.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  ]\n}\n");
        out
    }

    /// The digest of the published document.
    #[must_use]
    pub fn digest(&self) -> Sha256Digest {
        Sha256Digest::of(self.to_json().as_bytes())
    }

    /// How many rows the whole set carries, latest included.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.indexes
            .iter()
            .map(|index| index.rows.len())
            .sum::<usize>()
            + self.latest.len()
    }
}

/// Whether every row resolves to exactly one record the store carries.
///
/// ⛔ **This is the half a derived file cannot be trusted about.** An index is
/// built from records and is then read *instead of* them, so a row naming a
/// record nobody can open is an answer with nothing behind it. It is checked
/// against the corpus rather than against the builder that produced it, because
/// a builder checked against itself agrees with itself.
///
/// # Errors
///
/// | code | refused |
/// | --- | --- |
/// | `E-VIW-10` | a row naming a record the store does not carry |
/// | `E-VIW-11` | a row whose path is not where that record is filed |
pub fn rows_resolve(indexes: &Indexes, corpus: &Corpus) -> Result<(), Violations> {
    let mut known: BTreeMap<RecordId, &RelPath> = BTreeMap::new();
    for stored in corpus.profiles() {
        known.insert(stored.profile.id, &stored.path);
    }

    let mut errors = Vec::new();
    let mut check = |at: String, record: RecordId, path: &RelPath| match known.get(&record) {
        None => errors.push(SchemaError::new(
            "E-VIW-10",
            at,
            format!("names {record}, which this store does not carry"),
        )),
        Some(filed) if *filed != path => errors.push(SchemaError::new(
            "E-VIW-11",
            at,
            format!("says {path}, the store files that record at {filed}"),
        )),
        Some(_) => {}
    };

    for index in &indexes.indexes {
        for row in &index.rows {
            check(
                format!("{}[{}]", index.kind.as_str(), row.key),
                row.record,
                &row.path,
            );
        }
    }
    for row in &indexes.latest {
        check(
            format!("latest[{} {} {}]", row.target, row.platform, row.package),
            row.record,
            &row.path,
        );
    }

    Violations::from_errors(errors)
}

#[cfg(test)]
mod tests {
    use super::{BEP10_CLIENT_FIELD, INDEX_SCHEMA, IndexKind, PEER_ID_FIELD, build, rows_resolve};
    use crate::canonical::{RelPath, Sha256Digest, Slug, Version};
    use crate::corpus::Corpus;
    use crate::identity::{RecordId, RecordKey};
    use crate::resolution::VersionScheme;
    use crate::store::{Entry, ObjectRef, StoreKey, StoreTree};
    use crate::{Profile, RunManifest};
    use std::collections::BTreeMap;

    const PROFILE: &str = include_str!("../tests/fixtures/valid-profile.json");
    const MANIFEST: &str = include_str!("../tests/fixtures/valid-manifest.json");

    fn profile() -> Profile {
        Profile::from_json(PROFILE).expect("the fixture record validates")
    }

    fn slug(text: &str) -> Slug {
        Slug::parse(text).expect("a canonical slug")
    }

    /// Three dotted numbers, which is how most of the catalogue spells a
    /// version.
    fn schemes() -> BTreeMap<Slug, VersionScheme> {
        BTreeMap::from([(
            slug("fixture-client"),
            VersionScheme {
                tag_prefix: None,
                min_components: 3,
                max_components: 3,
            },
        )])
    }

    /// The fixture record, re-versioned and re-identified.
    ///
    /// ⛔ **The fixture's own version is `0.0.0-fixture`, which no numeric
    /// scheme can order**, so it proves the fail-closed path and cannot prove
    /// the ordering one. A record at a real version is built here instead, with
    /// every route's reported version moved with it, because `E-ACQ-04` refuses
    /// a route that installed a version the record does not declare, and the
    /// identifier re-derived, because it digests the version.
    fn record_at(version_text: &str) -> Profile {
        let mut record = profile();
        let version = Version::parse(version_text).expect("a reported version");
        record.build.version = version.clone();
        for route in &mut record.acquisition {
            route.installed_version = version.clone();
            route.resolved_version = version.clone();
        }
        record.capture.id = Slug::parse(&format!("cap-{}", version_text.replace('.', "-")))
            .expect("a canonical slug");
        record.id = RecordId::derive(&RecordKey {
            schema: &record.schema,
            target: &record.target.id,
            version: &record.build.version,
            platform: &record.build.platform,
            arch: &record.build.arch,
            package: &record.build.package,
            capture: &record.capture.id,
        });
        crate::validate::validate(&record).expect("a re-versioned record still validates");
        record
    }

    /// A store carrying one record per listed version, on one build line.
    fn corpus_at(versions: &[&str]) -> Corpus {
        let mut tree = StoreTree::new();
        let mut records = Vec::new();
        for text in versions {
            let record = record_at(text);
            let path = StoreKey::of_profile(&record)
                .profile_path()
                .expect("a publishable path");
            tree.insert(
                path.clone(),
                Entry::Object(ObjectRef {
                    bytes: PROFILE.len() as u64,
                    sha256: Sha256Digest::of(PROFILE.as_bytes()),
                }),
            );
            records.push((path, record));
        }
        let mut corpus = Corpus::new(tree);
        for (path, record) in records {
            corpus.insert_profile(path, record);
        }
        corpus
    }

    fn corpus() -> Corpus {
        corpus_at(&["1.2.3"])
    }

    /// The store as the fixtures ship it, whose version no scheme orders.
    fn fixture_corpus() -> Corpus {
        let record = profile();
        let manifest = RunManifest::from_json(MANIFEST).expect("the fixture manifest validates");
        let key = StoreKey::of_profile(&record);
        let profile_path = key.profile_path().expect("a publishable path");
        let manifest_path = key.manifest_path().expect("a publishable path");
        let mut tree = StoreTree::new();
        tree.insert(
            profile_path.clone(),
            Entry::Object(ObjectRef {
                bytes: PROFILE.len() as u64,
                sha256: Sha256Digest::of(PROFILE.as_bytes()),
            }),
        );
        let mut corpus = Corpus::new(tree);
        corpus.insert_profile(profile_path, record);
        corpus.insert_manifest(manifest_path, manifest);
        corpus
    }

    /// ⛔ Literals, not the constants that spell them. These strings are in a
    /// document a consumer parses, so a rename is a schema change.
    #[test]
    fn index_kinds_have_one_spelling() {
        assert_eq!(INDEX_SCHEMA, "bit-ids/index/1");
        assert_eq!(PEER_ID_FIELD, "peer_wire/peer_id");
        assert_eq!(BEP10_CLIENT_FIELD, "peer_wire/bep10.client");
        let spelled: Vec<&str> = IndexKind::ALL.iter().map(|kind| kind.as_str()).collect();
        assert_eq!(
            spelled,
            [
                "target",
                "peer_prefix",
                "bep10_client",
                "platform",
                "version",
                "captured_at"
            ]
        );
    }

    /// ⛔ The Prove's first half, and it is a real question rather than a
    /// tautology: the builder walks maps, and a map's iteration order is not a
    /// contract a consumer can check.
    #[test]
    fn two_builds_are_byte_identical() {
        let first = build(&corpus(), &schemes()).expect("a publishable store");
        let second = build(&corpus(), &schemes()).expect("a publishable store");
        assert_eq!(first.to_json(), second.to_json());
        assert_eq!(first.digest(), second.digest());
        assert!(first.rows() > 0, "an empty index proves nothing");
    }

    /// ⛔ **Determinism against the insertion order, not just against a second
    /// identical run.** Two builds over one corpus agreed even with the sort
    /// removed, because the corpus was read in one order both times: the sort
    /// was load-bearing and nothing could see it. Found by planting against it.
    #[test]
    fn the_document_does_not_depend_on_the_order_records_were_read_in() {
        let forward = corpus_at(&["1.2.3", "1.2.10", "1.2.9"]);
        let backward = corpus_at(&["1.2.9", "1.2.10", "1.2.3"]);
        let first = build(&forward, &schemes()).expect("a publishable store");
        let second = build(&backward, &schemes()).expect("a publishable store");
        assert_eq!(first.to_json(), second.to_json());
        assert!(first.rows() >= 18, "an empty document proves nothing");
    }

    /// ⛔ A provisional record is in the store and in no view. Nothing tested
    /// this until a plant removed the filter and every test stayed green.
    #[test]
    fn a_provisional_record_is_left_out_of_every_view() {
        let mut record = record_at("1.2.3");
        // A disagreement is what makes a record provisional, and the record
        // model keeps it on purpose: `validate` accepts it and `publishable`
        // refuses it.
        // ⚠ The observations have to actually differ. `E-COR-14` refuses a
        // claimed disagreement over observations that are all equal, which is
        // the schema keeping a conflict honest rather than an obstacle here.
        let entry = record
            .corroboration
            .first_mut()
            .expect("the fixture record carries corroboration");
        entry.observations[0].seen = crate::agreement::SeenValue::Bytes(
            crate::canonical::HexBytes::parse("00112233").expect("observed bytes"),
        );
        entry.agreement = crate::Agreement::Disagrees;
        entry.conflict = Some(
            crate::canonical::Label::parse("the two connectors read different bytes")
                .expect("a recorded fact"),
        );
        crate::validate::validate(&record).expect("a record carrying a conflict still validates");
        assert!(
            crate::agreement::publishable(&record).is_err(),
            "a disagreement is not publishable"
        );

        let path = StoreKey::of_profile(&record)
            .profile_path()
            .expect("a publishable path");
        let mut corpus = Corpus::new(StoreTree::new());
        corpus.insert_profile(path, record);

        let indexes = build(&corpus, &schemes()).expect("a store of one provisional record");
        assert_eq!(indexes.excluded, 1);
        assert_eq!(indexes.latest.len(), 0, "no latest row");
        assert_eq!(indexes.rows(), 0, "and no lookup row either");
    }

    /// ⚠ A peer ID whose first span varies has no prefix to key on, and a row
    /// keyed on an empty string would answer every lookup. The fixture's own
    /// peer ID begins with a fixed run, so this builds one that does not.
    #[test]
    fn a_peer_id_whose_first_span_varies_has_no_prefix_row() {
        use crate::observation::{BytePattern, FieldState, PatternRun, PatternedValue};
        use core::num::NonZeroU32;

        let mut record = record_at("1.2.3");
        let path = crate::observation::FieldPath::parse(PEER_ID_FIELD).expect("a field path");
        let observed = record
            .observations
            .iter_mut()
            .find(|field| field.path == path)
            .expect("the fixture record measures a peer id");
        observed.state = FieldState::Patterned(PatternedValue {
            pattern: BytePattern {
                length: 20,
                runs: vec![
                    PatternRun::Varying {
                        length: 12,
                        alphabet: None,
                    },
                    PatternRun::Fixed {
                        bytes: crate::canonical::HexBytes::parse("2d5858303030302d")
                            .expect("observed bytes"),
                    },
                ],
            },
            samples: NonZeroU32::new(8).expect("a sample count"),
        });
        crate::validate::validate(&record).expect("a varying-first pattern is a valid record");

        let path = StoreKey::of_profile(&record)
            .profile_path()
            .expect("a publishable path");
        let mut corpus = Corpus::new(StoreTree::new());
        corpus.insert_profile(path, record);

        let indexes = build(&corpus, &schemes()).expect("a publishable store");
        let prefixes = indexes
            .indexes
            .iter()
            .find(|index| index.kind == IndexKind::PeerPrefix)
            .expect("a peer prefix index");
        assert!(
            prefixes.rows.is_empty(),
            "a varying first span is not a prefix: {:?}",
            prefixes.rows
        );
        // The record is still findable by everything it does declare.
        let targets = indexes
            .indexes
            .iter()
            .find(|index| index.kind == IndexKind::Target)
            .expect("a target index");
        assert_eq!(targets.rows.len(), 1);
    }

    /// ⛔ The Prove's second half, checked against the store rather than against
    /// the builder that produced the rows.
    #[test]
    fn every_row_resolves_to_one_record() {
        let corpus = corpus();
        let indexes = build(&corpus, &schemes()).expect("a publishable store");
        rows_resolve(&indexes, &corpus).expect("every row names a record the store carries");

        // A row pointing at a store that does not carry it is refused, so the
        // check above is not vacuous.
        let empty = Corpus::new(StoreTree::new());
        let violations =
            rows_resolve(&indexes, &empty).expect_err("no record resolves in an empty store");
        assert!(violations.has("E-VIW-10"), "{violations}");
    }

    #[test]
    fn a_row_filed_elsewhere_is_refused() {
        let corpus = corpus();
        let mut indexes = build(&corpus, &schemes()).expect("a publishable store");
        indexes.indexes[0].rows[0].path =
            RelPath::parse("profiles/v1/elsewhere.json").expect("a canonical path");
        let violations = rows_resolve(&indexes, &corpus).expect_err("a row filed elsewhere");
        assert!(violations.has("E-VIW-11"), "{violations}");
    }

    /// ⭐ The peer prefix is the fixed span of a measured value, which is the
    /// whole product: a consumer sees `-XX0000-` on the wire and this says which
    /// build emitted it. The fixture's peer ID is patterned with that prefix.
    #[test]
    fn the_peer_prefix_row_is_the_measured_fixed_span() {
        let indexes = build(&corpus(), &schemes()).expect("a publishable store");
        let prefixes = indexes
            .indexes
            .iter()
            .find(|index| index.kind == IndexKind::PeerPrefix)
            .expect("a peer prefix index");
        assert_eq!(prefixes.rows.len(), 1);
        // `-XX0000-`, the fixed run of the fixture's patterned peer ID.
        assert_eq!(prefixes.rows[0].key, "2d5858303030302d");
    }

    /// ⛔ Ordered numerically, not as text. `1.2.10` sorts before `1.2.9` as a
    /// string, and a latest view that answered `1.2.9` would point a consumer at
    /// a superseded build with full confidence.
    #[test]
    fn the_latest_view_names_the_newest_record_per_build_line() {
        let corpus = corpus_at(&["1.2.3", "1.2.10", "1.2.9"]);
        let indexes = build(&corpus, &schemes()).expect("a publishable store");
        assert_eq!(indexes.latest.len(), 1, "one build line");
        assert_eq!(indexes.latest[0].target.as_str(), "fixture-client");
        assert_eq!(indexes.latest[0].version.as_str(), "1.2.10");
        assert_eq!(indexes.excluded, 0);

        // Every record still appears in the lookup indexes: finding a
        // measurement does not require it to be the newest one.
        let versions = indexes
            .indexes
            .iter()
            .find(|index| index.kind == IndexKind::Version)
            .expect("a version index");
        assert_eq!(versions.rows.len(), 3);
    }

    /// ⛔ Fails closed. An unorderable version blocks the view rather than being
    /// skipped, because a skip yields an older record selected confidently.
    #[test]
    fn an_unorderable_version_blocks_the_latest_view() {
        let strict = BTreeMap::from([(
            slug("fixture-client"),
            VersionScheme {
                tag_prefix: None,
                min_components: 4,
                max_components: 4,
            },
        )]);
        let violations = build(&corpus(), &strict).expect_err("1.2.3 is not four numbers");
        assert!(violations.has("E-VIW-02"), "{violations}");

        let none = BTreeMap::new();
        let violations = build(&corpus(), &none).expect_err("no scheme, no ordering");
        assert!(violations.has("E-VIW-01"), "{violations}");

        // ⚠ And the shipped fixture is itself unorderable, which is why the
        // cases above build their own records rather than using it.
        let violations =
            build(&fixture_corpus(), &schemes()).expect_err("0.0.0-fixture is not three numbers");
        assert!(violations.has("E-VIW-02"), "{violations}");
    }
}
