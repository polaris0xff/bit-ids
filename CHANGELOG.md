# Changelog

Nothing is released yet. Entries accumulate here until the first
`v1.*` tag, which is what `PUB-03` creates.

## Unreleased

### 2026-09-04T19:20:00Z

- Closed `ACQ-04`, and with it the acquisition group's blocking work. A client
  is now installed only on a host two independent guards have refused to
  disqualify, and they run before the install. Record:
  [`TODO/acquisition.md`](TODO/acquisition.md).
- ⛔ The boundary could not live in the record. `E-MAN-30` refuses to record a
  capture on a host somebody keeps, and cannot stop one: by the time a manifest
  exists, an untrusted installer has already run somewhere.
- A host is claimed by writing a marker and refused when one is already there,
  so a survived host produces evidence of itself. A provisioner token was the
  rejected design: a runner misconfigured to persist its disk still carries one
  and still means it.
- The egress guard reads the kernel's routing table and probes nothing. Reaching
  out from a machine the guard exists to establish is contained would be the
  wrong order.
- Two defects found by driving it. The egress test used gawk-only functions, so
  on a POSIX awk it reported "could not establish" over a machine with a plain
  default route; it failed closed, but a guard that cannot run on a minimal
  image does not run where it matters. And the runner test claimed the real
  machine, so it passed once and failed every run after, which only running it
  twice finds.
- The manifest records what the guard read and when. A claim stamped after the
  run started is refused, because that is a report rather than a boundary.
- `check-runner` joins the gate on every run. The Windows gate reports it as a
  skip naming the reason: ⛔ the guards are Linux-only, so a Windows capture is
  not permitted rather than permitted with a warning.
- Deployment: no data branch, release or capture service was created.

### 2026-09-04T18:30:00Z

- Closed `ACQ-03`. Every route already had to report the version the record
  declares, so the labels always agreed; what was missing was whether that
  agreement was backed by anything. Record:
  [`TODO/acquisition.md`](TODO/acquisition.md).
- Two schema changes carry the rest: an executable digest per route rather than
  one per record, and a capture that says which route's install went on the
  wire. One digest collapsed the very difference this entry detects, and a
  record silent about which install was watched let a reader assume both were.
- ⛔ A single capture of two byte-different installs is unresolved, not
  equivalent. Nothing put the other bytes on the wire, so nothing can say they
  behave the same. Reaching a positive verdict over differing bytes takes a
  capture through each route, which is what the cross-record comparison is for.
- Two routes that installed identical bytes are one build, and observing one
  observed it. That is the only case where a single capture settles it.
- The refusals are two codes, not one: a divergence needs adjudicating and an
  unresolved record needs a second capture. Both are asked from the existing
  publication gate rather than beside it.
- A door sweep found the cross-record comparison guarding against two captures
  of one route but not against one record passed twice, which would have agreed
  on every field for the most trivial reason available.
- Deployment: no data branch, release or capture service was created.

### 2026-09-04T17:35:00Z

- Closed `ACQ-02`. Something can now decide which version to acquire, and the
  decision is a document that keeps every candidate it weighed, the bytes each
  source answered with, and the instant it was made. Record:
  [`TODO/acquisition.md`](TODO/acquisition.md).
- ⛔ Version strings are not comparable in general. Sorting tags as text puts
  `4.1.10` before `4.1.9`, and a project can publish a preview without setting
  the prerelease flag. A target declares how it spells versions, both stability
  signals are believed, and a candidate the scheme cannot order blocks the
  resolution rather than being skipped: skipping yields an older version chosen
  confidently, with nothing saying a newer one was seen and not understood.
- What settles an unorderable candidate is a second signal, not a looser rule.
  One published strictly before the winner cannot be the newest whatever its tag
  says. With no date, or a later one, it still blocks.
- The shell half fetches and nothing else. The digest in a resolution is then of
  what arrived rather than of what a parser reconstructed.
- Driven against four real projects through the route `docs/AGENTS.md` rule 8
  prescribes, since direct `api.github.com` answers 403 from this host. The
  first run failed closed over 51 of `transmission`'s own decade-old tags and
  was right to; that is what produced the publication-order rule.
- Two defects the tests found: `4.1` and `4.1.0` were ordered rather than
  compared equal, because a shorter component vector sorts first; and a schema
  check `from_json` could never reach was deleted rather than left as a guard
  nobody knows works.
- CI now finds every tracked shell script rather than listing two directories.
  A list is what let a new directory arrive unchecked on the lane meant to
  check it.
- Deployment: no data branch, release or capture service was created.

### 2026-09-04T16:40:00Z

- Closed `ACQ-01`. A route record now carries the typed kind, what resolved it,
  what delivered it, the original URL, the immutable identity of what it asked
  for, the artifact digest and signature, and the evidence of the installed
  version. Record: [`TODO/acquisition.md`](TODO/acquisition.md).
- ⛔ Two routes are two routes only if nothing they depend on is shared. The
  resolver and the delivery mechanism are separate values and a record whose
  routes share either is refused, because the two-route rule was otherwise
  satisfiable by asking one index twice under two names.
