# Resume

**Task:** Take the work order in [`../../TODO/PROGRESS.md`](../../TODO/PROGRESS.md)
in order, committing and pushing each green unit to `main`.

⛔ **Nothing is blocked.** Every question an earlier session recorded as needing
the operator is answered in that file under *Settled decisions*. Do not re-raise
them, and do not record a new blocker without running the command that would
settle it.

**Tree:** Re-measure it. This file is a claim about a tree that has moved. Check
the branch, the remote, the clone depth, `git status` and `HEAD..origin/main`
before editing anything.

**The container may start on a `claude/*` branch with `user.name` set to an
agent and a shallow clone.** All three were true at the start of the last six
sessions. Correct them before any edit: the branch to `main` per rule 7, the
identity to the operator's own per rule 11, and the clone with
`git fetch --unshallow`. ⛔ Read the identity out of the history with
`git log --format='%an <%ae>' | sort -u`; never type it into a tracked file,
which is what `check-no-secrets --public` refuses.

⭐ **Install the three tools with one command:** `sh scripts/doctor/provision.sh`.
About four seconds on a host with none of them, every download verified against a
pinned digest first. ⛔ **Without `pwsh` the gate does not merely shrink - it goes
RED**: `check-capture` FAILS and `check-twins` skips, so a session that skipped
this step could read that failure as a defect in the tree.

---

## Where the work is

**In flight:** `OBS-07`, the second connector, which is the validity
prerequisite everything below waits on. `CI-09` sits behind it. `CLIENT-14`
closed on 2026-09-09 and its target gained a second acquisition route after
closure. `CLIENT-05` is open on the aria2 hang.

⭐ **Measured at the start of the 2026-09-10 session, rather than carried:** the
container started on `claude/zen-lovelace-kxmgvb` with `user.name=Claude` and a
shallow clone - all three, for the seventh session running. Corrected to `main`,
to the operator identity read out of `git log`, and unshallowed. The tree was
clean and level with `origin/main` at `4fd4bc9`, and `sh scripts/doctor/provision.sh`
installed `pwsh` 7.4.6, `shellcheck` 0.10.0 and `shfmt` 3.14.0 in one command.
The baseline gate over that tree is **33 checks, 32 passed, 0 failed, 1 skipped**,
the skip being `check-remote-items`.

⛔ **A CAPTURE BUNDLE ALONE CANNOT PRODUCE A VALID RECORD, measured 2026-09-09.**
`E-ACQ-09` and `E-ACQ-10` require each route's `installed_evidence` to name a
`process_output` entry the record carries - the bytes the build printed when it
was asked its version. That output is `version.err`, and it is in the **install**
artifact, not in the capture bundle: run 14's `capture-client-*` artifacts carry
the metainfo and the two transcripts and nothing the build said about itself. ⚠ So
an assembler reads TWO artifacts per lane, and `SHA256SUMS` inside the capture
bundle covers three files that are not all of the record's evidence.

⛔ **THE ROUTE HALF IS NOT SOLVED, AND THE PARAGRAPH THAT STOOD HERE SAID IT
WAS.** It said run 14 gave this project two routes, so a record could be written
and stored and would merely fail to publish for want of a second connector.
⚠ **Something tried it on 2026-09-09 and all of that is wrong.**
`assemble-capture` was driven over run 14's four artifacts and refuses:

| what the artifacts say | what refuses it |
| --- | --- |
| both lanes' `release/resolution.txt` differed **only in their timestamps** - one `source_url`, one `listing_sha256`, one `asset_url` | ⛔ `E-ACQ-07` - ⭐ **repaired**: the source route reads git refs, `ACQ-02` |
| every attestation declares **one** connector | ⛔ `E-CAP-01`, at the **validity** gate |
| nothing recorded how the artifact was **packaged**, and `package` is in `StoreKey` | ⛔ a guessed one files two packagings of a version at one path - ⭐ **repaired**: every adapter declares it per route |
| nothing the capture path writes carries the source route's commit | ⛔ `E-ACQ-06` - ⭐ **repaired**: the adapter now records `rev-parse HEAD` and `install-client` refuses a `source` route without one |

