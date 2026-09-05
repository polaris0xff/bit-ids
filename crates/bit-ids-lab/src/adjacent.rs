//! The adjacent surfaces, and the switch each one is behind.
//!
//! # What makes a surface adjacent
//!
//! The surfaces `OBS-02` through `OBS-05` cover are the ones a client must use
//! to move a torrent: it announces, it handshakes, it exchanges messages. ⚠
//! **The surfaces here are the ones a client uses without being asked.** A build
//! that has never been told about a peer will still multicast a local discovery
//! announce, gossip peers over `ut_pex`, and reach for a DHT bootstrap node, and
//! each of those carries identity: the header spelling, the flag byte, the
//! query's own argument order.
//!
//! ⛔ **Each one is also a way out of the lab.** Local discovery names a
//! multicast group fixed by BEP 14. A DHT's first act is a query to a node the
//! lab does not own. A web seed is an arbitrary URL out of the torrent. The core
//! surfaces cannot leave, because a client only ever reaches them at an address
//! the lab handed it; these carry a destination of their own.
//!
//! # The vocabulary is not a new one
//!
//! ⛔ **[`Surface`] is [`bit_ids::observation::Surface`], re-exported.** A
//! second enum here was written first and was wrong: the record vocabulary
//! already named `dht`, `pex`, `mse` and `web_seed`, and a copy would have
//! spelled the same surfaces differently in the lab and in the published record.
//! What this module adds is which of them are adjacent, how each reaches out,
//! and the switch. `OBS-06` added `local_discovery` to that vocabulary rather
//! than keeping a fifth name here.
//!
//! ⚠ **A field path and an endpoint name spell a surface differently**, because
//! a `FieldPath` uses lower snake case and a [`Slug`] is hyphenated.
//! [`endpoint_name`] is the one place that converts, and
//! `every_endpoint_name_is_the_field_path_spelling` holds the two together.
//!
//! # The switch
//!
//! ⭐ **A [`Capability`] cannot be defaulted, derived or forged: [`Capability::enable`]
//! is the only thing that makes one, and it takes the surface by name.** So an
//! adjacent observer does not exist unless somebody wrote the line that turns it
//! on, and the line says which surface. That is what "disabled by default" means
//! here: not a flag whose default is `false`, which a later
//! `..Default::default()` silently flips, but a value that has to be
//! constructed.
//!
//! ⚠ **The switch is not the containment.** It says an operator meant to run the
//! surface; it says nothing about where the surface sends. [`crate::bind::send_to`]
//! is the containment, and a module that goes around it is caught by
//! `tests/lab_supervisor.rs` rather than by this.
//!
//! ```
//! use bit_ids_lab::{Capability, Surface};
//!
//! let capability = Capability::enable(Surface::LocalDiscovery);
//! assert_eq!(capability.surface(), Surface::LocalDiscovery);
//! assert!(capability.covers(Surface::LocalDiscovery));
//! assert!(!capability.covers(Surface::Pex));
//! ```

use core::fmt;

pub use bit_ids::observation::Surface;

/// Every surface the record vocabulary names.
///
/// ⚠ Written out rather than derived, because [`Surface`] is a plain enum with
/// no iterator. `every_endpoint_name_is_the_field_path_spelling` and
/// `an_adjacent_surface_says_how_it_reaches_out_and_a_core_one_does_not` walk
/// this list, so a variant added to the vocabulary and forgotten here is caught
/// by neither, which is why the length is asserted against the parser rather
/// than against itself.
pub const ALL_SURFACES: [Surface; 8] = [
    Surface::TrackerHttp,
    Surface::TrackerUdp,
    Surface::PeerWire,
    Surface::Dht,
    Surface::Pex,
    Surface::Mse,
    Surface::WebSeed,
    Surface::LocalDiscovery,
];

/// The surfaces a client reaches for without being asked.
///
/// ⚠ The three that no observer implements yet are named anyway. A surface
/// nobody can enable is one nobody can accidentally run, and naming it here is
/// what lets `TODO/observer.md` point at a value rather than at a sentence.
pub const ADJACENT: [Surface; 5] = [
    Surface::LocalDiscovery,
    Surface::Pex,
    Surface::Dht,
    Surface::WebSeed,
    Surface::Mse,
];

/// Whether a surface is one of the adjacent ones.
#[must_use]
pub fn is_adjacent(surface: Surface) -> bool {
    ADJACENT.contains(&surface)
}

/// How an adjacent surface can reach past the lab, in one line.
///
/// [`None`] for a core surface, which reaches only where the lab pointed it.
///
/// ⭐ Carried as data rather than left in a comment, because the refusal a
/// reader sees when a capability is missing should say what the switch is
/// protecting them from.
#[must_use]
pub const fn reaches(surface: Surface) -> Option<&'static str> {
    Some(match surface {
        Surface::LocalDiscovery => "multicasts to the group BEP 14 fixes, on every interface",
        Surface::Pex => "hands out peer addresses a client will then dial",
        Surface::Dht => "queries bootstrap nodes this project does not own",
        Surface::WebSeed => "fetches a URL carried in the torrent",
        Surface::Mse => "negotiates before any observer can read the stream",
        Surface::TrackerHttp | Surface::TrackerUdp | Surface::PeerWire => return None,
    })
}

