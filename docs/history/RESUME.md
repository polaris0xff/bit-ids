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

⚠ **The container may start on a `claude/*` branch with `user.name` set to an
agent and a shallow clone.** All three were true at the start of the last five
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

**In flight:** `CLIENT-05` and `CI-09`, and they are now the same blocker.

⛔ **THE RECORD THE WORK ORDER WAITS ON CANNOT BE WRITTEN FROM ANY CAPTURE THIS
PROJECT HAS RUN.** Measured on 2026-09-09 by stripping the golden fixture twice:

| the capture that exists | what the library does | where it bites |
| --- | --- | --- |
| one connector | **validates**, then `E-PUB-02` refuses to publish it | publishability |
| one route | `E-ACQ-01` refuses it | ⛔ **validity** |

⭐ **`E-ACQ-01` is right and the sentence this file used to carry was wrong.**
This page said "a single-route, single-connector capture VALIDATES and refuses to
publish"; only the connector half is true. So `Profile::to_json` will not write a
single-route record and the store cannot hold one, and every capture so far -
transmission three times, qBittorrent once - is one route.

⭐ **What unblocks it is a two-route capture, and the machinery for one now
exists.** `capture-client.yml` takes the route as a matrix dimension beside the
adapter, and `resolve-release.sh` chooses the artifact a `release` route fetches.
⚠ aria2 is still the only target whose two routes resolve the same version.

**Next, in order:**

1. ⛔ **Get an aria2 job past *Install the client*.** That step now hangs on both
   aria2 routes and nothing else; the next section is what five runs establish
   about it. Everything below waits on it, because aria2 is the only two-route
   pair available.
2. **A record in the store**, once a two-route capture exists. `not_corroborated`
   is a recordable state; one route is not.
3. **`CI-07`**, the PowerShell halves for the fourteen declared rows.
4. ⚠ **Shard `check-workflow` across runners.** It is now the entire CI wall
   clock, 21 of 21.3 minutes. `CI-01`'s residual says why it is its own unit.

---

## The aria2 hang, as far as seven runs can answer it

⛔ **Three readings have been named and all three were wrong.**

| the reading | what refuted it |
| --- | --- |
| the letter in `NEEDRESTART_MODE` | both hung runs installed nothing, so `needrestart` never ran |
| the aria2 **package** rather than the route | the `release` route runs no package operation at all and hung identically |
| the **upload step** | two transmission lanes ran that same step in one second, in the same run as two aria2 lanes that hung |

⭐ **What run 7 establishes is where it sits.** With *Upload the install logs*
moved to the end of the job, the hang appeared in the step it used to follow: run
6's aria2 package install took **six seconds** and run 7's ran **ten minutes**
without completing. So what hangs is whatever step is adjacent to the aria2
install, whether that is a `uses:` upload or a `run:` block.

⚠ **Two local reproductions came back negative and neither settles it.** The
package route run directly on this host left 82 processes before and after, and
`aria2c --version` under the same bound left 78 before and after. ⛔ This host is
not the runner image and proved it in the same run: `aria2` was **absent** here,
so the install actually installed, where on `ubuntu-24.04` it is a no-op.

⛔ **A job whose runner was killed or cancelled has no log to fetch** - an
authenticated read answers **404** - which is why the install logs go up as an
artifact at all. ⭐ An artifact zip needs no `gh`; a dispatch and a job log do.

⚠ **`timeout-minutes` on the step does not bound it**, measured on run 6: a
five-minute bound over a step that ran seventeen. `CI-08` owns the cause.

---

## How this project is checked

⛔ **Run the gate with one command, `sh scripts/common/check-gate.sh`, after the
last edit.** The last edit is the one made while writing the record, not the one
that felt like the end of the work.

⭐ **It is about 120 seconds now rather than 198**, because its checks run
concurrently. ⚠ Two of them do not join that batch and the reason is in the file:
`check-capture` and `check-capture-client` drive real sockets against a deadline,
and under the full batch one of them reported a refusal that arrived for a reason
its case had not planted.

⚠ **The gate is not the whole of part (a).** `cargo clippy`, `cargo fmt --check`,
the test suite and `sh scripts/ci/check-workflow.sh` are separate.

⛔ **TWO CHECKS JOINED BY `&&` ARE ONE CHECK.** `shellcheck f && shfmt -d f
>/dev/null && echo OK` runs shfmt only when shellcheck says nothing, so an
info-level finding means shfmt never ran - and the absent "OK" reads exactly like
an empty diff. Measured on 2026-09-09; `check-workflow` caught the formatting
defect eleven minutes later. Run each and read each status.

⛔ **And a subset of the gate chosen by hand is not the gate.** `check-no-secrets`
is TWO rows: the second carries the public rules. A digest or a peer ID written
into prose without its algorithm or the words `peer ID` beside it turns that row
red, and it did twice this session.

⛔ **`check-workflow.sh` is not in the gate and cannot be**, because two of its
cases run the gate. It is about 20 minutes and the CI lane runs it in a job of
its own.

⛔ **Read exit codes from the process that produced them, unpiped.**

---

## What a review has to know before it starts

These are the defect classes this project has shipped and caught. Each cost a
pass to find.

⛔ **A count in prose is a value in two places with nothing comparing them.**
Found again on 2026-09-09 by a claim audit over this session's own writing: "nine
checks call `cargo build --example`" was wrong - it is fourteen - while "nine gate
runs inside `check-workflow`" was right. Prefer "every" or "most" to a number.

