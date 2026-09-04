//! How many separately initialized runs a claim about variability needs, and
//! what those runs can actually tell you.
//!
//! One observation establishes bytes for one connection. It does not establish
//! a lifetime. A peer ID that looked the same twice may be stored on disk, or
//! may be regenerated per process and simply have been read twice inside one
//! process. Those are different facts about a build and only a controlled
//! restart separates them.
//!
//! ⛔ **Nothing here is a confidence.** A classification is a function of the
//! samples: it says what the exercised runs prove and returns
//! [`Lifetime::Unknown`] for everything they do not. A dimension the plan never
//! varied cannot yield a conclusion about that dimension, however many samples
//! were taken along the others.

use core::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use crate::canonical::HexBytes;

/// What a run varied between samples.
///
/// Each field is how many distinct instances of that dimension the run
/// exercised, so a plan of all ones is a single sample and can conclude
/// nothing about any lifetime.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SamplingPlan {
    /// Separately started processes.
    pub sessions: NonZeroU32,
    /// Distinct torrents offered, within each session.
    pub torrents: NonZeroU32,
    /// Distinct connections made, within each torrent.
    pub connections: NonZeroU32,
}

impl SamplingPlan {
    /// How many observations the plan produces if every combination is taken.
    #[must_use]
    pub const fn observations(&self) -> u64 {
        (self.sessions.get() as u64)
            * (self.torrents.get() as u64)
            * (self.connections.get() as u64)
    }

    /// Whether the plan varied anything at all.
    #[must_use]
    pub const fn varies_anything(&self) -> bool {
        self.sessions.get() > 1 || self.torrents.get() > 1 || self.connections.get() > 1
    }

    /// Whether restarting the process was exercised, which is the only thing
    /// that can distinguish a stored value from a regenerated one.
    #[must_use]
    pub const fn restarts(&self) -> bool {
        self.sessions.get() > 1
    }
}

/// Where one observation came from within the plan.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sample {
    /// Which process start, counted from zero.
    pub session: u32,
    /// Which torrent within that session, counted from zero.
    pub torrent: u32,
    /// Which connection within that torrent, counted from zero.
    pub connection: u32,
    /// What the build emitted.
    pub value: HexBytes,
}

/// How long a byte stays the same.
///
/// ⚠ These are claims about a build, and each needs the plan to have varied the
/// dimension it names. `Unknown` is not a failure to classify; it is the
/// correct answer when nothing exercised the difference.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lifetime {
    /// Regenerated for every connection.
    PerConnection,
    /// Stable for one torrent and different for another in the same process.
    PerTorrent,
    /// Stable for one process run and different after a restart.
    PerSession,
    /// Survived a restart, so the build is keeping it somewhere.
    Persistent,
    /// The samples do not separate the cases. ⛔ This is the honest answer for a
    /// single sample, and for any dimension the plan did not vary.
    Unknown,
}

impl Lifetime {
    /// The canonical spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PerConnection => "per_connection",
            Self::PerTorrent => "per_torrent",
            Self::PerSession => "per_session",
            Self::Persistent => "persistent",
            Self::Unknown => "unknown",
        }
    }

    /// Whether this lifetime asserts that the byte changes.
    #[must_use]
    pub const fn varies(self) -> bool {
        matches!(
            self,
            Self::PerConnection | Self::PerTorrent | Self::PerSession
        )
    }
}

/// A run of adjacent bytes that behaved the same way.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpanClass {
    /// Where the run starts.
    pub offset: usize,
    /// How many bytes it covers.
    pub length: usize,
    /// What the samples prove about it.
    pub lifetime: Lifetime,
}

/// What a set of samples proves about one value.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariabilityReport {
    /// The width every sample shared, or `None` when the width itself varied.
    pub length: Option<usize>,
    /// The value's bytes, grouped into maximal runs that behaved the same way.
    pub spans: Vec<SpanClass>,
    /// How many observations went in.
    pub samples: usize,
}

impl VariabilityReport {
    /// Whether any span was shown to change.
    #[must_use]
    pub fn any_variation(&self) -> bool {
        self.spans.iter().any(|span| span.lifetime.varies())
    }

    /// The span covering one byte offset.
    #[must_use]
    pub fn span_at(&self, offset: usize) -> Option<&SpanClass> {
        self.spans
            .iter()
            .find(|span| offset >= span.offset && offset < span.offset + span.length)
    }
}

/// Whether every sample sharing a key also shares the byte at `offset`.
fn constant_within(
    samples: &[&Sample],
    offset: usize,
    key: impl Fn(&Sample) -> (u32, u32),
) -> bool {
    for (index, sample) in samples.iter().enumerate() {
        for other in &samples[index + 1..] {
            if key(sample) == key(other)
                && sample.value.as_slice()[offset] != other.value.as_slice()[offset]
            {
                return false;
            }
        }
    }
    true
}

/// Whether two samples anywhere differ at `offset`.
fn differs_anywhere(samples: &[&Sample], offset: usize) -> bool {
    let first = samples[0].value.as_slice()[offset];
    samples
        .iter()
        .any(|sample| sample.value.as_slice()[offset] != first)
}

fn classify_offset(samples: &[&Sample], plan: &SamplingPlan, offset: usize) -> Lifetime {
    if !differs_anywhere(samples, offset) {
        // It never changed. That is only a claim about persistence if the run
        // actually restarted the process; otherwise all it shows is that one
        // process kept using the same value, which every lifetime except
        // per-connection would also produce.
        return if plan.restarts() {
            Lifetime::Persistent
        } else {
            Lifetime::Unknown
        };
    }

    // It changed somewhere. Which dimension separates it is decided by the
    // narrowest grouping it is still constant within.
    if !constant_within(samples, offset, |sample| (sample.session, sample.torrent)) {
        return Lifetime::PerConnection;
    }
    if plan.torrents.get() > 1 && !constant_within(samples, offset, |sample| (sample.session, 0)) {
        return Lifetime::PerTorrent;
    }
    if plan.restarts() {
        return Lifetime::PerSession;
    }
    Lifetime::Unknown
}

/// Classifies every byte of a value from the samples that produced it.
///
/// Adjacent bytes that behaved the same way are reported as one span, which is
/// what turns a peer ID into the shape it actually has: a fixed prefix and a
/// suffix the build regenerates.
///
/// # Errors
///
/// Returns `None` when there are no samples. A report over nothing would be a
/// classification nobody measured.
#[must_use]
pub fn classify(samples: &[Sample], plan: &SamplingPlan) -> Option<VariabilityReport> {
    if samples.is_empty() {
        return None;
    }
    let refs: Vec<&Sample> = samples.iter().collect();
    let width = refs[0].value.len();
    let common = refs.iter().all(|sample| sample.value.len() == width);

    if !common {
        // ⚠ Nothing can be said per offset when the offsets do not line up. The
        // width itself varying is the finding.
        return Some(VariabilityReport {
            length: None,
            spans: Vec::new(),
            samples: samples.len(),
        });
    }

    let mut spans: Vec<SpanClass> = Vec::new();
    for offset in 0..width {
        let lifetime = classify_offset(&refs, plan, offset);
        match spans.last_mut() {
            Some(span) if span.lifetime == lifetime => span.length += 1,
            _ => spans.push(SpanClass {
                offset,
                length: 1,
                lifetime,
            }),
        }
    }

    Some(VariabilityReport {
        length: Some(width),
        spans,
        samples: samples.len(),
    })
}
