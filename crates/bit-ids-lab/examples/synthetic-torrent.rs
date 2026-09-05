//! Writes a generated `.torrent` out and prints everything derived from it, so
//! `OBS-08` can be driven by a reader that is not this project's own code.
//!
//! ```text
//! cargo run -p bit-ids-lab --example synthetic-torrent -- /tmp/fixture.torrent
//! ```
//!
//! ⭐ **The point is what it prints beside the file.** A third-party client or
//! library reads the same file, computes the info hash itself, and the two
//! answers either agree or they do not. `docs/methodology/gate.md` part (b)
//! exists for exactly this: the suite proves the generator against this
//! project's reading of BEP 3, and only an outside reader proves the reading.
//!
//! ⚠ It writes one file and speaks to nothing. Handing the torrent to a client
//! is a capture, and a capture needs a host the `docs/capture-host.md` guards
//! permit.

use bit_ids_lab::torrent::MIN_PIECE_LENGTH;
use bit_ids_lab::{SyntheticTorrent, TorrentSpec};

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "synthetic.torrent".to_owned());

    // Declared here rather than defaulted, because every one of these is an
    // input the file's bytes are a function of, and the point of printing them
    // is that a reader can regenerate the file from this alone.
    let spec = TorrentSpec {
        name: "bit-ids-obs-08".to_owned(),
        piece_length: MIN_PIECE_LENGTH * 2,
        piece_count: 3,
        payload_seed: 0x0bad_c0de_dead_beef,
        announce: Some("http://127.0.0.1:6969/announce".to_owned()),
        private: true,
        created_at: 1_262_304_000,
    };

    let torrent = match SyntheticTorrent::generate(spec.clone()) {
        Ok(torrent) => torrent,
        Err(error) => {
            eprintln!("the spec describes no usable torrent: {error}");
            std::process::exit(1);
        }
    };

    if let Err(error) = std::fs::write(&path, torrent.metainfo()) {
        eprintln!("could not write {path}: {error}");
        std::process::exit(1);
    }

    println!("path {path}");
    println!("spec.name {}", spec.name);
    println!("spec.piece_length {}", spec.piece_length);
    println!("spec.piece_count {}", spec.piece_count);
    println!("spec.payload_seed {:#018x}", spec.payload_seed);
    println!("spec.announce {}", spec.announce.unwrap_or_default());
    println!("spec.private {}", spec.private);
    println!("spec.created_at {}", spec.created_at);
    println!("metainfo.bytes {}", torrent.metainfo().len());
    println!("payload.bytes {}", torrent.payload().len());

    // ⚠ Two digests of two different things. The info hash is SHA-1 of the
    // encoded info dictionary and is what a client announces; the fixture
    // digest is SHA-256 of the whole file and is what `capture.fixture` cites.
    let mut info_hash = String::with_capacity(torrent.info_hash().len() * 2);
    for byte in torrent.info_hash() {
        use core::fmt::Write as _;
        write!(info_hash, "{byte:02x}").expect("writing to a String cannot fail");
    }
    println!("info_hash.sha1 {info_hash}");
    println!("capture.fixture {}", torrent.digest());
}
