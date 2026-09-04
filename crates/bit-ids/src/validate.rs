//! The publication invariants a profile record must satisfy.
//!
//! Deserialization already refused anything malformed: a value in a
//! non-canonical spelling, a field this schema does not define, a schema
//! identifier from another generation. What is left is what only the assembled
//! record can answer, and the largest of those is whether a field that claims
//! something about a build has any recoverable bytes behind it.
//!
//! Every violation carries a stable code. A diagnostic whose text a caller has
//! to match on is a diagnostic nobody can act on twice.

use core::fmt;

use crate::Agreement;
use crate::identity::{RecordId, RecordKey};
use crate::observation::{FieldState, ObservedField, PatternRun};
use crate::record::{EvidenceKind, Profile};

/// One refused invariant.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaError {
    code: &'static str,
    at: String,
    detail: String,
}

impl SchemaError {
    fn new(code: &'static str, at: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code,
            at: at.into(),
            detail: detail.into(),
        }
    }

    /// The stable diagnostic code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }

    /// Where in the record the violation is, as a reader-facing path.
    #[must_use]
    pub fn at(&self) -> &str {
        &self.at
    }

    /// What is wrong, in one line.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}: {}", self.code, self.at, self.detail)
    }
}

impl core::error::Error for SchemaError {}

/// Every invariant one record refused, in the order they were checked.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Violations(Vec<SchemaError>);

impl Violations {
    /// The refused invariants.
    #[must_use]
    pub fn errors(&self) -> &[SchemaError] {
        &self.0
    }

    /// The stable codes, for a caller that acts on the class rather than the
    /// wording.
    pub fn codes(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.0.iter().map(SchemaError::code)
    }

    /// Whether one code is among the refusals.
    #[must_use]
    pub fn has(&self, code: &str) -> bool {
        self.0.iter().any(|error| error.code == code)
    }

    /// How many invariants were refused.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether nothing was refused. A [`Violations`] is only constructed when
    /// something was, so this is always false.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for Violations {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, error) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str("\n")?;
            }
            write!(f, "{error}")?;
        }
        Ok(())
    }
}

impl core::error::Error for Violations {}

/// Reports whether a sequence is strictly ascending under a rendered key.
///
/// Order is checked rather than imposed. A validator that sorted on the way in
/// would accept two byte-different files as one record, which is the drift the
/// append-only store cannot survive.
fn strictly_ascending<T>(items: &[T], key: impl Fn(&T) -> String) -> Option<usize> {
    let mut previous: Option<String> = None;
    for (index, item) in items.iter().enumerate() {
        let current = key(item);
        if let Some(previous) = &previous
            && *previous >= current
        {
            return Some(index);
        }
        previous = Some(current);
    }
    None
}

fn check_display_name(profile: &Profile, out: &mut Vec<SchemaError>) {
    let name = &profile.target.display_name;
    let shaped = !name.is_empty()
        && name.len() <= 64
        && name.bytes().all(|b| b.is_ascii_graphic() || b == b' ')
        && !name.starts_with(' ')
        && !name.ends_with(' ');
    if !shaped {
        out.push(SchemaError::new(
            "E-TGT-01",
            "target.display_name",
            format!(
                "{name:?} is empty, over 64 characters, or carries a non-printable or edge space"
            ),
        ));
    }
    if profile.target.engine.as_ref() == Some(&profile.target.id) {
        out.push(SchemaError::new(
            "E-TGT-02",
            "target.engine",
            "a target cannot be its own engine",
        ));
    }
}

fn check_identity(profile: &Profile, out: &mut Vec<SchemaError>) {
    let derived = RecordId::derive(&RecordKey {
        schema: &profile.schema,
        target: &profile.target.id,
        version: &profile.build.version,
        platform: &profile.build.platform,
        arch: &profile.build.arch,
        package: &profile.build.package,
        capture: &profile.capture.id,
    });
    if derived != profile.id {
        out.push(SchemaError::new(
            "E-ID-01",
            "id",
            format!("declared {}, derived {derived}", profile.id),
        ));
    }
    if profile.supersedes == Some(profile.id) {
        out.push(SchemaError::new(
            "E-ID-02",
            "supersedes",
            "a record cannot supersede itself",
        ));
    }
}

