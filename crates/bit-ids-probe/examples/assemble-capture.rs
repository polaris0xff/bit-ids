//! Assemble the artifacts a dispatched capture left into records in a store.
//!
//! ```text
//! cargo run -p bit-ids-probe --example assemble-capture -- \
//!     --lane CAPTURE_DIR,INSTALL_DIR [--lane CAPTURE_DIR,INSTALL_DIR]... STORE_DIR
//! ```
//!
//! `CI-09`. A dispatched `capture-client` run uploads two artifacts per lane:
//! the capture bundle, which carries the attestation, the generated metainfo and
//! one transcript per surface, and the install artifact, which carries the
//! install record, the release resolution and the bytes the build printed when
//! it was asked its version. This reads a lane of each and writes the
//! [`Profile`] they support.
//!
//! ⚠ **It does NOT write the [`RunManifest`](bit_ids::RunManifest) that has to
//! sit beside a record**, and that is a declared gap rather than an oversight. A
//! manifest carries the run's ordered phases, both clocks, the sampling plan and
//! the host facts; an attestation carries a start, a finish, a platform string
//! and a claim fingerprint, and inventing the rest would be a document `bind`
//! then compares against a record that agrees with it for no reason. `CI-09`
//! carries it.
//!
//! # ⛔ Every field is derived from an artifact, and a field with no artifact is
//! a refusal
//!
//! **Nothing here invents a value to get a record past a gate.** That matters
//! most for the two fields `E-ACQ-07` and `E-ACQ-08` compare: the resolver is
//! read out of the resolution document's own source identifier and the delivery
//! out of the route the adapter ran, so two lanes that really did resolve
//! through one listing produce two routes naming one resolver and are refused.
//! ⚠ Deriving a resolver from the *lane* instead would manufacture exactly the
//! independence those codes exist to check, and the record would look correct.
//!
//! # ⛔ One capture is one record, and both routes are in both records
//!
//! `E-ACQ-01` needs two routes and `Capture::observed_route` names which one's
//! install was put on the wire. So two lanes of one target and one version are
//! **two** records over the **same** route list, differing in their capture and
//! in which route each watched - which is the pair
//! [`classify_across`](bit_ids::classify_across) compares, and the only input
//! shape that can reach `BuildEquivalent` rather than `Unresolved`.
//!
//! # What it prints
//!
//! Every refusal, with its code, the document it came from and the artifact the
//! value was read out of. A run that cannot produce a record says exactly what
//! the capture path would have to record for it to.
//!
//! Exit codes: 0 the store was written, 1 a record was refused, 2 an artifact
//! could not be read.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use bit_ids::Agreement;
use bit_ids::acquisition::{AcquisitionRoute, RouteKind, SignatureStatus, SourceIdentity};
use bit_ids::agreement::{ConnectorObservation, FieldCorroboration, Projection, SeenValue};
use bit_ids::canonical::{HexBytes, Instant, Label, RelPath, Sha256Digest, Slug, Url, Version};
use bit_ids::identity::{RecordId, RecordKey, SchemaVersion};
use bit_ids::observation::{ConstantValue, FieldPath, FieldState, ObservedField};
use bit_ids::record::{
    Build, Capture, Connector, EvidenceKind, EvidenceRef, Profile, Target, TargetKind,
};
use bit_ids::store::StoreKey;
use bit_ids::{PROFILE_SCHEMA, ReleaseChannel};

/// A key-value record of the shape every `bit-ids/*/1` text document uses.
///
/// ⚠ Read rather than parsed as JSON on purpose: `attestation.txt`,
/// `install-<route>.txt` and `resolution.txt` are all written by shell as
/// `key=value` lines, and a reader that accepted more shapes than the writers
/// emit would accept an artifact this project did not produce.
type Record = BTreeMap<String, String>;

fn read_record(path: &Path) -> Result<Record, String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut out = Record::new();
    for line in text.lines() {
        // ⚠ The banner line carries no `=` and is the document's own schema
        // name. Skipping it silently would accept a document with no banner, so
        // the callers below check the banner they expect.
        if let Some((key, value)) = line.split_once('=') {
            out.insert(key.to_owned(), value.to_owned());
        }
    }
    if !text.starts_with("bit-ids/") {
        return Err(format!(
            "{}: does not begin with a bit-ids document banner",
            path.display()
        ));
    }
    Ok(out)
}

fn need<'a>(record: &'a Record, key: &str, source: &Path) -> Result<&'a str, String> {
    record
        .get(key)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{}: carries no {key}", source.display()))
}

/// One dispatched lane: what it installed and what it then observed.
struct Lane {
    /// The route slug the adapter ran, as the install record spells it.
    route: String,
    /// The install record, from the capture bundle's own copy.
    install: Record,
    /// The release resolution, when the route had one.
    resolution: Option<Record>,
    /// The capture attestation.
    attestation: Record,
    /// Where the capture bundle's evidence sits.
    bundle: PathBuf,
    /// The bytes the build printed when asked its version.
    version_output: PathBuf,
}

