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

**In flight:** `CLIENT-05` and `CI-09`, and they are still the same blocker.

⛔ **THE RECORD THE WORK ORDER WAITS ON CANNOT BE WRITTEN FROM ANY CAPTURE THIS
PROJECT HAS RUN.** One connector **validates** and `E-PUB-02` refuses to publish
it; one route is refused at the **validity** gate by `E-ACQ-01`. Every capture so
far is one route, so `Profile::to_json` will not write it and the store cannot
hold it. ⭐ `E-ACQ-01` is right: the product IS the two-route claim.

**What unblocks it is a two-route capture**, and aria2 is the only target whose
two routes resolve the same version. `capture-client.yml` takes the route as a
matrix dimension and `resolve-release.sh` chooses the artifact.

**Next, in order:**

1. ⛔ **Get an aria2 job past *Install the client*.** Read the next section
   first: run 8 refuted a fourth reading and narrowed where the step is not.
2. **A record in the store**, once a two-route capture exists. `not_corroborated`
   is a recordable state; one route is not.
3. **`CI-07`**, the PowerShell halves for the declared rows. ⭐ Its cheap half is
   done: `check-gate-rows` compares the two runners' row lists.
4. **Shard `check-workflow` across runners.** It is still the whole CI wall
   clock. `CI-01`'s residual says why it is its own unit.

---

## The aria2 hang, as far as eight runs can answer it

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

⭐ **THE NEXT DISPATCH IS INSTRUMENTED AND THIS IS THE POINT OF IT.** The install
runs in the background and the step's own shell watches it, because that shell is
outside every bound that has failed: `timeout` ends its own child and cannot end
a shell blocked in a substitution around it, and `timeout-minutes` is the
runner's and was measured on run 6 not to end the step at all. Every five seconds
it appends `pid,ppid,pgid,stat,etimes,comm,args` to `watchdog.log` in the
uploaded workdir, and a deadline kills the install so the step ENDS and the
artifact uploads.

⚠ **`etimes` beside `stat` is the question this answers**: a process whose
elapsed time grows while its state is `R` is slow, and one sitting in `D` or `S`
is stopped. Timings alone cannot tell those apart and eight runs of timings have
not.

⛔ **A job whose runner is killed has no log and no artifact**, which is why the
step ending is the whole design. ⚠ Measured again on run 8: rule 8's route
answers **302** for a running job's log and redirects to a blob that answers
`BlobNotFound`, while a general-purpose GitHub tool answers a bare **404** and
hides the difference between *not written yet* and *not found*.

**Two local reproductions came back negative and neither settles it.** This
host is not the runner image and proved it in the same run: `aria2` is absent
here, so the install actually installs, where on `ubuntu-24.04` it is a no-op.

---

## How this project is checked

⛔ **Run the gate with one command, `sh scripts/common/check-gate.sh`, after the
last edit.** ⭐ It is **31 checks** and about **150 seconds** on this host.

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
the check before believing it runs.

**The strongest control available is a reader this project did not write.**

---

## Facts a session must not restate wrongly

⛔ **The publisher cannot run at all.** It downloads an artifact named `bundle`
and nothing in this tree produces that name.

⛔ **Nothing has been published and no measured record exists.** ⭐ Builds HAVE
been measured: Transmission 4.0.5 three times and qBittorrent 4.6.3 once, each
attesting `kind=client`, `stock_client=true`. ⚠ Those are evidence bundles and
attestations, not `Profile`s.

⭐ **A `release` route has acquired a build on a capture host**: aria2 compiled
from the vendor's tarball in 144 seconds, two builds at one version with
different features, TLS libraries and compilers.

**Three captures of Transmission 4.0.5 reported three different peer IDs.**

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