fn check_acquisition(profile: &Profile, out: &mut Vec<SchemaError>) {
    let routes = &profile.acquisition;
    if routes.len() < 2 {
        out.push(SchemaError::new(
            "E-ACQ-01",
            "acquisition",
            format!(
                "{} route(s); two independent routes are required",
                routes.len()
            ),
        ));
    }
    if let Some(index) = strictly_ascending(routes, |route| route.id.to_string()) {
        let code = if index > 0 && routes[index].id == routes[index - 1].id {
            "E-ACQ-02"
        } else {
            "E-ACQ-03"
        };
        out.push(SchemaError::new(
            code,
            format!("acquisition[{index}]"),
            format!(
                "route ids must be unique and ascending, found {}",
                routes[index].id
            ),
        ));
    }
    for (index, route) in routes.iter().enumerate() {
        if route.installed_version != profile.build.version {
            out.push(SchemaError::new(
                "E-ACQ-04",
                format!("acquisition[{index}].installed_version"),
                format!(
                    "route {} installed {}, the build declares {}",
                    route.id, route.installed_version, profile.build.version
                ),
            ));
        }
    }
}

fn check_capture(profile: &Profile, out: &mut Vec<SchemaError>) {
    let connectors = &profile.capture.connectors;
    if connectors.len() < 2 {
        out.push(SchemaError::new(
            "E-CAP-01",
            "capture.connectors",
            format!(
                "{} connector(s); the observer plus one independent connector are required",
                connectors.len()
            ),
        ));
    }
    if let Some(index) = strictly_ascending(connectors, |connector| connector.id.to_string()) {
        let code = if index > 0 && connectors[index].id == connectors[index - 1].id {
            "E-CAP-02"
        } else {
            "E-CAP-03"
        };
        out.push(SchemaError::new(
            code,
            format!("capture.connectors[{index}]"),
            format!(
                "connector ids must be unique and ascending, found {}",
                connectors[index].id
            ),
        ));
    }
    if !connectors
        .iter()
        .any(|connector| connector.id == profile.capture.observer)
    {
        out.push(SchemaError::new(
            "E-CAP-04",
            "capture.observer",
            format!(
                "{} is not among the declared connectors",
                profile.capture.observer
            ),
        ));
    }
}

fn check_evidence(profile: &Profile, out: &mut Vec<SchemaError>) {
    let evidence = &profile.evidence;
    if let Some(index) = strictly_ascending(evidence, |entry| entry.id.to_string()) {
        let code = if index > 0 && evidence[index].id == evidence[index - 1].id {
            "E-EVD-01"
        } else {
            "E-EVD-02"
        };
        out.push(SchemaError::new(
            code,
            format!("evidence[{index}]"),
            format!(
                "evidence ids must be unique and ascending, found {}",
                evidence[index].id
            ),
        ));
    }
    for (index, entry) in evidence.iter().enumerate() {
        if evidence[..index]
            .iter()
            .any(|prior| prior.path == entry.path)
        {
            out.push(SchemaError::new(
                "E-EVD-03",
                format!("evidence[{index}].path"),
                format!("{} is claimed by more than one entry", entry.path),
            ));
        }
        if entry.bytes == 0 {
            out.push(SchemaError::new(
                "E-EVD-04",
                format!("evidence[{index}].bytes"),
                format!("{} has no bytes; a parsed value with no recoverable bytes is not a measurement", entry.id),
            ));
        }
        if let Some(connector) = &entry.connector
            && !profile
                .capture
                .connectors
                .iter()
                .any(|declared| &declared.id == connector)
        {
            out.push(SchemaError::new(
                "E-EVD-05",
                format!("evidence[{index}].connector"),
                format!("{connector} is not declared in capture.connectors"),
            ));
        }
    }
}

