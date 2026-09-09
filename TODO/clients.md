# Client adapter entries

Every adapter resolves the latest stable release at run time, proves at least
two same-version acquisition routes on each supported host family, disables
public peer discovery, drives the isolated lab, and produces two-connector
evidence. Candidate routes remain hypotheses until the entry closes.

## The machinery every entry below shares

⭐ **An adapter is five subcommands and nothing else**, specified in
[`../scripts/capture/adapters/README.md`](../scripts/capture/adapters/README.md).
The parts that are not per-target live once:
[`install-client`](../scripts/acquisition/install-client.sh) runs a route while
the network is up, [`capture-client`](../scripts/capture/capture-client.sh) runs
the measurement under containment, and
[`client-capture`](../crates/bit-ids-probe/examples/client-capture.rs) is the
observer that hands out a torrent naming its own tracker.

⛔ **`stock_client` is the adapter's own declaration, copied into the
attestation.** A stub drives every guard in that path and is not a product, so a
run it drove records `false`. ⚠ The harness proves the field varies by running
one stub twice under both declarations; a runner that hardcoded either would
pass one of those cases and fail the other.

⛔ **Two things make a run a measurement of a build, and both are checked.** The
adapter answers `version` from the installed executable, and an announce arrives
carrying a peer ID this observer did not generate whose bytes the raw transcript
holds. ⚠ Without the second, a bundle of empty artifacts verifying against their
own empty digests satisfies everything else.

## CLIENT-01: qBittorrent capture adapter

Source: operator scope, upstream repository, and August 2026 priority sample
Priority: P1 | Effort: L | Status: OPEN

Problem: The highest-priority observed client lacks a reproducible live
profile across Linux and Windows acquisition routes.

Approach: Automate the Web UI or command controls, compare official releases
with package-manager routes, and capture all core observer surfaces.

Prove: a trusted run publishes agreeing Linux and Windows profiles for the
same stable version with two verified routes per host.

### What was measured on 2026-09-05, and why the entry stays open

⛔ **The acceptance cannot run on a session host, and three routes were tried
before saying so.** It is the same shape as `OBS-01`'s: an acceptance naming a
Windows capture that this repository is not permitted to perform at all.

1. Run it whole. Refused on a session host: `--egress` exits 1 there, so
   installing and driving a client would be the capture that boundary exists to
   refuse. ⚠ **That is a fact about a session host and never was one about
   hosted runners.** A runner is a fresh virtual machine per job and its default
   route can be deleted before the install, which is what `CI-03`'s workflow
   does; the Windows guard pair exists now too.
2. Run the Linux half alone. Refused for the first half of the same reason: a
   capture on one platform is still a capture, and the boundary does not care
   how much of the acceptance it satisfies.
3. Split off the provable prefix and close that. ⭐ **Partly possible, and the
   part that is was done.** Deciding which version to acquire needs no host, so
   it was run. What it cannot do is leave a record: `CORPUS-01` owns the
   append-only store and is open, so a measurement has nowhere durable to go and
   would be a file nobody can cite. The split is therefore not filed as an entry
   yet; it becomes one once there is a store to write into.

⭐ **The resolver met a real target for the first time.**
`sh scripts/acquisition/fetch-releases.sh qbittorrent/qBittorrent <file>`
answered through `https://api.gh.pkgforge.dev/`, which is the route
`docs/AGENTS.md` rule 8 names, and
`cargo run -p bit-ids --example resolve-stable -- qbittorrent release- 3 3 ...`
selected **5.2.3**, published 2026-07-07, over three superseded candidates with
every verdict kept and a digest of the bytes that were read. ⚠ One source is not
two: the resolution is single-sourced and `ACQ-01`'s independence rule is about
acquiring the artifact, which is a separate requirement this did not touch.

⛔ **The listing answered with four releases, and that is a property of the
source rather than of the mirror.** It looked like truncation and it is not:
page two of the same endpoint is empty, so four is the whole answer, while the
`tags` endpoint on the same route answers with at least a hundred, beginning
`release-5.2.3`, `release-5.2.2`, `release-5.2.1`, `release-5.2.0`,
`release-5.1.4`. The project tags far more versions than it publishes as
releases.

