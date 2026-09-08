//! What a tool asks this catalogue about **its own** declared identity.
//!
//! `LIB-02` owns it. [`crate::catalogue`] answers "what did you measure"; this
//! answers the different question a client actually has, which is "is what I
//! say about myself what you measured", and it is a comparison rather than a
//! lookup.
//!
//! ⛔ **THE ANSWER FAILS CLOSED AND THAT IS THE WHOLE POINT.** A tool that
//! carries a table of what it emits can drift from what it emits, and the
//! failure is silent: nothing in the tool is wrong, and the announce is. So
//! [`Answer::agrees`] is true for exactly one variant, and every way of not
//! knowing - no record, no observation, a state that asserts nothing - is
//! [`Answer::Unmeasured`] rather than a pass. ⚠ Today **every** answer about a
//! real client is `Unmeasured`, because nothing has been measured. A design
//! that let "not measured" read as agreement would report this project's own
//! emptiness as a clean bill of health.
//!
//! ⛔ **A DISAGREEMENT ANYWHERE OUTRANKS AGREEMENT EVERYWHERE ELSE.** A claim
//! comes from one constant in the tool's source and is therefore a claim about
//! every build of that version, so a record for one platform that disagrees is
//! a drift even when three others agree. Answering "agrees" because a majority
//! did would be a vote over a measurement.
//!
//! ⭐ **A COMPARISON RATHER THAN A GENERATED TABLE, and the Problem is why.**
//! The alternative interface is this project emitting a Rust source file the
//! tool vendors, which is a second copy of the measurement living in the
//! tool - the same shape as the table it would replace, drifting the same way,
//! and now with this project's name on it. A query answered at the tool's own
//! test time cannot go stale, because there is nothing to go stale.
//!
//! ⚠ **NOTHING HERE READS A CLIENT-ID TABLE.** The claim is bytes the caller
//! declares and the measurement is bytes this project observed;
//! `docs/capture-methodology.md` lists a decoder table among the inputs that
//! may seed a hypothesis and may not populate the catalogue, and a comparison
//! that resolved either side through one would put that refused input inside
//! the answer.

use crate::canonical::{Slug, Version};
use crate::catalogue::Catalogue;
use crate::identity::RecordId;
use crate::observation::{FieldPath, FieldState, PatternRun};

/// Which question the claimed bytes are asking.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Shape {
    /// Every sample carries exactly these bytes and nothing else.
    Whole,
    /// Every sample begins with these bytes; what follows is not claimed.
    ///
    /// ⚠ This is the peer-ID shape. A client prefix is fixed and the suffix is
    /// regenerated, so a whole-value claim about one would be false on its own
    /// terms even when the prefix is right.
    Prefix,
}

/// What a tool declares it puts on the wire.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Claim {
    /// The target the tool is, as this catalogue names it.
    pub target: Slug,
    /// The exact version the claim is about.
    ///
    /// ⚠ A claim is about a build, so it carries a version. A tool asking
    /// about "itself" without one would be asking whether any measured build
    /// ever emitted these bytes, which is a different and much weaker question.
    pub version: Version,
    /// Which observed field the claim is about.
    pub field: FieldPath,
    /// The bytes the tool says it emits.
    pub bytes: Vec<u8>,
    /// Whether those bytes are the whole value or its leading span.
    pub shape: Shape,
}

/// Why a claim could not be compared against a measurement.
///
/// ⛔ **None of these is agreement.** They are kept apart because the thing to
/// do about each differs: a missing record needs a capture, a varying field
/// needs a prefix claim rather than a whole one, and an absence is a
/// measurement that contradicts any claim about bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Unmeasured {
    /// The catalogue publishes no record for that target at that version.
    NoRecord,
    /// Records exist and none of them carries that field.
    NoField,
    /// The field is there and its state asserts nothing about the build.
    Unknown,
    /// The build was measured to emit nothing at all for that field.
    ///
    /// ⚠ Not a disagreement. A claim about bytes and a measured absence are
    /// about different things, and reporting it as a mismatch would send a
    /// caller looking for wrong bytes rather than for a surface the build does
    /// not speak.
    Absent {
        /// `not_observed` or `not_supported`, as the record spells it.
        state: &'static str,
    },
    /// The value varies with no fixed leading span, so a prefix claim has
    /// nothing to compare against.
    NoFixedPrefix,
}

/// What the catalogue says about a claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Answer {
    /// Every record that carries the field measured what the claim says.
    Agrees {
        /// The records that agreed, ascending.
        records: Vec<RecordId>,
    },
    /// At least one record measured something else.
    Disagrees {
        /// The first record that disagreed, ascending by identifier.
        record: RecordId,
        /// What that record measured, as the record spells it.
        measured: String,
    },
    /// There is no measurement to compare against.
    Unmeasured {
        /// Which of the ways of not knowing this is.
        reason: Unmeasured,
    },
}

