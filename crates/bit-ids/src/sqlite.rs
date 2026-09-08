//! The queryable rendering of the published record set.
//!
//! `PUB-05` owns it, and it is the one published format that needs a
//! third-party encoder. ⛔ **That is why it is a file of its own**: a reader
//! asking what in this crate pulls a vendored C library should find one answer,
//! and `docs/supply-chain.md` carries the argument for paying it.
//!
//! ⛔ **THE TABLES ARE DERIVED FROM THE CANONICAL DOCUMENTS, LIKE EVERY OTHER
//! RENDERING.** [`crate::formats`] parses each record's published bytes once and
//! hands the values here, so no rendering can carry a field another does not.
//! The first eleven columns of `record` are [`crate::formats::CSV_COLUMNS`] read
//! through [`crate::formats::CSV_POINTERS`], which is one derivation rather than
//! two spellings of one table.
//!
//! ⭐ **THE DATABASE IS SELF-DESCRIBING AND THE CSV IS NOT.** A tabular file
//! cannot say what it left out, so `PUB-03` publishes a columns document beside
//! it. A database can: `sqlite_master` says what it carries and the `omission`
//! table says what it does not, so there is no second file to drift from this
//! one.
//!
//! ⛔ **`document` IS WHAT MAKES THIS RENDERING LOSSLESS.** Every record's
//! canonical bytes are in it verbatim, so a value this schema does not tabulate
//! is still reachable, and a consumer can re-derive the published record from
//! the database rather than trusting the columns. The tables are an index over
//! those bytes and never a replacement for them.
//!
//! ⚠ **THE BYTES HAVE TO BE REPRODUCIBLE, WHICH IS NOT FREE FOR THIS FORMAT.**
//! A release is assembled twice and compared, so the file is built in one
//! transaction, in ascending record order, at a stated page size, and vacuumed,
//! and it is serialised out of memory rather than written to a path. ⭐ The
//! crate never touches the filesystem, which section 3 of `docs/architecture.md`
//! requires of it, and a temporary file would also carry a journal this would
//! then have to reason about.

use rusqlite::{Connection, params};

use crate::formats::{CSV_COLUMNS, CSV_POINTERS, FORMATS_SCHEMA};
use crate::record::Profile;
use crate::validate::SchemaError;

/// Every record, as a queryable database.
pub const SQLITE_FILE: &str = "formats/bit-ids-v1.sqlite3";

/// The page size the file is built at.
///
/// ⚠ Stated rather than inherited. It is a compile-time default of the bundled
/// library, so leaving it unsaid would make the published bytes change under a
/// dependency bump with nothing saying why - which is the class
/// `docs/conventions/shell.md` section 8 records about a PowerShell default.
const PAGE_SIZE: i64 = 4096;

/// What the tables do not carry, with where to find it instead.
///
/// ⛔ **PUBLISHED AS ROWS RATHER THAN AS PROSE**, for the reason `PUB-03` gives
/// about the CSV: a consumer reading only this file would otherwise have no way
/// to learn that a section exists at all. Each subject is a value that is
/// nested rather than scalar, so a column would either flatten it or hold a
/// second encoding of it.
const OMISSIONS: [(&str, &str); 7] = [
    (
        "observations[].state.detail",
        "the per-state detail is shaped by the state; `state` carries the kind and `document` the rest",
    ),
    (
        "observations[].evidence",
        "a list of evidence identifiers; join `evidence` on the record, or read `document`",
    ),
    (
        "corroboration[].observations",
        "what each connector saw, per field; `document` carries it",
    ),
    (
        "corroboration[].conflict",
        "`has_conflict` is the flag and `document` carries the detail",
    ),
    (
        "acquisition[].source",
        "the immutable source identity is typed to the route kind; `document` carries it",
    ),
    (
        "adjudication.evidence",
        "a list of evidence identifiers; join `evidence` on the record, or read `document`",
    ),
    (
        "capture.fixture, capture.observer, capture.observed_route, capture.connectors",
        "the run's own identity, which `raw/` and the run manifest describe; `document` carries it",
    ),
];

