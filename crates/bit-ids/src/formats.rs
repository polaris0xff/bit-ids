//! The consumer-facing renderings of the published record set.
//!
//! `PUB-01` assembles a release and `CORPUS-03` derives the lookups over it.
//! This is the third thing a consumer reads: the records themselves, in the
//! shapes a reader can actually consume. `PUB-03` owns it.
//!
//! ⛔ **THE RECORD SET IS ASKED FOR, NEVER RE-SELECTED.** Which records belong
//! in a published view is one rule and it lives in [`crate::index`]: only
//! publishable ones, and never one something corrects. A second selection here
//! would drift, and it would drift in the direction that publishes a retracted
//! measurement in the tabular view while the lookups had stopped naming it,
//! which is the shape `docs/methodology/reviews.md` calls a gate on one of two
//! doors.
//!
//! ⛔ **THE COMBINED JSON CARRIES EACH RECORD'S OWN BYTES.** The array elements
//! are the canonical documents verbatim rather than re-indented copies, so a
//! reader who slices one out has exactly the bytes that were published and
//! digested. Re-rendering them would be a second spelling of the record model,
//! which is what `canonical.rs` refuses for values and what this refuses for
//! documents.
//!
//! ⚠ **JSONL AND CBOR ARE DERIVED FROM THE CANONICAL DOCUMENT, NOT FROM THE
//! TYPE.** Both need a shape the canonical form does not have: one line, and a
//! binary encoding. Each is produced by reading the canonical document back and
//! re-emitting it, so neither can carry a field the published JSON does not.
//! `E-FMT-02` is the check that the read-back agrees.
//!
//! ⚠ **CSV IS LOSSY AND SAYS SO IN A FILE RATHER THAN IN PROSE.** A tabular row
//! cannot hold the acquisition routes, the observations, the corroboration or
//! the evidence list, so the columns and the omitted sections are published
//! beside it. A consumer that reads only the CSV can then discover what it is
//! not being told without reading this comment.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::canonical::RelPath;
use crate::corpus::Corpus;
use crate::identity::RecordId;
use crate::index::Indexes;
use crate::record::Profile;
use crate::validate::{SchemaError, Violations};

/// Identifier carried by the documents that describe these renderings.
pub const FORMATS_SCHEMA: &str = "bit-ids/formats/1";

/// Every record, as one JSON array of the canonical documents.
pub const JSON_FILE: &str = "formats/bit-ids-v1.json";

/// Every record, one compact JSON document per line.
pub const JSONL_FILE: &str = "formats/bit-ids-v1.jsonl";

/// The lossy tabular view.
pub const CSV_FILE: &str = "formats/bit-ids-v1.csv";

/// Every record, as deterministic CBOR.
pub const CBOR_FILE: &str = "formats/bit-ids-v1.cbor";

/// What the tabular view carries and what it leaves out.
pub const COLUMNS_FILE: &str = "formats/bit-ids-v1.columns.json";

/// The tabular columns, in order.
///
/// ⛔ Literals, not the field names that happen to spell them. These are a
/// published header row, so renaming one is a schema change rather than a
/// rename, and `csv_columns_have_one_spelling` pins them.
pub const CSV_COLUMNS: [&str; 11] = [
    "record",
    "schema",
    "target",
    "target_kind",
    "version",
    "platform",
    "arch",
    "package",
    "executable",
    "capture",
    "captured_at",
];

/// Where each column's value is read out of the canonical document.
///
/// ⛔ **THE CELLS COME FROM THE PUBLISHED BYTES, NOT FROM THE TYPE.** Reading
/// them off the record would be a second rendering of values the canonical
/// document already spells, and the two would differ the day one of them
/// changed how something is written. A pointer that resolves to nothing is
/// `E-FMT-05` rather than an empty cell, because a blank column and a missing
/// one look identical in a table.
pub const CSV_POINTERS: [&str; 11] = [
    "/id",
    "/schema",
    "/target/id",
    "/target/kind",
    "/build/version",
    "/build/platform",
    "/build/arch",
    "/build/package",
    "/build/executable",
    "/capture/id",
    "/capture/captured_at",
];