⭐ **That matters to every future resolution and it is not a defect in the
resolver.** `resolve` decides from the candidates it is handed, and the caller
chooses the endpoint; a caller that reads only `releases` is selecting from a
different and much smaller population than the project's versions, and a
resolution saying `candidates: 4` is a claim about that endpoint rather than
about the target. It changed nothing here because `5.2.3` leads both
populations, and it would change the answer for a target that stops minting
release objects. ⚠ A second source for a resolution has to be a different
endpoint or a different index, not the same one asked twice, which is the
independence rule `ACQ-01` already states for routes.

Measured from the same listing, the release offers a Linux `AppImage`, a Windows
`x64_setup.exe` and a source `tar.xz`, each with a detached `.asc` signature
beside it. That is two platform routes and a signature disposition to verify, and
`ACQ-05` owns the authenticity evidence.

### The observer a client can actually be pointed at, measured 2026-09-08

⭐ **The lab now hands out a torrent that names its own tracker**, which is what
a stock build needs and what nothing here provided before.
[`client-capture`](../crates/bit-ids-probe/examples/client-capture.rs) runs the
real `HttpTracker` observer, generates the torrent with `announce` set to the
endpoint the operating system just gave it, writes it to a path, and prints the
address, the info hash and the fixture digest before the line a driver waits on.

⛔ **`evidence-bundle` could not be driven by a client and that was structural
rather than an omission.** Its torrent carries a hard-coded
`http://127.0.0.1:6969/announce`, so the only thing that could reach its
observer was something told the endpoint separately - which `curl` is and a
stock build is not. A client reads the address out of the file or it does not
announce at all.

⭐ **A reader this project did not write confirmed the file says what the
observer claims.** `torf` 4.3.1, from the package index into a virtualenv, took
the announce URL out of the generated `.torrent` and it matched both the address
printed by the observer and the port `curl` then announced to; `torf` also
re-derived the info hash `4bc6a5c90be5c4fc6a3c4281f2400af2d2c7700d` from the
metainfo in the written bundle, which the observer had printed independently.
⚠ Parsing a file is not a capture and needs no disposable host.

Driven on 2026-09-08 with `curl/8.5.0` announcing as `driven-by-curl-000001`:
one announce kept, query key order
`info_hash,peer_id,port,uploaded,downloaded,left,compact,event,key`, header
order `Host,User-Agent,Accept`, two segments, both artifacts verifying.

⚠ **The peer surface is dialled rather than offered, and the reason is a
constructor.** `TrackerResponse` is cloned into the responder while the lab is
still being built, and `Lab` binds port zero, so a tracker answer naming this
lab's own peer port would have to predict a port that does not exist yet. The
client's listen port is an argument instead and the lab dials it once an
announce proves the build read the torrent. ⭐ That is also the stronger role:
the side that dials sends its handshake first, so what comes back is the build
answering.

⚠ **What this still does not establish is a client.** No build is installed on
any host this project may capture on, so every run of it so far was driven by
`curl`. The adapter is what changes that.

### The adapter, written 2026-09-08

[`qbittorrent.sh`](../scripts/capture/adapters/qbittorrent.sh) is the highest
priority target and not the simplest one. `qbittorrent-nox` keeps a profile
directory, asks once for a legal notice, and reads its switches out of an INI
file, so the adapter writes the whole profile rather than patching whatever is
in `HOME`: a capture that edited an existing configuration would measure a build
under settings the run did not record.

⭐ **The torrent is a positional argument**, which is the one unattended control
this product already has that needs no web interface, no credential and no
second process. An adapter driving the WebUI would be measuring a build through
an authenticated API it had to configure first.

⚠ **`--confirm-legal-notice` is not optional.** Without it the first run of a
fresh profile blocks on a prompt, and the capture would sit there until the
observer's deadline with nothing on the wire and no error to read.

⛔ **None of that is measured yet.** The adapter has never installed anything:
this session could not run it, because a session host is not disposable, and the
dispatch is what establishes whether it works. The Windows half of the Prove
above is untouched.

### ⛔ The release listing was read once and summarised one build short

