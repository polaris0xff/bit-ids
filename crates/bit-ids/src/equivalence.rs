//! Whether two routes delivered the same build, and what "same" is worth.
//!
//! `E-ACQ-04` already forces every route to report the version the record
//! declares, so by the time anything gets here the **labels** agree. That is
//! the easy half and it is worth almost nothing on its own: a distribution can
//! patch a build and keep the upstream version string, and two vendors can ship
//! one upstream release as different bytes for reasons that change no
//! behaviour. Equal labels are the question, not the answer.
//!
//! ⛔ **A run observes one installed build.** The second route proves the
//! version resolves the same. It does not prove the other bytes behave the
//! same, and nothing in a single capture can, because nothing put those bytes
//! on the wire. So when the two installs differ and only one was watched, the
//! honest outcome is [`Equivalence::Unresolved`] and the record does not
//! publish. Calling that "equivalent" would publish a claim about a build this
//! project never ran.
//!
//! ⭐ **The one case where observing one really is observing both** is
//! byte-identical installs. If every route delivered the same executable bytes,
//! there is only one build, and which route was watched stops mattering. That
//! is the whole reason the executable digest is recorded per route rather than
//! once per record.
//!
//! ⛔ **AND THE OTHER WAY OUT IS A SECOND RECORD, NOT A SECOND READING.** A
//! release binary against a local build never installs the same bytes, so
//! [`classify`] refuses that pair forever and [`routes_publishable`]'s own
//! wording - *an unresolved record needs a second capture through the other
//! route* - named something no caller could act on: the second capture is a
//! different record, and the per-record gate cannot see it.
//! [`routes_publishable_among`] is the same rule asked where the store is
//! visible, and it is what makes a measured record publishable at all.

use core::fmt;

use crate::acquisition::AcquisitionRoute;
use crate::canonical::Slug;
use crate::observation::FieldState;
use crate::record::Profile;
use crate::validate::{SchemaError, Violations};

/// What comparing the routes of a record established.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Equivalence {
    /// Every route installed the same executable bytes. There is one build.
    ByteIdentical,
    /// The installs differ in bytes and every one of them was observed, with no
    /// overlapping field disagreeing. Only [`classify_across`] can reach this,
    /// because it needs a capture per route.
    BuildEquivalent,
    /// Equal version labels over evidence that conflicts.
    Divergent,
    /// Not enough evidence to say. ⛔ Not a mild verdict: it is what a single
    /// capture of two byte-different installs produces, and it does not publish.
    Unresolved,
}

impl Equivalence {
    /// The canonical spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ByteIdentical => "byte_identical",
            Self::BuildEquivalent => "build_equivalent",
            Self::Divergent => "divergent",
            Self::Unresolved => "unresolved",
        }
    }

    /// Whether this outcome may be published.
    ///
    /// ⛔ Two of the four, and the two that are refused are refused for
    /// different reasons: `Divergent` because the evidence conflicts, and
    /// `Unresolved` because there is not enough of it. Collapsing them into one
    /// "not ok" would lose which.
    #[must_use]
    pub const fn publishable(self) -> bool {
        matches!(self, Self::ByteIdentical | Self::BuildEquivalent)
    }
}

impl fmt::Display for Equivalence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The outcome and what produced it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Comparison {
    /// What the routes established.
    pub outcome: Equivalence,
    /// One line per comparison made, kept whatever the outcome. A verdict with
    /// no reasoning is a verdict nobody can check.
    pub reasons: Vec<String>,
    /// The routes that were compared.
    pub routes: Vec<Slug>,
}

impl Comparison {
    fn new(outcome: Equivalence, reasons: Vec<String>, routes: Vec<Slug>) -> Self {
        Self {
            outcome,
            reasons,
            routes,
        }
    }
}

/// Compares the routes of one record.
///
/// Reaches [`Equivalence::ByteIdentical`] or [`Equivalence::Unresolved`] and
/// never [`Equivalence::BuildEquivalent`]: one capture cannot establish that
/// two different builds behave alike. [`classify_across`] is what can.
#[must_use]
pub fn classify(profile: &Profile) -> Comparison {
    let routes: Vec<Slug> = profile.acquisition.iter().map(|r| r.id.clone()).collect();
    let mut reasons = Vec::new();

    if profile.acquisition.len() < 2 {
        reasons.push(format!(
            "{} route(s); equivalence needs two to compare",
            profile.acquisition.len()
        ));
        return Comparison::new(Equivalence::Unresolved, reasons, routes);
    }

    let Some(first) = profile.acquisition.first() else {
        unreachable!("length was checked above");
    };
    let identical = profile
        .acquisition
        .iter()
        .all(|route| route.installed_executable == first.installed_executable);
    if identical {
        reasons.push(format!(
            "every route installed {}, so there is one build and observing one \
             observed it",
            first.installed_executable
        ));
        return Comparison::new(Equivalence::ByteIdentical, reasons, routes);
    }

    for route in &profile.acquisition {
        reasons.push(format!(
            "{} installed {} and reported {}",
            route.id, route.installed_executable, route.installed_version
        ));
    }
    // ⚠ Different artifact bytes are a packaging observation, not a failure.
    // What makes this unresolved is that the ones nobody watched were never put
    // on the wire, so nothing here can say whether they behave the same.
    let unobserved: Vec<&AcquisitionRoute> = profile
        .acquisition
        .iter()
        .filter(|route| route.id != profile.capture.observed_route)
        .collect();
    reasons.push(format!(
        "the capture observed {} and {} other install(s) were never put on the \
         wire, so equal version labels are all that connects them",
        profile.capture.observed_route,
        unobserved.len()
    ));
    Comparison::new(Equivalence::Unresolved, reasons, routes)
}

