# Resume

**Task:** Take the work order in [`../../TODO/PROGRESS.md`](../../TODO/PROGRESS.md)
in order, committing and pushing each green unit to `main`.

⛔ **Nothing is blocked and there is no open decision.** Every question an earlier
session recorded as needing the operator is answered in that file under *Settled
decisions*, the `rusqlite` one included since 2026-09-16. Do not record a new
blocker without running the command that would settle it, and do not re-raise a
settled one.

---

## Starting

**Re-measure the tree.** This file is a claim about a tree that has moved: check
the branch, the remote, the clone depth, `git status` and `HEAD..origin/main`
before editing anything.

⚠ **The container may start on a `claude/*` branch, with `user.name` set to an
agent, and a shallow clone.** All three were true again on 2026-09-16, which is
twelve starts running. Correct them first: the branch to `main` per rule 7, the
identity to the operator's own per rule 11, and the clone with
`git fetch --unshallow` - after which `git rev-list --count HEAD` answered **157**
here. ⚠ Measure the depth before AND after if the pair is going to be quoted; a
session that reads only the second number is copying the first from this page.

⛔ **AND THE SESSION HOST IS `root`, WHICH A RUNNER IS NOT.** That difference hid
the capture hang for eight dispatches: a bound that works here is refused by the
kernel there, because the work runs under `sudo` and the bound did not. ⭐ Drive
anything that uses `sudo` as an unprivileged user before believing a local pass -
`useradd -m runnerlike` plus a NOPASSWD line is about ten seconds, and `uid 1001`
is what a hosted runner uses.
⛔ Read the identity out of the history with
`git log --format='%an <%ae>' | sort -u`; never type it into a tracked file,
which is what `check-no-secrets --public` refuses.

**Install the tools with one command:** `sh scripts/doctor/provision.sh`. About
four seconds, every download verified against a pinned digest first. ⛔ Without
`pwsh` the gate goes RED rather than shrinking: `check-capture` fails and
`check-twins` skips.

**`go` is a gate dependency.** `check-bitcheck`, `check-cache` and
`check-defaults` each build [`../../tools/check/`](../../tools/check/)
themselves; without it those rows SKIP, which `--strict` turns into a red lane.
Both runner images carry it.

---

## Where the work is

**In flight:** `CI-10`, with its three named deliverables landed and the rest of
the port open.

### ⭐ A record exists, and a session can produce another

`capture-client` run 17 on 2026-09-15 uploaded two green lanes and
`assemble-capture` wrote **two `Profile`s and eight raw evidence files** out of
them, exit 0. ⛔ The store is scratch state, not the tree; nothing is published;
and both records are `provisional`, so the publish-on-green decision did not fire
and could not have. [`../../TODO/ci.md`](../../TODO/ci.md) under `CI-09` carries
the command and what the records hold.

**The whole loop runs from a session**: dispatch `capture-client` through the
Actions tooling on `ref: main`, download artifacts through rule 8's route
(`.../actions/artifacts/<id>/zip` answers 200), then assemble locally. ⛔ This
file used to say nothing in the tree could press that button. That was false.
Do not record it as a blocker again without trying it.

⛔ **Every defect three dispatches found was SCOPE**: a guard reading more than
ships, an upload shipping less than is read, and a reader looking in one route's
directory for every route's document. ⚠ Each list was complete until
`E-ACQ-07`'s repair made the source lane a real source lane. **One repair
uncovers the next, and only a dispatch shows it.**

### What the checking layer looks like now

**Ten rules are one Go binary** in [`../../tools/check/`](../../tools/check/),
and `bit-check --rows` is the measurement rather than this sentence. Both gate
runners invoke it, so those rows are the SAME row on both lanes.

⭐ **`check-adapters` is the tenth, 2026-09-15, and it was never a shell rule
either.** Every network fetch a capture adapter makes carries a time bound: a
`curl` that writes a file carries `--max-time`, a `git clone` is wrapped in
`timeout`. ⛔ It exists because `capture-client` runs 21 and 22 both hung thirty
minutes in *Install the client* on a lane that had taken six seconds, and
`docs/conventions/shell.md` section 9 had stated that rule for as long as four
adapters had been breaking it. ⚠ **It did not fix the hang** - run 23 carries
the bound and hung anyway - and the rule is kept on its own terms.

⛔ **Sixteen files are deleted, not translated.** `check-twins` went from twelve
file pairs to **five**, and from 69 seconds to **10.0**, measured on 2026-09-15.

