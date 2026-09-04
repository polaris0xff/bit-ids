//! How a build was obtained, in terms comparable across very different routes.
//!
//! A package manager, a vendor download page, a release asset, a module proxy
//! and a source tree publish nothing in common. What they do have in common is
//! the shape of the claim: something decided which version, something delivered
//! bytes, those bytes have a digest, and the thing that got installed was asked
//! its own version afterwards. That is the record here.
//!
//! ⛔ **A route in [`../../../catalogue/clients.toml`](../../../catalogue/clients.toml)
//! is a research lead, never an availability claim.** `candidate_routes` says
//! somebody expects a route to exist. Only an acquisition record says one
//! resolved, delivered and installed. Nothing in this crate reads the catalogue,
//! and the one test that does reads it as a vocabulary to check [`RouteKind`]
//! against, not as evidence that any route works.
//!
//! ⭐ **Two routes are two routes only if they are independent.** `architecture.md`
//! section 7 puts it exactly: two package-manager aliases pointing at one
//! manifest are one route. So a route records what resolved it and what
//! delivered it as separate values, and the validator refuses a record whose
//! routes share either. Counting them without that check would let the
//! two-route rule be satisfied by asking the same index twice.

use core::fmt;

use serde::{Deserialize, Serialize};

use crate::canonical::{HexBytes, Label, Sha256Digest, Slug, Url, Version};

/// The delivery mechanisms the catalogue names.
///
/// The set is closed, like [`crate::observation::Surface`]: a route shape
/// nobody has modelled is a schema change, not a free-text value a reader has
/// to guess at. `catalogue/clients.toml` names candidates from this vocabulary,
/// and `acquisition_route_kinds_cover_the_catalogue` holds the two in step.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteKind {
    /// A release asset published on a GitHub release.
    GithubRelease,
    /// A file published on the vendor's own download page.
    OfficialDownload,
    /// A release published by the upstream project outside GitHub.
    OfficialRelease,
    /// A package from a Linux distribution index.
    LinuxPackageManager,
    /// A package from a Windows package index.
    WindowsPackageManager,
    /// A build from source at an immutable commit.
    SourceBuild,
    /// A Go module from a module proxy.
    GoModule,
    /// A crate installed from the crates.io registry.
    CargoInstall,
}

impl RouteKind {
    /// Every variant, for a coverage check.
    pub const ALL: &'static [Self] = &[
        Self::GithubRelease,
        Self::OfficialDownload,
        Self::OfficialRelease,
        Self::LinuxPackageManager,
        Self::WindowsPackageManager,
        Self::SourceBuild,
        Self::GoModule,
        Self::CargoInstall,
    ];

    /// The catalogue spelling, hyphenated.
    ///
    /// ⚠ Hyphens, not the underscores the JSON form uses.
    /// `catalogue/clients.toml` is hand-maintained and reads in hyphens, and a
    /// record is machine-written and reads in the schema's snake case. Two
    /// spellings of one vocabulary is exactly the drift this project refuses
    /// elsewhere, so the mapping lives here, once, rather than in the reader.
    #[must_use]
    pub const fn as_catalogue_str(self) -> &'static str {
        match self {
            Self::GithubRelease => "github-release",
            Self::OfficialDownload => "official-download",
            Self::OfficialRelease => "official-release",
            Self::LinuxPackageManager => "linux-package-manager",
            Self::WindowsPackageManager => "windows-package-manager",
            Self::SourceBuild => "source-build",
            Self::GoModule => "go-module",
            Self::CargoInstall => "cargo-install",
        }
    }

    /// Parses the catalogue spelling.
    #[must_use]
    pub fn from_catalogue_str(text: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|kind| kind.as_catalogue_str() == text)
    }

    /// The one source-identity form this kind may carry.
    ///
    /// A release asset identified by a package version, or a package identified
    /// by a commit, is a record whose provenance cannot be followed back. The
    /// pairing is fixed here so a mismatch is a refusal rather than a reading.
    #[must_use]
    pub const fn source_form(self) -> &'static str {
        match self {
            Self::GithubRelease => "release_asset",
            Self::OfficialDownload | Self::OfficialRelease => "published_file",
            Self::LinuxPackageManager | Self::WindowsPackageManager => "indexed_package",
            Self::SourceBuild => "source_commit",
            Self::GoModule | Self::CargoInstall => "module_version",
        }
    }
}

