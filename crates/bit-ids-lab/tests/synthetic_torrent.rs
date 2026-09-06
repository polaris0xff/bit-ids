//! `OBS-08`'s acceptance: the four properties the Prove names, and the layout a
//! client would refuse the torrent over.
//!
//! ⛔ **The info hash is checked against the info dictionary as it sits in the
//! file, not against a re-encode of the value the generator kept.** The trap
//! this entry carries is an info hash naming a dictionary the file does not
//! contain, and comparing `encode(torrent.info())` against the same encoder's
//! output cannot see it: both halves would move together. So the tests below
//! walk the raw metainfo, cut out the bytes the `info` key maps to, and hash
//! those. That is what a client does and it is the only reading that refutes
//! the defect.
//!
//! ⭐ **The payload's byte stream is pinned twice over.** A torrent is generated
//! rather than committed because its bytes are then a function of its declared
//! inputs, and a record's `capture.fixture` is only checkable while that stays
//! true. A generator whose stream drifts silently invalidates every digest
//! already published, and nothing in the module's own unit tests would notice:
//! they assert reproducibility and seed-dependence, both of which survive any
//! change to the arithmetic. So the stream is compared against `SplitMix64`
//! restated here from the specification, and its first words for seed zero are
//! compared against the published reference implementation's own output.

use std::net::{Ipv4Addr, SocketAddrV4};

use bit_ids::canonical::Sha256Digest;
use bit_ids_lab::bind;
use bit_ids_lab::torrent::{MAX_PAYLOAD_BYTES, MIN_PIECE_LENGTH, PIECE_HASH_LEN, WebSeed};
use bit_ids_lab::{SyntheticTorrent, TorrentError, TorrentSpec};
use bit_ids_wire::bencode::{self, Value};
use sha1::{Digest as _, Sha1};

/// A spec that exercises every optional field, so a test that reads one is
/// reading a torrent that has it.
///
/// ⚠ The piece length is deliberately not the floor. A spec built at
/// `MIN_PIECE_LENGTH` makes `piece length` and the floor indistinguishable, so
/// a generator that wrote the constant instead of the declared value would read
/// as correct in every test that used one.
fn furnished() -> TorrentSpec {
    TorrentSpec {
        name: "obs-08-acceptance".to_owned(),
        piece_length: MIN_PIECE_LENGTH * 2,
        piece_count: 3,
        payload_seed: 0x0bad_c0de_dead_beef,
        announce: Some("http://127.0.0.1:6969/announce".to_owned()),
        private: true,
        created_at: 1_262_304_000,
        // ⚠ One web seed, so "every optional field" stays a true sentence.
        // `OBS-11` added the field and a helper claiming coverage it no longer
        // had would be a false claim in a doc comment, which is the shape a
        // claim audit exists to find.
        web_seeds: vec![
            WebSeed::new(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8080), "/payload")
                .expect("loopback is inside the allowed set"),
        ],
    }
}

fn generate(spec: TorrentSpec) -> SyntheticTorrent {
    SyntheticTorrent::generate(spec).expect("the spec describes a usable torrent")
}

/// The byte range the top-level `info` key maps to, located in the raw file.
///
/// ⛔ A walk of the document rather than a search for `3:info`, because that
/// byte sequence can occur inside a piece hash or a name. The walk is what a
/// client does: read the dictionary's entries in order and keep where each
/// value started and stopped.
fn info_span(metainfo: &[u8]) -> core::ops::Range<usize> {
    assert_eq!(
        metainfo.first(),
        Some(&b'd'),
        "a metainfo file is a bencoded dictionary"
    );
    let mut at = 1;
    loop {
        assert!(at < metainfo.len(), "the document ended mid-dictionary");
        assert!(metainfo[at] != b'e', "the document carries no info key");
        let (key, used) = bencode::decode_prefix(&metainfo[at..]).expect("a key decodes");
        at += used;
        let start = at;
        let (_, used) = bencode::decode_prefix(&metainfo[at..]).expect("a value decodes");
        at += used;
        if key == Value::bytes(b"info".to_vec()) {
            return start..at;
        }
    }
}