fn check_pattern(field: &ObservedField, at: &str, out: &mut Vec<SchemaError>) {
    let FieldState::Patterned(patterned) = &field.state else {
        return;
    };
    let pattern = &patterned.pattern;
    let mut covered = 0usize;
    let mut fixed = 0usize;
    let mut varying = 0usize;
    for run in &pattern.runs {
        covered = covered.saturating_add(run.len());
        match run {
            PatternRun::Fixed { .. } => fixed += 1,
            PatternRun::Varying { length, alphabet } => {
                varying += 1;
                if *length == 0 {
                    out.push(SchemaError::new(
                        "E-OBS-09",
                        at,
                        "a varying run covers no bytes",
                    ));
                }
                if let Some(alphabet) = alphabet
                    && alphabet
                        .as_slice()
                        .windows(2)
                        .any(|pair| pair[0] >= pair[1])
                {
                    out.push(SchemaError::new(
                        "E-OBS-15",
                        at,
                        "alphabet bytes must be unique and ascending",
                    ));
                }
            }
        }
    }
    if covered != pattern.length {
        out.push(SchemaError::new(
            "E-OBS-09",
            at,
            format!(
                "runs cover {covered} bytes, the pattern declares {}",
                pattern.length
            ),
        ));
    }
    if fixed == 0 || varying == 0 {
        out.push(SchemaError::new(
            "E-OBS-10",
            at,
            format!(
                "a pattern needs a fixed run and a varying run, found {fixed} and {varying}; \
                 otherwise it is a constant or a variable"
            ),
        ));
    }
    if patterned.samples.get() < 2 {
        out.push(SchemaError::new(
            "E-OBS-12",
            at,
            "a pattern claims bytes changed between samples, which one sample cannot show",
        ));
    }
}

fn check_widths(field: &ObservedField, at: &str, out: &mut Vec<SchemaError>) {
    let Some(expected) = field.path.fixed_width() else {
        return;
    };
    let found = match &field.state {
        FieldState::Constant(constant) => Some(constant.value.len()),
        FieldState::Patterned(patterned) => Some(patterned.pattern.length),
        FieldState::Variable(variable) => variable.length,
        _ => None,
    };
    if let Some(found) = found
        && found != expected
    {
        let code = if matches!(field.state, FieldState::Patterned(_)) {
            "E-OBS-11"
        } else {
            "E-OBS-08"
        };
        out.push(SchemaError::new(
            code,
            at,
            format!("{found} bytes, the protocol fixes this field at {expected}"),
        ));
    }
}

/// The unproven-field rule, and the two rules that bracket it.
///
/// A state that asserts anything about the build needs recoverable bytes behind
/// it; a state that asserts nothing must not carry any; and a claim that the
/// build produced nothing needs a control proving the surface was observable at
/// all. Without the last one, an observer that was never listening and a build
/// that never answered are the same record.
fn check_field_evidence(
    profile: &Profile,
    field: &ObservedField,
    at: &str,
    out: &mut Vec<SchemaError>,
) {
    for (cited, id) in field.evidence.iter().enumerate() {
        if profile.evidence_entry(id).is_none() {
            out.push(SchemaError::new(
                "E-OBS-03",
                at,
                format!("cites evidence {id}, which the record does not declare"),
            ));
        }
        if field.evidence[..cited].contains(id) {
            out.push(SchemaError::new(
                "E-OBS-04",
                at,
                format!("cites evidence {id} more than once"),
            ));
        }
    }

    if field.state.asserts_a_measurement() {
        if field.evidence.is_empty() {
            out.push(SchemaError::new(
                "E-OBS-05",
                at,
                format!(
                    "state {} asserts something about the build and cites no evidence",
                    field.state.as_str()
                ),
            ));
        }
    } else if !field.evidence.is_empty() {
        out.push(SchemaError::new(
            "E-OBS-06",
            at,
            "state unknown asserts nothing, so it cannot carry evidence",
        ));
    }

    if field.state.claims_absence() {
        let controlled = field.evidence.iter().any(|id| {
            profile
                .evidence_entry(id)
                .is_some_and(|entry| entry.kind == EvidenceKind::PositiveControl)
        });
        if !controlled {
            out.push(SchemaError::new(
                "E-OBS-07",
                at,
                format!(
                    "state {} claims the build produced nothing, which needs a positive \
                     control proving the surface was observable",
                    field.state.as_str()
                ),
            ));
        }
    }
}

fn check_variable(field: &ObservedField, at: &str, out: &mut Vec<SchemaError>) {
    let FieldState::Variable(variable) = &field.state else {
        return;
    };
    if variable.samples.get() < 2 {
        out.push(SchemaError::new(
            "E-OBS-12",
            at,
            "a variability claim cannot rest on one sample",
        ));
    }
    if variable.distinct > variable.samples {
        out.push(SchemaError::new(
            "E-OBS-13",
            at,
            format!(
                "{} distinct values from {} samples",
                variable.distinct, variable.samples
            ),
        ));
    }
    if variable.distinct.get() < 2 {
        out.push(SchemaError::new(
            "E-OBS-14",
            at,
            "one distinct value is a constant, not a variable",
        ));
    }
}

