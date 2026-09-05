# Acquisition entries

## ACQ-01: Acquisition route schema

Source: operator same-version multi-source requirement
Priority: P0 | Effort: L | Status: DONE

Problem: Package, official download, release artifact, and source-build routes
need comparable provenance despite different metadata.

Premise: Resolver input, immutable source identity, retrieved bytes, installed
version output, and host facts form a common route record.

Approach: Define typed route kinds and evidence requirements; do not treat the
candidate routes in the catalogue as verified availability.

Decision: the route record is one shape with a typed source identity, not one
record shape per kind. What the eight kinds have in common is the shape of the
claim, which is what the premise says: something decided a version, something
delivered bytes, the bytes have a digest, and the installed thing was asked its
own version. Only the identity of what was asked for genuinely differs, so that
is the enum and the rest is flat. Eight parallel record types was the rejected
alternative: it would have made every consumer match on the kind to read a
digest.

Decision: `SignatureStatus` moved out of the manifest module into the route
module. Both documents record it and `bind` compares them, so a second copy of
the enum is a value in two places that can disagree about what its own variants
mean.

Decision: the catalogue-vocabulary test scans `clients.toml` rather than adding
a `toml` dependency. One hand-maintained file read by one test does not carry
the argument the supply-chain rules require for a third-party crate. The scan
fails loud instead of quietly finding nothing: it refuses a result too small to
be real, and that guard was planted against by renaming the key it reads.

Prove: schema tests cover all catalogue route kinds and reject an acquisition
without a content digest or observed installed version.

Closure evidence: run on 2026-09-04. `cargo test --workspace --locked
--all-targets` is 123 passed, 0 failed. `tests/acquisition.rs` asserts every
`candidate_routes` entry in the catalogue parses as a `RouteKind` and every
variant is asked for by the catalogue, over 42 scanned entries. A content digest
and an installed version are non-optional fields, so a record without either
fails to deserialize; what needed new invariants was everything around them.

⭐ The route record grew six invariants, each planted against in
[`../crates/bit-ids/tests/profile_schema.rs`](../crates/bit-ids/tests/profile_schema.rs):

- `E-ACQ-05` a kind carrying another kind's source identity, which reads as a
  complete record and names nothing anyone can resolve again;
- `E-ACQ-06` an abbreviated commit, the shape `FOUND-02` measured passing an
  action-pin rule written to refuse floating refs;
- `E-ACQ-07` and `E-ACQ-08` two routes sharing a resolver or a delivery
  mechanism. ⛔ This is the one that matters. Without it the two-route rule is
  satisfiable by asking one index twice under two names, which is the failure
  `architecture.md` section 7 exists to prevent rather than a technicality;
- `E-ACQ-09` an installed version citing evidence that is not in the record;
- `E-ACQ-10` one citing evidence that is not process output;
  [`../docs/architecture.md`](../docs/architecture.md) section 7 carries why.

`E-BND-13` compares the signature disposition across the two documents. It was
written after checking it can fail, which `E-BND-10` could not: nothing else
forces the run's record and the profile's into step, so `verified` over a run
that says `not_checked` is publishable without it.

Driven pass: the two example validators were run against the golden pair, which
they accept, and against three hand-planted records. Every refusal named the
route, the field and the reason. ⚠ It found a real defect the suite could not:
`E-BND-13` printed the Rust `Debug` spelling, telling an operator the run
recorded `Unsigned` over a document that says `unsigned`, and sending them to
look for a value written nowhere. `SignatureStatus` now has one canonical
spelling and the diagnostic quotes it.

⚠ Found while wiring the bind check: `E-BND-12` was already taken by the
capture-instant comparison, and the new code silently reused it. Two checks
under one code is worse than an unclear message, because a caller acting on the
class cannot tell which fired. Renumbered to `E-BND-13`, and the manifest
coverage test refused the change until it had a planted defect.

Residual: `origin` is an HTTPS URL for every kind, including a package-manager
route where the natural reference is a pool path rather than a page. That has
held for every catalogue route so far and is recorded here rather than assumed
to hold forever.

Residual: a bare `serde_json::from_str::<AcquisitionRoute>` produces an
unvalidated route. That is true of `Build` and `Capture` too and is not a new
door: the route-level invariants need the build tuple and the evidence list, so
a route alone cannot answer them. `Profile` remains the validated boundary.

