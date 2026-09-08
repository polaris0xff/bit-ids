# Resume

**Task:** Take the work order in [`../../TODO/PROGRESS.md`](../../TODO/PROGRESS.md)
in order, committing and pushing each green unit to `main`.

⛔ **Nothing is blocked.** Every question an earlier session recorded as needing
the operator is answered in that file under *Settled decisions*. Do not re-raise
them, and do not record a new blocker without running the command that would
settle it.

**In flight:** `CI-03`. Its Windows guard pair and mutation harness are done,
green and in both gate lanes. Its capture workflow is not written;
[`../../TODO/ci.md`](../../TODO/ci.md) carries the step order and why the order
is forced.

**Tree:** Re-measure it. This file is a claim about a tree that has moved. Check
the branch, the remote, the clone depth, `git status` and `HEAD..origin/main`
before editing anything.

⚠ **The container may start on a `claude/*` branch with `user.name` set to an
agent and a shallow clone.** All three were true at the start of the last
session. Correct them before any edit: the branch to `main` per rule 7, the
identity to the operator's own per rule 11, and the clone with
`git fetch --unshallow`. ⛔ Read the identity out of the history with
`git log --format='%an <%ae>' | sort -u`; never type it into a tracked file,
which is what `check-no-secrets --public` refuses.

⭐ **Install `pwsh`, `shellcheck` and `shfmt` first.** The commands are in
`TODO/PROGRESS.md` under *Known gaps in the local gate*, and the `chmod +x` on
the PowerShell tarball is needed exactly as that note says. Without them this
host runs a smaller gate than CI.

---

## How this project is checked

⛔ **Run the gate with one command, `sh scripts/common/check-gate.sh`, after the
last edit.** The last edit is the one made while writing the record, not the one
that felt like the end of the work.

⚠ **The gate is not the whole of part (a).** `cargo clippy`, `cargo fmt --check`,
the test suite and `sh scripts/ci/check-workflow.sh` are separate. A clippy
failure passed a green gate three times across two sessions before being caught,
most recently over a `len() > 0` in a test.

⛔ **`check-workflow.sh` is not in the gate and cannot be**, because two of its
cases run the gate. Every other harness is in it.

⛔ **Read exit codes from the process that produced them, unpiped.** A pipeline's
status is not the check's.

---

## What a review has to know before it starts

These are the defect classes this project has shipped and caught. Each cost a
pass to find.

⛔ **A blocker nobody tested is not a blocker.** Nineteen entries were recorded as
waiting on a capture host across several sessions. The guard that supposedly
refused every available host takes its routing table as an argument, so one
command settles it, and no session ran that command. Before recording anything as
blocked, run the thing that would prove it.

⛔ **Two guards answering one code mask each other.** Deleting either leaves every
case green, because the survivor produces the code the cases assert. Separate them
by the path or the message a refusal names, and give each shape a case.

⛔ **A refusal the deriving path cannot reach is a refusal nothing tests.** Where a
builder derives a field and a validator checks it, the two agree by construction
and the check can never fire on anything the builder produced. Test those at the
document level, where a file is the input.

⛔ **A published document with a writer and no reader hides a live defect.** A
consumer comparing a bundle against a manifest re-derived *from that bundle*
agrees with itself, so a described file that is missing is not described either.
Both published documents round-trip now; a third would need the same.

⛔ **A collision test is not an encoding test.** It proves injectivity for the one
pair it names. Pin an encoding against its own restated specification, byte for
byte, and vary each component of a key in turn.

⛔ **A count written in prose is a value in two places with nothing comparing
them.** Three were found stale in one sweep. Prefer "every" to a number, or put
the number behind a check.

⛔ **Vocabulary written in a consumer grows a third spelling next.** A published
name lives beside its type.

⚠ **A surviving plant is a question, not a verdict**, and a harness exit of 2 is
*could not run*, never *refused*. A plant that did not **apply** is a third status
and is neither. Read what a plant changed before believing either answer: of the
survivors last session, one was the design working and one was equivalent on the
fixture data.

⚠ **A sweep's needle list is what rots.** Check it against a file that really
carries what it hunts for, or it reports the same clean answer over a tree full of
them.

⭐ **The strongest control available is a reader this project did not write.**
`sha256sum -c` verifies a release, `cbor2` reads a canonical encoding,
`libtorrent` and `torf` read a generated torrent, `curl` is a complete HTTP
client, Python's `pow` is an arbitrary-precision modexp and its `hashlib`
re-derives an identifier from a restated encoding.

⭐ **A push path drives for real with no network and no credential.** A bare
repository in a scratch directory is a remote as far as git is concerned.

⭐ **A documented command is a copy that drifts unless the document is what
runs.** Both documentation pages have their blocks extracted and executed on every
gate, and each refused a command on its first run.

---

## Facts a session must not restate wrongly

⛔ **Nothing has been published and no capture has been taken.** Everything in the
tree is synthetic and says so. The publisher must not run against this
repository's own remote until a measured record exists.

⛔ **No observer has been driven by a stock client.** Every driver so far is an
independent implementation written from a specification, which shares this
project's reading of the protocol.

⛔ **The nine commit stamps before 2026-09-06T07:56Z are fabricated**, which is
why `CHANGELOG.md`'s ordering is not monotonic there. They are not
retro-corrected. Read the machine clock with `date -u +%Y-%m-%dT%H:%M:%SZ`.

⛔ **No repository owner or name is hardcoded anywhere in this tree.** The project
moves to another owner once it is finished here, so anything needing the
repository derives it: workflows use `github.repository`, the issue template uses
a repository-relative path, and the README names no clone URL. Do not reintroduce
one.

⛔ `check-remote-items` cannot run on a session host and installing `gh` does not
fix it. It is the one observed skip, and it is why the gate exits 1 under
`--strict` here and 0 on the Linux lane.

⭐ The earlier session record is
[`SESSION-2026-09-06-ADJACENT.md`](SESSION-2026-09-06-ADJACENT.md). ⚠ It is linked
from here and nowhere else, so a rewrite of this file that drops the link orphans
it and `check-docs` refuses that.
