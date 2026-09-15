# Resume

**Task:** Take the work order in [`../../TODO/PROGRESS.md`](../../TODO/PROGRESS.md)
in order, committing and pushing each green unit to `main`.

⛔ **Nothing is blocked.** Every question an earlier session recorded as needing
the operator is answered in that file under *Settled decisions*, except the one
open decision named there. Do not record a new blocker without running the
command that would settle it.

---

## Starting

**Re-measure the tree.** This file is a claim about a tree that has moved: check
the branch, the remote, the clone depth, `git status` and `HEAD..origin/main`
before editing anything.

⚠ **The container may start on a `claude/*` branch, with `user.name` set to an
agent, and a shallow clone.** All three were true again on 2026-09-15, which is
eleven starts running. Correct them first: the branch to `main` per rule 7, the
identity to the operator's own per rule 11, and the clone with
`git fetch --unshallow` - which took this one from 50 commits to 151.
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

0. ⛔ **THE CAPTURE WORKFLOW HANGS AND THAT IS THE FIRST THING TO SETTLE.**
   `capture-client` runs **21 and 22** both spent thirty minutes in *Install the
   client* on the RELEASE lane, which took **six seconds** on runs 19 and 20.
   Every bound failed: the inner 420s, the outer `timeout -k 30 1080`, and the
   job's own `timeout-minutes: 25`, which ended it at thirty. ⛔ A cancelled job
   leaves **no log and no artifact**, so neither run measured anything.
   ⛔ **A CAUSE WAS LOCATED, FIXED AND REFUTED, AND THE REFUTATION IS THE
   FINDING.** The adapters fetched with no time limit, which is a real defect
   and is fixed; run **23** on `80ec75a` carries `--max-time 300` and its
   release install still ran past **twelve minutes**, so the step is not waiting
   in the fetch. ⚠ A control re-run at `95e90f5` installed in **six seconds**,
   so the hang correlates with this session's commits - and the two hangs are
   contiguous in time, which that control does not separate.
   ⭐ **The next dispatch that reaches an upload answers it in one file.**
   `install-step.sh` writes a `ps` timeline into the workdir every five seconds
   and the workdir ships as the install artifact; runs 21, 22 and 23 were
   cancelled before any upload. ⛔ Read that timeline before theorising again.
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
chosen for thirteen fewer locked packages. A dependabot pull request offers the
bump and is red on the licence register; that red is the register working. ⚠ Do
not take the bump to make a bot green - `PROGRESS.md` carries it as an open
operator decision.

⛔ **The nine commit stamps before 2026-09-06T07:56Z are fabricated**, and so are
the subjects of `62e1a68`, `d64c51c`, `facf9a9`, `e2d1891` and `aba7142`. They
are not retro-corrected. ⚠ One clock read at the start of a session is not a
stamp for the commits that follow it.

**No repository owner or name is hardcoded anywhere in this tree.**

`check-remote-items` cannot run on a session host and installing `gh` does not
fix it. It is the one observed skip.

Every session record is listed in [`README.md`](README.md), and `check-docs`
refuses one that page does not link.
