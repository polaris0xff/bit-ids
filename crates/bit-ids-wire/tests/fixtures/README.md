# Protocol fixtures

⛔ **Nothing here is a measurement.** Every byte in this directory was written
by hand from a published BEP, never captured from a running build. The peer ID
in all of them is `bit-ids-fixture-0001`, which no client emits, and the
advertised client string is `bit-ids-fixture/0`. These files may seed a parser;
they may never populate the catalogue.
[`../../../../docs/capture-methodology.md`](../../../../docs/capture-methodology.md)
says what may.

The synthetic identity is the guard against exactly that mistake, and it is
checked rather than trusted:
`fixtures_all_declare_a_synthetic_origin_and_carry_only_the_synthetic_identity_token`
pulls every one out **through the codec** and refuses any other value. A fixture
carrying a real client's prefix is one search away from being read as a result
about that client.

⚠ **"Identity token" rather than "peer ID", because `dht` has neither.** A KRPC
message carries a *node* id, which BEP 5 fixes at the same twenty bytes and which
is the same thing for the check's purpose: the value a search would match a real
build on. `bit-ids-fixture-0001` is twenty bytes and serves as both.

⛔ **A version string names a build as squarely as an identifier does**, and
nothing checked one until `dht` put a `v` on a second surface.
`no_fixture_carries_a_version_string_that_could_name_a_real_build` reads BEP 5's
`v` and BEP 10's `v` through their codecs and holds both to
`bit-ids-fixture/0`. It also asserts it read at least two, because a sweep that
found nothing would otherwise report nothing wrong.

## What each one is for

| file | surface | what it proves survives a decode |
| --- | --- | --- |
| `tracker-http-announce-started.json` | `tracker_http` | query order, the 20 peer-ID bytes, `compact`, `no_peer_id`, `numwant`, `key`, `event` |
| `tracker-http-announce-unusual-encoding.json` | `tracker_http` | uppercase escape case, an over-encoded peer ID, a duplicate query key, a duplicate header, a lowercase header name, no space after a colon, and one bare `\n` among `\r\n` lines |
| `tracker-udp-connect-then-announce.json` | `tracker_udp` | the BEP 15 magic, a transaction id, `key`, and `num_want` of `-1` read as signed |
| `tracker-udp-announce-with-options.json` | `tracker_udp` | the BEP 41 options past the fixed 98 bytes |
| `peer-wire-handshake-only.json` | `peer_wire` | a handshake advertising nothing, every reserved byte zero |
| `peer-wire-extended-handshake.json` | `peer_wire` | the extension, fast and DHT bits, a sorted BEP 10 dictionary, `m` order, `reqq`, `p`, `v`, and two messages written together |
| `peer-wire-early-message-sequence.json` | `peer_wire` | unsorted dictionary keys, the non-canonical integer `i-0e`, a keep-alive, and an unassigned message id |
| `dht-bootstrap-queries.json` | `dht` | a sorted KRPC query, the twenty-byte node id, a `v` of non-standard width, a transaction id with an unprintable byte, and `want` kept as a list |
| `dht-announce-unusual-shape.json` | `dht` | unsorted message keys, the non-canonical integer `i01e` in `implied_port`, and BEP 43's `ro` beside the message rather than inside `a` |
| `index.json` | none | the digest of every fixture above, and of the corpus |

⭐ **The unusual ones are the load-bearing ones.** A fixture that only carries a
well-formed request cannot catch a decoder that repairs what it reads, and
repairing is the failure mode: a parser that quietly sorts keys, folds header
case or normalises a terminator destroys the evidence and passes every test
written against tidy input.

## The bytes are hexadecimal

A `.bin` beside each document would be the obvious shape and the wrong one.
[`../../../../scripts/common/check-control-bytes.sh`](../../../../scripts/common/check-control-bytes.sh)
sets out the cost of a literal control byte in a tracked file: `grep` calls it
binary and skips it, and `git diff` prints "Binary files differ", so a review of
the one artefact that most needs reading shows no diff at all. Hex is lossless,
so byte-exactness gives up nothing for reviewability.

## Regenerating

⚠ **Each document is byte-exact.**
`fixtures_are_stored_in_the_canonical_form_they_are_digested_from` compares
`Fixture::to_json` against the file, so a hand edit that is valid but not
canonical fails the suite.

`index.json` is derived, never hand-edited. After adding or changing a fixture:

```sh
cargo run --quiet -p bit-ids-wire --example fixture-digests \
  > crates/bit-ids-wire/tests/fixtures/index.json
cargo test --workspace fixtures
```

⛔ **Read the resulting diff.** A digest that moved for a fixture you did not
touch is a finding, not a formality: the index is what makes
[`../../../../TODO/foundation.md`](../../../../TODO/foundation.md)'s "identical
fixture digests" survive past the session that measured it.