**Measured 2026-09-09, re-reading the same endpoint.** The paragraph above says
release-5.2.3 "offers a Linux `AppImage`, a Windows `x64_setup.exe` and a source
`tar.xz`, each with a detached `.asc` signature beside it". It carries
**fourteen** assets, and two of them are Linux AppImages:

```text
qbittorrent-5.2.3_x86_64.AppImage
qbittorrent-5.2.3_lt20_x86_64.AppImage
```

⛔ **The summary was one build short in exactly the place a route has to
choose.** A pattern written from it - `qbittorrent-*_x86_64.AppImage` - matches
both, and a selector that took the first match would install whichever the vendor
listed first, with the record reading the same either way.
`qbittorrent-{version}_x86_64.AppImage` matches one, and two matches is a refusal
rather than a first-match answer. `ACQ-02` carries the selector.

⚠ **What is still not measured is which libtorrent each AppImage carries.** The
suffix plainly names a variant and nothing here has established either, so the
adapter selects the vendor's unsuffixed build and the record names the asset it
took. A capture through this route measures whichever it installed, and says
which.

⭐ **And the resolver skipped a prerelease on a live listing rather than on a
fixture.** The newest release object on 2026-09-09 is `release-5.3.0beta1`, which
the source flags a prerelease and whose version text says so too; the resolution
selected 5.2.3 from five candidates. That listing is the recorded response
`check-release-route` runs over, kept whole for exactly that reason.

### What the info hash cost, measured 2026-09-08

⛔ **Writing that measurement into this record turned CI run 67 red on both
lanes**, and the check was right to fire: an info hash is forty hex digits, and
`check-no-secrets --public` hunts long hex because that is the shape of a
credential. ⭐ The rule is narrowed rather than switched off, to the phrase and
both backticks this project spells one with; a bare forty-digit run elsewhere in
a sentence is still a finding. ⚠ A local gate had been green over this file
minutes earlier, because the checks re-run after the edit were the ones the edit
looked like it touched and `--public` is a second invocation of a check whose
first invocation passes.

## CLIENT-02: qBittorrent Enhanced capture adapter

Source: operator scope and upstream Enhanced Edition repository
Priority: P1 | Effort: L | Status: OPEN

Problem: Enhanced Edition may alter observable identity and must not inherit a
qBittorrent profile by name or source similarity.

Approach: Acquire and drive it independently, retaining flavor metadata and
comparing observations only after both profiles exist.

Prove: two-route live runs either publish a distinct corroborated profile or
prove byte-for-byte observable equivalence without copying source-derived data.

## CLIENT-03: uTorrent capture adapter

Source: operator scope and August 2026 priority sample
Priority: P1 | Effort: L | Status: OPEN

Problem: The proprietary Windows client lacks reproducible active evidence.

Approach: Use a disposable Windows runner, official and package-manager
routes, silent automation where supported, and UI automation only as a bounded
fallback.

Prove: two same-version Windows installations emit agreeing connector records
and leave no state or traffic outside the laboratory.

## CLIENT-04: BitComet capture adapter

Source: operator scope and bit-cli historical identity mismatch
Priority: P1 | Effort: L | Status: OPEN

Problem: Historical BitComet-like identity assumptions make guessed mapping
especially unsafe.

Approach: Drive official and package-managed Windows builds through the lab
and retain raw handshake, tracker, and extension behavior.

Prove: a two-route Windows capture passes agreement without consulting a
source-code peer-ID mapping.

## CLIENT-05: aria2 capture adapter and connector

Source: operator scope and requirement for an independent CLI connector
Priority: P1 | Effort: L | Status: OPEN

Problem: aria2 is both a required target and a useful independently controlled
client, but those roles must not create circular corroboration.

Approach: Use JSON-RPC for lifecycle control, capture aria2 as a target with a
separate packet oracle, and use it as a connector only for other targets.

Prove: tests reject aria2 self-corroboration and a live two-route capture
publishes a profile supported by the Rust observer plus packet decoding.

### The adapter, written 2026-09-08

⭐ [`aria2.sh`](../scripts/capture/adapters/aria2.sh) is the simplest target in
the matrix. `aria2c` takes every setting on its command line, needs no daemon,
no profile directory and no interactive acceptance, and prints its version to
stdout, so nothing about one run of it depends on state a previous run left
behind.