⚠ **`classify_across` never ran on run 14** - it takes two `Profile`s and neither
exists. What is measured is that the pair's **shape** is `Divergent`, read off
two records the harness builds that differ only in their peer-ID tails.

⛔ **AND THE CONNECTOR CLAIM WAS WRONG IN THE DIRECTION THAT MATTERS.** Measured
by stripping the golden fixture two ways, each exit code read unpiped: a record
declaring **two** connectors where only one observed each field is *valid* and
`provisional, not publishable` with six `E-PUB-02` rows; a record declaring
**one** connector is `refused`, `invalid document`, `E-CAP-01`. Every capture
this project has run declares one. ⭐ So `OBS-07` is a prerequisite for a record
EXISTING, not for publishing one, and it moved up the work order accordingly.

⛔ **`BuildEquivalent` is unreachable for a real client through this path.** A
peer ID carries a per-connection random tail; a single capture can only state
such a field as `constant` with one sample, because `patterned` and `variable`
both need two; and `classify_across` calls any disagreement on a shared field
`Divergent`. ⭐ `SCHEMA-04`'s sampling model is where several captures become a
`patterned` field, it sits above the record, and nothing has run it.

### ⛔ Two measurements of run 14 that no earlier session had read

⛔ **THE PEER ID DIFFERS BETWEEN SURFACES INSIDE ONE RUN.** Every record here has
called the twelve bytes after `-qB5230-` a per-**run** tail, measured by comparing
one announce per run across runs. Reading both transcripts of one bundle refutes
the wording: the tracker announce and the peer-wire handshake of a single capture
carry different tails.

| lane | `tracker_http` announce | `peer_wire` handshake |
| --- | --- | --- |
| release | `-qB5230-3SGS8~CB*gUf` | `-qB5230-O*4PZEf5_2aU` |
| source | `-qB5230-*eQ2phy)!)RO` | `-qB5230-kTAq!FSXbxh2` |

⭐ Four tails from two runs, one prefix, **two per run**. ⚠ The attestation records
only the tracker one as `measured_peer_id`, which is why the distinction stayed
invisible: nothing had read the second transcript. ⭐ The schema already expects
it - `tracker_http/peer_id` and `peer_wire/peer_id` are separate field paths with
separate fixed widths - so this is a record that has to be written as two
observations, and a `constant` on either would be false.

⛔ **THE TWO BUILDS DIFFER IN THEIR COMPILER AND IN NOTHING ELSE THEY REPORT.**
Both lanes' `version.err` are 1259 bytes and `diff` says `20c20`: `gcc 11.4.0`
against `gcc 13.3.0`. Enabled Features, Hash Algorithms and Libraries - including
`libtorrent/2.1.1` and `OpenSSL/3.5.6` - are identical strings.
⚠ That is a **weaker** difference than `ACQ-03`'s aria2 pair, where the two builds
differed in features, TLS library and compiler. Two installs whose bytes differ
and whose self-description differs only by the compiler that produced them is
what `BuildEquivalent` describes; it is not a reason to expect them to behave
differently on the wire, and both lanes did put different peer-ID tails on it.

⭐ **`CLIENT-14` IS CLOSED AND THE HANG IS NOT IN FRONT OF THAT TARGET.**
`capture-client` runs 11 and 12 both reached the *Capture* step - the eleventh
and twelfth dispatches, after ten that did not - each in about two minutes with a
one-second install, each attesting `stock_client=true`, `measured_build=2.7.5`,
`egress=closed`, and each bundle verifying with `sha256sum -c` outside the run
that wrote it. ⛔ **Both measured a peer ID beginning `-qB5230-`**, qBittorrent
5.2.3.0's prefix, emitted by a stock `aria2-next`; the twelve bytes after it
differ between the runs. ⚠ This paragraph said "so the tail is per-run", and the
section above refutes the wording: it differs between two surfaces of one run,
so it is per-connection.
⛔ **Still not a record**, and `E-ACQ-01` is no longer the reason: run 14 gave
that target a second lane, and the table above is what refuses the pair.