/// The schema, in the order it is created.
///
/// ⛔ **`STRICT` ON EVERY TABLE.** Without it SQLite stores whatever a caller
/// binds, so a column declared `TEXT` would accept an integer and a consumer
/// reading the published file would get a different type from the same query
/// depending on which record it landed on.
const SCHEMA: &str = "\
CREATE TABLE meta(key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL) STRICT;
CREATE TABLE omission(subject TEXT PRIMARY KEY NOT NULL, reason TEXT NOT NULL) STRICT;
CREATE TABLE record(
  id TEXT PRIMARY KEY NOT NULL,
  schema TEXT NOT NULL,
  target TEXT NOT NULL,
  target_kind TEXT NOT NULL,
  version TEXT NOT NULL,
  platform TEXT NOT NULL,
  arch TEXT NOT NULL,
  package TEXT NOT NULL,
  executable TEXT NOT NULL,
  capture TEXT NOT NULL,
  captured_at TEXT NOT NULL,
  supersedes TEXT
) STRICT;
CREATE TABLE document(
  record TEXT PRIMARY KEY NOT NULL REFERENCES record(id),
  canonical TEXT NOT NULL
) STRICT;
CREATE TABLE acquisition(
  record TEXT NOT NULL REFERENCES record(id),
  id TEXT NOT NULL,
  kind TEXT NOT NULL,
  resolver TEXT NOT NULL,
  delivery TEXT NOT NULL,
  origin TEXT NOT NULL,
  artifact TEXT NOT NULL,
  signature TEXT NOT NULL,
  resolved_version TEXT NOT NULL,
  installed_version TEXT NOT NULL,
  installed_executable TEXT NOT NULL,
  installed_probe TEXT NOT NULL,
  installed_evidence TEXT NOT NULL,
  PRIMARY KEY(record, id)
) STRICT;
CREATE TABLE observation(
  record TEXT NOT NULL REFERENCES record(id),
  path TEXT NOT NULL,
  state TEXT NOT NULL,
  PRIMARY KEY(record, path)
) STRICT;
CREATE TABLE corroboration(
  record TEXT NOT NULL REFERENCES record(id),
  path TEXT NOT NULL,
  agreement TEXT NOT NULL,
  has_conflict INTEGER NOT NULL,
  PRIMARY KEY(record, path)
) STRICT;
CREATE TABLE normalization(
  record TEXT NOT NULL REFERENCES record(id),
  id TEXT NOT NULL,
  summary TEXT NOT NULL,
  preserves_order INTEGER NOT NULL,
  preserves_unknown_bytes INTEGER NOT NULL,
  PRIMARY KEY(record, id)
) STRICT;
CREATE TABLE evidence(
  record TEXT NOT NULL REFERENCES record(id),
  id TEXT NOT NULL,
  kind TEXT NOT NULL,
  path TEXT NOT NULL,
  bytes INTEGER NOT NULL,
  sha256 TEXT NOT NULL,
  connector TEXT,
  PRIMARY KEY(record, id)
) STRICT;
CREATE TABLE adjudication(
  record TEXT PRIMARY KEY NOT NULL REFERENCES record(id),
  decided_at TEXT NOT NULL,
  reason TEXT NOT NULL,
  summary TEXT NOT NULL
) STRICT;
CREATE INDEX record_by_target ON record(target, version);
CREATE INDEX record_by_build ON record(platform, arch, package);
CREATE INDEX record_by_captured_at ON record(captured_at);
CREATE INDEX record_by_supersedes ON record(supersedes);
CREATE INDEX acquisition_by_record ON acquisition(record);
CREATE INDEX observation_by_path ON observation(path);
CREATE INDEX corroboration_by_path ON corroboration(path);
CREATE INDEX normalization_by_record ON normalization(record);
CREATE INDEX evidence_by_record ON evidence(record);
CREATE INDEX evidence_by_sha256 ON evidence(sha256);
";

/// Renders the record set as a SQLite database and returns its bytes.
///
/// The documents are `(profile, canonical text, parsed document)` in the order
/// they are to be published, which is ascending by record identifier.
///
/// # Errors
///
/// | code | refused |
/// | --- | --- |
/// | `E-FMT-05` | a value the schema needs is not in the canonical document |
/// | `E-FMT-06` | the database could not be built or serialised |
pub(crate) fn render(
    documents: &[(&Profile, String, serde_json::Value)],
) -> Result<Vec<u8>, Vec<SchemaError>> {
    let mut errors = Vec::new();
    let bytes = match build(documents, &mut errors) {
        Ok(bytes) => bytes,
        Err(error) => {
            errors.push(SchemaError::new(
                "E-FMT-06",
                "sqlite",
                format!("the database could not be built: {error}"),
            ));
            Vec::new()
        }
    };
    if errors.is_empty() {
        Ok(bytes)
    } else {
        Err(errors)
    }
}