⚠ **This is the target half alone and not the connector.** Driving aria2 as a
target and using it to corroborate another target are the two roles this entry
says must not create circular corroboration; nothing in that file observes
anything.

### What the four dispatches actually establish, read 2026-09-08

⭐ **The install logs answer the hang, and they answer it by refuting the
question.** `install-aria2-<run>-1` from client capture runs 3 and 4 was
downloaded through the route `AGENTS.md` rule 8 names, unauthenticated, and both
`install.log` files say the same thing:

```text
aria2 is already the newest version (1.37.0+debian-1build3).
0 upgraded, 0 newly installed, 0 to remove
```

⛔ **So no package was ever installed.** `apt-get install aria2` on
`ubuntu-24.04` is a no-op, `needrestart` runs only after a package operation, and
`NEEDRESTART_MODE` therefore could not have been the cause under any letter. Run
3 carried `a` and run 4 carried `l`; the two jobs hung identically. ⚠ Three
sessions of reasoning about that variable were reasoning about a code path the
runs did not take.

⛔ **And the hang is not in the install step.** The job step records, read from
the same route, put it somewhere else entirely:

| run | install step | where the job stopped |
| --- | --- | --- |
| 1 | never returned | *Install the client*, unbounded |
| 2 | never returned | *Install the client*, a bound that sent TERM and waited |
| 3 | success in 6s | *Upload the install logs* |
| 4 | success in 6s | *Upload the install logs* |

⚠ **The uploaded artifact from both hung runs is complete and downloadable**, so
that step's work finished and the step still never returned. What holds a runner
open after its work is done is not something these four runs can separate, and
naming a cause would be a guess with an artifact beside it rather than a
measurement. It is `CI-08`'s shape - a runner default nobody swept - and it is
recorded there rather than guessed at here.

⭐ **The adapter itself is sound, and this is its first driven pass.** Run on a
real `ubuntu-24.04` host: a genuine install in 8s, the no-op repeat in 3s, and
`version` answering `1.37.0` in under a second. Two of the four assumptions in
the file's own unmeasured block are now measured: `aria2c --version` opens
`aria2 version 1.37.0`, so the third field is the version, and the package route
installs without a prompt. The two that remain are about `start`, which needs a
capture.

⛔ **The finding that outlives the hang is what the no-op means.** A route that
installs nothing exits 0, and `install-client` reported `1.37.0` through the
`package` route for a binary that came with the image. `acquired` is the field
that says so now, `ACQ-03` carries what two such routes do to the two-route rule,
and the guard is proved by planting the derivation in
[`../scripts/capture/check-capture-client.sh`](../scripts/capture/check-capture-client.sh).

### The second route exists, and what it costs, measured 2026-09-08

⭐ **aria2 is the one target in the matrix whose two routes currently resolve the
SAME version.** Ubuntu 24.04 ships 1.37.0 and the vendor's newest release is
`release-1.37.0`, published 2023-11-15 - aria2 has not released since. ⛔ The
other two are not close: Ubuntu ships Transmission 4.0.5 against upstream 4.1.3,
and qBittorrent 4.6.3 against upstream 5.2.3, and `AGENTS.md` rule 5 forbids
backfilling an old version to make a pair agree. So the first two-route capture
this project can attempt is this entry's, not `CLIENT-01`'s.

⛔ **The vendor publishes no Linux binary**, which is a measured property of the
target rather than a gap in the adapter: the 1.37.0 release carries source
tarballs, two Windows zips and an Android build. A Linux release route is
therefore a source build, and it was driven here: 19 seconds to configure and 126
seconds to `make -j4`, producing a binary that answers `aria2 version 1.37.0`
with BitTorrent enabled.

⚠ **Two builds, one version, different features and different bytes.** `ACQ-03`
carries the table and the consequence; the short form is that the package route's
`/usr/bin/aria2c` is a 14-kilobyte shim over `libaria2.so.0` while the source
build is self-contained, and their feature lists differ.