**`check-ignores` is the one that was never a shell rule.** A new checking rule
goes into the binary; a new `.ps1` twin is work added to a layer being removed.

⛔ **A PAIR MAY ONLY LEAVE THAT LIST ONE WAY.**
`sh scripts/common/check-bitcheck.sh --compare` runs every case against BOTH
halves and refuses any difference in exit code or in the `--json` line.
⛔ **The window closes when the halves go**: after a deletion those cases print
*the sh half is gone, not compared*, and the run gets cheaper precisely because
it is checking less. The pre-deletion run is the one that counts, and
`TODO/ci.md` records it.

**It has earned that rule.** `check-docs`' comparison caught the two shell
halves DISAGREEING about whether a page cited only inside backticks is an orphan
- on a shape this tree does not contain, which `check-twins` could never see.

**The five pairs left** are `check-project`, 997 lines and its own unit, plus
`check-cache`, `check-catalogue`, `check-remote-items` and `mine-repo`. Their
PowerShell halves together are **5.3 seconds**, timed on 2026-09-15, against the
**96** the layer started at. ⛔ So the remaining wall-clock value is small and the
DRIFT value is unchanged. That number was carried at 12.3, then decremented to 11 and 10 as pairs
left, which is arithmetic rather than measurement; it is timed here.

### Next, in order