impl fmt::Display for RouteKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_catalogue_str())
    }
}

/// The immutable identity of what a route asked for.
///
/// Immutable is the operative word. A record has to name something a reader can
/// resolve again and get the same bytes, which is a tag plus an asset name, an
/// exact package version, or a whole commit. A branch, a `latest` alias or a
/// download page is not one of those.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "form", content = "detail", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum SourceIdentity {
    /// An asset attached to a named release of a named repository.
    ReleaseAsset {
        /// The upstream repository, as `owner/name`.
        repository: Label,
        /// The release tag the asset hung from.
        tag: Label,
        /// The asset's file name.
        asset: Label,
    },
    /// A file the vendor published, identified by its name at the origin URL.
    PublishedFile {
        /// The file name as published.
        asset: Label,
    },
    /// An exact version of a named package in a named index.
    IndexedPackage {
        /// The index or repository the package came from.
        index: Label,
        /// The package name in that index.
        package: Label,
        /// The exact package version, as the index spells it. ⚠ This is the
        /// index's version, which routinely differs from the version the
        /// installed build reports; `E-ACQ-04` compares the reported one.
        version: Version,
    },
    /// A source tree at one commit.
    SourceCommit {
        /// The upstream repository, as `owner/name`.
        repository: Label,
        /// The full object name. An abbreviation is refused: `FOUND-02` learned
        /// on action pins that a short commit is not an immutable reference.
        commit: HexBytes,
    },
    /// A module or crate version a registry verifies by checksum.
    ModuleVersion {
        /// The registry or proxy that served it.
        registry: Label,
        /// The module or crate path.
        module: Label,
        /// The exact version.
        version: Version,
    },
}

impl SourceIdentity {
    /// The form name, matched against [`RouteKind::source_form`].
    #[must_use]
    pub const fn form(&self) -> &'static str {
        match self {
            Self::ReleaseAsset { .. } => "release_asset",
            Self::PublishedFile { .. } => "published_file",
            Self::IndexedPackage { .. } => "indexed_package",
            Self::SourceCommit { .. } => "source_commit",
            Self::ModuleVersion { .. } => "module_version",
        }
    }

    /// The commit, when this is a source tree.
    #[must_use]
    pub const fn commit(&self) -> Option<&HexBytes> {
        match self {
            Self::SourceCommit { commit, .. } => Some(commit),
            _ => None,
        }
    }
}

/// Whether the artifact's signature was checked, and what happened.
///
/// Lives here rather than beside the manifest because both documents record it
/// and `bind` compares them. One definition is the only way two copies of a
/// value cannot disagree about what its variants mean.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignatureStatus {
    /// A signature was present and verified against a published key.
    Verified,
    /// The publisher ships no signature for this artifact.
    Unsigned,
    /// A signature exists and this run did not check it. ⚠ Not the same as
    /// unsigned, and recorded separately so it cannot be read as one.
    NotChecked,
}

impl SignatureStatus {
    /// The canonical spelling, as it appears in a document.
    ///
    /// ⚠ A diagnostic must quote the spelling the reader can find in the file.
    /// The first version of `E-BND-13` printed the Rust `Debug` form, so it told
    /// an operator the run recorded `Unsigned` over a document that says
    /// `unsigned` and sent them looking for a value that is not written
    /// anywhere. Found by running the validator rather than by reading it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Unsigned => "unsigned",
            Self::NotChecked => "not_checked",
        }
    }
}