⭐ **The release route installs now, and was driven end to end here.** It fetches
the tarball, unpacks it, configures, builds and installs into a prefix
`binary()` prefers, and refuses at each step rather than returning 0 over an
install it did not perform. 132 seconds from URL to a runnable `aria2c` that
answers `1.37.0`. ⚠ It deliberately does not `apt-get` its build dependencies:
that would make the release route reach the package index, and arguing afterwards
about whether headers count as acquisition is worse than refusing.

⛔ **Still not captured.** No run of this adapter has started a build against the
lab, so the entry stays open on the same four gaps every client entry has. What
changed is that the second route now exists rather than being fetched and
discarded.

### ⭐ The second route is wired into the workflow, 2026-09-09

**`capture-client.yml` can now dispatch it**, which it could not before: the
route is a matrix dimension beside the adapter, and the artifact it fetches is
resolved in a step of its own before the route is cut.

⛔ **What was actually missing was the artifact, not the plumbing.** This
adapter's release route refuses without `BIT_IDS_RELEASE_URL` "resolved before
the route was cut", and nothing in the repository produced one - `resolve-stable`
orders versions and selects no asset. `ACQ-02` carries the half that closes it;
what lands here is the declaration:

```text
release_repo=aria2/aria2
release_tag_prefix=release-
release_min_components=3
release_max_components=3
release_asset=aria2-{version}.tar.bz2
```

⚠ **`bz2` rather than `gz` or `xz`, recorded rather than implied.** The release
carries all three and they are the same source; the `bz2` is the one `ACQ-03`
already built from, so its digest is a control this project can check a fetch
against without acquiring anything new. ⛔ A pattern written `aria2-*.tar.*`
matches all three and is refused rather than resolved - measured, as a case in
[`../scripts/acquisition/check-release-route.sh`](../scripts/acquisition/check-release-route.sh).

⭐ **Driven here, end to end, against the live listing**: 24 candidates, 1.37.0
selected, one of six assets matched, and the URL that came out served bytes
`sha256sum -c` verified against the digest the 2026-09-08 measurement recorded.

⛔ **A dispatch is still what this entry needs**, and now there is one to make: a
`["package","release"]` dispatch is two hosts, one per route, and `ACQ-03` has
two installs to compare for the first time. ⚠ Nothing here establishes that the
runner image carries the C++ toolchain and OpenSSL headers the source build
needs; the route refuses with `configure`'s own log if it does not, which is a
route that failed rather than one that silently produced nothing.

## CLIENT-06: Transmission capture adapter

Source: operator scope and bit-cli source-profile generator study
Priority: P1 | Effort: L | Status: OPEN

Problem: Existing source-derived formulas are useful hypotheses but violate
this corpus's live-only evidence rule.

Approach: Automate transmission-remote, compare official and package routes,
and test all formula expectations solely against emitted traffic.

Prove: a two-route Linux and Windows run derives every published field from
raw observations and labels source expectations only as non-authoritative notes.

### The adapter, written 2026-09-08

⭐ [`transmission.sh`](../scripts/capture/adapters/transmission.sh) drives the
daemon through `transmission-remote`, which is the supported unattended path
since `transmission-cli` was removed upstream. Its whole settings file is
written, and the torrent is added by the tool the product ships for that.

⚠ **The package route stops and disables the service the distribution starts on
install.** A capture must own the only running copy, or it would measure
whichever one the torrent reached.

⚠ **`--foreground`, so the backgrounded process is the daemon itself.** A daemon
that forked would leave `stop` holding the identifier of a shell that had already
returned, and the build would outlive the capture.

⛔ **Nothing published here comes from a formula.** This entry exists because
source-derived peer-ID expectations are hypotheses, and the adapter contributes a
running process rather than one.

### ⭐ The first client capture, measured 2026-09-08

**Client capture run 1, job `transmission`, on `ubuntu-24.04`.** The first time
this project has observed a `BitTorrent` identity from a running build.

| what | value |
| --- | --- |
| installed | `transmission-daemon` `4.0.5`, package route, `Linux 6.17.0-1022-azure x86_64` |
| version asked of | the installed executable, by the adapter, before the route was cut |
| containment | `no route off this host (read /proc/net/route)`, asserted after the deletion and again by `capture-client` |
| announces | 2 |
| peer ID on the wire | `2d5452343035302d756435383564356171646f73` |
| transcript | 9 segments, 3 artifacts, verified by `sha256sum -c` |
| restore | the inverted egress guard refused, which is the route proved back |
| evidence | uploaded, 18 files |

