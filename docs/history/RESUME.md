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
agent, and a shallow clone.** All three were true again on 2026-09-17, which is
fourteen starts running. Correct them first: the branch to `main` per rule 7, the
identity to the operator's own per rule 11, and the clone with
`git fetch --unshallow` - after which `git rev-list --count HEAD` answered **170**
here, from **50** before. ⚠ Measure the depth before AND after if the pair is
going to be quoted; a session that reads only the second number is copying the
first from this page.

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

**In flight:** nothing. `ACQ-06` closed on 2026-09-17 and with it every open P0;
`check-project` was ported, compared and deleted the same day. The next unstarted
work is `CI-10`'s four remaining twin pairs, or `CI-09`'s `RunManifest` producer.

### ⚠ The state of the tree, as this was last written

The gate is **40 checks, 39 passed, 0 failed, 1 skipped**, exit 0 - the one skip
is `check-remote-items`, which needs an authenticated `gh` no session host has.
⚠ It was 39 rows at the 2026-09-17 session's start; `check-rootless` joined with
`ACQ-06`.

⚠ **Seven commits landed in that session and each was read back through rule 8's
route.** CI runs **158** through **164** are green on all six jobs, the last of
them on `5b331ca`. ⛔ **The commit a session ends on is the one with no run read
back yet**, whatever its number: confirming it is a starting step rather than an
assumption.

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

**Eleven rules are one Go binary** in [`../../tools/check/`](../../tools/check/),
and `bit-check --rows` is the measurement rather than this sentence. Both gate
runners invoke it, so those rows are the SAME row on both lanes.

⭐ **`check-project` is the eleventh and the last `common/` pair**, ported,
compared and deleted on 2026-09-17. Every `common/` half is in the binary now.

⛔ **Eighteen files are deleted, not translated.** `check-twins` went from twelve
file pairs to **four**: 69 seconds to 10.0 over the first eleven, measured on
2026-09-15, and `check-project`'s two halves on 2026-09-17.

**The four pairs left** are `check-cache`, `check-catalogue`,
`check-remote-items` and `mine-repo`. ⭐ `check-project` was the fifth and the
biggest, and it left on 2026-09-17; the five together were **5.3 seconds**, timed
on 2026-09-15, of which it was **2.60**. ⛔ So the remaining wall-clock value is
small and the DRIFT value is unchanged.

⛔ **A PAIR MAY ONLY LEAVE THAT LIST ONE WAY.**
`sh scripts/common/check-bitcheck.sh --compare` runs every case against BOTH
halves and refuses any difference in exit code or in the `--json` line.
⛔ **The window closes when the halves go**: after a deletion those cases print
*the sh half is gone, not compared*, and the run gets cheaper precisely because
it is checking less. The pre-deletion run is the one that counts.

### Next, in order

⛔ **THE CAPTURE HANG IS A DEAD END AND IS NOT ON THIS LIST.** `TODO/RULES.md`
carries it as a rule and `AGENTS.md` as absolute 16: a hang is a dead end, not a
subject; two bounds is the limit; take the route that does not depend on the
hanging thing. ⚠ **Thirteen bounds have already failed** - inside the script,
around it, privileged, a watchdog, the job's `timeout-minutes`, and GitHub's own
step supervisor - across two targets and both routes, on byte-identical code.
⛔ Do not diagnose it, instrument it, or add a fourteenth.

0. ⭐ **`ACQ-06` IS CLOSED.** No install on the capture path needs a privilege:
   one rootless installer fetches with a bound, settles the digest before
   anything is made executable, and writes into a prefix the current user owns.
   Driven as uid 1001, and repeated with `sudo` absent from `PATH` entirely.
   ⛔ **AND IT HAS BEEN DISPATCHED: THE HANG IS NOT PRIVILEGE.** `capture-client`
   run 32 wedged in *Install the client* on both lanes and uploaded zero
   artifacts, where the unprivileged claim took 0s and the resolve step - which
   fetches through the same installer - took 1s. ⚠ Do not reopen it: absolute 16.
1. **`CI-10`**, four twin pairs left: `check-cache`, `check-catalogue`,
   `check-remote-items` and `mine-repo`. ⛔ A pair leaves that list only after
   `check-bitcheck --compare` has run it against BOTH halves.
   ⭐ **`check-project` is ported, compared and deleted, 2026-09-17** - the
   twenty-nine refusals are in the Go binary, 49 cases ran them against both
   halves, and the comparison found three drifts that were all the PowerShell
   twin's and are all repaired. ⚠ The deletion was not one file: both gate
   runners queued the row, `check-twins` compared the pair, `check-workflow`
   planted against that exact path, and `check-defaults` ran it as a subject.
2. **`CLIENT-01` and `CLIENT-06`**, the remaining vertical captures. They sat
   behind `ACQ-06` and no longer do.