/// The record sections no tabular row can hold.
///
/// ⚠ Published rather than described. A consumer reading the CSV alone would
/// otherwise have no way to learn that the acquisition routes and the evidence
/// list exist at all.
pub const CSV_OMITS: [&str; 7] = [
    "acquisition",
    "observations",
    "corroboration",
    "normalizations",
    "evidence",
    "supersedes",
    "adjudication",
];

/// The rendered files, each with the path it is published at.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Formats {
    /// Path and bytes, ascending by path.
    pub files: Vec<(RelPath, Vec<u8>)>,
    /// How many records every rendering describes.
    ///
    /// ⚠ One number for all of them on purpose. Every file here is a function
    /// of the same document list, so a count that differed between two of them
    /// would be a defect rather than a fact worth reporting per file.
    pub records: usize,
}

/// Renders every published format over the records the views include.
///
/// # Errors
///
/// | code | refused |
/// | --- | --- |
/// | `E-FMT-01` | a record the views name that the store does not carry |
/// | `E-FMT-02` | a canonical document that does not read back as itself |
/// | `E-FMT-03` | a value no deterministic CBOR encoding covers |
/// | `E-FMT-04` | nothing to publish |
/// | `E-FMT-05` | a tabular column whose value is not in the document |
pub fn render(corpus: &Corpus, indexes: &Indexes) -> Result<Formats, Violations> {
    let mut errors = Vec::new();
    let mut by_id: BTreeMap<RecordId, &Profile> = BTreeMap::new();
    for stored in corpus.profiles() {
        by_id.insert(stored.profile.id, &stored.profile);
    }

    // ⛔ Ascending by identifier, and never by the order the store was read in.
    // Two runs over one store have to produce identical bytes, and a map's
    // iteration order is not a contract a consumer can check.
    let mut chosen: Vec<&Profile> = Vec::new();
    for id in indexes.records() {
        match by_id.get(&id) {
            Some(profile) => chosen.push(profile),
            None => errors.push(SchemaError::new(
                "E-FMT-01",
                "records",
                format!("the views name {id}, which this store does not carry"),
            )),
        }
    }

    if chosen.is_empty() && errors.is_empty() {
        errors.push(SchemaError::new(
            "E-FMT-04",
            "records",
            "no record is publishable, so there is nothing to render",
        ));
    }
    if !errors.is_empty() {
        return Violations::from_errors(errors).map(|()| unreachable!());
    }

    // ⛔ THE CANONICAL DOCUMENT IS THE SOURCE FOR ALL FOUR. Every rendering
    // below is a function of these bytes, so none of them can disagree with the
    // record as published.
    let mut documents: Vec<(&Profile, String, serde_json::Value)> =
        Vec::with_capacity(chosen.len());
    for profile in chosen {
        let document = match profile.to_json() {
            Ok(text) => text,
            Err(error) => {
                errors.push(SchemaError::new(
                    "E-FMT-02",
                    "records",
                    format!("{} cannot be written: {error}", profile.id),
                ));
                continue;
            }
        };
        // ⚠ Read back rather than trusted. A renderer that took the type's word
        // for what it wrote would agree with itself about a document nobody can
        // parse, which is the same defect `PUB-01` avoids by handing its
        // checksum file to `sha256sum -c`.
        match serde_json::from_str::<serde_json::Value>(&document) {
            Ok(value) => documents.push((profile, document, value)),
            Err(error) => errors.push(SchemaError::new(
                "E-FMT-02",
                "records",
                format!(
                    "{}'s canonical document does not re-parse: {error}",
                    profile.id
                ),
            )),
        }
    }
    if !errors.is_empty() {
        return Violations::from_errors(errors).map(|()| unreachable!());
    }

    let mut cbor = Vec::new();
    if let Err(error) = write_cbor_array(&documents, &mut cbor) {
        errors.push(error);
    }
    let table = match csv(&documents) {
        Ok(text) => text,
        Err(found) => {
            errors.extend(found);
            String::new()
        }
    };
    if !errors.is_empty() {
        return Violations::from_errors(errors).map(|()| unreachable!());
    }

    let mut files = vec![
        (path(CBOR_FILE), cbor),
        (path(COLUMNS_FILE), columns_document().into_bytes()),
        (path(CSV_FILE), table.into_bytes()),
        (path(JSON_FILE), combined_json(&documents).into_bytes()),
        (path(JSONL_FILE), jsonl(&documents).into_bytes()),
    ];
    files.sort_by(|left, right| left.0.cmp(&right.0));

    Ok(Formats {
        records: documents.len(),
        files,
    })
}