⛔ **The peer ID is the measurement and the prefix is not a lookup.** What this
establishes is that a build reporting `4.0.5` put those exact bytes on the wire,
which is section 4's peer-prefix rule inverted rather than waived: a prefix this
project measured, against the run that measured it.

⛔ **It does not close this entry, and four things are why.** One route ran
rather than two, so `ACQ-03` has nothing to compare; one connector observed it,
so `SCHEMA-03`'s overlap is `not_corroborated` by construction; nothing wrote a
`Profile` into the store, so there is no record to cite; and the Prove names
Windows, which is untouched.

### ⛔ What the same run cost, and the defect it bought

**Two of its three jobs sat in `Install the client` for over half an hour and
reported nothing at all.** The job timeout killed the runner and took the log
with it, so the cause is not recoverable from that run.

⭐ **The defect is in the contract rather than in either adapter: no adapter call
had a time limit.** A hung install and a slow one are indistinguishable until
something bounds them, and `shell.md` section 9 already said so about any tool a
script shells out to. Every adapter call is bounded now, in
[`install-client`](../scripts/acquisition/install-client.sh) and
[`capture-client`](../scripts/capture/capture-client.sh) rather than only in the
adapters, because a convention in each adapter is a bound the next adapter
forgets. ⚠ A refusal now names the timeout and prints the route's own log, since
the workdir is uploaded only when the capture that follows succeeds.

Three narrower causes were fixed with it, each of which would produce exactly
that silence:

- stdin is `/dev/null` on every adapter call, so a product that asks a question
  gets end-of-file instead of a wait;
- `NEEDRESTART_MODE=a` is set for apt, because Ubuntu 24.04 ships `needrestart`
  and it opens an interactive dialog that `DEBIAN_FRONTEND` does not suppress;
- ⛔ `qbittorrent-nox --version` carries `--confirm-legal-notice`, which the
  `start` call had and the `version` call did not. That is a control on one of
  two paths into the same product, which is the shape this project's reviews
  call the most recurring hole there is.

⚠ **Which of the four it actually was is not established**, and saying so is the
honest form: the log is gone. The next dispatch answers it, because a bounded
call reports its own failure with the product's log beside it.

### ⛔ Client capture run 2, and the two defects it found in the fix

**It answered for one of the two and not the other**, which is worth stating in
that order because the second is the more interesting failure.

⭐ **`qbittorrent` now fails in fifty seconds with a readable verdict**:
`installed through the package route but would not report a version`. The
install itself succeeded in fourteen seconds; it is the `version` call that
refuses. ⛔ **And the run could not say WHICH refusal fired, because of two
defects in code written the same day:**

1. **The adapters read `--version` through a pipe.** `LINE=$(... | head -1) ||
   cannot` reads `head`'s status, and `head` exits 0 over anything, so the guard
   was dead code and a build that refused to answer reached the parse instead of
   the refusal. ⚠ This repository's oldest stated rule, broken in every adapter
   at once, on the day they were written.
2. **The caller discarded the adapter's stderr and then reported its absence.**
   `install-client` ran `version` under `2>/dev/null`, so an adapter that said
   exactly which of its three refusals fired arrived as a bare exit code.

Both are fixed and both have a case: the exit code is read unpiped, the
adapter's message is printed with the refusal, and the harness asserts a
planted message reaches the log.

⛔ **`aria2` is not answered, and the bound did not fire.** Its install step ran
for thirty-five minutes under a 900-second bound and the job's own timeout killed
the runner, taking the log with it for the second time. ⚠ **Two possibilities
remain open and the run cannot separate them**: `timeout` sends `TERM` and then
waits, so a child that blocks or ignores it is never killed; or the bound fired
and something the adapter left running held the step open. Both are now
addressed - `-k` forces a `KILL`, the bound sits at 420 seconds under a
25-minute job so it reports with margin - and neither is confirmed.