/// Reads one required string out of a document, recording a refusal if it is
/// absent.
///
/// ⚠ A missing value is `E-FMT-05` rather than an empty cell, for the reason
/// the CSV gives: a blank and a missing one are indistinguishable afterwards.
fn text<'doc>(
    value: &'doc serde_json::Value,
    pointer: &str,
    profile: &Profile,
    subject: &str,
    errors: &mut Vec<SchemaError>,
) -> &'doc str {
    let Some(found) = value.pointer(pointer).and_then(serde_json::Value::as_str) else {
        errors.push(SchemaError::new(
            "E-FMT-05",
            format!("sqlite {subject}"),
            format!("{pointer} is not a string in {}", profile.id),
        ));
        return "";
    };
    found
}

/// Reads one required boolean out of a document as SQLite's 0 or 1.
fn flag(
    value: &serde_json::Value,
    pointer: &str,
    profile: &Profile,
    errors: &mut Vec<SchemaError>,
) -> i64 {
    let Some(found) = value.pointer(pointer).and_then(serde_json::Value::as_bool) else {
        errors.push(SchemaError::new(
            "E-FMT-05",
            "sqlite normalization",
            format!("{pointer} is not a boolean in {}", profile.id),
        ));
        return 0;
    };
    i64::from(found)
}

/// The elements of an array at a pointer, or nothing where there is no array.
///
/// ⚠ An absent optional section is not a refusal here: `supersedes` and
/// `adjudication` are absent on every original record, and
/// [`crate::validate`] is what refuses one that should have been there. A
/// second rule would be a second answer.
fn array<'doc>(value: &'doc serde_json::Value, pointer: &str) -> &'doc [serde_json::Value] {
    value
        .pointer(pointer)
        .and_then(serde_json::Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn build(
    documents: &[(&Profile, String, serde_json::Value)],
    errors: &mut Vec<SchemaError>,
) -> Result<Vec<u8>, rusqlite::Error> {
    let connection = Connection::open_in_memory()?;
    // ⛔ BEFORE ANY TABLE EXISTS. A page size set after the first page is
    // written is silently ignored, and the file would then be built at whatever
    // the library's default is - the thing this line exists to stop depending
    // on.
    connection.pragma_update(None, "page_size", PAGE_SIZE)?;
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    connection.execute_batch(SCHEMA)?;

    let transaction = connection.unchecked_transaction()?;
    write_meta(&transaction, documents.len())?;
    for (profile, canonical, value) in documents {
        // ⚠ The record row is written first because every other table
        // references it, and foreign keys are ON.
        let id = write_record(&transaction, profile, canonical, value, errors)?;
        write_acquisition(&transaction, &id, profile, value, errors)?;
        write_observations(&transaction, &id, profile, value, errors)?;
        write_corroboration(&transaction, &id, profile, value, errors)?;
        write_normalizations(&transaction, &id, profile, value, errors)?;
        write_evidence(&transaction, &id, profile, value, errors)?;
        write_adjudication(&transaction, &id, profile, value, errors)?;
    }
    transaction.commit()?;

    // ⛔ VACUUM AFTER THE COMMIT, so the file carries no free pages left over
    // from the order rows happened to be inserted in. Two assemblies of one
    // store have to produce identical bytes, and a freelist is a function of the
    // insertion history rather than of the data.
    connection.execute_batch("VACUUM;")?;

    Ok(connection.serialize(rusqlite::MAIN_DB)?.to_vec())
}

/// What this file is and what it does not carry.
fn write_meta(transaction: &rusqlite::Transaction<'_>, records: usize) -> rusqlite::Result<()> {
    transaction.execute(
        "INSERT INTO meta(key, value) VALUES('schema', ?1)",
        params![FORMATS_SCHEMA],
    )?;
    transaction.execute(
        "INSERT INTO meta(key, value) VALUES('file', ?1)",
        params![SQLITE_FILE],
    )?;
    transaction.execute(
        "INSERT INTO meta(key, value) VALUES('records', ?1)",
        params![records.to_string()],
    )?;
    for (subject, reason) in OMISSIONS {
        transaction.execute(
            "INSERT INTO omission(subject, reason) VALUES(?1, ?2)",
            params![subject, reason],
        )?;
    }
    Ok(())
}

/// The identity row, and the canonical bytes beside it.
///
/// ⛔ The columns are [`CSV_COLUMNS`] read through [`CSV_POINTERS`], so the
/// tabular rendering and this one cannot describe a record differently.
fn write_record(
    transaction: &rusqlite::Transaction<'_>,
    profile: &Profile,
    canonical: &str,
    value: &serde_json::Value,
    errors: &mut Vec<SchemaError>,
) -> rusqlite::Result<String> {
    let mut cells: Vec<&str> = Vec::with_capacity(CSV_POINTERS.len());
    for (column, pointer) in CSV_COLUMNS.iter().zip(CSV_POINTERS) {
        cells.push(text(value, pointer, profile, column, errors));
    }
    // ⚠ `supersedes` is absent on an original record, and that is a NULL rather
    // than an empty string: the two mean different things and only one of them
    // joins.
    let supersedes = value
        .pointer("/supersedes")
        .and_then(serde_json::Value::as_str);
    transaction.execute(
        "INSERT INTO record(id, schema, target, target_kind, version, platform, arch, \
         package, executable, capture, captured_at, supersedes) \
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            cells[0], cells[1], cells[2], cells[3], cells[4], cells[5], cells[6], cells[7],
            cells[8], cells[9], cells[10], supersedes
        ],
    )?;
    transaction.execute(
        "INSERT INTO document(record, canonical) VALUES(?1, ?2)",
        params![cells[0], canonical],
    )?;
    Ok(cells[0].to_owned())
}