⛔ **An in-place `sed` edits every line that matches, not the one you meant.**
`sed -i 's/^SECS=[0-9]*/SECS=6/'` changed two lines, because `[0-9]*` matches
zero digits and a line inside an embedded stub read `SECS="$2"`. The edit lands,
the command exits 0, and nothing says the file has a second change in it.

⛔ **A literal counted one way and replaced another plants somewhere nobody
named.** `replace_once` counted with `grep -F` and edited with `sed`: over `axb
then a.b` the literal occurs once, the count accepted, and sed replaced `axb`.
⚠ A literal carrying a `/` could not plant at all, which turned a probe whose
expected outcome is a FAILURE into a pass over a sed that never parsed.

⛔ **A gate run must leave the working tree as it found it, and nothing compared
the two until 2026-09-09.** Two probe cases wrote scratch files beside the file
they were handed and two callers hand that function a tracked path.
`tree-unchanged` is a row in both halves now. ⚠ It sees the run that makes the
mess and not the one after.

⛔ **A row name is a name.** `check-gate` labels a row with a check's basename, so
two harnesses in different directories collide into two rows a reader cannot tell
apart. It refuses a duplicate with exit 2 now.

⛔ **A faster check that is red for no defect is worse than a slow one.** The
all-concurrent gate reached 73 seconds and changed what a deadline-bounded
harness answered. ⚠ And the obvious fix was measured and rejected: raising that
deadline costs eight to eleven seconds of gate per second, paid nine times over
by `check-workflow`.

⛔ **A summary of a source is not the source.** `CLIENT-01` recorded that
qBittorrent's release "offers a Linux AppImage, a Windows `x64_setup.exe` and a
source `tar.xz`". It carries fourteen assets and **two** Linux AppImages - one
build short, in exactly the place a route has to choose.

⛔ **A blocker nobody tested is not a blocker.** Nineteen entries were recorded as
waiting on a capture host across several sessions; one command settled it and no
session ran that command.

**Two guards answering one code mask each other.** Deleting either leaves every
case green, because the survivor produces the code the cases assert.

⛔ **A green local gate does not mean a green lane, because a DEFAULT can change
under you.** `$PSNativeCommandUseErrorActionPreference` is `$false` in pwsh 7.4
and `$true` from 7.5. Every `.ps1` states the behaviour it needs now.

⛔ **A step's exit status is whatever the block LEFT BEHIND, unless it is a
decision.** GitHub's `pwsh` wrapper reads the residual `$LASTEXITCODE`, so an
INVERTED guard - one whose refusal is the proof - fails the step it was proving.

**A rule over `.ps1` files is not a rule over the same language in a
workflow**, and a rule over one workflow is not a rule over its sibling.

**Not knowing is not agreement, and a comparison has to say so in its type.**

**A refusal the deriving path cannot reach is a refusal nothing tests.**

**A collision test is not an encoding test.** It proves injectivity for the one
pair it names.

⚠ **A surviving plant is a question, not a verdict**, a harness exit of 2 is
*could not run*, and a plant that did not **apply** is a third status.

⚠ **A plant whose expected outcome is a PASS proves nothing.** Make the plant fail
and read *which* failure it is.

**A sweep's needle list is what rots.**

⛔ **A rule a document says this repository has is not a rule it has.** Grep for
the check before believing it runs.

⭐ **The strongest control available is a reader this project did not write.**
`sha256sum -c` verifies a release, `torf` and `libtorrent` read a generated
torrent, `curl` is a complete HTTP client, python's `sqlite3` is stdlib, a bare
repository is a real remote, and a product compiled from its own source tarball
is a second acquisition route that needs no runner.

---

## Facts a session must not restate wrongly

⛔ **The publisher cannot run at all.** It downloads an artifact named `bundle`;
the uploads in this tree are `install-*`, `capture-client-*`, `capture-linux-*`
and `capture-windows-*`. `check-project` refuses an undeclared download with no
producer and refuses a declaration once a producer appears.

⛔ **Nothing has been published and no measured record exists.** Everything in the
store is synthetic and says so. ⭐ **Builds HAVE been measured**: Transmission
4.0.5 three times and qBittorrent 4.6.3 once, each attesting `kind=client`,
`stock_client=true`. ⚠ Those are evidence bundles and attestations, not
`Profile`s, and the reason is now `E-ACQ-01` rather than an unwritten step.

⭐ **A `release` route has acquired a build on a capture host**, on 2026-09-09:
aria2 compiled from the vendor's tarball in 144 seconds. Two builds, one version,
measured on the host - different features, different TLS library, different
compiler.

⭐ **Three captures of Transmission 4.0.5 reported three different peer IDs.**
That is what a per-session identity looks like; three samples is not a lifetime
measurement.

⛔ **A hosted Windows runner's fingerprint is not a freshness signal.** The claim
marker is what detects a survived host.

⛔ **The nine commit stamps before 2026-09-06T07:56Z are fabricated.** They are
not retro-corrected. Read the machine clock with `date -u +%Y-%m-%dT%H:%M:%SZ`,
and do not type a stamp ahead of it - one was amended this session for exactly
that.

⛔ **No repository owner or name is hardcoded anywhere in this tree.** The
adapters name third-party upstreams, which is a different fact, and
`check-release-route` compares each against the catalogue.

⛔ `check-remote-items` cannot run on a session host and installing `gh` does not
fix it. It is the one observed skip.

⭐ This session's record is
[`SESSION-2026-09-09-SECONDROUTE.md`](SESSION-2026-09-09-SECONDROUTE.md).

⭐ Every session record is listed in [`README.md`](README.md), and `check-docs`
refuses one that page does not link.