⭐ **The certain fix is the one that does not depend on the diagnosis.** The
workflow uploads the install logs on `always()` now, so the next hang leaves its
own evidence whatever kills the runner. ⚠ Two dispatches lost their diagnosis to
a step whose only upload came after a capture that never happened.

### ⭐ Client capture run 3: both fixes worked, and the product answered

**The log carried the product's own words for the first time:**

```text
-- the adapter said
qbittorrent: qbittorrent-nox --version exited 1: Bad command line:
  --confirm-legal-notice is an unknown command line parameter.
install-client: qbittorrent installed through the package route but would not
  report a version (adapter exit 2)
```

⛔ **So `--confirm-legal-notice` was never a control. It is an argument this
build refuses**, and it was on BOTH paths into the product: adding it to the
version call to match `start` looked like closing a one-gated door, and what it
actually did was spread a refused argument to a second place. ⭐ The acceptance
is written into the profile as `[LegalNotice] Accepted=true` now, which is the
next assumption and is not measured either.

⚠ **That is what the unmeasured-assumption block in each adapter is for**, and
this is the first entry in one to be refuted rather than confirmed. The block
said the flag was what stops the prompt; the product says the flag does not
exist.

### ⛔ And aria2 moved its hang rather than losing it

**Its install succeeded in run 3 and the step AFTER it hung instead.** The
install logs uploaded, so nothing was lost this time.

⚠ **The most likely reading is the letter in `NEEDRESTART_MODE`.** Run 1 had no
such variable, so `needrestart` was interactive and the install itself hung; run
3 set `a`, which stops the dialog by RESTARTING the services it lists - on a
runner that means restarting daemons the job is standing on. `l` is `list`: it
reports and touches nothing, and both adapters use it now.

⛔ **That is a reading and not a measurement**, and it is the third guess in this
area. What is measured is the shape: the hang moved from the install step to the
one after it when that variable changed, and nothing else about the adapter did.

### ⭐ Client capture run 4: a second client, and a second capture of the first

**Two of the three jobs captured a build, and the third isolated its own
failure.**

| target | build | announces | peer ID on the wire | transcript |
| --- | --- | ---: | --- | --- |
| qbittorrent | `4.6.3` | 1 | peer ID `2d7142343633302d596939654d4d7e38664f7866` | 5 segments, 3 artifacts |
| transmission | `4.0.5` | 2 | peer ID `2d5452343035302d756435383564356171646f73` | 9 segments, 3 artifacts |

⭐ **`CLIENT-01` has its first measurement.** The profile the adapter writes is
what a fresh `qbittorrent-nox` reads, the torrent is a positional argument, and
the build announced to the lab under containment; the `[LegalNotice]` key in the
profile is what the refused flag was supposed to do, and it works.

⛔ **Neither entry closes.** Every one of the four gaps the first capture had is
still open for both: one route, one connector, no `Profile` in the store, and no
Windows.

⚠ **Transmission's second capture reports the same build and a different peer
ID**, which is what a per-session identity looks like and is exactly what
`SCHEMA-04`'s sampling model exists for. Two samples is not a lifetime
measurement and nothing here claims one.

### ⛔ What is left of the aria2 hang, isolated

**`NEEDRESTART_MODE=l` did not fix it.** Its install succeeded again and the step
after it hung again, so the reading that the letter was the cause is refuted for
`l` and unsettled for `a`.

⭐ **What run 4 does establish is the boundary.** The hang is specific to this one
package install, it survives the step that caused it, and it is not the install
command's own duration: `install-client` returned, the record was written, and
the next step is where the job stops. ⚠ Two adapters doing the same `apt-get`
on the same image do not hang, so it is the `aria2` package rather than the
route.

⚠ Its install log uploaded on every run since the `always()` step landed, so the
next session reads `update.log` and `install.log` from
`install-aria2-<run>-1` rather than dispatching to find out. ⚠ That artifact name
carries the route now - `install-aria2-package-<run>-1` - because two matrix legs
share an adapter.

### ⚠ The release route declares its artifact and still refuses, 2026-09-09

**Both halves are true at once and the split is the honest one.** This adapter
now declares which file a release route would fetch:

```text
release_repo=transmission/transmission
release_tag_prefix=-
release_min_components=3
release_max_components=3
release_asset=transmission-{version}.tar.xz
```