## ACQ-02: Latest stable release resolver

Source: operator latest-stable-only scope and bit-cli tag-selection guards
Priority: P0 | Effort: L | Status: DONE

Problem: Lexical tag sorting and package channel labels can select previews,
older trains, or incomparable versions.

Premise: Stable selection must be product-specific, time-stamped, fail closed,
and retain every candidate considered.

Approach: Implement Rust resolvers with explicit prerelease rules and shell
fetch wrappers. Record source responses; never derive profile fields from
release source code.

Decision: the shell half fetches and nothing else. It writes the bytes to a
file and prints the URL that answered; the Rust half parses, orders and decides.
That is the line `../scripts/README.md` already draws, and it is what makes the
digest in a resolution mean something: it is of what arrived, not of what a
parser reconstructed. A shell script that also picked the newest version would
be a second implementation of the ordering rule with no test behind it.

Decision: an unorderable candidate blocks the resolution rather than being
skipped, and only a second signal releases it. Skipping was the rejected
alternative and it is the dangerous one: it produces an older version selected
confidently, with nothing saying a newer one was seen and not understood.

Decision: no date crate. One function formats an instant, and a dependency for
that is one this project would have to argue for under
`../docs/supply-chain.md`.

Prove: resolver fixtures cover stable, prerelease, missing, divergent, and
non-semantic version sets, and a live dry run emits a complete decision trace.

Closure evidence: run on 2026-09-04. `cargo test --workspace --locked
--all-targets` is 146 passed, 0 failed.
[`../crates/bit-ids/tests/resolution.rs`](../crates/bit-ids/tests/resolution.rs)
carries all five sets the acceptance names and eight more, with a planted defect
for every diagnostic code.

⭐ Live dry run, four real targets, through the route `docs/AGENTS.md` rule 8
prescribes, `https://api.gh.pkgforge.dev/`. Direct `api.github.com` answered 403
from this host, which is the reason that rule exists.

| target | tag shape | candidates | selected | trace |
| --- | --- | ---: | --- | --- |
| `qbittorrent` | `release-` + 3 or 4 | 4 | `5.2.3` | 1 selected, 3 superseded |
| `transmission` | bare, 3 | 83 | `4.1.3` | 1 selected, 10 superseded, 21 prerelease, 51 predating |
| `aria2` | `release-` + 3 | 24 | `1.37.0` | 1 selected, 23 superseded |
| `qbittorrent-enhanced` | `release-` + 4 | 100 | `5.2.3.10` | 1 selected, 99 superseded |

⚠ **The driven pass changed the design, which is what it is for.** The first
live run against `transmission` failed closed over 51 candidates and was
correct to: they are two-component tags from a decade ago that a
three-component scheme cannot read, and nothing in the resolver could rule out
that one of them was newer. Refusing over a project's own history made the
resolver correct and useless. The fix is not a loosened rule but a second
signal: a candidate published strictly before the winner cannot be the newest
whatever its tag says, so it is recorded as `predates_selection`. One with no
date, or a date at or after the winner's, still blocks, and both cases are
tested.

Two more findings, both fixed:

1. **`4.1` and `4.1.0` were ordered rather than compared equal.** A shorter
   component vector sorts first, so the resolver called them different versions
   with the longer one newer. They are one release. Components are now padded to
   the scheme's width, which makes the pair ambiguous and fails closed. Found by
   writing the ambiguity test and watching it select.
2. **`E-RES-01` checked a schema identifier `from_json` could never reach**,
   because the version probe answers first. The schema became a type that cannot
   hold a wrong value, the same construction the manifest already used, and the
   invariant was deleted rather than left as a guard nobody knows works. That is
   the path `E-BND-10` took in `SCHEMA-02`.

⭐ Door sweep, three findings, all fixed:

1. **`Verdict::excludes_from_ordering` had no callers.** It was a leftover from
   an earlier shape of `decide`, and a little dead code is a safety margin only
   when it is one. Deleted.
2. **`Verdict::as_str` and serde's `rename_all` were two spellings of one
   vocabulary with nothing comparing them**, which is the drift refused
   everywhere else here. A test now holds them together and refuses a variant
   added without a spelling.