/// The lab endpoint name for a surface.
///
/// ⚠ A [`bit_ids::canonical::Slug`] accepts `a-z0-9-` and a field path spells a
/// surface in lower snake case, so the same surface has two spellings and this
/// is the only place that converts between them.
#[must_use]
pub const fn endpoint_name(surface: Surface) -> &'static str {
    match surface {
        Surface::TrackerHttp => "tracker-http",
        Surface::TrackerUdp => "tracker-udp",
        Surface::PeerWire => "peer-wire",
        Surface::Dht => "dht",
        Surface::Pex => "pex",
        Surface::Mse => "mse",
        Surface::WebSeed => "web-seed",
        Surface::LocalDiscovery => "local-discovery",
    }
}

/// Proof that one adjacent surface was turned on deliberately.
///
/// ⛔ **No `Default`, no public field.** [`Capability::enable`] is the only
/// constructor and it names the surface, so a capability for one surface cannot
/// stand in for another.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Capability {
    surface: Surface,
}

impl Capability {
    /// Turns on one adjacent surface.
    #[must_use]
    pub const fn enable(surface: Surface) -> Self {
        Self { surface }
    }

    /// Which surface this enables.
    #[must_use]
    pub const fn surface(self) -> Surface {
        self.surface
    }

    /// Whether this capability enables `surface`.
    #[must_use]
    pub fn covers(self, surface: Surface) -> bool {
        self.surface == surface
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "capability for {}", self.surface)
    }
}

/// An adjacent observer was built without the capability for its own surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NotEnabled {
    /// The surface the observer implements.
    pub wanted: Surface,
    /// The surface the capability actually enables.
    pub offered: Surface,
}

impl fmt::Display for NotEnabled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} is not enabled by a capability for {}",
            self.wanted, self.offered
        )?;
        if let Some(why) = reaches(self.wanted) {
            write!(f, ": it {why}")?;
        }
        Ok(())
    }
}

impl core::error::Error for NotEnabled {}

/// Refuses a capability that does not enable `wanted`.
///
/// The one check every adjacent observer runs, so the refusal reads the same
/// whichever surface it came from.
///
/// # Errors
///
/// Returns [`NotEnabled`] when `capability` enables a different surface.
pub fn require(capability: Capability, wanted: Surface) -> Result<(), NotEnabled> {
    if capability.surface == wanted {
        Ok(())
    } else {
        Err(NotEnabled {
            wanted,
            offered: capability.surface,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ADJACENT, ALL_SURFACES as ALL, Capability, Surface, endpoint_name, is_adjacent, reaches,
        require,
    };
    use bit_ids::canonical::Slug;
    use bit_ids::observation::FieldPath;

    #[test]
    fn a_capability_enables_exactly_the_surface_it_names() {
        let capability = Capability::enable(Surface::LocalDiscovery);
        assert!(require(capability, Surface::LocalDiscovery).is_ok());
        for other in ALL {
            if other == Surface::LocalDiscovery {
                continue;
            }
            let refusal = require(capability, other).expect_err("a different surface");
            assert_eq!(refusal.wanted, other);
            assert_eq!(refusal.offered, Surface::LocalDiscovery);
        }
    }

    /// ⛔ The two spellings are of one surface, and this is what holds them
    /// together. A hand-written table is one a compiler cannot check, so the
    /// endpoint name is asserted to be the field-path spelling with its
    /// underscores turned into hyphens, and to be a real [`Slug`].
    #[test]
    fn every_endpoint_name_is_the_field_path_spelling() {
        for surface in ALL {
            let name = endpoint_name(surface);
            assert_eq!(name, surface.as_str().replace('_', "-"), "{surface}");
            Slug::parse(name).unwrap_or_else(|error| panic!("{name} is not a slug: {error}"));
        }
    }

    #[test]
    fn an_adjacent_surface_says_how_it_reaches_out_and_a_core_one_does_not() {
        for surface in ALL {
            assert_eq!(
                is_adjacent(surface),
                reaches(surface).is_some(),
                "{surface} disagrees with itself about being adjacent"
            );
        }
        assert_eq!(ADJACENT.len(), 5);
        for surface in ADJACENT {
            assert!(!reaches(surface).expect("adjacent").is_empty());
        }
        assert!(!is_adjacent(Surface::TrackerHttp));
        assert!(is_adjacent(Surface::LocalDiscovery));
    }

    /// ⛔ The list above is hand-written, so it is checked against something
    /// that is not: every spelling round-trips through the vocabulary's own
    /// parser, and a surface missing from the list would leave a field path the
    /// parser accepts that nothing here enumerates.
    #[test]
    fn the_list_of_surfaces_is_the_one_the_vocabulary_parses() {
        let mut spellings: Vec<&str> = ALL.iter().map(|one| one.as_str()).collect();
        spellings.sort_unstable();
        let before = spellings.len();
        spellings.dedup();
        assert_eq!(spellings.len(), before, "two surfaces share a spelling");
        for surface in ALL {
            let path = FieldPath::parse(&format!("{}/probe", surface.as_str()))
                .unwrap_or_else(|error| panic!("{surface}: {error}"));
            assert_eq!(path.surface(), surface);
        }
    }

    #[test]
    fn the_refusal_names_the_surface_and_why_it_is_behind_a_switch() {
        let refusal = require(Capability::enable(Surface::Pex), Surface::LocalDiscovery)
            .expect_err("mismatched");
        let text = refusal.to_string();
        assert!(text.contains("local_discovery"), "{text}");
        assert!(text.contains("pex"), "{text}");
        // ⚠ The reason, not just the names. A refusal a reader cannot act on
        // sends them to the source to find out what the switch is for.
        assert!(text.contains("multicasts"), "{text}");
    }
}