**Next, in order:**

1. ⭐ **DONE, 2026-09-10. `OBS-07`'s second connector exists and the capture
   path refuses to run without it.**
   `scripts/capture/connectors/cpython-stdlib.py` fills the contract
   `assemble-capture` declares: `connector/<id>.txt` in the bundle, one
   `field_path=value` line per field, the value lowercase hex, `absent` or
   `out_of_scope`, and the attestation naming it in `connectors=`.
   ⚠ **It is two READINGS of one capture's bytes, not two observations of the
   wire**, and it is unproved on a runner. The Approach's other half - a stock
   libtorrent harness putting its own bytes on the wire - is a residual that
   needs a disposable host.
2. ⭐ **DONE, and unproved on a runner.** The source route resolves its own tag
   with `git ls-remote --tags --refs`; `ACQ-02` carries the reader. ⚠ No
   dispatch has taken the new step, so what is proved is the resolver and the
   harness, not a lane.
3. **`CI-09`**, which now sits behind the connector alone. ⭐ `assemble-capture` and
   `check-assemble` are written and green; what is missing is a capture whose
   artifacts it accepts. ⚠ It writes a `Profile` and not the `RunManifest` that
   has to sit beside one; the entry's residuals say why.
   ⚠ **The bounds are freshly sized and only just.** The source install took 496
   seconds where this host takes 366 - a runner is about 1.35x slower - so a
   locally measured build time is a lower bound and never an estimate.
4. **`CI-07`**, the PowerShell halves for the declared rows. ⭐ Two cheap halves
   are done: `check-gate-rows` compares the two runners' row lists, and the sweep
   has classified every declared row - only the ones whose subject is portable
   Rust are this entry's work, and `store-lib.ps1` is most of that. ⚠ `check-assemble` is another declared-unavailable row and
   it needs `store-lib.ps1` like the rest. ⛔ **No count of those rows is written
   here**: `CI-01` and `CI-07` each record one going stale in prose, and this
   sentence carried a third - "the nineteenth" - over a runner declaring
   eighteen, until a claim audit counted them.
5. **`CI-08`'s new residual**, the load-sensitive `check-step-bodies` row.
6. **Shard `check-workflow` across runners.** It is still the whole CI wall
   clock. `CI-01`'s residual says why it is its own unit.

---

## The aria2 hang, as far as eleven runs can answer it

⛔ **ELEVEN aria2 DISPATCHES, NO aria2 CAPTURE, AND NOT ONE REACHED THE *Capture*
STEP.** Every lane stops inside *Install the client*; transmission, qBittorrent
and `aria2-next` pass through the same step on the same image.

⛔ **FOUR BOUNDS AT FOUR LEVELS HAVE BEEN MEASURED NOT TO FIRE**: `timeout -k 20
420` inside `install-client` (run 8), the runner's `timeout-minutes` (run 6), a
watchdog loop with a 480-second deadline in the step's own shell (run 9), and
`timeout -k 30 540` around the step's own command (run 10, terminal state read:
29.5 minutes in that step, `cancelled`, zero artifacts).

⚠ **And no such job has ever written a log.** The blob is never created, which is
why rule 8's route answers 302 to a `BlobNotFound`. A step that were merely stuck
would leave a runner able to enforce one of four bounds and to upload a log.
⛔ That is a reading and it is not recorded as a cause; what it changes is where
to look - at the host rather than at the shell.

⛔ **A SIXTH READING WAS REFUTED ON 2026-09-09 AND IT WAS THE STRONGEST ONE.**
The reading: aria2 ships on `ubuntu-24.04`, so an aria2 lane's *Install the
client* asks the adapter for a version and that call executes a real binary,
while a transmission lane finds none and runs nothing - present in every hung
run, absent from every green one. ⭐ Run 13 is the first aria2 lane to carry
`CI-08`'s probes, and *Probe the preinstalled product* runs that exact call one
step earlier: it took **0 seconds** and succeeded, and the next step hung anyway.
⚠ All three probes returned in 0 seconds, so host resources, `sudo` and the
adapter are all cleared. What *Install the client* still does that they do not is
run `install-client` under `sudo -E`.