3. ⛔ **`retrieved_at` was the response file's modification time.** An mtime
   survives a copy, an archive restore and a checkout, so a published retrieval
   instant would have been one no retrieval produced. The fetch wrapper now
   writes the instant beside the body and the resolver reads that, refusing to
   proceed when it is absent.

Residual: `min_components` and `max_components` describe a target's current
tagging convention, and a project that changes convention will produce
unorderable candidates until the scheme is updated. That is the intended
behaviour rather than a gap: it fails closed and says so. The catalogue does not
yet carry a scheme per target; `ACQ-03` needs one to compare two routes and is
where it lands.

Residual: `fetch-releases.sh` has no PowerShell twin. `../scripts/README.md`
carries why, and `ACQ-04` owns the Windows runner contract where one is needed.

## ACQ-03: Same-version multi-route verifier

Source: operator requirement to install the same build from two or more routes
Priority: P0 | Effort: L | Status: DONE

Problem: Equal display versions can identify repackaged or patched binaries,
while different version formats can identify the same upstream build.

Premise: Installed version output, upstream version mapping, artifact hashes,
binary metadata, and observed profile comparison can make equality explicit.

Approach: Require at least two independent routes on the same host family;
classify byte-identical, build-equivalent, divergent, or unresolved and block
publication for the last two.

Decision: the executable digest moved from the record to the route, and the
capture now names which route's install was observed. One digest per record
collapsed exactly the difference this entry exists to detect, and a record that
did not say which install was watched let a reader assume both were. Everything
else here follows from those two fields.

Decision: a single capture of two byte-different installs is `unresolved`, not
`build_equivalent`. That was the fork. Calling it equivalent is what the version
labels invite, and it publishes a claim about bytes this project never put on
the wire. Reaching `build_equivalent` needs a capture per route, which is what
`classify_across` compares, and the rejected alternative would have made the
whole classification decorative.

Decision: two refusal codes rather than one. A divergence needs adjudicating and
an unresolved record needs a second capture; collapsing them into "not
publishable" loses which.

Decision: the rule is asked from `publishable` rather than beside it. A second
publication gate a caller has to remember is the shape that lets a record ship
past one of them.

Prove: an end-to-end fixture accepts two equivalent routes and rejects version
label equality when binary or observed identity evidence conflicts.

Closure evidence: run on 2026-09-04. `cargo test --workspace --locked
--all-targets` is 157 passed, 0 failed.
[`../crates/bit-ids/tests/equivalence.rs`](../crates/bit-ids/tests/equivalence.rs)
accepts the golden pair as `byte_identical` and publishes it, and refuses four
distinct ways of being wrong: equal labels over installs only one of which was
observed, a field both captures measured disagreeing, two captures of one route,
and one record passed twice.

Driven pass: the golden pair validates and publishes. A copy whose second route
installed different bytes still **validates** and refuses to publish with
`E-PUB-04`, naming both digests and the route that was watched. That split is
the point: the difference has to be recordable or there is nowhere to keep the
evidence of it. A copy whose observed route does not match the record's
executable digest is refused outright by `E-CAP-06`.

⭐ Door sweep, one finding, fixed: `classify_across` guarded against comparing
two captures of one **route** and not against comparing two records of one
**run**. One record passed twice would have agreed on every field for the most
trivial reason available. It is refused now, and the test that was meant to
cover the route case was passing through the wrong guard until the capture ids
were made distinct.

⚠ Found while writing the divergence test: the first version set the field it
was changing to the value the fixture already carried, so the records agreed and
the test passed for the wrong reason. An `assert_ne!` against the current value
now stands in front of the edit. That is the "when a negative test passes
suspiciously well, doubt the harness" rule firing.

Residual: the manifest carries no `observed_route`, so `bind` covers it only
transitively, through the route sets already having to match. That is enough
today because both documents list the same routes; a manifest that recorded
which install was driven would let `bind` compare it directly, and `CI-03` is
where a capture matrix would produce one.

Residual: `classify_across` is not yet called by anything that assembles a
publication. `CORPUS-02` owns the whole-store invariants and is where the
cross-record comparison becomes a gate rather than a function.