impl fmt::Display for SignatureStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One independent way the build was obtained.
///
/// ⛔ **`installed_version` is the value the same-version gate compares, and it
/// is what the installed build said when asked, never what was requested.**
/// `architecture.md` section 7 is explicit: a filename and a package version are
/// not evidence of what got installed. `installed_probe` records how it was
/// asked and `installed_evidence` points at the bytes it answered with, so the
/// claim is replayable rather than asserted.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcquisitionRoute {
    /// Route identifier, unique within the record.
    pub id: Slug,
    /// Which delivery mechanism this is.
    pub kind: RouteKind,
    /// What decided which version to take, such as a release index or a
    /// distribution's package database.
    pub resolver: Slug,
    /// What delivered the bytes. Separate from the resolver because two routes
    /// are independent only when both differ.
    pub delivery: Slug,
    /// Where a reader can go to see the same thing.
    pub origin: Url,
    /// The immutable identity of what was asked for.
    pub source: SourceIdentity,
    /// The version this route's resolver advertised before download.
    pub resolved_version: Version,
    /// Digest of the artifact this route delivered. Two routes may legitimately
    /// deliver different bytes for one version; that is a packaging
    /// observation, not a failure.
    pub artifact: Sha256Digest,
    /// What was done about the signature.
    pub signature: SignatureStatus,
    /// How the installed build was asked its version, as the command run.
    pub installed_probe: Label,
    /// The evidence entry holding what it answered.
    pub installed_evidence: Slug,
    /// Digest of the executable this route installed.
    ///
    /// ⛔ Per route, not per record. Two routes can deliver the same version as
    /// different bytes, and `architecture.md` section 7 is explicit that those
    /// are a packaging observation rather than a failure and are never silently
    /// collapsed. Collapsing them is what a single record-level digest does.
    pub installed_executable: Sha256Digest,
    /// The version the installed build reported when asked. This is the value
    /// the same-version gate compares, never the requested one.
    pub installed_version: Version,
}

#[cfg(test)]
mod tests {
    use super::{RouteKind, SourceIdentity};
    use crate::canonical::{HexBytes, Label};

    #[test]
    fn every_route_kind_round_trips_through_its_catalogue_spelling() {
        for kind in RouteKind::ALL {
            let text = kind.as_catalogue_str();
            assert_eq!(RouteKind::from_catalogue_str(text), Some(*kind));
            assert!(
                !text.contains('_'),
                "the catalogue spelling is hyphenated: {text}"
            );
        }
        assert_eq!(RouteKind::from_catalogue_str("no-such-route"), None);
    }

    #[test]
    fn every_route_kind_names_a_source_form_that_exists() {
        let forms = [
            SourceIdentity::ReleaseAsset {
                repository: Label::parse("owner/name").expect("label"),
                tag: Label::parse("v1.0.0").expect("label"),
                asset: Label::parse("thing.tar.gz").expect("label"),
            },
            SourceIdentity::PublishedFile {
                asset: Label::parse("thing.exe").expect("label"),
            },
            SourceIdentity::IndexedPackage {
                index: Label::parse("arch-extra").expect("label"),
                package: Label::parse("thing").expect("label"),
                version: crate::canonical::Version::parse("1.0.0-1").expect("version"),
            },
            SourceIdentity::SourceCommit {
                repository: Label::parse("owner/name").expect("label"),
                commit: HexBytes::parse(&"ab".repeat(20)).expect("hex"),
            },
            SourceIdentity::ModuleVersion {
                registry: Label::parse("crates.io").expect("label"),
                module: Label::parse("thing").expect("label"),
                version: crate::canonical::Version::parse("1.0.0").expect("version"),
            },
        ];
        let available: Vec<&'static str> = forms.iter().map(SourceIdentity::form).collect();
        for kind in RouteKind::ALL {
            assert!(
                available.contains(&kind.source_form()),
                "{kind} names the form {:?}, which no variant produces",
                kind.source_form()
            );
        }
    }
}