⭐ **Three bounded probes run before the install** - host resources, `sudo`, and
the preinstalled product - because when a job produces no log, no artifact and no
bound, the one signal left is which step the API last reported in progress.
⭐ **They have now been dispatched on both kinds of lane and they are what
refuted the sixth reading**: all three returned in 0 seconds on runs 11 and 12
(`aria2-next`, which then captured) and all three returned in 0 seconds on run 13
(`aria2`, which then hung).

### The four readings refuted before those bounds

⛔ **FOUR READINGS HAVE BEEN NAMED AND ALL FOUR WERE WRONG.**

| the reading | what refuted it |
| --- | --- |
| the letter in `NEEDRESTART_MODE` | both hung runs installed nothing, so `needrestart` never ran |
| the aria2 **package** rather than the route | the `release` route runs no package operation and hung identically |
| the **upload step** | two transmission lanes ran that step in one second, in the same run as two aria2 lanes that hung |
| a process holding **the step's own output pipe** | ⛔ run 8, where the route's output went to a FILE and both lanes hung for twenty-two minutes anyway |

⭐ **What run 8 does establish is where the step is NOT.** `install-client` bounds
the install call at 420 seconds and kills 20 seconds after that, so the last
moment that call could have ended is 440 seconds in. Both lanes ran three times
that. ⛔ **Whatever is slow or stopped is outside the bounded install call**,
which leaves that script's unbounded parts - its command substitutions and its
digests - and the step itself.

**Run 8 also carries a confound, stated rather than argued away.** It changed
two things at once: the redirection and a holder report added to the same step.
Both are bounded now.

⚠ **The watchdog paragraph that stood here described run 9's design and run 9
refuted it.** A loop in the step's own shell with a 480-second deadline did not
fire, and `timeout` around the step's own command did not fire on run 10 either.
Both are still in the tree because a step that ends is still what the evidence
needs; neither is a bound anyone should now expect to work.

⚠ **`etimes` beside `stat` is the question all of it was built to answer**: a
process whose elapsed time grows while its state is `R` is slow, and one sitting
in `D` or `S` is stopped. Ten runs of step timings cannot tell those apart, and
no run has yet produced a process table from inside the window.

**Two local reproductions came back negative and neither settles it.** This
host is not the runner image and proved it in the same run: `aria2` is absent
here, so the install actually installs, where on `ubuntu-24.04` it is a no-op.

---

## How this project is checked

⛔ **Run the gate with one command, `sh scripts/common/check-gate.sh`, after the
last edit.** ⭐ It is about **150 seconds** on this host.

⚠ **A count of its rows is not written here.** This sentence said `32 checks`
and the runner reported 33 on 2026-09-10, which is the fourth count of a row set
to go stale in this repository's prose - `CI-01` and `CI-07` record two and
`scripts/README.md` a third. `sh scripts/common/check-gate.sh --rows` prints the
list, and `check-gate-rows` is what compares it against the other lane's.

**The gate is not the whole of part (a).** `cargo clippy`, `cargo fmt --check`,
the test suite and `sh scripts/ci/check-workflow.sh` are separate.

⭐ **A capture workflow's step bodies now RUN**, which nothing here did before.
`scripts/ci/check-step-bodies.sh` lifts a block out of `capture.yml` or
`capture-client.yml` and executes it the way the runner does, with the output on
a **pipe**. ⛔ A step is over when its command has exited AND that pipe has
reached end of file, so a process the body leaves behind holding the step's
stdout keeps the runner waiting on work that finished - and no exit code says so.

⛔ **TWO CHECKS JOINED BY `&&` ARE ONE CHECK.** Run each and read each status.

**And a subset of the gate chosen by hand is not the gate.**

