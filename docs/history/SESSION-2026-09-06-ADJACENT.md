# Session 2026-09-06: the adjacent surfaces

What `OBS-11` took, what each review pass found, and the three findings that were
not about the code being written.

⚠ **This is narrative history and goes stale on purpose.** The current truth is
[`../architecture.md`](../architecture.md) and the entries in
[`../../TODO/`](../../TODO/). Nothing here is amended to match a later tree.

## What closed

`OBS-11`, in five commits, each green and pushed before the next began:

| unit | what it added |
| --- | --- |
| the prerequisite | a datagram responder that sees the source address |
| `bit_ids_wire::dht` | BEP 5's KRPC codec and two fixtures |
| `bit_ids_probe::dht` | the DHT observer, and `bind::check_offered` |
| `bit_ids_probe::web_seed` | BEP 19, and `url-list` in `TorrentSpec` |
| `bit_ids_wire::mse`, `bit_ids_probe::mse` | message stream encryption |

The observer layer now covers every surface a build reaches for. What stands
between here and a first capture is a client adapter and a host the
[`../capture-host.md`](../capture-host.md) guards permit.

## The finding that mattered most

⛔ **There is a third door and it is not a socket.**

`OBS-06` established two: where the lab listens, and where the lab sends. Writing
the DHT observer produced a third question neither answers. A `find_node` or
`get_peers` response carries `nodes` and `values`, which are lists of addresses
the build will then dial **itself**. Those packets leave the build's socket, so
`bind::send_to` is never called on them and no guard on this project's sockets
can ever see them. A routable address offered that way reaches the network as
surely as one the lab sent.

⚠ **The hazard was already written down, against the wrong surface.**
`adjacent::reaches` said `pex` "hands out peer addresses a client will then
dial", and said nothing of the kind about `dht`, which does the same thing
through a different field while also querying out. A hazard recorded against one
surface and not the sibling that shares it is the one-gated-door defect
[`../methodology/reviews.md`](../methodology/reviews.md) calls the most recurring
hole there is.

⛔ **And the door sweep then found it in the two oldest observers.** After
`bind::check_offered` had closed the DHT case and BEP 19's `url-list`, grepping
for every type carrying an address a build would be handed turned up
`OfferedPeer`: shared by both tracker observers, with **public fields and no
check at all**, since `OBS-02`. Its fields are private now and `OfferedPeer::new`
is the only constructor.

⭐ **That is the lens working exactly as specified.** `reviews.md` says the list
you wrote from memory has never been complete and to grep for the callers you did
not enumerate. The enumeration named DHT and web seed. The grep named the
trackers.

## The two findings that were not about this work

⛔ **The previous session's commit stamps are fabricated.** `conventions/git.md`
section 3 says to read the stamp from the machine and never type it. The nine
commits before this session are stamped 2026-09-06T10:30Z through 23:55Z, evenly
spaced and round, while their committer dates run 2026-09-05T17:42Z to 23:37Z:
wrong day, wrong hours, invented spacing. This session's stamps are machine-read,
which is why they go backwards against the record above them and forwards against
the clock.

⚠ **The consequence is visible in `CHANGELOG.md`.** The file is ordered newest
first, and a truthful stamp cannot sit at the top because the entries above carry
later fabricated ones. This session's entry therefore sits in stamp order rather
than work order, with the reason written into it. ⛔ **The old stamps are not
retro-corrected.** The committer dates would supply real values, but rewriting
somebody else's record of when they worked is worse than the gap it closes, and
`git.md` says so.

⛔ **`synthetic-torrent` turned the gate red by being run.** Its output path
defaulted to `synthetic.torrent`, so running the example the obvious way wrote a
`.torrent` into whatever directory `cargo run` was invoked from, which is the
repository root. `check-licences` reads `git ls-files` **and** the untracked
files that are not ignored, and refuses an artifact of that shape. The path is
required now.

⚠ **An ignore rule was the other repair and it is the wrong one.**
`.gitignore`'s own header says an ignore is a deletion nobody notices, and hiding
a redistributable-shaped artifact from the check that exists to find one is worse
than the red gate. ⭐ Nothing in the suite could have found this: no test runs
that example from the repository root, which is what part (b) of the gate is for.

## What each review pass swept

⛔ **Five passes, five different questions**, per
[`../methodology/reviews.md`](../methodology/reviews.md).