fn check_observations(profile: &Profile, out: &mut Vec<SchemaError>) {
    let fields = &profile.observations;
    if let Some(index) = strictly_ascending(fields, |field| field.path.to_string()) {
        let code = if index > 0 && fields[index].path == fields[index - 1].path {
            "E-OBS-01"
        } else {
            "E-OBS-02"
        };
        out.push(SchemaError::new(
            code,
            format!("observations[{index}]"),
            format!(
                "field paths must be unique and ascending, found {}",
                fields[index].path
            ),
        ));
    }

    for (index, field) in fields.iter().enumerate() {
        let at = format!("observations[{index}] {}", field.path);
        check_field_evidence(profile, field, &at, out);
        check_variable(field, &at, out);
        check_pattern(field, &at, out);
        check_widths(field, &at, out);
    }

    if !fields
        .iter()
        .any(|field| field.state.asserts_a_measurement())
    {
        out.push(SchemaError::new(
            "E-OBS-16",
            "observations",
            "no field asserts a measurement; the record measures nothing",
        ));
    }
}

fn check_corroboration(profile: &Profile, out: &mut Vec<SchemaError>) {
    let entries = &profile.corroboration;
    if let Some(index) = strictly_ascending(entries, |entry| entry.path.to_string()) {
        let code = if index > 0 && entries[index].path == entries[index - 1].path {
            "E-COR-02"
        } else {
            "E-COR-03"
        };
        out.push(SchemaError::new(
            code,
            format!("corroboration[{index}]"),
            format!(
                "corroborated paths must be unique and ascending, found {}",
                entries[index].path
            ),
        ));
    }

    for (index, entry) in entries.iter().enumerate() {
        let at = format!("corroboration[{index}] {}", entry.path);
        if profile.field(&entry.path).is_none() {
            out.push(SchemaError::new(
                "E-COR-01",
                &at,
                "names a field the record does not observe",
            ));
        }
        for (cited, id) in entry.connectors.iter().enumerate() {
            if !profile
                .capture
                .connectors
                .iter()
                .any(|declared| &declared.id == id)
            {
                out.push(SchemaError::new(
                    "E-COR-04",
                    &at,
                    format!("names connector {id}, which the capture does not declare"),
                ));
            }
            if entry.connectors[..cited].contains(id) {
                out.push(SchemaError::new(
                    "E-COR-06",
                    &at,
                    format!("names connector {id} twice; one connector is not two observations"),
                ));
            }
        }
        let expected_two = entry.agreement != Agreement::NotCorroborated;
        let counted = entry.connectors.len();
        if expected_two && counted < 2 {
            out.push(SchemaError::new(
                "E-COR-05",
                &at,
                format!(
                    "outcome {:?} compares observations, but {counted} connector(s) are named",
                    entry.agreement
                ),
            ));
        }
        if !expected_two && counted != 1 {
            out.push(SchemaError::new(
                "E-COR-05",
                &at,
                format!("outcome not_corroborated names {counted} connector(s), not one"),
            ));
        }
    }

    for field in &profile.observations {
        if field.state.asserts_a_measurement()
            && !entries.iter().any(|entry| entry.path == field.path)
        {
            out.push(SchemaError::new(
                "E-COR-07",
                format!("corroboration {}", field.path),
                "a field that asserts a measurement carries no corroboration outcome; \
                 silence is not the same as not_corroborated",
            ));
        }
    }
}

/// Checks every publication invariant this schema owns.
///
/// All invariants are checked, not just the first that fails, because a record
/// is repaired once rather than once per refusal.
///
/// # Errors
///
/// Returns every refused invariant, each with a stable code.
pub fn validate(profile: &Profile) -> Result<(), Violations> {
    let mut out = Vec::new();
    check_identity(profile, &mut out);
    check_display_name(profile, &mut out);
    check_acquisition(profile, &mut out);
    check_capture(profile, &mut out);
    check_evidence(profile, &mut out);
    check_observations(profile, &mut out);
    check_corroboration(profile, &mut out);
    if out.is_empty() {
        Ok(())
    } else {
        Err(Violations(out))
    }
}