- The identity of what a route asked for is typed to its kind, so a release
  asset is a repository, a tag and a file name, and a source build is a full
  commit. An abbreviated commit is refused: that is the shape `FOUND-02`
  measured passing an action-pin rule written to refuse floating references.
- An installed version cites the process output the build printed. A version
  read out of a packet capture or a filename is not the build speaking, and the
  citation is checked to resolve and to be process output.
- The signature disposition is compared across the two documents. That
  comparison can fail, which is why it exists where `E-BND-10` was deleted:
  nothing else forces the run's record and the record's claim into step.
- A new code silently reused `E-BND-12`, which the capture-instant comparison
  already held. Two checks under one code is worse than an unclear message, and
  the manifest coverage test refused the change until the renumbered one had a
  planted defect.
- Driving the validators found what the suite could not: the signature
  diagnostic printed a spelling that appears nowhere in the document it was
  telling an operator to go and read.
- Deployment: no data branch, release or capture service was created.

### 2026-09-04T15:40:00Z

- Closed `FOUND-03`, the last foundation piece before acquisition. A new
  `bit-ids-wire` crate carries byte-exact codecs for the HTTP tracker, the UDP
  tracker and the peer wire, plus a synthetic fixture corpus with a committed
  digest index. Record: [`TODO/foundation.md`](TODO/foundation.md).
- The invariant is that decode then encode reproduces the input byte for byte.
  Every retention rule in `docs/architecture.md` section 5, meaning query and
  header order, duplicate fields, percent-encoding hex case, all eight reserved
  bytes and early message order, is destroyed by the convenient implementation, and a
  round trip catches all of them at once because a decoder that dropped a
  detail has nothing to write back.
- The codecs observe rather than impose. Unsorted bencode keys, `i-0e`, a bare
  newline terminator, an unassigned message id and a non-standard handshake
  protocol string are recorded, because each is a difference between builds and
  refusing one turns an observation into a parse failure. Nothing maps a
  peer-ID prefix or a `v` string to a client name, which would put a refused
  input inside the component every observer trusts.
- Nine lossy defects were planted in the codecs one at a time and every one was
  refused by the fixture corpus alone. Two were not caught on the first attempt
  and both are fixed here: the corpus never reached `bencode::encode`, because a
  message re-encodes from payload bytes held verbatim, and no fixture carried a
  bare newline for a terminator-repairing decoder to fail against.
- A door sweep found three holes and all three are fixed: `FixtureIndex`
  derived `Deserialize`, so `serde_json::from_str` skipped the corpus-digest
  check; `load_directory` filtered for `*.json` over a non-recursive listing, so
  a fixture in a subdirectory would have been silently skipped rather than
  refused; and the HTTP head cap only fired when there was no blank line at all,
  so a head that ended just past the cap parsed in full.
- The corpus was replayed over real loopback TCP one byte per write and decoded
  incrementally. Reading the same announce datagram with the wrong direction
  yields an unassigned action and refuses to be an announce, rather than a
  plausible wrong answer.
- No new third-party crate. The lockfile diff is the workspace member and
  nothing else.
- Deployment: no data branch, release or capture service was created.

### 2026-09-04T14:47:22Z

- Closed `SCHEMA-04`, and with it the whole schema group. A run records what it
  varied, and a classifier turns samples into per-byte lifetimes, so a peer ID
  comes out as the shape it actually has: a fixed prefix and a suffix the build
  regenerates. Record: [`TODO/schema.md`](TODO/schema.md).
- Nothing here is a confidence. A dimension the run never varied yields
  `unknown` rather than a guess, so a value that held still inside one process
  is not called persistent: only a restart separates a stored value from a
  regenerated one, and one sample yields `unknown` for everything.
- `bind` now refuses a field claiming variation from a run that varied nothing,
  and a field resting on more samples than the plan could produce. The manifest
  coverage test refused the change until both had a planted defect.
- Deployment: no data branch, release or capture service was created.

### 2026-09-04T14:40:23Z

- Closed `SCHEMA-03`. Corroboration now keeps what each connector saw, the
  artifact it read the value out of and what was applied before comparing,
  rather than a verdict and a list of names. Record:
  [`TODO/schema.md`](TODO/schema.md).
- A connector that cannot see a surface says so. Left out, a single observation
  looks like a pair that happened to agree, which is the easiest false
  agreement available; named, it is the reason that connector's silence proves
  nothing. An outcome of exact or normalized over fewer than two observers who
  could actually see the field is refused.
- Validity and publishability became separate gates, and both are needed. A
  disagreement has to be recordable or the project loses the evidence of one,
  so a conflicted record reads and validates; `publishable` is what refuses to
  ship it. A record that supersedes another now has to say why, out of the four
  causes a disagreement actually has.
- A normalization used to reach agreement must declare that order and unknown
  bytes survive it. That rule was written in the architecture and enforced by
  nothing.