fn bytes_of<'a>(value: &'a Value, key: &[u8]) -> &'a [u8] {
    match value.get(key) {
        Some(Value::Bytes(bytes)) => bytes,
        other => panic!("expected a byte string under {key:?}, got {other:?}"),
    }
}

fn integer_of(value: &Value, key: &[u8]) -> i64 {
    match value.get(key) {
        Some(Value::Integer(integer)) => integer.to_i64().expect("a canonical integer"),
        other => panic!("expected an integer under {key:?}, got {other:?}"),
    }
}

/// `SplitMix64`, restated from the specification in `torrent.rs`'s own module
/// documentation rather than called from it.
///
/// ⚠ Deliberately a second copy. `forbidden-patterns.md` bans copied parsing
/// logic between two callers that must agree; this is the opposite shape, a
/// golden restatement whose whole job is to disagree the moment the production
/// arithmetic moves. The published reference outputs below are what keeps it
/// from being a copy of whatever the generator happens to do.
fn splitmix64_stream(seed: u64, length: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(length);
    let mut state = seed;
    while out.len() < length {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut word = state;
        word = (word ^ (word >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        word = (word ^ (word >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        word ^= word >> 31;
        let take = (length - out.len()).min(8);
        out.extend_from_slice(&word.to_be_bytes()[..take]);
    }
    out
}

/// The first five words the public-domain `splitmix64.c` emits for seed zero.
///
/// ⭐ This is what makes [`splitmix64_stream`] a specification rather than a
/// mirror of the generator. Both could be edited together; neither can be
/// edited into agreement with these.
const SPLITMIX64_SEED_ZERO: [u64; 5] = [
    0xe220_a839_7b1d_cdaf,
    0x6e78_9e6a_a1b9_65f4,
    0x06c4_5d18_8009_454f,
    0xf88b_b8a8_724c_81ec,
    0x1b39_896a_51a8_749b,
];

#[test]
fn the_published_constants_are_the_ones_the_specification_fixes() {
    // ⛔ Every other test here reads these constants, so each one moves with
    // them and none of them can see one drift. A piece hash narrowed to sixteen
    // bytes would re-chunk the `pieces` string and the comparison against it
    // in the same step. BEP 3 fixes the first two; the cap is this project's,
    // and it is stated here so a test that walks the boundary is walking the
    // boundary it was written for.
    assert_eq!(PIECE_HASH_LEN, 20, "BEP 3 fixes a piece hash at SHA-1");
    assert_eq!(
        MIN_PIECE_LENGTH,
        16 * 1024,
        "the smallest piece length BEP 3 practice expects a client to accept"
    );
    assert_eq!(MAX_PAYLOAD_BYTES, 64 * 1024 * 1024);
}

// --- the four properties the Prove names ------------------------------------

#[test]
fn the_generated_document_round_trips_through_the_project_codec() {
    // ⭐ The round trip is the invariant `architecture.md` section 5 puts on
    // every codec, and a generator that emitted non-canonical bencode would be
    // producing a file this project's own decoder reads differently from the
    // way it was written.
    for spec in [TorrentSpec::default(), furnished()] {
        let torrent = generate(spec);
        let decoded = bencode::decode(torrent.metainfo()).expect("the metainfo decodes");
        assert_eq!(
            bencode::encode(&decoded),
            torrent.metainfo(),
            "decode then encode did not reproduce the metainfo"
        );

        // And the same for the info dictionary as it sits in the file, so the
        // value the generator kept is the value the file carries.
        let span = info_span(torrent.metainfo());
        let from_file = bencode::decode(&torrent.metainfo()[span]).expect("the info dictionary");
        assert_eq!(
            &from_file,
            torrent.info(),
            "the info dictionary in the file is not the one the generator reports"
        );
    }
}

#[test]
fn the_info_hash_is_sha1_of_the_info_dictionary_as_it_sits_in_the_file() {
    let torrent = generate(furnished());
    let span = info_span(torrent.metainfo());
    let in_file = &torrent.metainfo()[span.clone()];

    assert_eq!(
        Sha1::digest(in_file).as_slice(),
        torrent.info_hash(),
        "the info hash does not name the dictionary the file carries"
    );

    // ⛔ And not of the whole document, which is the confusion the entry names.
    assert_ne!(
        Sha1::digest(torrent.metainfo()).as_slice(),
        torrent.info_hash(),
        "the info hash was taken over the whole metainfo"
    );
    assert!(
        span.start > 0 && span.end < torrent.metainfo().len(),
        "the info dictionary is a proper sub-range of the file, so the two \
         digests are over genuinely different bytes"
    );
    assert_eq!(torrent.info_hash().len(), PIECE_HASH_LEN);
}

#[test]
fn identical_parameters_produce_identical_bytes() {
    let spec = furnished();
    let first = generate(spec.clone());
    let second = generate(spec.clone());

    assert_eq!(first.metainfo(), second.metainfo());
    assert_eq!(first.info_hash(), second.info_hash());
    assert_eq!(first.digest(), second.digest());
    assert_eq!(first.payload(), second.payload());
    assert_eq!(first, second, "two generations of one spec are one torrent");

    // A third generation after other torrents have been built, so nothing the
    // generator carries between calls can leak into the bytes.
    let _ = generate(TorrentSpec::default());
    let third = generate(spec);
    assert_eq!(first.metainfo(), third.metainfo());
}

/// One declared input, and the smallest change to it that stays a valid spec.
type Change = (&'static str, fn(&mut TorrentSpec));

#[test]
fn one_changed_parameter_changes_the_bytes_and_the_fixture_digest() {
    let base = generate(furnished());
    let mutations: [Change; 7] = [
        ("name", |spec| spec.name.push('x')),
        ("piece_length", |spec| spec.piece_length *= 2),
        ("piece_count", |spec| spec.piece_count += 1),
        ("payload_seed", |spec| spec.payload_seed ^= 1),
        ("announce", |spec| {
            spec.announce = Some("http://127.0.0.1:6970/announce".to_owned());
        }),
        ("private", |spec| spec.private = !spec.private),
        ("created_at", |spec| spec.created_at += 1),
    ];

    for (field, mutate) in mutations {
        let mut spec = furnished();
        mutate(&mut spec);
        assert_ne!(spec, *base.spec(), "{field} was not actually changed");
        let changed = generate(spec);
        assert_ne!(
            changed.metainfo(),
            base.metainfo(),
            "{field} did not change the metainfo bytes"
        );
        assert_ne!(
            changed.digest(),
            base.digest(),
            "{field} did not change the fixture digest"
        );
    }
}

// --- what the four rest on --------------------------------------------------

#[test]
fn changing_a_field_inside_the_info_dictionary_moves_the_info_hash_and_one_outside_it_does_not() {
    // ⭐ The sharper half of the property above. Every field moves the file's
    // bytes; only the ones BEP 3 puts inside the info dictionary may move the
    // value a client announces. A generator that folded the announce URL into
    // the info hash would make two runs of one experiment look like two
    // different torrents to every tracker.
    let base = generate(furnished());

    let mut inside = furnished();
    inside.name.push('x');
    assert_ne!(generate(inside).info_hash(), base.info_hash());

    let mut also_inside = furnished();
    also_inside.private = !also_inside.private;
    assert_ne!(generate(also_inside).info_hash(), base.info_hash());

    for outside in [
        {
            let mut spec = furnished();
            spec.announce = Some("http://127.0.0.1:6970/announce".to_owned());
            spec
        },
        {
            let mut spec = furnished();
            spec.created_at += 1;
            spec
        },
        // ⭐ BEP 19's `url-list` sits beside `announce`, outside the info
        // dictionary, so a web seed moves the file and must not move the value
        // a client announces.
        {
            let mut spec = furnished();
            spec.web_seeds = vec![
                WebSeed::new(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 9090), "/other")
                    .expect("loopback"),
            ];
            spec
        },
    ] {
        let changed = generate(outside);
        assert_eq!(
            changed.info_hash(),
            base.info_hash(),
            "a field outside the info dictionary moved the info hash"
        );
        assert_ne!(changed.digest(), base.digest());
    }
}

#[test]
fn the_fixture_digest_is_sha256_of_the_whole_metainfo_and_not_of_the_info_dictionary() {
    let torrent = generate(furnished());
    assert_eq!(torrent.digest(), Sha256Digest::of(torrent.metainfo()));

    let span = info_span(torrent.metainfo());
    assert_ne!(
        torrent.digest(),
        Sha256Digest::of(&torrent.metainfo()[span]),
        "capture.fixture was taken over the info dictionary rather than the file"
    );
    assert!(
        torrent.digest().to_string().starts_with("sha256:"),
        "the digest a record cites carries its algorithm"
    );
}

#[test]
fn the_payload_is_the_splitmix64_stream_the_module_documents() {
    // The published reference first, so the restatement below is anchored to
    // something outside this repository.
    let reference: Vec<u8> = SPLITMIX64_SEED_ZERO
        .iter()
        .flat_map(|word| word.to_be_bytes())
        .collect();
    assert_eq!(
        splitmix64_stream(0, reference.len()),
        reference,
        "the restated generator disagrees with the published reference"
    );

    let spec = furnished();
    let torrent = generate(spec.clone());
    let expected = splitmix64_stream(
        spec.payload_seed,
        usize::try_from(u64::from(spec.piece_length) * u64::from(spec.piece_count))
            .expect("the payload fits in memory on this host"),
    );
    assert_eq!(
        torrent.payload(),
        expected,
        "the generated payload is not the documented stream, so every fixture \
         digest already recorded names bytes this build no longer produces"
    );
}

#[test]
fn every_piece_hash_is_sha1_of_the_piece_it_names_and_the_pieces_tile_the_payload() {
    let spec = furnished();
    let torrent = generate(spec.clone());
    let pieces = bytes_of(torrent.info(), b"pieces");

    let count = usize::try_from(spec.piece_count).expect("a small count");
    assert_eq!(
        pieces.len(),
        count * PIECE_HASH_LEN,
        "the pieces string is not exactly one hash per piece"
    );

    let mut rebuilt = Vec::new();
    for (index, hash) in pieces.chunks(PIECE_HASH_LEN).enumerate() {
        let index = u32::try_from(index).expect("a small index");
        let piece = torrent.piece(index).expect("every piece is present");
        assert_eq!(
            piece.len(),
            usize::try_from(spec.piece_length).expect("a small piece length"),
            "piece {index} is not one piece long"
        );
        assert_eq!(
            Sha1::digest(piece).as_slice(),
            hash,
            "the hash for piece {index} is not the hash of piece {index}"
        );
        rebuilt.extend_from_slice(piece);
    }
    assert_eq!(
        rebuilt,
        torrent.payload(),
        "the pieces do not tile the payload exactly"
    );
}

#[test]
fn the_declared_length_the_piece_length_and_the_payload_agree() {
    let spec = furnished();
    let torrent = generate(spec.clone());
    let info = torrent.info();

    let declared = integer_of(info, b"length");
    let total = i64::from(spec.piece_length) * i64::from(spec.piece_count);
    assert_eq!(declared, total, "the declared length is not the real one");
    assert_eq!(
        i64::try_from(torrent.payload().len()).expect("a small payload"),
        declared,
        "a client asked to verify this torrent would run out of payload"
    );
    assert_eq!(
        integer_of(info, b"piece length"),
        i64::from(spec.piece_length)
    );
    assert_eq!(bytes_of(info, b"name"), spec.name.as_bytes());
}

#[test]
fn both_dictionaries_carry_sorted_unique_keys() {
    // ⛔ BEP 3 requires sorted keys, and a second implementation that
    // re-encodes an unsorted document computes a different info hash for the
    // same torrent. The generator sorts; this is what says so.
    for spec in [TorrentSpec::default(), furnished()] {
        let torrent = generate(spec);
        let document = bencode::decode(torrent.metainfo()).expect("the metainfo decodes");
        for (what, value) in [("document", &document), ("info", torrent.info())] {
            assert_eq!(
                value.keys_are_sorted(),
                Some(true),
                "the {what} dictionary's keys are not in ascending order"
            );
            assert_eq!(
                value.has_duplicate_keys(),
                Some(false),
                "the {what} dictionary carries a key twice"
            );
        }
    }
}

#[test]
fn the_announce_url_and_the_private_flag_appear_only_when_the_spec_declares_them() {
    let bare = generate(TorrentSpec::default());
    let document = bencode::decode(bare.metainfo()).expect("the metainfo decodes");
    assert!(
        document.get(b"announce").is_none(),
        "a torrent with no tracker declared one anyway"
    );
    assert!(
        bare.info().get(b"private").is_none(),
        "a torrent that is not private carries the flag"
    );

    let spec = furnished();
    let full = generate(spec.clone());
    let document = bencode::decode(full.metainfo()).expect("the metainfo decodes");
    assert_eq!(
        bytes_of(&document, b"announce"),
        spec.announce.expect("the spec declares one").as_bytes()
    );
    assert_eq!(
        integer_of(full.info(), b"private"),
        1,
        "BEP 27's flag is the integer one"
    );
}

#[test]
fn the_creation_date_is_the_declared_one_and_never_the_clock() {
    // ⚠ A generator that read the clock would still be reproducible within one
    // second, so determinism alone does not catch it. The declared value does.
    let spec = furnished();
    let torrent = generate(spec.clone());
    let document = bencode::decode(torrent.metainfo()).expect("the metainfo decodes");
    assert_eq!(integer_of(&document, b"creation date"), spec.created_at);

    let mut older = furnished();
    older.created_at = -1;
    let older = generate(older);
    let document = bencode::decode(older.metainfo()).expect("the metainfo decodes");
    assert_eq!(
        integer_of(&document, b"creation date"),
        -1,
        "a declared instant before the epoch was not the one written"
    );
}

// --- the refusals -----------------------------------------------------------

#[test]
fn a_spec_that_describes_no_usable_torrent_is_refused_with_the_reason() {
    let refuse = |spec: TorrentSpec| {
        SyntheticTorrent::generate(spec).expect_err("the spec should have been refused")
    };

    let mut nameless = furnished();
    nameless.name = String::new();
    assert_eq!(refuse(nameless), TorrentError::EmptyName);

    let mut empty = furnished();
    empty.piece_count = 0;
    assert_eq!(refuse(empty), TorrentError::NoPieces);

    // Below the floor, and above it but not a power of two. A client that
    // cannot divide the payload into pieces refuses the torrent outright.
    for length in [0, 1, 1024, MIN_PIECE_LENGTH - 1] {
        let mut spec = furnished();
        spec.piece_length = length;
        assert_eq!(refuse(spec), TorrentError::PieceLength(length));
    }
    for length in [
        MIN_PIECE_LENGTH + 1,
        MIN_PIECE_LENGTH + MIN_PIECE_LENGTH / 2,
    ] {
        let mut spec = furnished();
        spec.piece_length = length;
        assert_eq!(refuse(spec), TorrentError::PieceLength(length));
    }
}

#[test]
fn a_payload_at_the_cap_is_generated_and_one_piece_over_it_is_refused() {
    // ⭐ The boundary itself, because a cap tested only from above cannot tell
    // `>` from `>=` and a cap tested only from below cannot tell it from no cap
    // at all.
    //
    // ⚠ Asserted before the work rather than derived from it: a raised cap would
    // otherwise send this test generating whatever the new one is.
    assert_eq!(
        MAX_PAYLOAD_BYTES,
        64 * 1024 * 1024,
        "the cap this test walks"
    );
    let piece_length = 16 * 1024 * 1024;
    let at_cap = u32::try_from(MAX_PAYLOAD_BYTES / u64::from(piece_length)).expect("a small count");

    let mut spec = TorrentSpec {
        piece_length,
        piece_count: at_cap,
        ..TorrentSpec::default()
    };
    let torrent = generate(spec.clone());
    assert_eq!(
        u64::try_from(torrent.payload().len()).expect("the payload fits"),
        MAX_PAYLOAD_BYTES,
        "the payload at the cap is exactly the cap"
    );

    spec.piece_count = at_cap + 1;
    let over = MAX_PAYLOAD_BYTES + u64::from(piece_length);
    assert_eq!(
        SyntheticTorrent::generate(spec).expect_err("over the cap"),
        TorrentError::PayloadTooLarge(over)
    );
}

#[test]
fn an_enormous_payload_is_refused_before_anything_is_allocated() {
    // ⛔ The order matters more than the refusal. A cap checked after the
    // allocation is a lab that takes its host down and then explains why.
    // These parameters cannot be allocated on any host, so a run that reaches
    // the allocation aborts this test rather than passing it.
    let spec = TorrentSpec {
        piece_length: 1 << 31,
        piece_count: u32::MAX,
        ..TorrentSpec::default()
    };
    let expected = u64::from(1_u32 << 31) * u64::from(u32::MAX);
    assert_eq!(
        SyntheticTorrent::generate(spec).expect_err("far over the cap"),
        TorrentError::PayloadTooLarge(expected)
    );
}

#[test]
fn a_piece_past_the_last_one_is_none_rather_than_a_panic() {
    let spec = furnished();
    let torrent = generate(spec.clone());
    assert!(torrent.piece(spec.piece_count - 1).is_some());
    assert!(
        torrent.piece(spec.piece_count).is_none(),
        "a piece index past the layout answered with bytes"
    );
    assert!(
        torrent.piece(u32::MAX).is_none(),
        "an index that overflows the offset must answer nothing, not panic"
    );
}

#[test]
fn a_generated_torrent_reports_the_spec_it_came_from() {
    // ⚠ The record cites the spec to rebuild the torrent, so a torrent that
    // reported a spec other than the one it was generated from would make the
    // fixture digest uncheckable.
    let spec = furnished();
    let torrent = generate(spec.clone());
    assert_eq!(torrent.spec(), &spec);
    assert_eq!(
        generate(torrent.spec().clone()).metainfo(),
        torrent.metainfo(),
        "regenerating from the reported spec did not reproduce the file"
    );
}

/// ⛔ A spec naming no web seed writes no `url-list` key at all.
///
/// ⚠ **This is what keeps `OBS-11`'s new field free.** `capture.fixture` is a
/// digest of the metainfo a run used, so a key written as an empty list would
/// have moved the bytes of every torrent generated from a spec that never asked
/// for a web seed, and every digest recorded against one. Nothing has been
/// captured yet, which is the only reason such a change would have cost nothing
/// today; it is still not one to make by accident.
#[test]
fn a_torrent_naming_no_web_seed_carries_no_url_list_key() {
    let mut without = furnished();
    without.web_seeds = Vec::new();
    let torrent = generate(without.clone());
    let document = bencode::decode(torrent.metainfo()).expect("it is bencode");
    assert!(
        document.get(b"url-list").is_none(),
        "an empty web seed list wrote a key"
    );

    // ⭐ And the control: the key appears exactly when one is named, so this
    // case cannot pass over a generator that never writes it.
    let with = generate(furnished());
    let document = bencode::decode(with.metainfo()).expect("it is bencode");
    let Some(Value::List(urls)) = document.get(b"url-list") else {
        panic!("a spec naming a web seed writes a list");
    };
    assert_eq!(urls.len(), 1);
    assert_eq!(
        urls[0],
        Value::bytes(b"http://127.0.0.1:8080/payload".to_vec())
    );
    assert_ne!(torrent.digest(), with.digest());
}

/// ⛔ The third door, at the place a torrent opens it.
///
/// A `url-list` entry is where the build will fetch from, on its own socket, so
/// nothing this crate guards is on that path. `WebSeed` is the only way to make
/// one and its constructor is the check.
#[test]
fn a_web_seed_outside_the_allowed_set_cannot_be_put_in_a_torrent() {
    for elsewhere in [
        SocketAddrV4::new(Ipv4Addr::new(198, 51, 100, 7), 80),
        SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 80),
        SocketAddrV4::new(Ipv4Addr::new(93, 184, 216, 34), 80),
    ] {
        assert!(
            matches!(
                WebSeed::new(elsewhere, "/payload"),
                Err(bind::BindError::NotReachable { .. })
            ),
            "{elsewhere} was not refused"
        );
    }
    // ⚠ The control, so a constructor that refused everything would fail here.
    let allowed = WebSeed::new(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8080), "/payload")
        .expect("loopback is inside the allowed set");
    assert_eq!(allowed.url(), "http://127.0.0.1:8080/payload");
}