0. ⭐ **THE HANG IS REPRODUCED ON THIS HOST AND REPAIRED, 2026-09-16.** Eight
   dispatches had been spent asking a runner what one command answers here.
   ⛔ **NEITHER DEFECT WAS FINDABLE BY READING THE INSTALL PATH.** Both were
   found by driving it as an **unprivileged** user, which is what a runner is and
   what no local drive had ever been: every session host here is `root`, so every
   previous local pass ran the step with privileges the runner does not have.
   ⛔ **A bound inside `$( )` is not a bound.** A substitution ends when its PIPE
   reaches end of file, not when the command exits. `install-client` reached its
   adapter four times through a pipe and **three carried no bound at all**, while
   the file's own header claimed every adapter call was bounded. Measured: an
   adapter leaving one `sleep` behind still blocked at **25s** under a **4s**
   bound.
   ⛔ **An unprivileged bound cannot signal a root tree.** `timeout -k 2 5`
   around `sudo -E sh -c 'sleep 120'` exited **124 on schedule** and left the root
   process alive with **PPID 1**. ⭐ That predicts the number nothing explained:
   run 21's `timeout-minutes: 25` ended at **thirty**.
   ⭐ **Mutation-proved, same plant both halves**: `HEAD` exited **124 still
   blocked at 70s with NO OUTPUT AT ALL** - runs 21/22/23's exact signature - and
   the repaired tree exited **0 in 0s** with a full record. ⭐ Driven as uid 1001
   against a 600s install under a 20s step bound: **124 at 21s, `watchdog.log`
   with four samples, `holders.log` naming the survivor.**
   ⛔ **AND THE CAUSE IS NOT IN THIS REPOSITORY, WHICH ONE COMMAND SETTLES.**
   `git diff 95e90f5 40ed628` over `scripts/acquisition/`, the adapters and
   `capture-client.yml` is **empty**. Run 20 installed in six seconds and runs 21
   and 23 hung for thirty minutes on **byte-identical install code**. The window's
   commits moved `capture-client.sh` and the assembler, neither of which
   *Install the client* reads. ⚠ A correlation with a commit window is not a
   correlation with a change.
   ⛔ **AND IT IS INTERMITTENT ON IDENTICAL BYTES.** The adapter,
   `install-client.sh` and `install-step.sh` are byte-identical at `95e90f5` and
   `40ed628`, by digest, and the green run 20 attempt 2 on the first finished at
   **15:59:43** - between run 21 ending at 15:15 and run 23 starting at 16:07,
   both hung on the second. ⚠ So it is neither the commit nor a host-wide defect:
   a host defect does not go green in the middle of the hung window. ⭐ **The same
   bytes hang sometimes and not others.** Do not record a code cause without a
   digest comparison first.
   ⚠ **The `aria2` vs `aria2-next` lead is checked and does not fit runs 19-31**:
   all dispatched `aria2-next`, `aria2c-next` is nowhere in this history, and the
   vendor binary answers `--version` in 0s under any `argv[0]`.
   ⛔ **RUN 24 THEN REFUTED BOTH BOUNDS, AND THAT IS THE REAL FINDING.** The
   first dispatch after the repair hung identically: *Install the client* still
   open **twenty minutes** in, past the privileged 780s and the outer 900s. The
   tally of bounds measured not to end this step is **ten**.
   ⭐ **Ten bounds failing the same way is ONE problem.** A `run:` step ends when
   its command has exited AND its output pipe has reached end of file; every
   bound ever added here acts on the first condition only, so a held pipe defeats
   all ten and would defeat an eleventh.
   ⭐ **So the repair stopped being a bound.** The step body hands
   `install-step.sh` and its whole subtree a FILE and cats it afterwards, so the
   step's pipe is held by the step's own shell and nothing else. ⚠ The workflow
   had claimed that guarantee in a comment while holding it one level too low -
   `install-step.sh` redirected everything it started and then inherited the pipe
   itself.
   ⚠ **`report-holders.sh` is aimed at the wrong descriptor** and has reported
   `0 holder(s)` on every green run: the file that keeps a step open is the
   step's own stdout, which nothing asks about. It reports both now.
   ⛔ **THE TIMELINE WAS STILL NOT OBTAINED. Runs 24, 25 and 26 all hung and all
   uploaded ZERO artifacts**, read back from the API. ⭐ **Run 26 is the sharpest
   measurement this entry has**: it carried three independent endings, one of
   which - `install-step.sh`'s own loop deadline - depends on no signal reaching
   anything, and all three passed without the step ending.
   ⛔ **So stop reading this as a bound that fails to fire.** The step's process
   is not reaching its own deadline check, which no bound repairs.
   ⭐ **RUN 27 THEN LOCALISED IT, IN ONE SECOND.** Three named probes now walk the
   release route's own operations, and all three passed before the wedge:
   the fetch of the whole artifact took **1s**, staging (`chmod`, `mkdir`, a `cp`
   across filesystems into `/usr/local/bin`) **0s**, and an `exec` of the
   downloaded binary **0s**. ⛔ The vendor's endpoint, the network, the filesystem
   and the binary are ruled OUT by measurement. *Install the client* performs
   those same operations and still never returns.
   ⚠ **What is left is what the install does and the probes do not**: the `sudo`
   plumbing, `install-client`'s guard and rule-12 scan, the holder report's walk
   of `/proc`, and the watchdog's own `ps`.
   ⛔ **`ps -e` WAS THE LAST UNBOUNDED COMMAND IN THAT SHELL** and it reads
   `/proc` for every process - so the instrument could block on the very condition
   it was written to record, and the deadline two lines above it would never be
   reached. It is bounded now. ⚠ An earlier version of this page said the loop
   "only sleeps and compares two integers"; that was written without re-reading
   the loop and is corrected here and in `ci.md`.
   ⛔ **RUN 28 CARRIES THAT BOUND AND HUNG ANYWAY**, twenty-five minutes, past the
   loop's own 780s and the outer 900s - so the `ps` is **excluded**. ⭐ It also
   REPRODUCED run 27: its three probes completed inside the same second, so two
   runs agree the route is instantaneous while the step performing it does not
   return.
   ⭐ **FOUR MORE PROBES ARE BUILT AND PUSHED**, one per remaining candidate: the
   claim guard, a detached `sudo` launch, the rule-12 scan over the real
   fourteen-megabyte artifact, and the `/proc` holder walk. ⛔ Names localised in
   one run what twelve bounds could not; do not add a thirteenth bound - read
   which step the API last reported.
   ⚠ **Driving those bodies found a defect the reading did not**: GitHub runs a
   `run:` block as `bash -e`, and `grep` exits 1 on NO MATCH, which is the
   ordinary outcome for a scan that finds no secret. Two probes would have failed
   the capture they were written to be harmless to.
   ⛔ **RUN 29: ALL SEVEN PROBES PASSED IN FOUR SECONDS AND THE STEP STILL
   WEDGED.** Every operation *Install the client* performs is now measured
   individually; they sum to four seconds and the step composing them runs for
   thirty minutes. ⭐ **Each candidate is excluded on its own, so no further probe
   of a component can reach it** - what is left is the composition.
   ⭐ **SO BUY THE ARTIFACT RATHER THAN MORE LOCALISATION.** A **step-level**
   `timeout-minutes` is the RUNNER's bound, not a twelfth shell bound, and a step
   that exceeds it is marked FAILED rather than cancelling the job - so the
   `if: always()` upload runs and the timeline ships. ⚠ It was measured once not
   to fire, on a `uses:` step; this is a `run:` step, which the runner supervises
   directly. ⛔ And a failed install skips *Cut the route*, so the host still has
   the network that upload needs.
   ⚠ A residual is filed in `TODO/ci.md`: nested `timeout`s each make their own
   process group, so an orphan survives the bound.