/// One row per acquisition route.
fn write_acquisition(
    transaction: &rusqlite::Transaction<'_>,
    id: &str,
    profile: &Profile,
    value: &serde_json::Value,
    errors: &mut Vec<SchemaError>,
) -> rusqlite::Result<()> {
    for route in array(value, "/acquisition") {
        let mut cell = |pointer: &str, subject: &str| -> String {
            text(route, pointer, profile, subject, errors).to_owned()
        };
        transaction.execute(
            "INSERT INTO acquisition(record, id, kind, resolver, delivery, origin, artifact, \
             signature, resolved_version, installed_version, installed_executable, \
             installed_probe, installed_evidence) \
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                id,
                cell("/id", "acquisition id"),
                cell("/kind", "acquisition kind"),
                cell("/resolver", "acquisition resolver"),
                cell("/delivery", "acquisition delivery"),
                cell("/origin", "acquisition origin"),
                cell("/artifact", "acquisition artifact"),
                cell("/signature", "acquisition signature"),
                cell("/resolved_version", "acquisition resolved_version"),
                cell("/installed_version", "acquisition installed_version"),
                cell("/installed_executable", "acquisition installed_executable"),
                cell("/installed_probe", "acquisition installed_probe"),
                cell("/installed_evidence", "acquisition installed_evidence"),
            ],
        )?;
    }
    Ok(())
}

/// One row per observed field, carrying the state's kind.
fn write_observations(
    transaction: &rusqlite::Transaction<'_>,
    id: &str,
    profile: &Profile,
    value: &serde_json::Value,
    errors: &mut Vec<SchemaError>,
) -> rusqlite::Result<()> {
    for observation in array(value, "/observations") {
        let path = text(observation, "/path", profile, "observation path", errors).to_owned();
        let state = text(
            observation,
            "/state/kind",
            profile,
            "observation state",
            errors,
        )
        .to_owned();
        transaction.execute(
            "INSERT INTO observation(record, path, state) VALUES(?1, ?2, ?3)",
            params![id, path, state],
        )?;
    }
    Ok(())
}