/// A published path this module owns.
///
/// ⚠ The constants are the only inputs, so a failure here is a defect in this
/// file rather than in a store, which is why it panics rather than growing a
/// refusal a caller would have to handle.
fn path(text: &str) -> RelPath {
    RelPath::parse(text).expect("a published format path is canonical")
}

/// The array of canonical documents, each verbatim.
fn combined_json(documents: &[(&Profile, String, serde_json::Value)]) -> String {
    let mut out = String::from("[\n");
    for (index, (_, document, _)) in documents.iter().enumerate() {
        // ⚠ The canonical document ends in a newline and the separator supplies
        // its own, so the trailing one is trimmed rather than doubled.
        out.push_str(document.trim_end_matches('\n'));
        if index + 1 < documents.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("]\n");
    out
}

/// One compact document per line.
fn jsonl(documents: &[(&Profile, String, serde_json::Value)]) -> String {
    let mut out = String::new();
    for (_, _, value) in documents {
        // ⚠ serde_json's compact writer is deterministic over a `Value` whose
        // maps preserve insertion order, and the value was parsed from the
        // canonical document, so the order is the canonical one.
        out.push_str(&value.to_string());
        out.push('\n');
    }
    out
}

/// The tabular view, read out of the canonical documents by pointer.
fn csv(documents: &[(&Profile, String, serde_json::Value)]) -> Result<String, Vec<SchemaError>> {
    let mut errors = Vec::new();
    let mut out = String::new();
    out.push_str(&CSV_COLUMNS.join(","));
    out.push_str("\r\n");
    for (profile, _, value) in documents {
        let mut cells = Vec::with_capacity(CSV_POINTERS.len());
        for (column, pointer) in CSV_COLUMNS.iter().zip(CSV_POINTERS) {
            if let Some(text) = value.pointer(pointer).and_then(serde_json::Value::as_str) {
                cells.push(quote(text));
            } else {
                errors.push(SchemaError::new(
                    "E-FMT-05",
                    format!("csv {column}"),
                    format!("{pointer} is not a string in {}", profile.id),
                ));
                cells.push(String::new());
            }
        }
        out.push_str(&cells.join(","));
        out.push_str("\r\n");
    }
    if errors.is_empty() {
        Ok(out)
    } else {
        Err(errors)
    }
}

/// RFC 4180 quoting, applied only where it is needed.
///
/// ⚠ Every value here is canonical and none can currently contain a comma, a
/// quote or a newline. The quoting is not decoration: a `Version` is whatever
/// the build printed, and `canonical.rs` deliberately does not impose a grammar
/// on one.
fn quote(cell: &str) -> String {
    if cell.contains([',', '"', '\r', '\n']) {
        format!("\"{}\"", cell.replace('"', "\"\""))
    } else {
        cell.to_owned()
    }
}

/// What the tabular view carries and what it leaves out.
fn columns_document() -> String {
    let mut out = String::new();
    out.push_str("{\n  \"schema\": \"");
    out.push_str(FORMATS_SCHEMA);
    out.push_str("\",\n  \"file\": \"");
    out.push_str(CSV_FILE);
    out.push_str("\",\n  \"columns\": [\n");
    for (index, column) in CSV_COLUMNS.iter().enumerate() {
        let _ = write!(out, "    \"{column}\"");
        if index + 1 < CSV_COLUMNS.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ],\n  \"omits\": [\n");
    for (index, section) in CSV_OMITS.iter().enumerate() {
        let _ = write!(out, "    \"{section}\"");
        if index + 1 < CSV_OMITS.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n}\n");
    out
}