## ACQ-04: Disposable-host execution boundary

Source: proprietary installers and active network client execution
Priority: P0 | Effort: L | Status: DONE

Problem: Running untrusted clients on persistent CI hosts can leak state,
escape the lab, contaminate later samples, or violate package terms.

Premise: Fresh VM images, restricted egress, non-secret accounts, and verified
teardown can make captures reproducible and bounded.

Approach: Define Linux and Windows runner contracts, deny public BitTorrent
traffic, snapshot inputs, wipe the guest, and attest cleanup after every run.

Decision: the claim guard detects the failure of disposal rather than trusting
a declaration of it. A token the provisioner writes saying "this host is
disposable" was the rejected alternative and it is the shape that fails
silently: a runner misconfigured to persist its disk still carries the token and
still means it. Claiming by writing a marker makes a survived host produce
evidence of itself, and a misconfiguration cannot produce the marker's absence.

Decision: the egress guard reads `/proc/net/route` and never probes anything.
Reaching out from a machine the guard exists to establish is contained would be
the wrong order, and a connection attempt against a third party is not this
project's to make.

Decision: `check-runner` runs on every gate rather than only where captures
happen. It is hermetic and takes no network, and it is the check whose silence
would be worst.

Prove: the runner test detects forbidden egress and persistent state, then
shows a fresh host fingerprint for the next job.

Closure evidence: run on 2026-09-04.
[`../scripts/acquisition/check-runner.sh`](../scripts/acquisition/check-runner.sh)
is 7 guard cases, 7 passed, 0 failed, run twice with the exit code read unpiped
from its own process both times. It plants a default route that is up (refused),
one that is configured and down (allowed, so the flags are actually read), a
loopback-only table (allowed), an unreadable table (exit 2, because not knowing
is not a pass), a second claim on one host (refused), and a claim after teardown
(allowed). It prints the host fingerprint the next job compares.

The gate is 12 checks, 11 passed, 0 failed, 1 skipped, with `check-runner`
among them. The Windows gate reports it as a skip naming the reason rather than
omitting the row, because a lane that leaves a check out looks like one that
checked it.

⚠ **Two defects found by driving it, both fixed:**

1. **The egress guard could not run on this host.** It tested the route flags
   with `and(strtonum(...))`, which are gawk extensions; against a POSIX awk
   they are undefined functions and the guard reported "could not establish"
   over a machine with a plain default route. The direction of the failure was
   right and its reach was not;
   [`../docs/capture-host.md`](../docs/capture-host.md) says why that matters.
   The UP bit is the low bit of the flags word, so the last hex digit being odd
   is the same test with no extensions.
2. ⛔ **The runner test claimed the real machine.** One case ran `--claim`
   without the scratch state directory, so it wrote `/var/lib/bit-ids/host-claimed`
   on the session host: it passed once and failed every run after. On a
   persistent CI runner that is a check which goes green the day it is written
   and red forever, for a reason that looks like the guard being broken rather
   than the test being. Found by running it twice, which is the only thing that
   finds it. The marker it left was removed.

⚠ A third finding was in the measurement rather than the code: the exit code was
first read after a pipe into `tail`, so a script that exited 1 read as 0. That
is this repository's oldest stated rule, broken while checking a guard.

The manifest gained `isolation.claim`, carrying the fingerprint the guard read
and when it claimed the host. `E-MAN-33` refuses a claim stamped after the run
started, which is the distinction the whole entry rests on: a guard that ran
after the install is a report, not a boundary.

Residual: ⛔ **the executable guards are Linux-only**, because they read
`/proc/net/route` and `/etc/machine-id`. A Windows capture host needs its own
pair reading `Get-NetRoute` and the machine GUID, and until it exists a Windows
capture is not permitted rather than permitted-with-a-warning: there is no
boundary to run before the install. `CI-03` owns the runner matrix and is where
that pair lands. [`../docs/capture-host.md`](../docs/capture-host.md) carries
both contracts.

Residual: the guards prove they fire, not that they are sufficient. A host can
be non-disposable in ways neither models, and
[`../docs/capture-host.md`](../docs/capture-host.md) lists the ones considered
rather than leaving them implied.