/// One row per corroborated field.
fn write_corroboration(
    transaction: &rusqlite::Transaction<'_>,
    id: &str,
    profile: &Profile,
    value: &serde_json::Value,
    errors: &mut Vec<SchemaError>,
) -> rusqlite::Result<()> {
    for entry in array(value, "/corroboration") {
        // ⚠ Derived by asking whether the value is null rather than by reading
        // the conflict's own shape, so a conflict this schema has never seen
        // still counts as one.
        let has_conflict = i64::from(!matches!(
            entry.pointer("/conflict"),
            None | Some(serde_json::Value::Null)
        ));
        let path = text(entry, "/path", profile, "corroboration path", errors).to_owned();
        let agreement = text(
            entry,
            "/agreement",
            profile,
            "corroboration agreement",
            errors,
        )
        .to_owned();
        transaction.execute(
            "INSERT INTO corroboration(record, path, agreement, has_conflict) \
             VALUES(?1, ?2, ?3, ?4)",
            params![id, path, agreement, has_conflict],
        )?;
    }
    Ok(())
}

/// One row per declared normalization.
fn write_normalizations(
    transaction: &rusqlite::Transaction<'_>,
    id: &str,
    profile: &Profile,
    value: &serde_json::Value,
    errors: &mut Vec<SchemaError>,
) -> rusqlite::Result<()> {
    for entry in array(value, "/normalizations") {
        let normalization = text(entry, "/id", profile, "normalization id", errors).to_owned();
        let summary = text(entry, "/summary", profile, "normalization summary", errors).to_owned();
        let order = flag(entry, "/preserves_order", profile, errors);
        let unknown = flag(entry, "/preserves_unknown_bytes", profile, errors);
        transaction.execute(
            "INSERT INTO normalization(record, id, summary, preserves_order, \
             preserves_unknown_bytes) VALUES(?1, ?2, ?3, ?4, ?5)",
            params![id, normalization, summary, order, unknown],
        )?;
    }
    Ok(())
}

/// One row per cited artifact.
fn write_evidence(
    transaction: &rusqlite::Transaction<'_>,
    id: &str,
    profile: &Profile,
    value: &serde_json::Value,
    errors: &mut Vec<SchemaError>,
) -> rusqlite::Result<()> {
    for entry in array(value, "/evidence") {
        let Some(bytes) = entry.pointer("/bytes").and_then(serde_json::Value::as_i64) else {
            errors.push(SchemaError::new(
                "E-FMT-05",
                "sqlite evidence bytes",
                format!("/bytes is not an integer in {}", profile.id),
            ));
            continue;
        };
        let evidence = text(entry, "/id", profile, "evidence id", errors).to_owned();
        let kind = text(entry, "/kind", profile, "evidence kind", errors).to_owned();
        let path = text(entry, "/path", profile, "evidence path", errors).to_owned();
        let digest = text(entry, "/sha256", profile, "evidence sha256", errors).to_owned();
        // ⚠ A connector is absent on evidence the observer itself produced, and
        // that is a NULL rather than an empty string.
        let connector = entry
            .pointer("/connector")
            .and_then(serde_json::Value::as_str);
        transaction.execute(
            "INSERT INTO evidence(record, id, kind, path, bytes, sha256, connector) \
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, evidence, kind, path, bytes, digest, connector],
        )?;
    }
    Ok(())
}

/// The adjudication row, on a correction and on nothing else.
fn write_adjudication(
    transaction: &rusqlite::Transaction<'_>,
    id: &str,
    profile: &Profile,
    value: &serde_json::Value,
    errors: &mut Vec<SchemaError>,
) -> rusqlite::Result<()> {
    let Some(adjudication) = value
        .pointer("/adjudication")
        .filter(|found| !found.is_null())
    else {
        return Ok(());
    };
    let decided_at = text(
        adjudication,
        "/decided_at",
        profile,
        "adjudication decided_at",
        errors,
    )
    .to_owned();
    let reason = text(
        adjudication,
        "/reason",
        profile,
        "adjudication reason",
        errors,
    )
    .to_owned();
    let summary = text(
        adjudication,
        "/summary",
        profile,
        "adjudication summary",
        errors,
    )
    .to_owned();
    transaction.execute(
        "INSERT INTO adjudication(record, decided_at, reason, summary) VALUES(?1, ?2, ?3, ?4)",
        params![id, decided_at, reason, summary],
    )?;
    Ok(())
}