1. ⛔ **Make a MEASURED record publishable, which is one dispatch away.** Runs 19
   and 20 refuted both recorded readings: the observer now offers a distinct peer
   per connection and an interval the run can outlive, and `aria2-next` still
   answered one of two connections and announced once. ⭐ **Run 20 then gave two
   announces** - `started` and `stopped`, the first pair a stock build has ever
   given this project - **and both carried the same peer ID**, so the tail is
   stable within a session.
   ⭐ **So the last lever is a second SESSION, and it is built.**
   `capture-client.sh --sessions` defaults to two and starts and stops the build
   once per session inside the window. ⚠ **No dispatch has taken it**: whether
   `aria2-next` regenerates its peer ID per run is the open question, and a
   `patterned` field is what two lanes can agree on.
2. **`CLIENT-01`, `CLIENT-06`**, the remaining vertical captures. `aria2-next`
   is the worked example: five dispatches took it from four refusals to two
   written records and two announces.
3. **`CI-10`**, five pairs left, `check-project` its own unit.
4. **`CI-09`'s** remaining residuals, no longer about reaching a record: the
   `RunManifest` a record needs beside it, and the publisher, which cannot run at
   all because it downloads an artifact named `bundle` that nothing produces.
   ⚠ And a third: `sampling::classify` computes a `Lifetime` per span and
   `field_state` DISCARDS it, so no record can say whether a tail is
   per-connection or per-session. `SCHEMA-04` owns the field.
5. **`CI-07`**, whose class-A backlog shrinks as `CI-10` ports pairs.
6. **`CI-08`'s** load-sensitive `check-step-bodies` row.

---

## How this project is checked

⛔ **Run the gate with one command, `sh scripts/common/check-gate.sh`, after the
last edit.** **39 checks, about 165 seconds** on a four-processor host. Its
wall clock is `max(concurrent batch) + max(check-capture, check-capture-client)`
rather than a sum: those two run alone, after the batch, on purpose.

⛔ **DO NOT EDIT THE TREE WHILE IT RUNS.** `tree-unchanged` compares before and
after, so an edit during a run fails that row and the failure names a check
rather than the editor.

⛔ **DO NOT RUN THE GATE OR `git add` WHILE `check-workflow` IS RUNNING**, which
corrupts its plant-and-restore accounting. ⚠ `check-workflow` unsharded is about
21 minutes; use `--shard 1/4`.

⛔ **`git checkout -- <dir>` DISCARDS UNSTAGED WORK IN THAT DIRECTORY.** It cost
four files of edits this session. Stage first, or name the exact file.

**A count of gate rows goes stale.** `check-gate.sh --rows` prints the list and
`check-gate-rows` compares it against the other lane's.

**The gate is not the whole of part (a).** `cargo clippy`, `cargo fmt --check`,
the test suite and `sh scripts/ci/check-workflow.sh` are separate.

⛔ **TWO CHECKS JOINED BY `&&` ARE ONE CHECK.** Run each and read each status.
⛔ **Read exit codes from the process that produced them, unpiped.**

---

## What a review has to know before it starts

These are the defect classes this project has shipped and caught.

⛔ **A guard whose SCOPE is wider or narrower than the thing it guards.** Three
instances in one day, all in the capture path. Ask what actually ships.

⛔ **A guard that sees ONE SPELLING of the thing it forbids reports clean over
the others.**

⛔ **A one-gated-door defect arrives in the change that fixes something else.** A
subject left outside a build gate answers 127 under every environment, which
reads as *the same answer under all 6*.

⛔ **A corpus only tests the defects it contains an example of.** A rule
differing only on a shape the tree does not contain is invisible to any
comparison over that tree. Plant the shape.

⛔ **A harness that plants a pattern cannot spell it** - and neither can the
comment explaining why.

⚠ **A plant that did not APPLY is a third status**, not a survivor. Diff the file
before believing either answer. ⚠ A harness exit of 2 is *could not run*, never
*refused*. ⚠ A plant whose expected outcome is a PASS proves nothing, except
where the rule's risk is over-strictness.

⛔ **A guard that nothing can refute is still worth keeping, and saying so is the
work.** Label the case with the rule that really refuses it.