Residual: the fingerprint's discriminating power is argued from its inputs
rather than measured across two machines, because this session had one host. It
is a digest of the machine id, the boot id, the hostname, the kernel string and
the root device number; any single one of those can collide and the combination
is what is relied on.

## ACQ-05: Artifact cache and authenticity evidence

Source: repeatability and upstream availability risk
Priority: P1 | Effort: M | Status: DONE

Problem: Upstream URLs and package indexes change, but storing installers may
be disallowed.

Premise: Hashes, signatures, certificate metadata, package receipts, and
permitted artifacts can preserve verification without illegal redistribution.

Approach: Implement a policy-aware cache that stores bytes only when allowed
and otherwise stores sufficient authenticity and retrieval evidence.

Prove: cache tests enforce the licence register and reproduce artifact
identity after a simulated source URL change.

### ⭐ The second half of the Prove asks for less than it sounds like

"Reproduce artifact identity after a source URL change" needs no reproduction at
all once the identity is the digest. A URL is where bytes were found, which is a
fact about a moment; the digest is what they were. So a retrieval from a new
location is recorded against the artifact the digest already names, and the
lookup that finds it never mentions a URL. ⛔ **A cache keyed on the location
would stop answering the moment a vendor reorganised a download page**, which is
exactly the failure this entry exists to survive.

⚠ Repeating a retrieval is not a second retrieval either. A cache that grew a
row every time somebody re-ran an acquisition would report a popularity contest
rather than a provenance, so `observe` reports `Unchanged` and adds nothing.

### Decision: the register is asked, never re-read

⛔ `catalogue/licences.toml` has one parser per twin and it is `check-licences`.
The Rust cache takes a disposition map, the way the store rules take a tree a
caller has already read; the driving example takes `--permitted TARGET` flags;
and `check-cache.sh` fills them from `check-licences --permitted`. A second
reader of that file would be a second answer to what it permits.

⭐ `--permitted` was added to **both** halves and both were run against a
register with a planted `permitted` row: both printed `aria2` and both still
agreed on `--json`. ⚠ That matters because the flag's normal answer is empty,
which is also what a broken parser prints.

### ⛔ The refusal case needed a control, and the harness carries it

`FOUND-04`'s register refuses every target, so the scenario's storage attempt is
refused. On its own that case passes equally over a cache that can never store
anything, which is a different program. The harness runs the same scenario with
a target explicitly permitted, watches the same cache accept it, and asserts the
two runs differ on exactly the one line. ⚠ Without that, the whole policy half
would be theatre.

### Acceptance, all run on 2026-09-06

- `cargo test -p bit-ids --locked --all-targets`
- `sh scripts/acquisition/check-cache.sh`
- `sh scripts/common/check-gate.sh`

### Closure evidence, 2026-09-06

| what | measured |
| --- | --- |
| `sh scripts/acquisition/check-cache.sh` | 11 cases, 11 passed, 0 failed |
| `cargo test --workspace --locked --all-targets` | 38 binaries, 356 passed, 0 failed, six of them new here |
| guard mutation | 9 plants over `cache.rs`, 9 refused, each verified to compile first |
| driven pass | one artifact observed from two locations: one artifact, two retrievals, the digest still resolving, keeping nothing accepted and keeping the bytes refused as `E-CAC-01` |
| register tie | the permitted set came from `check-licences --permitted`, which answers nothing today, and both halves were separately shown to report a planted `permitted` row |

⚠ **Nothing here fetches anything.** The scenario is about identity and
permission, and a real download would make the run depend on a vendor's uptime
to answer a question about this project's own rules. The bytes are generated in
the example and are nobody's installer.

### Residuals

- ⚠ The cache has no on-disk document yet. `CACHE_SCHEMA` is declared and
  nothing writes one, because the first thing worth writing is a real
  acquisition's cache and no acquisition has run. The rules are pure over a
  cache a caller holds, which is what let this close without one.
- ⚠ `check-cache.sh` does not plant in the register, because that file is
  tracked and a gate check that edits one leaves a dirty tree when it is
  interrupted. The planted-register measurement is in the table above instead.
- ⚠ Certificate metadata and package receipts from the Premise are not modelled.
  `SignatureStatus` records what was done about a signature and the manifest
  carries the rest; a receipt has no producer until a real acquisition runs.
