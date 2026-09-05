//! The artifact cache: what it knows about a downloaded artifact, and what it
//! is allowed to keep.
//!
//! `ACQ-05` owns this. Upstream URLs and package indexes move, so an artifact
//! this project measured last month may not be retrievable from where it came
//! from. Storing the bytes is the obvious answer and is usually not this
//! project's to give: an installer is somebody else's to distribute.
//!
//! ⛔ **THE IDENTITY IS THE DIGEST AND NEVER THE LOCATION.** A URL is where an
//! artifact was found, which is a fact about a moment; the digest is what it
//! was. So a second retrieval of the same bytes from a new location adds a
//! retrieval to the artifact already known, and never a second artifact. That
//! is the whole of "reproduce artifact identity after a source URL change":
//! nothing has to be reproduced, because nothing that identifies the artifact
//! changed.
//!
//! ⛔ **STORING BYTES IS A PERMISSION, NOT A CAPABILITY.** `FOUND-04`'s register
//! carries a disposition per target and every row in it says refused, so today
//! this cache never stores an artifact. It keeps the digest, the size, the
//! signature status and every place the bytes were retrieved from, which is
//! what makes the acquisition replayable without redistributing anything.
//!
//! ⚠ **The disposition is passed in and never read from a file here.** The rules
//! in this crate are pure over data a caller has already read, the way the store
//! rules are, and `catalogue/licences.toml` has exactly one parser:
//! `check-licences`. A second one here would be two answers to what the register
//! permits.

use std::collections::BTreeMap;

use crate::acquisition::{RouteKind, SignatureStatus};
use crate::canonical::{Instant, Sha256Digest, Slug, Url, Version};
use crate::validate::{SchemaError, Violations};

/// Identifier carried by a first-generation cache document.
pub const CACHE_SCHEMA: &str = "bit-ids/cache/1";

/// What the licence register permits for one target's artifacts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disposition {
    /// The bytes are never kept. Everything else about the artifact is.
    Refused,
    /// The bytes may be kept, which needs a licence somebody established.
    Permitted,
}

/// One retrieval of one artifact: when, by what kind of route, from where.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Retrieval {
    /// When the bytes were fetched, UTC.
    pub at: Instant,
    /// Which delivery mechanism answered.
    pub route: RouteKind,
    /// Where a reader could go at that instant.
    pub origin: Url,
}

/// One artifact the cache knows about.
///
/// ⚠ `stored` is a claim about this repository, not about the artifact. It says
/// the bytes are kept, and it is the one field a licence can forbid.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CachedArtifact {
    /// The catalogue target the artifact belongs to.
    pub target: Slug,
    /// The version it was published as.
    pub version: Version,
    /// What the bytes are. This is the identity.
    pub sha256: Sha256Digest,
    /// How many bytes, which a digest alone does not tell a reader.
    pub bytes: u64,
    /// What was done about the signature.
    pub signature: SignatureStatus,
    /// Whether this repository keeps the bytes.
    pub stored: bool,
    /// Every place the bytes were retrieved from, ascending and unique.
    pub retrievals: Vec<Retrieval>,
}

/// Everything the cache knows, ascending by digest.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Cache {
    /// The artifacts, ascending by digest and unique on it.
    pub artifacts: Vec<CachedArtifact>,
}

/// What observing an artifact did to the cache.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Observation {
    /// The digest was not known and the artifact was added.
    Added,
    /// The digest was known and this retrieval was added to it.
    Retrieved,
    /// The digest was known and this retrieval was already recorded.
    Unchanged,
}

impl Cache {
    /// The artifact these bytes are, if the cache has seen them.
    ///
    /// ⛔ **By digest, because that is the identity.** A lookup by URL would
    /// stop answering the moment a vendor reorganised a download page, which is
    /// the failure this cache exists to survive.
    #[must_use]
    pub fn resolve(&self, sha256: &Sha256Digest) -> Option<&CachedArtifact> {
        self.artifacts
            .iter()
            .find(|artifact| &artifact.sha256 == sha256)
    }