/// Compares captures of one build taken through different routes.
///
/// Every profile must describe the same target, version, platform, architecture
/// and package, and each must have observed a different route. That is what
/// makes the comparison mean something: two records of the same install would
/// agree trivially.
///
/// ⛔ Where they disagree on a field both measured, the outcome is
/// [`Equivalence::Divergent`]. Equal version labels over a behavioural
/// difference is exactly the case `architecture.md` section 7 says is never
/// silently collapsed.
#[must_use]
pub fn classify_across(profiles: &[&Profile]) -> Comparison {
    let routes: Vec<Slug> = profiles
        .iter()
        .map(|profile| profile.capture.observed_route.clone())
        .collect();
    let mut reasons = Vec::new();

    if profiles.len() < 2 {
        reasons.push(format!(
            "{} record(s); a cross-route comparison needs two",
            profiles.len()
        ));
        return Comparison::new(Equivalence::Unresolved, reasons, routes);
    }
    let Some(first) = profiles.first() else {
        unreachable!("length was checked above");
    };
    for other in &profiles[1..] {
        if other.target.id != first.target.id
            || other.build.version != first.build.version
            || other.build.platform != first.build.platform
            || other.build.arch != first.build.arch
        {
            reasons.push(format!(
                "{} and {} do not describe one build",
                first.capture.id, other.capture.id
            ));
            return Comparison::new(Equivalence::Unresolved, reasons, routes);
        }
    }
    // ⛔ Two records of one run are one record. Nothing about the second route
    // is established by reading the same capture twice, and the pair would
    // otherwise agree on every field for the most trivial reason there is.
    let mut runs: Vec<&Slug> = Vec::new();
    for profile in profiles {
        if runs.contains(&&profile.capture.id) {
            reasons.push(format!(
                "{} appears twice; two records of one run are one record",
                profile.capture.id
            ));
            return Comparison::new(Equivalence::Unresolved, reasons, routes);
        }
        runs.push(&profile.capture.id);
    }
    // Two captures of one route compare a build against itself, which agrees
    // for a reason that says nothing about the other route.
    let mut seen: Vec<&Slug> = Vec::new();
    for profile in profiles {
        if seen.contains(&&profile.capture.observed_route) {
            reasons.push(format!(
                "{} was observed twice; comparing a build with itself proves nothing \
                 about the other route",
                profile.capture.observed_route
            ));
            return Comparison::new(Equivalence::Unresolved, reasons, routes);
        }
        seen.push(&profile.capture.observed_route);
    }

    let mut conflicts = 0_usize;
    let mut compared = 0_usize;
    for field in &first.observations {
        for other in &profiles[1..] {
            let Some(theirs) = other.field(&field.path) else {
                continue;
            };
            // Only fields both records actually measured. `unknown` on either
            // side is an absence of evidence, and comparing against it would
            // manufacture a disagreement out of nobody looking.
            if !field.state.asserts_a_measurement() || !theirs.state.asserts_a_measurement() {
                continue;
            }
            compared += 1;
            if !states_agree(&field.state, &theirs.state) {
                conflicts += 1;
                reasons.push(format!(
                    "{} is {} in {} and {} in {}",
                    field.path,
                    field.state.as_str(),
                    first.capture.id,
                    theirs.state.as_str(),
                    other.capture.id
                ));
            }
        }
    }

    if conflicts > 0 {
        reasons.push(format!(
            "{conflicts} of {compared} overlapping field(s) disagree across routes"
        ));
        return Comparison::new(Equivalence::Divergent, reasons, routes);
    }
    if compared == 0 {
        reasons.push("no field was measured in both records, so nothing was compared".to_owned());
        return Comparison::new(Equivalence::Unresolved, reasons, routes);
    }
    reasons.push(format!(
        "{compared} overlapping field(s) agree across {} route(s)",
        routes.len()
    ));
    Comparison::new(Equivalence::BuildEquivalent, reasons, routes)
}

/// Whether two measurements of one field say the same thing.
///
/// ⚠ Equality of the whole state, not of a rendered value. A field that is
/// `constant` in one record and `patterned` in another is a real difference in
/// what the build did, and a comparison that only looked at bytes would miss
/// it.
fn states_agree(left: &FieldState, right: &FieldState) -> bool {
    match (left, right) {
        // Sample counts legitimately differ between runs; what must match is
        // what was measured, not how many times.
        (FieldState::Constant(a), FieldState::Constant(b)) => a.value == b.value,
        (FieldState::Patterned(a), FieldState::Patterned(b)) => a.pattern == b.pattern,
        (FieldState::Variable(a), FieldState::Variable(b)) => a.length == b.length,
        _ => left.as_str() == right.as_str(),
    }
}