⛔ **`check-workflow.sh` is not in the gate and cannot be**, because two of its
cases run the gate. It is about 20 minutes and the CI lane runs it in a job of
its own. ⚠ Killing and restarting it costs the whole 20 minutes: make every edit
first, then run it once.

⛔ **AND DO NOT RUN THE GATE WHILE `check-workflow` IS RUNNING**, or `git add`
while it is, which corrupts its plant-and-restore accounting.

⭐ **`check-step-bodies` WAS THE FLAKY ROW AND IT IS FIXED.** It went red three
times on 2026-09-09 inside a gate and passed alone every time. ⛔ The obvious
reading - that the pipe-close bound was too tight - was wrong, and two
reproductions built on it came back green. What found it was capturing the
failing run's own output: the gate PRINTS the failing check's log, and one case
named itself. The planted leak was a `sleep 8` spawned inside a body that runs
the whole install step, and the pipe is only examined after the body exits, so
under load the plant expired before it was measured. `LEAK_SECONDS=45` now names
it once and a guard checks `LEAK_SECONDS > CLOSE_SECONDS`. `CI-08` carries it.
⭐ **The lesson is the method, not the constant:** when a gate row goes red at
random, run the gate in a loop and READ THE LOG IT PRINTS before theorising.
⭐ Corroborated by the harness that was failing: `check-workflow` runs the whole
gate about ten times and lost one `gate_control` case on each of its two runs
before the fix, a different one each time; on the final tree it reported
**97 of 97**. ⭐ Its cases time
things - a bound that must fire as `124`, "a product slower than one tick and
inside the bound survives", and whether a step's output pipe reaches end of file
- so a loaded host can move a case across its own boundary.

⚠ **That is one measured instance plus a mechanism, not a proven cause of every
red.** `check-workflow` failed one `gate_control` case on each of two runs, a
DIFFERENT case each time, and those cases only report "the clean tree failed a
check" without naming which - and the harness deletes its workdir, so the naming
log is gone. ⛔ Do not spend a session bisecting a tree over this: run the failing
check alone first, and believe a red only when it repeats unloaded.

⛔ **Read exit codes from the process that produced them, unpiped.**

---

## What a review has to know before it starts

These are the defect classes this project has shipped and caught.

⛔ **A step does not end when its command exits.** Every harness here redirected
step output to a file, and a file has no reader to wait on, so the whole class
was invisible until something ran a body through a real pipe.

⛔ **A shell applies redirections left to right.** `>log 2>&1 3>&1` points fd 3
at the LOG, not at the step's pipe - so a plant written that way applies
somewhere the case did not name and reports the defect not happening.

⛔ **`timeout 0` MEANS NO LIMIT.** A ceiling edited to zero is an infinite one.

⛔ **A PowerShell `[switch]` collides with a local differing only in case**, and
they are then one variable: `[switch]$Rows` against `$rows` made every invocation
of that runner fail to bind. Third instance here, after `$args` and
`[switch]$Marker` against `$marker`. All three were found by running something
once rather than by reading it.

**A count in prose is a value in two places with nothing comparing them.**
`scripts/README.md` called `check-workflow` "a thirteenth mutation prover" and it
was wrong the moment another landed. Prefer "every" or "most" to a number.

**An in-place `sed` edits every line that matches, not the one you meant.**

**A gate run must leave the working tree as it found it.** `tree-unchanged` is
a row in both halves.

⛔ **A green local gate does not mean a green lane, because a DEFAULT can change
under you**, and **a step's exit status is whatever the block LEFT BEHIND**
unless it is a decision.

⚠ **A surviving plant is a question, not a verdict**, a harness exit of 2 is
*could not run*, and a plant that did not **apply** is a third status. All three
happened this session.

**A plant whose expected outcome is a PASS proves nothing.**

⛔ **A rule a document says this repository has is not a rule it has.** Grep for
the check before believing it runs. ⚠ Third instance, found on 2026-09-09:
`docs/client-matrix.md` said its target set is "pinned by `check-project` against
the catalogue in both directions", and `check-project` in fact carries a **list
of ids** and asks that each appear in both files. A target added to the catalogue
and forgotten in the matrix is caught by nothing, in either half of the check.