3. **`CI-09`'s residuals**: the `RunManifest` a record needs beside it, and the
   publisher, which cannot run at all because it downloads an artifact named
   `bundle` that nothing produces. ⛔ Its third clause about a discarded
   `Lifetime` was STALE and is struck - that is a `SCHEMA-04` decision rather
   than a defect, and the consumer-side half it really lacked is
   `SamplingPlan::lifetime_of`. ⭐ The manifest clause is half done, 2026-09-17:
   the assembler now asks `validate_corpus` about the store it wrote and prints
   what it hears, so the store's incompleteness is stated rather than inferred.
   ⚠ Writing the document is the other half and is still open.
4. ⭐ **`mine-repo`'s clone is bounded, 2026-09-17**, in both halves and with one
   idiom: git's own low-speed settings rather than a wrapper.
5. **`CI-07`**, whose class-A backlog shrinks as `CI-10` ports pairs.

---

## How this project is checked

⛔ **Run the gate with one command, `sh scripts/common/check-gate.sh`, after the
last edit.** **40 checks, about 165 seconds** on a four-processor host. Its
wall clock is `max(concurrent batch) + max(check-capture, check-capture-client)`
rather than a sum: those two run alone, after the batch, on purpose.

⛔ **DO NOT EDIT THE TREE WHILE IT RUNS.** `tree-unchanged` compares before and
after, so an edit during a run fails that row and the failure names a check
rather than the editor.

⛔ **DO NOT RUN THE GATE OR `git add` WHILE `check-workflow` IS RUNNING**, which
corrupts its plant-and-restore accounting. ⚠ `check-workflow` unsharded is about
21 minutes; use `--shard 1/4`, and run all four shards only when a
`.github/workflows/` file actually changed.

⛔ **`git checkout -- <dir>` DISCARDS UNSTAGED WORK IN THAT DIRECTORY.** It cost
four files of edits one session. Stage first, or name the exact file.

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

⛔ **A RULE WHOSE PRECONDITION IS ABSENT READS CLEAN, FOREVER.** `E-CRP-03`
refuses an artifact a store does not carry and is checked per run manifest, so
with no manifest in the store the assembler filed four citations per record at
paths nothing had written and every gate stayed green. ⚠ Ask which rules a
missing document takes with it.

⛔ **A validator handed an EMPTY input answers about nothing and looks green.**
The same corpus check, given an empty tree instead of one read off the disk,
found half of what was wrong and said so in the same confident sentence.

⛔ **A report that cannot print its own failures is a silent run.** `store_report`
counted rows by LINES, a disagreement is three lines, and the one comparison with
something to say returned 1 having printed nothing at all.

⚠ **`@(...) | Sort-Object` wraps the INPUT and leaves the output unwrapped**, so
an empty result is `$null` and `.Count` throws. A rule written for the empty case
was the one case it could not reach.

⛔ **A harness that plants a pattern cannot spell it** - and neither can the
comment explaining why.

⚠ **A plant that did not APPLY is a third status**, not a survivor. Diff the file
before believing either answer. ⚠ A harness exit of 2 is *could not run*, never
*refused*. ⚠ A plant whose expected outcome is a PASS proves nothing, except
where the rule's risk is over-strictness.

⛔ **A guard that nothing can refute is still worth keeping, and saying so is the
work.** Label the case with the rule that really refuses it.

⛔ **A global regex replace where the pattern also matches what it preserves.**

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
Windows lane can.

**A count in prose is a value in two places with nothing comparing them.**
**An in-place `sed` edits every line that matches, not the one you meant.**

⛔ **A rule a document says this repository has is not a rule it has.** Grep for
the check before believing it runs. Three were found missing this way.

**The strongest control available is a reader this project did not write.**

⛔ **A CAPTURE'S WORKDIR IS ITS EVIDENCE BUNDLE**, so rule 12 applies to what an
adapter writes there - bounded by what the artifact actually ships.

⚠ **GitHub runs a `run:` block as `bash -e` and `grep` exits 1 on NO MATCH**, so
a trailing `true` is unreachable unless the status is captured on its own line.

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

---

## The paste

```text
Read docs/AGENTS.md in full and execute its session-start protocol. Work on main.
Re-measure identity, write access, working tree, branch, remote and clone depth -
the container may start you on a claude/* branch, with user.name set to an agent,
and a shallow clone. Read the operator identity out of history with
git log --format='%an <%ae>' | sort -u; never credit an agent, model or tool.
Install tools with sh scripts/doctor/provision.sh; go is a gate dependency.
The gate is 40 checks, about 165 seconds: sh scripts/common/check-gate.sh.
Read back CI for the tip commit through AGENTS.md rule 8's route before trusting
any recorded green. The work order is TODO/PROGRESS.md and nowhere else; item 0
is ACQ-06. A hang is a dead end, not a subject - do not reopen the capture hang.
Then read docs/history/RESUME.md and the required reading it names.
```