- A door sweep found `is_publishable` and `publishable` answering the same
  question at different scopes with nothing holding them together. A test now
  drives a record through all four outcomes and asserts the two agree.
- Deployment: no data branch, release or capture service was created.

### 2026-09-04T14:29:09Z

- Closed `SCHEMA-02`. The run manifest records how a capture was produced: the
  host, the isolation it ran under, both clocks, every tool at the version that
  ran, where each artifact came from, the phases of the state machine the run
  walked, the content-addressed evidence and what was scrubbed from it. Record:
  [`TODO/schema.md`](TODO/schema.md).
- It is a second document rather than a larger section inside the profile, so
  a consumer of the catalogue does not have to carry a whole run. `bind`
  compares every value the two share, which is what stops a deliberate overlap
  from becoming drift. Pairing a manifest with the profile of a different run
  of the same build is refused.
- Absence stays as constrained as it is in a profile: a run that reached beyond
  loopback says why, a host that is not disposable is refused outright, a phase
  cannot be skipped, and an artifact marked redacted must say what was taken out
  of it so that "raw" cannot quietly mean "edited".
- One guard was written and then deleted. `E-BND-10` compared the installed
  version across the two documents and could not fail, because three existing
  invariants already implied it. It was found while trying to plant a defect
  for it, which is the only way that class of dead guard shows up.
- `ProfileError` is now `DocumentError` and carries the schema it expected. It
  covered both documents while naming only one, and its message told a reader
  with a manifest problem to go and look at the profile schema.
- Deployment: no data branch, release or capture service was created.

### 2026-09-04T14:11:29Z

- Closed `FOUND-02`. The action pin rule is now an allowlist of forms that are
  immutable by construction rather than a denylist of the floating ones
  somebody thought of, and the lockfile is checked for a crates.io source and a
  checksum on every package. Record: [`TODO/foundation.md`](TODO/foundation.md).
- Three shapes passed the old rule and are refused now: a branch named anything
  other than `main` or `master`, an abbreviated commit, and a bare commit with
  no version comment. The comment is load-bearing, because it is what
  `check-remote-items` resolves against the tag it claims to be.
- Added [`docs/supply-chain.md`](docs/supply-chain.md) with the three pinned
  layers and the procedure for updating one. No register of pins was added: the
  lockfile and the workflow already hold those commits, and a third copy would
  be the drift the rules exist to prevent.
- Deployment: no data branch, release or capture service was created.

### 2026-09-04T14:02:39Z

- Fixed `check-docs.ps1`, which resolved a link going up more than two
  directory levels to the wrong path and reported it broken. `[^/]+` matches
  `..` as readily as a directory name and PowerShell's `-replace` is global, so
  one call collapsed a real segment and the `../..` pair after it. It now
  replaces one leftmost match per pass, which is what the `sed` loop in the sh
  twin was already doing. Record:
  [`docs/conventions/forbidden-patterns.md`](docs/conventions/forbidden-patterns.md).
- The defect was latent from the bootstrap and surfaced by the first link in
  the tree that goes up four levels, added with the schema fixtures. That link
  is now the standing regression case: `check-twins` compares the two halves
  against it on every run.
- Deployment: no data branch, release or capture service was created.

### 2026-09-04T13:57:55Z

- Closed `SCHEMA-01`. The `bit-ids` crate carries the versioned profile record,
  the six field states, the derived record identifier, the canonical value
  forms and every publication invariant this schema owns, read and written
  through one validating path. Record:
  [`TODO/schema.md`](TODO/schema.md).
- The unproven-field rule is the point of it: a state that asserts anything
  about a build must cite recoverable evidence, and a claim that a build
  produced nothing must cite a positive control. Without the second half, an
  observer that was never listening and a build that never answered would be
  the same record.
- Added the first third-party crates, `serde`, `serde_json` and `sha2`, with
  the lockfile committed. A hand-written JSON reader was rejected: it would put
  a new silent-corruption surface in the one layer that must not corrupt.
- `Profile` no longer derives `Deserialize`. It did until the door sweep on
  this entry found that `serde_json::from_str::<Profile>` handed back an
  unvalidated record while the crate documentation said `from_json` was the
  only way in. The derive now sits on a private field mirror and every serde
  route validates.
- `check-no-secrets` grew two exclusions, for registry lockfile digests and for
  the algorithm-tagged digests and observed bytes a profile record is made of.
  Its long-hex rule had also been dropping whole lines to allow one item on
  them, so a credential beside a pinned action commit was never reported. Both
  halves now delete the allowed item and re-test what is left. Record:
  [`docs/conventions/forbidden-patterns.md`](docs/conventions/forbidden-patterns.md).
- Deployment: no data branch, release or capture service was created.

### 2026-09-04

- Added the initial repository foundation, research sweep, architecture,
  machine-readable target catalogue, Rust library skeleton, CI gate and full
  implementation backlog. Record: [`TODO/PROGRESS.md`](TODO/PROGRESS.md).
- Deployment: no data branch, release or capture service was created.