⛔ **A global regex replace where the pattern also matches what it preserves.**
Reproduced this session in a Go port of a defect already written down.

⛔ **A step does not end when its command exits.** ⛔ **A shell applies
redirections left to right.** ⛔ **`timeout 0` MEANS NO LIMIT.** ⛔ **A PowerShell
`[switch]` collides with a local differing only in case**, and `$args` inside a
function is automatic.

⛔ **A BOUND INSIDE `$( )` IS NOT A BOUND.** A command substitution ends on the
PIPE's end of file, not on the child's exit, so one process the product leaves
behind blocks it forever while the bound reports success. Redirect an untrusted
process to a FILE and read the file.

**AN UNPRIVILEGED BOUND CANNOT END A ROOT PROCESS.** Put the bound inside the
`sudo`, never around it. A bound outside fires on schedule, reports 124, and
leaves the work orphaned at PPID 1. And nested `timeout`s each create their own
process group, so an outer bound does not reach an inner one's children.

**A LOCAL PASS AS `root` SAYS NOTHING ABOUT A RUNNER.** Both of the above were
invisible to every local drive for eight dispatches, because this host is `root`
and `ubuntu-24.04`'s runner is uid 1001 with `sudo`.

⛔ **A Python text-mode rewrite of a `.ps1` silently converts CRLF to LF**, and
`git diff` shows nothing. `git ls-files --eol` is the only thing that does.

⛔ **In Go, `filepath` is the HOST's separator and `path` is slashes.** A
repo-relative key derived with `filepath.Dir` matches on Linux and misses on
Windows. ⚠ Running `check-gate.ps1` on Linux does not test Windows: only the
Windows lane can. CI run 132 caught it where three local passes did not.

**A count in prose is a value in two places with nothing comparing them.**
**An in-place `sed` edits every line that matches, not the one you meant.**

⛔ **A rule a document says this repository has is not a rule it has.** Grep for
the check before believing it runs. Three were found missing this way.

**The strongest control available is a reader this project did not write.**

⛔ **A CAPTURE'S WORKDIR IS ITS EVIDENCE BUNDLE**, so rule 12 applies to what an
adapter writes there - bounded by what the artifact actually ships.

---

## Facts a session must not restate wrongly

⭐ **A `Profile` exists**, assembled from run 17's own artifacts. ⛔ It is not in
the tree, nothing is published, and both records are `provisional`.

⛔ **The publisher cannot run at all.** It downloads an artifact named `bundle`
and nothing in this tree produces that name.

⛔ **A capture declaring ONE connector is INVALID, not merely unpublishable.**
`E-CAP-01` fires inside `validate`. The capture path declares two.

⛔ **THE PEER ID DIFFERS BETWEEN SURFACES INSIDE ONE RUN.** The announce and the
handshake carry different twelve-byte tails after `-qB5230-`, so the tail is
per-CONNECTION, and that is why `classify_across` answers `divergent` over two
routes.

⛔ **A stock `aria2-next` announces as qBittorrent 5.2.3**, observed on the wire
and corroborated by a connector this project did not write.

⛔ **`rusqlite` is pinned at 0.37.0 deliberately**, measured against 0.40.2 and
chosen for thirteen fewer locked packages. ⭐ **The operator settled it on
2026-09-16: the answer is no.** Pull request 1 is closed unmerged, and the pin is
0.37.0 in both the manifest and the lockfile, read back the same day. ⚠ Do not
take the bump to make a bot green, and do not re-raise the question.

⛔ **The nine commit stamps before 2026-09-06T07:56Z are fabricated**, and so are
the subjects of `62e1a68`, `d64c51c`, `facf9a9`, `e2d1891` and `aba7142`. They
are not retro-corrected. ⚠ One clock read at the start of a session is not a
stamp for the commits that follow it.

⛔ **AND `cc86999`'s SUBJECT SAYS `05:02:41Z` WHERE ITS AUTHOR DATE IS
`04:58:19Z`.** The stamp was typed into the message before the `date` in the same
command had printed, so the clock WAS read and its answer was not the one used.
It is recorded rather than rewritten, like the others. ⚠ The habit that prevents
it is reading the clock in a SEPARATE call and copying the value it printed;
composing both in one command puts the writing before the reading.

**No repository owner or name is hardcoded anywhere in this tree.**

`check-remote-items` cannot run on a session host and installing `gh` does not
fix it. It is the one observed skip.

Every session record is listed in [`README.md`](README.md), and `check-docs`
refuses one that page does not link.