⛔ **A product's help text names a switch only if you already suspect it; its
socket table names it whether or not you do.** `aria2-next` was run with all
three discovery switches the adapter contract names turned off and was still
bound to **UDP 1900**, because `bt-port-mapping` - UPnP and NAT-PMP - defaults to
true. It was found in `/proc/<pid>/fd` and `/proc/net/udp`, not in the help.
⚠ Whether the other three adapters have surfaces of their own that nobody has
read this way is an open question.

⚠ **A version is not always the third field.** `aria2` prints `aria2 version
1.37.0` and `aria2-next` prints `Aria2 Next version 2.7.4`, so the adapter beside
it would have published the literal string `version` - and it would have passed
every check here, because it is a non-empty field.

**The strongest control available is a reader this project did not write.**

⛔ **A CAPTURE'S WORKDIR IS ITS EVIDENCE BUNDLE, so rule 12 applies to what an
adapter writes there.** Found on capture-client run 11 by downloading the
artifact and listing it: `aria2-next` wrote a per-run RPC token where `stop`
could read it, and `capture-client` packed that directory, so the token shipped
inside the artifact as `client/rpc-token`. ⚠ Neither step was wrong alone, no
reading found it, and the gate structurally cannot - the bundle is not in the
tree `check-no-secrets` scans. ⭐ Look in the artifact a real dispatch produced.

---

## Facts a session must not restate wrongly

⛔ **The publisher cannot run at all.** It downloads an artifact named `bundle`
and nothing in this tree produces that name.

⛔ **Nothing has been published and no measured record exists.** ⭐ Builds HAVE
been measured: Transmission 4.0.5 four times and qBittorrent 4.6.3 once, each
attesting `kind=client`, `stock_client=true`. ⚠ Those are evidence bundles and
attestations, not `Profile`s.

⛔ **A capture declaring ONE connector is INVALID, not merely unpublishable.**
`E-CAP-01` fires inside `validate`. The "validates, then `E-PUB-02` refuses"
sentence three handoffs carried is true only of a record declaring two
connectors where one observed each field. ⭐ **Since 2026-09-10 the capture path
declares two and refuses to run otherwise**, so no future dispatch can produce
the shape that refused run 14; every capture already in existence still has one.

⛔ **Two lanes of one dispatch are not two routes if one resolution fed both.**
`E-ACQ-07` compares what DECIDED the version, and `capture-client.yml` resolves
once on purpose.

⛔ **A single capture can only state a varying field as `constant` with one
sample**, so two such records disagree and `classify_across` answers `Divergent`.
`BuildEquivalent` needs the sampling model above the record, which nothing runs.

⭐ **A `release` route has acquired a build on a capture host**: aria2 compiled
from the vendor's tarball in 144 seconds, two builds at one version with
different features, TLS libraries and compilers.

**Four captures of Transmission 4.0.5 have reported THREE distinct peer IDs**, so
the count of captures and the count of identities are not the same number: runs 1
and 4 reported the same one and runs 7 and 9 each reported another.

⛔ **A hosted Windows runner's fingerprint is not a freshness signal.** The claim
marker is what detects a survived host.

⛔ **The nine commit stamps before 2026-09-06T07:56Z are fabricated.** They are
not retro-corrected. Read the machine clock with `date -u +%Y-%m-%dT%H:%M:%SZ`.

**No repository owner or name is hardcoded anywhere in this tree.**

`check-remote-items` cannot run on a session host and installing `gh` does not
fix it. It is the one observed skip.

⚠ **The step-body harness passes `-NoProfile` and GitHub's wrapper does not.**
That departure is deliberate and stated: a gate row that loaded a contributor's
profile would go red for something outside this repository. So a defect a profile
would cause is outside what that harness can see.

Every session record is listed in [`README.md`](README.md), and `check-docs`
refuses one that page does not link.