impl Answer {
    /// Whether the catalogue confirms the claim.
    ///
    /// ⛔ **True for one variant only.** A caller that wants a hard assertion
    /// calls this; a caller that wants to tell "wrong" from "unknown" reads the
    /// enum. Anything that treated `Unmeasured` as a pass would turn an empty
    /// catalogue into a clean bill of health for every client in it.
    #[must_use]
    pub const fn agrees(&self) -> bool {
        matches!(self, Self::Agrees { .. })
    }

    /// Whether the catalogue contradicts the claim.
    #[must_use]
    pub const fn disagrees(&self) -> bool {
        matches!(self, Self::Disagrees { .. })
    }
}

/// Compares one declared identity against every published measurement of it.
///
/// ⚠ The catalogue is the input rather than a path, for the reason
/// [`crate::catalogue`] gives: nothing in this crate reaches a network, and a
/// publication is opened over bytes the caller already holds.
#[must_use]
pub fn check(catalogue: &Catalogue, claim: &Claim) -> Answer {
    let records = catalogue.at_version(&claim.target, &claim.version);
    if records.is_empty() {
        return Answer::Unmeasured {
            reason: Unmeasured::NoRecord,
        };
    }

    // ⚠ Ascending by record identifier, so the record a disagreement names is
    // the same one on every run over one publication. `at_version` already
    // answers in that order and this does not depend on it.
    let mut ordered: Vec<_> = records;
    ordered.sort_by_key(|profile| profile.id);

    let mut agreed = Vec::new();
    let mut unmeasured: Option<Unmeasured> = None;
    for profile in ordered {
        let Some(observed) = profile
            .observations
            .iter()
            .find(|field| field.path == claim.field)
        else {
            unmeasured.get_or_insert(Unmeasured::NoField);
            continue;
        };
        match compare(&observed.state, claim) {
            Verdict::Agrees => agreed.push(profile.id),
            // ⛔ The first disagreement ends it. A claim is one constant in the
            // tool, so a build that measured something else is a drift whatever
            // the others did.
            Verdict::Disagrees(measured) => {
                return Answer::Disagrees {
                    record: profile.id,
                    measured,
                };
            }
            Verdict::Unmeasured(reason) => {
                unmeasured.get_or_insert(reason);
            }
        }
    }

    if agreed.is_empty() {
        return Answer::Unmeasured {
            reason: unmeasured.unwrap_or(Unmeasured::NoField),
        };
    }
    Answer::Agrees { records: agreed }
}

/// The per-record verdict, before the fail-closed rule combines them.
enum Verdict {
    Agrees,
    Disagrees(String),
    Unmeasured(Unmeasured),
}

fn compare(state: &FieldState, claim: &Claim) -> Verdict {
    match state {
        FieldState::Unknown => Verdict::Unmeasured(Unmeasured::Unknown),
        // ⚠ One arm for two states, and the state's own spelling is what
        // tells them apart in the answer. They are the same fact to a caller
        // comparing bytes - the build emitted none - and different facts to
        // whoever reads the reason.
        FieldState::NotObserved | FieldState::NotSupported => {
            Verdict::Unmeasured(Unmeasured::Absent {
                state: state.as_str(),
            })
        }
        FieldState::Constant(value) => match claim.shape {
            Shape::Whole => {
                if value.value.as_slice() == claim.bytes.as_slice() {
                    Verdict::Agrees
                } else {
                    Verdict::Disagrees(hex(value.value.as_slice()))
                }
            }
            // ⭐ A constant answers a prefix claim, and this is the case a
            // reader gets wrong: a value that never varies still HAS the
            // claimed prefix, so refusing to compare here would report a
            // client whose whole peer id is fixed as unmeasured.
            Shape::Prefix => {
                if value.value.as_slice().starts_with(&claim.bytes) {
                    Verdict::Agrees
                } else {
                    Verdict::Disagrees(hex(value.value.as_slice()))
                }
            }
        },
        FieldState::Patterned(value) => {
            let Some(PatternRun::Fixed { bytes }) = value.pattern.runs.first() else {
                return Verdict::Unmeasured(Unmeasured::NoFixedPrefix);
            };
            match claim.shape {
                // ⛔ A patterned value cannot satisfy a whole-value claim, and
                // that is a disagreement rather than a gap: the tool says the
                // value never changes and the measurement says it does.
                Shape::Whole => {
                    Verdict::Disagrees(format!("patterned, fixed prefix {}", hex(bytes.as_slice())))
                }
                Shape::Prefix => {
                    // ⚠ The claim has to fit INSIDE the fixed run. A claim
                    // longer than the measured fixed span reaches into bytes
                    // the measurement says vary, and calling that a match would
                    // confirm a prefix nothing established.
                    if bytes.as_slice().starts_with(&claim.bytes) {
                        Verdict::Agrees
                    } else {
                        Verdict::Disagrees(hex(bytes.as_slice()))
                    }
                }
            }
        }
        FieldState::Variable(_) => Verdict::Unmeasured(Unmeasured::NoFixedPrefix),
    }
}

/// Lowercase hexadecimal, the one spelling `canonical.rs` allows for bytes.
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}