    /// Records one retrieval, merging it into the artifact the digest names.
    ///
    /// # Errors
    ///
    /// | code | refused |
    /// | --- | --- |
    /// | `E-CAC-10` | one digest presented as two different artifacts |
    ///
    /// ⛔ **A digest names bytes, so two artifacts that share one and disagree
    /// about anything else cannot both be right.** Merging them would let a
    /// wrong size or a wrong target arrive through a second retrieval and
    /// overwrite a measurement; keeping both would put one digest in the cache
    /// twice. Refusing is the only answer that loses nothing.
    pub fn observe(
        &mut self,
        artifact: CachedArtifact,
        retrieval: Retrieval,
    ) -> Result<Observation, Violations> {
        let position = self
            .artifacts
            .iter()
            .position(|known| known.sha256 == artifact.sha256);
        let Some(position) = position else {
            let mut added = artifact;
            added.retrievals = vec![retrieval];
            self.artifacts.push(added);
            self.artifacts.sort_by_key(|known| known.sha256);
            return Ok(Observation::Added);
        };

        let known = &self.artifacts[position];
        let mut disagreements = Vec::new();
        if known.target != artifact.target {
            disagreements.push(format!("target {} and {}", known.target, artifact.target));
        }
        if known.version != artifact.version {
            disagreements.push(format!(
                "version {} and {}",
                known.version, artifact.version
            ));
        }
        if known.bytes != artifact.bytes {
            disagreements.push(format!("{} bytes and {}", known.bytes, artifact.bytes));
        }
        if !disagreements.is_empty() {
            return Violations::from_errors(vec![SchemaError::new(
                "E-CAC-10",
                artifact.sha256.to_string(),
                format!(
                    "one digest presented as two artifacts: {}",
                    disagreements.join(", ")
                ),
            )])
            .map(|()| unreachable!());
        }

        let known = &mut self.artifacts[position];
        if known.retrievals.contains(&retrieval) {
            return Ok(Observation::Unchanged);
        }
        known.retrievals.push(retrieval);
        known.retrievals.sort();
        Ok(Observation::Retrieved)
    }
}

/// Whether the cache is one this repository is allowed to have.
///
/// `register` is the disposition per target, as `check-licences` reports it.
///
/// # Errors
///
/// | code | refused |
/// | --- | --- |
/// | `E-CAC-01` | stored bytes for a target whose disposition refuses them |
/// | `E-CAC-02` | an artifact whose target the register does not carry |
/// | `E-CAC-03` | an artifact with no retrieval at all |
/// | `E-CAC-04` | retrievals out of order or repeated |
/// | `E-CAC-05` | a zero-length artifact |
/// | `E-CAC-06` | artifacts out of order or one digest twice |
pub fn validate_cache(
    cache: &Cache,
    register: &BTreeMap<Slug, Disposition>,
) -> Result<(), Violations> {
    let mut errors = Vec::new();

    for (index, artifact) in cache.artifacts.iter().enumerate() {
        let at = artifact.sha256.to_string();

        // ⛔ THE RULE THIS MODULE EXISTS FOR. Everything else here is hygiene;
        // this is the one that keeps somebody else's installer out of this
        // repository, and it fails closed: a target the register does not
        // mention is refused rather than defaulted to permitted.
        match register.get(&artifact.target) {
            None => errors.push(SchemaError::new(
                "E-CAC-02",
                &at,
                format!(
                    "target {} is not in the licence register, so nothing permits keeping its \
                     bytes",
                    artifact.target
                ),
            )),
            Some(Disposition::Refused) if artifact.stored => errors.push(SchemaError::new(
                "E-CAC-01",
                &at,
                format!(
                    "the bytes are kept and {}'s disposition is refused",
                    artifact.target
                ),
            )),
            Some(_) => {}
        }

        if artifact.retrievals.is_empty() {
            errors.push(SchemaError::new(
                "E-CAC-03",
                &at,
                "no retrieval, so nothing says where these bytes came from",
            ));
        }
        for window in artifact.retrievals.windows(2) {
            if window[0] >= window[1] {
                errors.push(SchemaError::new(
                    "E-CAC-04",
                    &at,
                    "the retrievals are not ascending and unique",
                ));
                break;
            }
        }
        if artifact.bytes == 0 {
            errors.push(SchemaError::new(
                "E-CAC-05",
                &at,
                "a zero-length artifact is not an artifact",
            ));
        }
        if index > 0 && cache.artifacts[index - 1].sha256 >= artifact.sha256 {
            errors.push(SchemaError::new(
                "E-CAC-06",
                &at,
                "the artifacts are not ascending and unique by digest",
            ));
        }
    }

    Violations::from_errors(errors)
}