and the route still refuses, because nothing here knows how to build it. ⛔ This
project can say exactly which artifact a route would take and cannot say how to
compile it; declaring the first is what makes the second a named gap rather than
a route that is simply unreachable.

⚠ **Measured against the live listing on 2026-09-09**: release 4.1.3 carries
eleven assets - four `.msi` installers, four `-pdb.7z` symbol archives, a macOS
`.dmg`, a `-dsym.zip` and exactly one `transmission-4.1.3.tar.xz`. ⛔ Two of them
are capitalised `Transmission-`, which is why the pattern is matched
case-sensitively and a case in the Rust suite pins it.

⚠ **And the versions still would not meet.** Ubuntu 24.04 ships 4.0.5 against
upstream 4.1.3, so a `["package","release"]` dispatch of this adapter is two
hosts that acquire two different versions - which `AGENTS.md` rule 5 forbids
resolving by backfilling. The pair to attempt first is `CLIENT-05`'s.

## CLIENT-07: Deluge capture adapter

Source: operator scope and upstream project
Priority: P1 | Effort: L | Status: OPEN

Problem: Deluge's daemon and UI separation can change automation and engine
version evidence across packages.

Approach: Drive the daemon through its supported interface, record bundled
engine identity, and compare official packaging with host package routes.

Prove: live Linux and Windows records prove product and embedded-engine
versions plus agreeing observed surfaces.

## CLIENT-08: BitTorrent capture adapter

Source: operator scope and August 2026 priority sample
Priority: P1 | Effort: L | Status: OPEN

Problem: The proprietary Windows product must be distinguished empirically
from uTorrent despite related ownership and code lineage.

Approach: Isolate installations and compare emitted fields only after separate
two-route captures complete.

Prove: the profile contains independent raw evidence and no inherited field
from the uTorrent adapter.

## CLIENT-09: BiglyBT capture adapter

Source: operator scope and upstream project
Priority: P1 | Effort: L | Status: OPEN

Problem: Java runtime and packaging differences can affect observable client
behavior and must be represented, not flattened.

Approach: Record JVM and package metadata, automate the supported interface,
and acquire official and package-manager builds on both host families.

Prove: runs correlate product, JVM, and host facts with agreeing protocol
observations for the same stable release.

## CLIENT-10: Tixati capture adapter

Source: operator scope and August 2026 priority sample
Priority: P1 | Effort: L | Status: OPEN

Problem: The proprietary Linux and Windows client has no open control source
and may require UI-driven setup.

Approach: Prefer documented controls, use bounded UI automation if necessary,
and verify installation and cleanup independently of the UI.

Prove: two-route captures on supported hosts pass connector agreement and
re-run unattended from a clean guest.

## CLIENT-11: KTorrent capture adapter

Source: operator scope and KDE upstream
Priority: P1 | Effort: L | Status: OPEN

Problem: Linux distribution packages may trail or patch the upstream stable
release.

Approach: Compare a verified official build route with a distribution package
and block publication when their upstream build identities diverge.

Prove: two Linux routes resolve to one stable build and produce agreeing live
profiles, or emit a documented blocking divergence.

## CLIENT-12: Free Download Manager capture adapter

Source: operator scope and August 2026 priority sample
Priority: P1 | Effort: L | Status: OPEN

Problem: FDM combines general download behavior with BitTorrent support and
needs a torrent-specific controlled path.

Approach: Exercise only synthetic torrents, compare official and package
routes, and separate HTTP download headers from BitTorrent identity fields.

Prove: Linux and Windows runs produce torrent-specific two-connector evidence
without contacting a public swarm.

## CLIENT-13: Zona capture adapter

Source: operator scope and August 2026 priority sample
Priority: P1 | Effort: L | Status: OPEN

Problem: Zona is proprietary, Windows-oriented, and may not expose unattended
controls or a dependable second package route.

Approach: First measure current availability and terms, then use a disposable
Windows adapter. Keep the entry open if two independent routes cannot be
verified; never substitute an inferred profile.

Prove: a same-version two-route capture passes, or the entry records a precise
blocker and no Zona profile is published.