impl Lane {
    fn read(capture_dir: &Path, install_dir: &Path) -> Result<Self, String> {
        let attestation_path = capture_dir.join("capture/attestation.txt");
        let attestation = read_record(&attestation_path)?;
        // ⛔ The route is taken off the install record's own field rather than
        // off the artifact's directory name. A directory is named by whoever
        // unpacked it; the record is written by the step that ran the route.
        let mut found = None;
        for entry in std::fs::read_dir(capture_dir)
            .map_err(|error| format!("{}: {error}", capture_dir.display()))?
        {
            let path = entry.map_err(|error| error.to_string())?.path();
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            // ⚠ The extension is asked for rather than matched off the end of
            // the name, so `install-release.TXT` is the same file to this reader
            // as it is to a case-insensitive filesystem.
            let is_text = path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("txt"));
            if name.starts_with("install-") && is_text {
                found = Some(path);
            }
        }
        let Some(install_path) = found else {
            return Err(format!(
                "{}: carries no install-<route>.txt, so nothing says what acquired the build",
                capture_dir.display()
            ));
        };
        let install = read_record(&install_path)?;
        let route = need(&install, "route", &install_path)?.to_owned();

        let resolution_path = install_dir.join("release/resolution.txt");
        let resolution = if resolution_path.is_file() {
            Some(read_record(&resolution_path)?)
        } else {
            None
        };

        let version_output = install_dir.join(format!("install-{route}/version.err"));
        if !version_output.is_file() {
            // ⛔ `E-ACQ-10` needs the bytes the build printed, and they are in
            // the INSTALL artifact rather than in the capture bundle. A lane
            // handed only its capture cannot produce a record at all.
            return Err(format!(
                "{}: absent. `E-ACQ-10` needs what the build printed when asked its version, and \
                 that is in the install artifact rather than in the capture bundle",
                version_output.display()
            ));
        }
        Ok(Self {
            route,
            install,
            resolution,
            attestation,
            bundle: capture_dir.join("capture/bundle"),
            version_output,
        })
    }

    /// The slug identifying what decided this route's version.
    ///
    /// ⛔ **Read out of the resolution document's own source identifier**, so
    /// two lanes that consulted one listing name one resolver and `E-ACQ-07`
    /// refuses the pair. A resolver named after the lane would be this
    /// project's own tooling manufacturing the independence the rule checks.
    fn resolver(&self) -> Result<Slug, String> {
        match &self.resolution {
            Some(record) => {
                let url = need(record, "source_url", Path::new("release/resolution.txt"))?;
                slugify_source(url)
            }
            // ⚠ A route with no resolution document consulted the host's own
            // index, which is a different resolver from any listing and is
            // named as one rather than left blank.
            None => Ok(slug("host-package-index")),
        }
    }

    /// The slug identifying what delivered this route's bytes.
    fn delivery(&self) -> Slug {
        match self.route.as_str() {
            "release" => slug("https-release-asset"),
            "source" => slug("git-clone-and-local-build"),
            _ => slug("host-package-manager"),
        }
    }

    fn kind(&self) -> RouteKind {
        match self.route.as_str() {
            "release" => RouteKind::GithubRelease,
            "source" => RouteKind::SourceBuild,
            _ => RouteKind::LinuxPackageManager,
        }
    }

    fn route_id(&self) -> Slug {
        slug(&format!("route-{}", self.route))
    }

    /// Every connector the attestation says observed the run.
    ///
    /// ⛔ **The observer is one of them, not a third thing beside them.** The
    /// attestation names it in `observer`; a second connector appears in
    /// `connectors`, comma-separated, and no capture has yet written that key.
    /// `OBS-07` is what produces one.
    fn connectors(&self) -> Result<Vec<Connector>, String> {
        let version = Version::parse("0.0.0").map_err(|error| error.to_string())?;
        let mut ids = vec![slug("bit-ids-probe")];
        for extra in self
            .attestation
            .get("connectors")
            .map(String::as_str)
            .unwrap_or_default()
            .split(',')
            .filter(|text| !text.is_empty())
        {
            let id = Slug::parse(extra.trim())
                .map_err(|error| format!("a connector named {extra:?}: {error}"))?;
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        ids.sort_by_key(std::string::ToString::to_string);
        Ok(ids
            .into_iter()
            .map(|id| Connector {
                id,
                version: version.clone(),
            })
            .collect())
    }

    fn evidence_id(&self, suffix: &str) -> Slug {
        slug(&format!("ev-{}-{suffix}", self.route))
    }
}

fn slug(text: &str) -> Slug {
    Slug::parse(text).expect("a derived identifier is canonical by construction")
}