#[cfg(test)]
mod tests {
    use super::{
        CACHE_SCHEMA, Cache, CachedArtifact, Disposition, Observation, Retrieval, validate_cache,
    };
    use crate::acquisition::{RouteKind, SignatureStatus};
    use crate::canonical::{Instant, Sha256Digest, Slug, Url, Version};
    use std::collections::BTreeMap;

    fn slug(text: &str) -> Slug {
        Slug::parse(text).expect("a canonical slug")
    }

    fn digest(seed: &str) -> Sha256Digest {
        Sha256Digest::of(seed.as_bytes())
    }

    fn retrieval(url: &str, at: &str) -> Retrieval {
        Retrieval {
            at: Instant::parse(at).expect("a canonical instant"),
            route: RouteKind::GithubRelease,
            origin: Url::parse(url).expect("a retrieval location"),
        }
    }

    fn artifact(seed: &str) -> CachedArtifact {
        CachedArtifact {
            target: slug("aria2"),
            version: Version::parse("1.37.0").expect("a reported version"),
            sha256: digest(seed),
            bytes: 4096,
            signature: SignatureStatus::Unsigned,
            stored: false,
            retrievals: Vec::new(),
        }
    }

    fn refused() -> BTreeMap<Slug, Disposition> {
        BTreeMap::from([(slug("aria2"), Disposition::Refused)])
    }

    #[test]
    fn the_schema_has_one_spelling() {
        assert_eq!(CACHE_SCHEMA, "bit-ids/cache/1");
    }

    /// ⛔ THE ENTRY'S OWN PROVE. The source moved and the artifact did not: the
    /// digest still names it, the cache holds one artifact, and the new location
    /// is recorded beside the old rather than replacing it.
    #[test]
    fn a_source_url_change_adds_a_retrieval_and_not_an_artifact() {
        let mut cache = Cache::default();
        let first = retrieval(
            "https://example.invalid/old/aria2.tar.gz",
            "2026-01-01T00:00:00Z",
        );
        let second = retrieval(
            "https://example.invalid/new/aria2.tar.gz",
            "2026-06-01T00:00:00Z",
        );

        assert_eq!(
            cache
                .observe(artifact("bytes"), first.clone())
                .expect("a new artifact"),
            Observation::Added
        );
        assert_eq!(
            cache
                .observe(artifact("bytes"), second.clone())
                .expect("the same bytes"),
            Observation::Retrieved
        );

        assert_eq!(cache.artifacts.len(), 1, "one digest, one artifact");
        let known = cache
            .resolve(&digest("bytes"))
            .expect("the digest still names it");
        assert_eq!(known.retrievals, vec![first.clone(), second]);

        // ⚠ And the same retrieval twice is not a second retrieval. A cache that
        // grew a row every time somebody re-ran an acquisition would report a
        // popularity contest rather than a provenance.
        assert_eq!(
            cache
                .observe(artifact("bytes"), first)
                .expect("the same retrieval"),
            Observation::Unchanged
        );
        assert_eq!(cache.artifacts[0].retrievals.len(), 2);
        validate_cache(&cache, &refused()).expect("a cache that keeps no bytes");
    }