// -- deterministic CBOR -------------------------------------------------------
//
// ⛔ WRITTEN HERE RATHER THAN TAKEN FROM A CRATE, for the reason
// `docs/supply-chain.md` gives about the wire codecs: the published bytes are
// what a digest names, so the encoder has to be one this project can read. The
// subset needed is small and closed, because the input is a JSON document and
// not an arbitrary value.
//
// RFC 8949 section 4.2.1, core deterministic encoding:
//   - definite lengths everywhere;
//   - the shortest form for every integer and every length;
//   - map keys sorted by their own encoded bytes, bytewise.
//
// ⚠ A JSON document has no floats in this model and a float has the one
// encoding rule this subset would get subtly wrong, so one is refused rather
// than guessed at.

/// Writes the head of one CBOR item: a major type and its argument.
fn write_head(major: u8, argument: u64, out: &mut Vec<u8>) {
    let high = major << 5;
    match argument {
        0..=23 => out.push(high | u8::try_from(argument).expect("below 24")),
        24..=0xff => {
            out.push(high | 0x18);
            out.push(u8::try_from(argument).expect("below 256"));
        }
        0x100..=0xffff => {
            out.push(high | 0x19);
            out.extend_from_slice(&u16::try_from(argument).expect("below 65536").to_be_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            out.push(high | 0x1a);
            out.extend_from_slice(&u32::try_from(argument).expect("below 2^32").to_be_bytes());
        }
        _ => {
            out.push(high | 0x1b);
            out.extend_from_slice(&argument.to_be_bytes());
        }
    }
}

fn write_text(text: &str, out: &mut Vec<u8>) {
    write_head(3, text.len() as u64, out);
    out.extend_from_slice(text.as_bytes());
}

fn write_value(value: &serde_json::Value, out: &mut Vec<u8>) -> Result<(), SchemaError> {
    match value {
        serde_json::Value::Null => out.push(0xf6),
        serde_json::Value::Bool(false) => out.push(0xf4),
        serde_json::Value::Bool(true) => out.push(0xf5),
        serde_json::Value::Number(number) => {
            if let Some(unsigned) = number.as_u64() {
                write_head(0, unsigned, out);
            } else if let Some(signed) = number.as_i64() {
                // ⚠ CBOR encodes a negative integer as -1 minus the argument,
                // so the argument is one less than the magnitude.
                let magnitude = signed
                    .checked_neg()
                    .and_then(|value| u64::try_from(value).ok())
                    .and_then(|value| value.checked_sub(1));
                match magnitude {
                    Some(argument) => write_head(1, argument, out),
                    None => {
                        return Err(SchemaError::new(
                            "E-FMT-03",
                            "cbor",
                            format!("{number} has no deterministic encoding in this subset"),
                        ));
                    }
                }
            } else {
                return Err(SchemaError::new(
                    "E-FMT-03",
                    "cbor",
                    format!(
                        "{number} is not an integer, and a float's shortest form is a rule this subset does not hold"
                    ),
                ));
            }
        }
        serde_json::Value::String(text) => write_text(text, out),
        serde_json::Value::Array(items) => {
            write_head(4, items.len() as u64, out);
            for item in items {
                write_value(item, out)?;
            }
        }
        serde_json::Value::Object(entries) => {
            // ⛔ SORTED BY THE ENCODED KEY, NOT BY THE KEY. RFC 8949 orders map
            // keys by their own CBOR bytes, which for text differs from ordering
            // the strings whenever two keys differ in length: the length is part
            // of the head. Sorting the strings would produce a document that
            // reads correctly and is not the deterministic encoding.
            let mut pairs: Vec<(Vec<u8>, &serde_json::Value)> = Vec::with_capacity(entries.len());
            for (key, value) in entries {
                let mut encoded = Vec::new();
                write_text(key, &mut encoded);
                pairs.push((encoded, value));
            }
            pairs.sort_by(|left, right| left.0.cmp(&right.0));
            write_head(5, pairs.len() as u64, out);
            for (key, value) in pairs {
                out.extend_from_slice(&key);
                write_value(value, out)?;
            }
        }
    }
    Ok(())
}

fn write_cbor_array(
    documents: &[(&Profile, String, serde_json::Value)],
    out: &mut Vec<u8>,
) -> Result<(), SchemaError> {
    write_head(4, documents.len() as u64, out);
    for (_, _, value) in documents {
        write_value(value, out)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        CBOR_FILE, COLUMNS_FILE, CSV_COLUMNS, CSV_FILE, CSV_OMITS, CSV_POINTERS, FORMATS_SCHEMA,
        JSON_FILE, JSONL_FILE, render, write_value,
    };
    use crate::index::build;
    use crate::index::tests::{corpus_of, correction_at, record_at, schemes};

    /// ⛔ Literals, not the constants that spell them. Every one of these is in
    /// a document or a header row a consumer parses, so a rename is a schema
    /// change rather than a rename.
    #[test]
    fn the_published_names_have_one_spelling() {
        assert_eq!(FORMATS_SCHEMA, "bit-ids/formats/1");
        assert_eq!(JSON_FILE, "formats/bit-ids-v1.json");
        assert_eq!(JSONL_FILE, "formats/bit-ids-v1.jsonl");
        assert_eq!(CSV_FILE, "formats/bit-ids-v1.csv");
        assert_eq!(CBOR_FILE, "formats/bit-ids-v1.cbor");
        assert_eq!(COLUMNS_FILE, "formats/bit-ids-v1.columns.json");
        assert_eq!(
            CSV_COLUMNS.join(","),
            "record,schema,target,target_kind,version,platform,arch,package,executable,capture,\
             captured_at"
        );
    }

    /// ⛔ The omitted list is a claim about the columns and nothing checked it.
    /// A section named in `CSV_OMITS` that a column actually reads would be a
    /// published document telling a consumer it is missing something it has.
    #[test]
    fn the_omitted_sections_are_ones_no_column_reads() {
        assert_eq!(CSV_COLUMNS.len(), CSV_POINTERS.len());
        for section in CSV_OMITS {
            let touched = CSV_POINTERS
                .iter()
                .any(|pointer| pointer.starts_with(&format!("/{section}")));
            assert!(
                !touched,
                "a column reads {section}, which is listed as omitted"
            );
        }
        // And the other direction: every pointer names a top-level section that
        // is not in the omitted list.
        for pointer in CSV_POINTERS {
            let section = pointer
                .trim_start_matches('/')
                .split('/')
                .next()
                .unwrap_or("");
            assert!(
                !CSV_OMITS.contains(&section),
                "{pointer} reads {section}, which is listed as omitted"
            );
        }
    }

    /// ⛔ THE DOOR THIS MODULE SHARES WITH `CORPUS-04`. A corrected record is
    /// out of the lookups; a rendering that filtered the store on its own would
    /// still publish it, and the tabular view is the one a reader is least
    /// likely to cross-check.
    #[test]
    fn a_superseded_record_is_in_no_rendering() {
        let original = record_at("1.2.3");
        let fix = correction_at("1.2.3", "cap-re-run", original.id);
        let corpus = corpus_of(vec![original.clone(), fix.clone()]);
        let indexes = build(&corpus, &schemes()).expect("a store with one correction");
        let formats = render(&corpus, &indexes).expect("the store renders");

        assert_eq!(formats.records, 1, "one record is current");
        let corrected = original.id.to_string();
        let current = fix.id.to_string();

        let file = |name: &str| -> Vec<u8> {
            let found = formats.files.iter().find(|(path, _)| path.as_str() == name);
            match found {
                Some((_, bytes)) => bytes.clone(),
                None => panic!("{name} was rendered"),
            }
        };

        // ⚠ NOT "the identifier appears nowhere", which is false by design: a
        // correction names what it corrects, so the corrected identifier is in
        // the published bytes as the value of `supersedes`. The rule is that it
        // is not published AS A RECORD, so the identifiers are compared rather
        // than the text searched. The first version of this test asserted the
        // stronger thing and failed correctly.
        let published: Vec<String> =
            serde_json::from_slice::<Vec<serde_json::Value>>(&file(JSON_FILE))
                .expect("the combined document parses")
                .iter()
                .map(|record| {
                    record
                        .pointer("/id")
                        .and_then(serde_json::Value::as_str)
                        .expect("every record carries an id")
                        .to_owned()
                })
                .collect();
        assert_eq!(published, vec![current.clone()]);

        let table = String::from_utf8(file(CSV_FILE)).expect("the table is text");
        let ids: Vec<&str> = table
            .lines()
            .skip(1)
            .filter_map(|line| line.split(',').next())
            .collect();
        assert_eq!(ids, vec![current.as_str()]);
        assert!(
            !table.contains(&corrected),
            "the table has no supersedes column, so the corrected id has no business in it"
        );

        // ⚠ The CBOR is not decoded here, because decoding it with this
        // project's own encoder would be checking the writer against itself.
        // `check-formats.sh` hands it to `cbor2` and compares the identifiers.
        assert!(!file(CBOR_FILE).is_empty());
        assert!(!file(JSONL_FILE).is_empty());
        assert!(!file(COLUMNS_FILE).is_empty());
    }

    /// Two renders of one store are byte-identical, which is what a published
    /// digest needs and what a map's iteration order would quietly break.
    #[test]
    fn two_renders_are_byte_identical() {
        let corpus = corpus_of(vec![record_at("1.2.3"), record_at("1.3.0")]);
        let indexes = build(&corpus, &schemes()).expect("a two-record store");
        let first = render(&corpus, &indexes).expect("the store renders");
        let second = render(&corpus, &indexes).expect("the store renders again");
        assert_eq!(first, second);
        assert_eq!(first.records, 2, "an empty render proves nothing");
    }

    /// ⛔ THE RULE MOST EASILY GOT WRONG, AND THE ONE A NAIVE TEST MISSES.
    /// RFC 8949 orders map keys by their own encoded bytes, and for text keys
    /// that differs from ordering the strings whenever two keys differ in
    /// length, because the length lives in the head. These two are the smallest
    /// pair where the two orders disagree: as strings `aa` sorts first, and
    /// encoded, `z` does.
    #[test]
    fn cbor_map_keys_are_sorted_by_their_encoded_bytes() {
        let value: serde_json::Value =
            serde_json::from_str(r#"{"aa": 1, "z": 2}"#).expect("a small object");
        let mut out = Vec::new();
        write_value(&value, &mut out).expect("no float in it");
        assert_eq!(
            out,
            vec![
                0xa2, // map of 2
                0x61, b'z', 0x02, // "z" first, because its head is shorter
                0x62, b'a', b'a', 0x01,
            ],
            "the keys are ordered as strings rather than as encoded bytes"
        );
    }

    /// ⚠ A float has one deterministic-encoding rule this subset does not hold,
    /// so it is refused rather than written as something plausible.
    #[test]
    fn a_float_has_no_encoding_here() {
        let value: serde_json::Value = serde_json::from_str("1.5").expect("a float");
        let mut out = Vec::new();
        let error = write_value(&value, &mut out).expect_err("a float is refused");
        assert_eq!(error.code(), "E-FMT-03");
    }

    /// ⛔ Rendering nothing is refused rather than published as an empty table.
    /// A consumer cannot tell an empty catalogue from a broken run.
    #[test]
    fn a_store_with_nothing_publishable_is_refused() {
        let mut record = record_at("1.2.3");
        crate::index::tests::make_provisional(&mut record);
        let corpus = corpus_of(vec![record]);
        let indexes = build(&corpus, &schemes()).expect("a provisional store still indexes");
        let violations = render(&corpus, &indexes).expect_err("nothing to publish");
        assert!(violations.has("E-FMT-04"), "{violations}");
    }
}