/// A resolver identifier derived from the endpoint a resolution actually read.
///
/// ⛔ **The query string is dropped and everything else is kept.** A query
/// carries paging and per-request parameters, which are properties of one call
/// rather than of the index; the host and the path are what say *which* index
/// answered. Two routes that read one endpoint therefore slugify to one name and
/// `E-ACQ-07` refuses them, which is the whole point of deriving this rather
/// than naming it.
///
/// ⚠ Capped at [`Slug::MAX_LEN`] from the LEFT, so a long path cannot make two
/// different endpoints collide on a shared prefix - the host is the part most
/// likely to be shared, so the tail is what is kept when something must go.
fn slugify_source(url: &str) -> Result<Slug, String> {
    let without_query = url.split('?').next().unwrap_or(url);
    let mut out = String::new();
    for byte in without_query.to_ascii_lowercase().bytes() {
        let ch = char::from(byte);
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            out.push(ch);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    let capped = if trimmed.len() > Slug::MAX_LEN {
        trimmed[trimmed.len() - Slug::MAX_LEN..].trim_matches('-')
    } else {
        trimmed
    };
    Slug::parse(capped).map_err(|error| format!("a resolver derived from {url:?}: {error}"))
}

/// The immutable identity of what a route asked for.
///
/// ⛔ **A source route needs a full commit and no record the capture path
/// writes carries one.** `E-ACQ-06` refuses an abbreviation, the install record
/// has no field for it, and the object name survives only inside the git
/// chatter in `install.log`. That is a gap in what the capture records, and the
/// refusal here is what names it.
fn source_identity(lane: &Lane) -> Result<SourceIdentity, String> {
    let resolution = lane.resolution.as_ref();
    match lane.kind() {
        RouteKind::GithubRelease => {
            let record = resolution.ok_or_else(|| {
                "the release route has no resolution document, so nothing names the asset it took"
                    .to_owned()
            })?;
            let path = Path::new("release/resolution.txt");
            Ok(SourceIdentity::ReleaseAsset {
                repository: label(need(record, "repository", path)?)?,
                tag: label(need(record, "selected_tag", path)?)?,
                asset: label(need(record, "asset", path)?)?,
            })
        }
        RouteKind::SourceBuild => {
            let record =
                resolution.ok_or_else(|| "the source route has no resolution".to_owned())?;
            let path = Path::new("release/resolution.txt");
            let commit = lane
                .install
                .get("source_commit")
                .map(String::as_str)
                .unwrap_or_default();
            if commit.is_empty() {
                return Err(
                    "the source route records no `source_commit`, and `E-ACQ-06` refuses a source \
                     identity without a full object name. `install-client` writes the install \
                     record and does not carry one; the commit exists only in the git output \
                     inside install.log"
                        .to_owned(),
                );
            }
            Ok(SourceIdentity::SourceCommit {
                repository: label(need(record, "repository", path)?)?,
                commit: HexBytes::parse(commit)
                    .map_err(|error| format!("source_commit: {error}"))?,
            })
        }
        _ => Err(format!(
            "route {} has no source identity this example derives",
            lane.route
        )),
    }
}

fn label(text: &str) -> Result<Label, String> {
    Label::parse(text).map_err(|error| format!("{text:?}: {error}"))
}

fn route_of(lane: &Lane, version: &Version) -> Result<AcquisitionRoute, String> {
    let install = Path::new("install-<route>.txt");
    // ⚠ THE ARTIFACT DIGEST IS THE INSTALLED EXECUTABLE'S, WHICHEVER ROUTE RAN.
    // The resolution records the asset URL and the size the source *declared*,
    // and nothing the capture uploads carries a digest of the bytes that
    // arrived. That is a narrower claim than the field's name suggests, it is
    // what the evidence supports, and `ACQ-05` is where a retrieval digest would
    // come from.
    let artifact = need(&lane.install, "installed_binary_sha256", install)?;
    let origin = match &lane.resolution {
        Some(record) => need(record, "asset_url", Path::new("release/resolution.txt"))?.to_owned(),
        None => return Err("a package route has no origin URL in the record".to_owned()),
    };
    Ok(AcquisitionRoute {
        id: lane.route_id(),
        kind: lane.kind(),
        resolver: lane.resolver()?,
        delivery: lane.delivery(),
        origin: Url::parse(&origin).map_err(|error| format!("origin: {error}"))?,
        source: source_identity(lane)?,
        resolved_version: version.clone(),
        artifact: Sha256Digest::parse(&format!("sha256:{artifact}"))
            .map_err(|error| format!("artifact: {error}"))?,
        signature: SignatureStatus::NotChecked,
        installed_probe: Label::parse("aria2-next --version").map_err(|error| error.to_string())?,
        installed_evidence: lane.evidence_id("installed-version"),
        installed_executable: Sha256Digest::parse(&format!(
            "sha256:{}",
            need(&lane.install, "installed_binary_sha256", install)?
        ))
        .map_err(|error| format!("installed_executable: {error}"))?,
        installed_version: version.clone(),
    })
}

/// The evidence one lane contributes, with every size and digest read off disk.
///
/// ⛔ The digests are computed here rather than copied out of the bundle's
/// `SHA256SUMS`, because a record that repeated a document's own claim about
/// its bytes would agree with it whatever the bytes are.
fn evidence_of(lane: &Lane) -> Result<Vec<(EvidenceRef, PathBuf)>, String> {
    let mut out = Vec::new();
    let mut add = |id: Slug, kind: EvidenceKind, rel: &str, path: PathBuf| -> Result<(), String> {
        let bytes = std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        if bytes.is_empty() {
            return Err(format!("{}: has no bytes", path.display()));
        }
        out.push((
            EvidenceRef {
                id,
                kind,
                path: RelPath::parse(rel).map_err(|error| format!("{rel}: {error}"))?,
                bytes: bytes.len() as u64,
                sha256: Sha256Digest::of(&bytes),
                connector: match kind {
                    EvidenceKind::Metainfo | EvidenceKind::ProcessOutput => None,
                    _ => Some(slug("bit-ids-probe")),
                },
            },
            path,
        ));
        Ok(())
    };
    add(
        lane.evidence_id("metainfo"),
        EvidenceKind::Metainfo,
        &format!("{}/fixture/generated.torrent", lane.route),
        lane.bundle.join("fixture/generated.torrent"),
    )?;
    add(
        lane.evidence_id("tracker"),
        EvidenceKind::TrackerCapture,
        &format!("{}/tracker-http.transcript.json", lane.route),
        lane.bundle.join("tracker-http.transcript.json"),
    )?;
    add(
        lane.evidence_id("peer"),
        EvidenceKind::PeerTranscript,
        &format!("{}/peer-wire-dialled.transcript.json", lane.route),
        lane.bundle.join("peer-wire-dialled.transcript.json"),
    )?;
    add(
        lane.evidence_id("installed-version"),
        EvidenceKind::ProcessOutput,
        &format!("{}/version.err", lane.route),
        lane.version_output.clone(),
    )?;
    out.sort_by_key(|entry| entry.0.id.to_string());
    Ok(out)
}

/// What the transcripts say this build put on the wire.
///
/// ⛔ **Every value is decoded with `bit-ids-wire`'s own codecs**, which are the
/// project's one reading of these formats. A second parser written here would
/// be a second reading, and `FOUND-03` records what that costs.
///
/// ⚠ **`constant` with one sample is the honest state and `patterned` is not
/// available.** `E-OBS` refuses a pattern claiming bytes changed between
/// samples over fewer than two, and this record rests on one capture. Two
/// records of two lanes are what a later sampling pass compares.
fn observations_of(lane: &Lane) -> Result<Vec<ObservedField>, String> {
    use bit_ids_lab::evidence::parse_transcript_document;
    use bit_ids_wire::peer_wire::Handshake;
    use bit_ids_wire::tracker_http::HttpRequest;
    use bit_ids_wire::tracker_udp::Direction;

    let mut out = Vec::new();
    let tracker = std::fs::read_to_string(lane.bundle.join("tracker-http.transcript.json"))
        .map_err(|error| format!("the tracker transcript: {error}"))?;
    let (_, tracker) = parse_transcript_document(&tracker)
        .map_err(|error| format!("the tracker transcript: {error}"))?;
    let announce = tracker
        .segments()
        .iter()
        .find(|segment| segment.direction() == Direction::FromTarget)
        .ok_or_else(|| "the tracker transcript carries nothing the build sent".to_owned())?;
    let request = HttpRequest::parse(announce.bytes())
        .map_err(|error| format!("the announce is not an HTTP request: {error}"))?;

    let peer_id = request
        .query_values(b"peer_id")
        .first()
        .ok_or_else(|| "the announce carries no peer_id".to_owned())?
        .decoded_value()
        .map_err(|error| format!("the announce peer_id: {error}"))?
        .ok_or_else(|| "the announce peer_id has no value".to_owned())?;
    out.push(constant("tracker_http/peer_id", &peer_id, lane, "tracker")?);

    let agent = request
        .headers()
        .iter()
        .find(|header| header.name().eq_ignore_ascii_case(b"user-agent"))
        .ok_or_else(|| "the announce carries no User-Agent".to_owned())?;
    out.push(constant(
        "tracker_http/user_agent",
        agent.value(),
        lane,
        "tracker",
    )?);

    let peer = std::fs::read_to_string(lane.bundle.join("peer-wire-dialled.transcript.json"))
        .map_err(|error| format!("the peer transcript: {error}"))?;
    let (_, peer) = parse_transcript_document(&peer)
        .map_err(|error| format!("the peer transcript: {error}"))?;
    let answered = peer
        .segments()
        .iter()
        .find(|segment| segment.direction() == Direction::FromTarget)
        .ok_or_else(|| "the peer transcript carries nothing the build sent".to_owned())?;
    let handshake = Handshake::parse_prefix(answered.bytes())
        .map_err(|error| format!("the peer handshake: {error}"))?
        .0;
    out.push(constant(
        "peer_wire/peer_id",
        handshake.peer_id(),
        lane,
        "peer",
    )?);
    out.push(constant(
        "peer_wire/reserved",
        handshake.reserved(),
        lane,
        "peer",
    )?);

    out.sort_by_key(|entry| entry.path.to_string());
    Ok(out)
}

/// What each declared connector saw, per field.
///
/// ⛔ **THE SECOND CONNECTOR'S VALUES COME OUT OF ITS OWN ARTIFACT, and this is
/// the contract `OBS-07` has to satisfy.** A connector declared in the
/// attestation writes `connector/<id>.txt` into the bundle, one
/// `field_path=value` line per field the observer measured, where the value is
/// lowercase hex, `absent` or `out_of_scope`. A declared connector that reports
/// nothing about a field is refused rather than recorded as silent: `E-COR-07`
/// already says silence is not the same as `not_corroborated`, and a connector
/// this example filled in for would be corroboration the run never produced.
///
/// ⚠ No capture has written one. Every attestation so far names one connector,
/// which is why `E-CAP-01` refuses the record before this is reached.
fn corroboration_of(
    lane: &Lane,
    fields: &[ObservedField],
    connectors: &[Connector],
) -> Result<Vec<FieldCorroboration>, String> {
    let mut reports: BTreeMap<String, Record> = BTreeMap::new();
    for connector in connectors {
        if connector.id.as_str() == "bit-ids-probe" {
            continue;
        }
        let path = lane.bundle.join(format!("connector/{}.txt", connector.id));
        let text = std::fs::read_to_string(&path).map_err(|error| {
            format!(
                "{}: {error}. A declared connector reports what it saw, one field per line",
                path.display()
            )
        })?;
        let mut record = Record::new();
        for line in text.lines() {
            if let Some((key, value)) = line.split_once('=') {
                record.insert(key.to_owned(), value.to_owned());
            }
        }
        reports.insert(connector.id.to_string(), record);
    }

    let mut out = Vec::new();
    for field in fields {
        let mut observations = Vec::new();
        for connector in connectors {
            let seen = if connector.id.as_str() == "bit-ids-probe" {
                match &field.state {
                    FieldState::Constant(value) => SeenValue::Bytes(value.value.clone()),
                    _ => SeenValue::OutOfScope,
                }
            } else {
                let report = &reports[connector.id.as_str()];
                let key = field.path.to_string();
                match report.get(&key).map(String::as_str) {
                    Some("absent") => SeenValue::Absent,
                    Some("out_of_scope") => SeenValue::OutOfScope,
                    Some(hex) => SeenValue::Bytes(
                        HexBytes::parse(hex).map_err(|error| format!("{key}: {error}"))?,
                    ),
                    None => {
                        return Err(format!(
                            "connector {} reports nothing about {key}, and silence is not the \
                             same as not_corroborated",
                            connector.id
                        ));
                    }
                }
            };
            observations.push(ConnectorObservation {
                connector: connector.id.clone(),
                evidence: field
                    .evidence
                    .first()
                    .cloned()
                    .ok_or_else(|| format!("{} cites no evidence", field.path))?,
                projection: Projection::Raw,
                seen,
            });
        }
        observations.sort_by_key(|entry| entry.connector.to_string());
        let in_scope: Vec<&SeenValue> = observations
            .iter()
            .map(|entry| &entry.seen)
            .filter(|seen| seen.in_scope())
            .collect();
        let (agreement, conflict) = match in_scope.as_slice() {
            [] | [_] => (Agreement::NotCorroborated, None),
            [first, rest @ ..] if rest.iter().all(|other| *other == *first) => {
                (Agreement::Exact, None)
            }
            _ => (
                Agreement::Disagrees,
                Some(
                    Label::parse("connectors read different bytes for this field")
                        .map_err(|error| error.to_string())?,
                ),
            ),
        };
        out.push(FieldCorroboration {
            path: field.path.clone(),
            observations,
            agreement,
            conflict,
        });
    }
    out.sort_by_key(|entry| entry.path.to_string());
    Ok(out)
}

fn constant(
    path: &str,
    value: &[u8],
    lane: &Lane,
    evidence: &str,
) -> Result<ObservedField, String> {
    Ok(ObservedField {
        path: FieldPath::parse(path).map_err(|error| format!("{path}: {error}"))?,
        state: FieldState::Constant(ConstantValue {
            value: HexBytes::new(value.to_vec()).map_err(|error| format!("{path}: {error}"))?,
            samples: core::num::NonZeroU32::new(1).expect("one is not zero"),
        }),
        evidence: vec![lane.evidence_id(evidence)],
    })
}

/// The host family, the architecture and the package format the record is filed
/// under.
///
/// ⛔ **THESE THREE ARE IN [`StoreKey`], SO HARDCODING THEM IS THE
/// NON-INJECTIVE-PATH DEFECT `store.rs` EXISTS TO REFUSE.** A path built from
/// fewer components than the identity tuple files two measurements at one name
/// and one of them silently wins. ⚠ The first draft of this example wrote
/// `linux`, `x86-64` and `elf-binary` as literals, which would have filed a
/// Windows capture of the same version at the Linux capture's path.
///
/// ⭐ The first two are derived from the attestation's own `platform` line,
/// which is `uname -srm`: a system name, a kernel release and a machine.
/// ⭐ The third comes from the INSTALL record, because the install is what
/// delivered the build and the adapter is what knows the form. ⚠ Nothing wrote
/// it until 2026-09-09 - not the attestation, not the install record, not the
/// release resolution, and not `catalogue/clients.toml`, which carries
/// `candidate_routes` and no package format - so a lane from an older run is
/// refused here rather than filed under a guess.
fn identity_of(lane: &Lane) -> Result<(Slug, Slug, Slug), String> {
    let attestation = Path::new("capture/attestation.txt");
    let platform_line = need(&lane.attestation, "platform", attestation)?;
    let mut parts = platform_line.split_whitespace();
    let (Some(system), Some(_release), Some(machine)) = (parts.next(), parts.next(), parts.next())
    else {
        return Err(format!(
            "the attestation's platform is {platform_line:?}, which is not `uname -srm` output"
        ));
    };
    let platform = Slug::parse(&system.to_ascii_lowercase())
        .map_err(|error| format!("a platform derived from {system:?}: {error}"))?;
    // ⚠ `uname -m` spells it `x86_64` and this project's vocabulary spells it
    // `x86-64`, so the underscore is mapped rather than the value passed
    // through. A slug cannot hold an underscore at all, so an unmapped machine
    // name is a refusal naming the value rather than a silently wrong path.
    let arch = Slug::parse(&machine.to_ascii_lowercase().replace('_', "-"))
        .map_err(|error| format!("an architecture derived from {machine:?}: {error}"))?;
    let package = match lane.install.get("package").map(String::as_str) {
        Some(text) if !text.is_empty() => {
            Slug::parse(text).map_err(|error| format!("package: {error}"))?
        }
        _ => {
            return Err(
                "the install record carries no `package`, and it is part of the identity tuple a \
                 store path is derived from. The adapter's `install` writes it beside its log \
                 because the adapter is what knows the form; hardcoding one here would file two \
                 packagings of one version at one path"
                    .to_owned(),
            );
        }
    };
    Ok((platform, arch, package))
}

fn profile_of(lanes: &[Lane], watched: usize, version: &Version) -> Result<Profile, String> {
    let lane = &lanes[watched];
    let attestation = Path::new("capture/attestation.txt");
    let (platform, arch, package) = identity_of(lane)?;
    let mut acquisition = Vec::new();
    for other in lanes {
        acquisition.push(route_of(other, version)?);
    }
    acquisition.sort_by_key(|entry| entry.id.to_string());

    let evidence: Vec<EvidenceRef> = lanes
        .iter()
        .map(evidence_of)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .map(|(entry, _)| entry)
        .collect();

    let capture = Capture {
        id: slug(&format!("cap-{}", lane.route)),
        captured_at: Instant::parse(need(&lane.attestation, "started_at", attestation)?)
            .map_err(|error| format!("started_at: {error}"))?,
        fixture: Sha256Digest::parse(need(&lane.attestation, "fixture", attestation)?)
            .map_err(|error| format!("fixture: {error}"))?,
        observed_route: lane.route_id(),
        observer: slug("bit-ids-probe"),
        // ⛔ READ OFF THE ATTESTATION, NEVER PADDED. Every capture this project
        // has run declared its own Rust observer and nothing else, and
        // `E-CAP-01` refuses a record with one connector at the VALIDITY gate -
        // not at publication. Listing a second here to get a record written
        // would be this example inventing the corroboration the whole project
        // exists to measure.
        connectors: lane.connectors()?,
    };

    let mut profile = Profile {
        schema: SchemaVersion::parse(PROFILE_SCHEMA).map_err(|error| error.to_string())?,
        id: RecordId::derive(&RecordKey {
            schema: &SchemaVersion::parse(PROFILE_SCHEMA).map_err(|error| error.to_string())?,
            target: &slug(need(&lane.attestation, "target", attestation)?),
            version,
            platform: &platform,
            arch: &arch,
            package: &package,
            capture: &capture.id,
        }),
        target: Target {
            id: slug(need(&lane.attestation, "target", attestation)?),
            // ⚠ THE IDENTIFIER, BECAUSE NOTHING A CAPTURE UPLOADS CARRIES A
            // DISPLAY NAME. `catalogue/clients.toml` does, and reading it would
            // mean a TOML dependency for one cosmetic field, which
            // `docs/supply-chain.md` would refuse. It is not part of the
            // identity tuple, so unlike the three above it cannot file a record
            // at the wrong path; whatever publishes a record is where a display
            // name should be joined on.
            display_name: need(&lane.attestation, "target", attestation)?.to_owned(),
            kind: match need(&lane.attestation, "kind", attestation)? {
                "client" => TargetKind::Client,
                "library" => TargetKind::Library,
                // ⛔ `kind=fixture` is what `capture.yml` writes, and a fixture
                // is not a measurement of anything. Refusing it here is what
                // stops a fixture bundle becoming a record that reads like one.
                other => {
                    return Err(format!(
                        "the attestation says kind={other}, which is not a target kind. A \
                         `fixture` capture measured no build"
                    ));
                }
            },
            // ⚠ `libtorrent/2.1.1` is in what the build printed, and nothing has
            // measured the relationship. `null` is the honest value until
            // something does.
            engine: None,
        },
        build: Build {
            version: version.clone(),
            channel: ReleaseChannel::Stable,
            platform,
            arch,
            package,
            executable: Sha256Digest::parse(&format!(
                "sha256:{}",
                need(
                    &lane.install,
                    "installed_binary_sha256",
                    Path::new("install-<route>.txt")
                )?
            ))
            .map_err(|error| format!("executable: {error}"))?,
        },
        acquisition,
        capture,
        observations: Vec::new(),
        corroboration: Vec::new(),
        normalizations: Vec::new(),
        evidence,
        supersedes: None,
        adjudication: None,
    };
    profile.observations = observations_of(lane)?;
    profile.corroboration =
        corroboration_of(lane, &profile.observations, &profile.capture.connectors)?;
    profile.evidence.sort_by_key(|entry| entry.id.to_string());
    Ok(profile)
}

/// Everything one lane's artifacts cannot supply, probed independently.
///
/// ⛔ **Each derivation is asked separately so one gap cannot hide another.** The
/// record-building path is a chain of `?`, which reports what it tripped on
/// first and nothing about the rest; this file promises a reader every field the
/// capture path would have to record, and that promise needs a probe per field.
fn lane_gaps(lane: &Lane) -> Vec<String> {
    let mut out = Vec::new();
    if let Err(error) = identity_of(lane) {
        out.push(error);
    }
    match lane.connectors() {
        // ⛔ COUNTED HERE AND NOT ONLY AT VALIDATION. `E-CAP-01` fires inside
        // `validate`, which a lane missing any other field never reaches - so
        // the one gap every capture this project has run shares would be the
        // one a report never mentioned.
        Ok(connectors) if connectors.len() < 2 => out.push(format!(
            "the attestation declares {} connector(s). `E-CAP-01` refuses a record with fewer \
             than two at the VALIDITY gate, so this is not a publication question: name the \
             second in `connectors=` and give it a report in the bundle",
            connectors.len()
        )),
        Ok(_) => {}
        Err(error) => out.push(error),
    }
    if let Err(error) = source_identity(lane) {
        out.push(error);
    }
    if let Err(error) = lane.resolver() {
        out.push(error);
    }
    out
}

/// The one version every lane must have reported, or a refusal naming the pair.
///
/// ⛔ Absolute 4 is checked here rather than trusted: two lanes that reported two
/// versions are two builds, and a record over them would be a claim no capture
/// made. ⭐ This is also why one shared resolution is the wrong fix for that
/// worry - the comparison happens after installation, on what each build said.
fn one_version(lanes: &[Lane]) -> Result<Version, String> {
    let install = Path::new("install-<route>.txt");
    let version = Version::parse(need(&lanes[0].install, "reported_version", install)?)
        .map_err(|error| format!("reported_version: {error}"))?;
    for lane in lanes {
        let reported = need(&lane.install, "reported_version", install)?;
        if reported != version.to_string() {
            return Err(format!(
                "lane {} reported {reported} and lane {} reported {version}; two routes must \
                 resolve one stable version",
                lane.route, lanes[0].route
            ));
        }
    }
    Ok(version)
}

/// What `E-ACQ-07` and `E-ACQ-08` compare, asked of the artifacts.
///
/// ⛔ **It runs before any record is built.** Those codes fire inside `validate`,
/// which is reached only once every route has been assembled - so a lane missing
/// some other field would hide a resolver collision behind its own refusal, and
/// the collision is the more expensive fact to learn late.
fn independence(lanes: &[Lane]) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for (index, lane) in lanes.iter().enumerate() {
        for earlier in &lanes[..index] {
            if earlier.resolver()? == lane.resolver()? {
                out.push(format!(
                    "  ⛔ E-ACQ-07: lanes {} and {} both resolved through {}, so they are one \
                     route. Read out of each lane's own resolution document, which name one \
                     source and carry one listing digest.",
                    earlier.route,
                    lane.route,
                    lane.resolver()?
                ));
            }
            if earlier.delivery() == lane.delivery() {
                out.push(format!(
                    "  ⛔ E-ACQ-08: lanes {} and {} were both delivered by {}",
                    earlier.route,
                    lane.route,
                    lane.delivery()
                ));
            }
        }
    }
    Ok(out)
}