/// The publication rule this module contributes, over one record alone.
///
/// ⚠ **A record alone cannot settle a pair**, which is the whole of `E-PUB-04`:
/// [`classify`] reaches `byte_identical` or `unresolved` and never
/// `build_equivalent`, so a record whose routes installed different bytes is
/// refused here however many captures exist. [`routes_publishable_among`] is
/// the same rule asked where the second capture is visible.
///
/// # Errors
///
/// Returns `E-PUB-03` when the routes are divergent and `E-PUB-04` when they
/// are unresolved. Two codes rather than one, because the fix differs: a
/// divergence needs adjudicating and an unresolved record needs a second
/// capture through the other route.
pub fn routes_publishable(profile: &Profile) -> Result<(), Violations> {
    routes_publishable_among(profile, &[])
}

/// The same rule, with the store's other records available.
///
/// ⛔ **The second capture through the other route is a DIFFERENT RECORD**, and
/// [`routes_publishable`]'s own wording has always said that is what an
/// unresolved record needs. Nothing could act on it: the per-record gate cannot
/// see a sibling, and `build_equivalent` is reachable only from
/// [`classify_across`], whose answer nothing carried back. So a release binary
/// against a local build - which never install the same bytes - was refused for
/// a reason no amount of capturing could address.
///
/// ⛔ **Every comparable sibling is read, not the first that agrees.** A record
/// with one sibling that agrees and one that conflicts has conflicting
/// evidence, and publishing on the agreeable half would bury the finding. A
/// divergence anywhere in the set refuses the record.
///
/// ⚠ **`others` may hold anything the store does.** [`classify_across`] is what
/// decides which of them are comparable - the same target, version, platform and
/// architecture, a different run, and a different observed route - so a caller
/// passes the store rather than pre-selecting, and the selection stays in one
/// place.
///
/// # Errors
///
/// As [`routes_publishable`], and `E-PUB-04`'s message additionally says
/// whether any sibling was comparable at all, because "no second capture" and
/// "a second capture that settled nothing" are different states of the store.
pub fn routes_publishable_among(profile: &Profile, others: &[&Profile]) -> Result<(), Violations> {
    let comparison = classify(profile);
    let mut out = Vec::new();
    match comparison.outcome {
        Equivalence::Divergent => out.push(SchemaError::new(
            "E-PUB-03",
            "acquisition",
            format!(
                "the routes are divergent: {}",
                comparison.reasons.join("; ")
            ),
        )),
        Equivalence::Unresolved => {
            let mut settled = 0_usize;
            let mut conflicts = Vec::new();
            for other in others {
                let across = classify_across(&[profile, other]);
                match across.outcome {
                    // Not a pair at all: a different build, the same run twice,
                    // or the same route watched twice. Those say nothing about
                    // this record, so they are not held against it either.
                    Equivalence::Unresolved => {}
                    Equivalence::Divergent => conflicts.push(SchemaError::new(
                        "E-PUB-03",
                        "acquisition",
                        format!(
                            "{} watched {} and the two captures disagree: {}",
                            other.capture.id,
                            other.capture.observed_route,
                            across.reasons.join("; ")
                        ),
                    )),
                    Equivalence::ByteIdentical | Equivalence::BuildEquivalent => settled += 1,
                }
            }
            // ⛔ A conflict anywhere refuses the record, whatever else agreed.
            // Reporting only the agreement would publish a measurement while
            // the store holds the evidence against it.
            if !conflicts.is_empty() {
                out.extend(conflicts);
            } else if settled == 0 {
                out.push(SchemaError::new(
                    "E-PUB-04",
                    "acquisition",
                    format!(
                        "the routes are unresolved and no capture of another route settles it: {}",
                        comparison.reasons.join("; ")
                    ),
                ));
            }
        }
        Equivalence::ByteIdentical | Equivalence::BuildEquivalent => {}
    }
    Violations::from_errors(out)
}

#[cfg(test)]
mod tests {
    use super::Equivalence;

    #[test]
    fn only_the_two_established_outcomes_publish() {
        assert!(Equivalence::ByteIdentical.publishable());
        assert!(Equivalence::BuildEquivalent.publishable());
        assert!(!Equivalence::Divergent.publishable());
        assert!(
            !Equivalence::Unresolved.publishable(),
            "not enough evidence is not the same as enough evidence"
        );
    }

    #[test]
    fn every_outcome_has_one_spelling() {
        let all = [
            Equivalence::ByteIdentical,
            Equivalence::BuildEquivalent,
            Equivalence::Divergent,
            Equivalence::Unresolved,
        ];
        let mut spellings: Vec<&str> = all.iter().map(|o| o.as_str()).collect();
        spellings.sort_unstable();
        spellings.dedup();
        assert_eq!(spellings.len(), all.len());
    }
}