    /// ⛔ A digest names bytes, so two artifacts sharing one and disagreeing
    /// about anything else cannot both be right.
    #[test]
    fn one_digest_presented_as_two_artifacts_is_refused() {
        let mut cache = Cache::default();
        cache
            .observe(
                artifact("bytes"),
                retrieval("https://example.invalid/a", "2026-01-01T00:00:00Z"),
            )
            .expect("a new artifact");
        let mut different = artifact("bytes");
        different.bytes = 8192;
        let violations = cache
            .observe(
                different,
                retrieval("https://example.invalid/b", "2026-02-01T00:00:00Z"),
            )
            .expect_err("the sizes disagree");
        assert!(violations.has("E-CAC-10"), "{violations}");
        assert_eq!(cache.artifacts.len(), 1, "and nothing was merged in");
    }

    /// ⛔ THE POLICY RULE. Keeping the bytes is what a licence forbids, and the
    /// register says refused for every target today.
    #[test]
    fn kept_bytes_are_refused_when_the_register_refuses_them() {
        let mut cache = Cache::default();
        cache
            .observe(
                artifact("bytes"),
                retrieval("https://example.invalid/a", "2026-01-01T00:00:00Z"),
            )
            .expect("a new artifact");
        cache.artifacts[0].stored = true;

        let violations = validate_cache(&cache, &refused()).expect_err("the bytes may not be kept");
        assert!(violations.has("E-CAC-01"), "{violations}");

        // ⚠ And the same cache is fine the moment the register permits it, so
        // the rule is the register's answer rather than a refusal to store at
        // all. Nothing in the register says permitted today.
        let permitted = BTreeMap::from([(slug("aria2"), Disposition::Permitted)]);
        validate_cache(&cache, &permitted).expect("permitted bytes may be kept");
    }

    /// ⛔ FAILS CLOSED. A target nobody has recorded a disposition for is
    /// refused, not defaulted to permitted, because the register is the thing
    /// that grants permission and silence is not a grant.
    #[test]
    fn a_target_the_register_does_not_carry_is_refused() {
        let mut cache = Cache::default();
        cache
            .observe(
                artifact("bytes"),
                retrieval("https://example.invalid/a", "2026-01-01T00:00:00Z"),
            )
            .expect("a new artifact");
        let violations =
            validate_cache(&cache, &BTreeMap::new()).expect_err("nothing permits this target");
        assert!(violations.has("E-CAC-02"), "{violations}");
    }

    /// Every hygiene rule, planted one at a time.
    #[test]
    fn the_structural_rules_are_refused_by_code() {
        let base = |stored: bool| {
            let mut one = artifact("bytes");
            one.stored = stored;
            one.retrievals = vec![retrieval(
                "https://example.invalid/a",
                "2026-01-01T00:00:00Z",
            )];
            one
        };

        let mut empty = base(false);
        empty.retrievals.clear();
        let cache = Cache {
            artifacts: vec![empty],
        };
        assert!(
            validate_cache(&cache, &refused())
                .expect_err("no retrieval")
                .has("E-CAC-03"),
        );

        let mut repeated = base(false);
        let same = retrieval("https://example.invalid/a", "2026-01-01T00:00:00Z");
        repeated.retrievals = vec![same.clone(), same];
        let cache = Cache {
            artifacts: vec![repeated],
        };
        assert!(
            validate_cache(&cache, &refused())
                .expect_err("repeated retrieval")
                .has("E-CAC-04"),
        );

        let mut zero = base(false);
        zero.bytes = 0;
        let cache = Cache {
            artifacts: vec![zero],
        };
        assert!(
            validate_cache(&cache, &refused())
                .expect_err("zero length")
                .has("E-CAC-05"),
        );

        // ⚠ Built by hand rather than through `observe`, which sorts. The rule
        // is about a cache a caller assembled, and that is the only way to
        // reach it.
        let mut high = base(false);
        high.sha256 = digest("zzzz");
        let mut low = base(false);
        low.sha256 = digest("aaaa");
        let ordered = high.sha256 < low.sha256;
        let cache = Cache {
            artifacts: if ordered {
                vec![low, high]
            } else {
                vec![high, low]
            },
        };
        assert!(
            validate_cache(&cache, &refused())
                .expect_err("out of order")
                .has("E-CAC-06"),
        );
    }
}