fn run(lanes: &[Lane], out: &Path) -> Result<String, String> {
    let version = one_version(lanes)?;

    let mut report = String::new();
    let mut refusals = 0_usize;
    let mut written = Vec::new();
    for line in independence(lanes)? {
        refusals += 1;
        writeln!(report, "{line}").expect("a String cannot fail");
    }

    for index in 0..lanes.len() {
        // ⛔ EVERY GAP, NOT THE FIRST. This file's own header promises that a run
        // which cannot produce a record says exactly what the capture path would
        // have to record, and a `?` chain says only what it tripped on first.
        // ⚠ Measured on run 14: adding the `package` derivation moved that
        // refusal in front of the commit one, and two gaps a previous run had
        // named silently disappeared from the report.
        let gaps = lane_gaps(&lanes[index]);
        if !gaps.is_empty() {
            for gap in gaps {
                refusals += 1;
                writeln!(report, "  ⛔ lane {}: {gap}", lanes[index].route)
                    .expect("a String cannot fail");
            }
            continue;
        }
        let profile = match profile_of(lanes, index, &version) {
            Ok(profile) => profile,
            Err(error) => {
                refusals += 1;
                writeln!(report, "  ⛔ lane {}: {error}", lanes[index].route)
                    .expect("a String cannot fail");
                continue;
            }
        };
        // ⛔ `to_json` validates, so a record that cannot be written is a record
        // that does not exist rather than one filed under a caveat.
        match profile.to_json() {
            Ok(document) => {
                let key = StoreKey::of_profile(&profile);
                let path = key.profile_path().map_err(|error| error.to_string())?;
                let full = out.join(path.as_str());
                if let Some(parent) = full.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|error| format!("{}: {error}", parent.display()))?;
                }
                std::fs::write(&full, document.as_bytes())
                    .map_err(|error| format!("{}: {error}", full.display()))?;
                for (entry, source) in evidence_of(&lanes[index])? {
                    let rel = key
                        .evidence_path(&entry.path)
                        .map_err(|error| error.to_string())?;
                    let target = out.join(rel.as_str());
                    if let Some(parent) = target.parent() {
                        std::fs::create_dir_all(parent)
                            .map_err(|error| format!("{}: {error}", parent.display()))?;
                    }
                    std::fs::copy(&source, &target)
                        .map_err(|error| format!("{}: {error}", target.display()))?;
                }
                writeln!(report, "  ⭐ lane {}: {}", lanes[index].route, path)
                    .expect("a String cannot fail");
                written.push(profile);
            }
            Err(error) => {
                refusals += 1;
                writeln!(report, "  ⛔ lane {}: refused", lanes[index].route)
                    .expect("a String cannot fail");
                for line in error.to_string().lines() {
                    writeln!(report, "       {line}").expect("a String cannot fail");
                }
            }
        }
    }

    if written.len() == 2 {
        // ⭐ The pair is what `ACQ-03` exists to classify, and this is the first
        // input that can reach `BuildEquivalent` rather than `Unresolved`.
        let comparison = bit_ids::classify_across(&[&written[0], &written[1]]);
        writeln!(report, "  classify_across: {}", comparison.outcome)
            .expect("a String cannot fail");
        for reason in &comparison.reasons {
            writeln!(report, "       {reason}").expect("a String cannot fail");
        }
    }
    if refusals > 0 {
        return Err(report);
    }
    Ok(report)
}