**1. The door sweep.** Enumerated every affordance the session added, then
grepped for the ones not enumerated. Found `OfferedPeer` in both tracker
observers, above. It also confirmed `bind::send_to` is still the only outbound
datagram path: reverting `serve_datagram` to `socket.send_to` is refused by
`no_module_outside_the_bind_guard_reaches_the_network`, which is `OBS-06`'s sweep
still holding.

**2. The guard mutation.** Seventeen plants across the session, each read unpiped
and each checked for whether it compiled first. ⛔ **One survived and it was a
finding**: deleting the MSE verification check left every test passing, because
the only case exercising a wrong key relies on random plaintext and random
plaintext trips the pad-length check first, reporting `Unreadable` instead. So
`VerificationFailed` was a refusal nothing could produce, which is the shape
`OBS-06` found in `peer_exchange`'s `BeforeHandshake`. The case that reaches it
is a build that keys its stream correctly and writes the wrong constant. Re-planted
afterwards and refused.

⚠ **In the same pass, an assertion that was too narrow was corrected rather than
forced.** A case required `VerificationFailed` over a mismatched torrent; the
observer answered `Unreadable` with `padC is 64448 bytes`, and the observer was
right. The guarantee worth holding is that a wrong key never reads as conforming,
by whichever route, and that is what is asserted now.

**3. The claim audit.** Re-measured every count written into the record against
the tree, and checked that every path cited resolves. ⛔ **It found one number
wrong**: `bit-ids-wire` was recorded as 60 unit cases and is 48. It also caught
an em dash and a duplicated sentence that the gate would have caught, and one it
would not: a doc comment promising `tests/mse_arithmetic.rs` before that file
existed.

**4. What was measured but never verified.** ⛔ **The RC4 test vector.** It was
written from memory and the test passing proves only that the implementation
agrees with the constant, which is circular. An independent Python `RC4`
confirmed it, and in doing so found the comment named the wrong offset: MSE
discards 1024 keystream bytes, so a freshly keyed cipher produces the stream at
offset 1024 rather than the offset-zero block RFC 6229 also publishes.

⚠ **And curl's header order was a one-sample claim.** It was stated as a property
after a single run. Five fetches across two request shapes confirm it is stable
per shape and differs between them, which is the two-sample discipline this
project's own field-state model requires before calling anything constant.

**5. What the driven pass showed that the suite could not.** The
`synthetic.torrent` finding above. ⭐ **And the measurement that justifies the
whole prerequisite chain**: driven by a `libtorrent`-encoded `announce_peer`
carrying `implied_port`, the observer recorded
`Implied { observed: 37466, stated: Some(6881) }`, taking the source port rather
than the number in the message, and the driver independently reported the same
37466. No unit test produces that number; it comes from a kernel.

⭐ **And MSE interoperated with an implementation nobody here wrote.** The suite
tests this project's initiator against this project's receiver, which is one
reading of the specification agreeing with itself. A Python initiator using its
own `pow`, `RC4` and `SHA-1` completed the handshake: the verification constant
decrypted to eight zero bytes, the selection came back, and the observer recorded
`padA` 73, `padC` 19 and the peer ID from inside the encrypted `IA`.

## What is still not true

⛔ **No observer has been driven by a stock `BitTorrent` client.** `curl`,
`libtorrent`'s bencode and the Python MSE initiator are independent
implementations written from specifications, which share this project's reading
of the protocol. `OBS-07` owns the stock-client controls and needs a host
`assert-disposable.sh --egress` does not refuse.

⛔ **Nothing has been published and no capture has been taken.** Everything in
the tree is synthetic and says so.

⚠ **`mse` and `web_seed` still have no fixture** and are still refused with
`E-FIX-07`, which is what the negative control in the fixture suite now names. A
fixture needs a codec that round-trips a whole transcript, and MSE's encrypted
section cannot be re-encoded without the run's key.

⚠ **The MSE observer stops at the handshake.** The payload stream after
`crypto_select` is `RC4` under `keyB` and is not read, because `OBS-04` reads a
peer wire and this module reads a negotiation, and one decoding the other's bytes
is how two readings of one stream disagree. Composing them is a `CLIENT-*`
adapter's job.

⚠ **A journal segment still carries no source address**, so the live and the
recorded reading of one datagram differ, and `PortClaim::NotObserved` says so
rather than inventing a value. Re-deriving an `implied_port` announcement from an
evidence bundle alone would need `bit-ids/transcript/1` widened. Left as a
decision for the entry that first needs it in a published field.