fn main() -> ExitCode {
    let mut lanes = Vec::new();
    let mut positional = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--lane" {
            let Some(pair) = args.next() else {
                let _ = writeln!(std::io::stderr(), "--lane needs CAPTURE_DIR,INSTALL_DIR");
                return ExitCode::from(2);
            };
            let Some((capture, install)) = pair.split_once(',') else {
                let _ = writeln!(std::io::stderr(), "--lane needs CAPTURE_DIR,INSTALL_DIR");
                return ExitCode::from(2);
            };
            lanes.push((PathBuf::from(capture), PathBuf::from(install)));
            continue;
        }
        positional.push(arg);
    }
    let [out] = positional.as_slice() else {
        let _ = writeln!(
            std::io::stderr(),
            "usage: assemble-capture --lane CAPTURE_DIR,INSTALL_DIR ... STORE_DIR"
        );
        return ExitCode::from(2);
    };
    if lanes.len() < 2 {
        let _ = writeln!(
            std::io::stderr(),
            "assemble-capture: {} lane(s). `E-ACQ-01` needs two routes, so a record needs a lane \
             per route",
            lanes.len()
        );
        return ExitCode::from(2);
    }
    let out = Path::new(out);
    if !out.is_dir() {
        let _ = writeln!(std::io::stderr(), "{}: not a directory", out.display());
        return ExitCode::from(2);
    }

    let mut read = Vec::new();
    for (capture, install) in &lanes {
        match Lane::read(capture, install) {
            Ok(lane) => read.push(lane),
            Err(error) => {
                let _ = writeln!(std::io::stderr(), "assemble-capture: {error}");
                return ExitCode::from(2);
            }
        }
    }

    match run(&read, out) {
        Ok(report) => {
            let _ = write!(std::io::stdout(), "{report}");
            let _ = writeln!(
                std::io::stdout(),
                "assemble-capture: {} record(s) written into {}",
                read.len(),
                out.display()
            );
            ExitCode::SUCCESS
        }
        Err(report) => {
            let _ = write!(std::io::stderr(), "{report}");
            let _ = writeln!(
                std::io::stderr(),
                "assemble-capture: no record was written. Every line above is a field the capture \
                 path would have to record."
            );
            ExitCode::from(1)
        }
    }
}
