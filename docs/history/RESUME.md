# Resume

**Task:** Take the work order in [`../../TODO/PROGRESS.md`](../../TODO/PROGRESS.md)
in order, committing and pushing each green unit to `main`.

⛔ **Nothing is blocked.** Every question an earlier session recorded as needing
the operator is answered in that file under *Settled decisions*. Do not re-raise
them, and do not record a new blocker without running the command that would
settle it.

**In flight:** `CLIENT-01`, `CLIENT-06` and `CLIENT-05`, which share one body of
machinery and now have all of it except a measurement. Written and proved this
session: an observer that hands a build the torrent naming its own tracker, a
an adapter contract with a file per target, an install step that runs before
the route is cut, a capture runner that refuses to attest to a build it did not
observe announce, a mutation harness in the gate that runs every shipped
adapter's `describe`, and a second capture workflow that installs a product.

⭐ **Client capture run 1 was dispatched and one of its three jobs measured a
build.** Transmission installed from the package index in under two minutes,
announced twice under containment, and its bundle verified and uploaded.
⛔ **The other two sat in the install step for over half an hour and reported
nothing**, because no adapter call had a time limit: a hung install is
indistinguishable from a slow one until the job's own timeout kills the runner
and takes the log with it. Both are bounded now, stdin is `/dev/null` on every
adapter call, `NEEDRESTART_MODE=a` is set for apt, and `qbittorrent-nox
--version` carries `--confirm-legal-notice` - which the `start` call had and the
`version` call did not, a control on one of two paths into one product.

⭐ **Run 2 answered for `qbittorrent` and not for `aria2`.** qbittorrent fails in
fifty seconds now with a readable verdict: the install works and the `version`
call refuses. ⛔ It also found two defects in the fix itself - the adapters read
`--version` through a pipe, so `head`'s status masked the product's, and the
caller discarded the adapter's stderr and then reported its absence. Both are
fixed with cases. ⚠ `aria2` hung again for thirty-five minutes and the 900-second
bound did not end the job, so the log went with the runner for the second time;
`-k` and a bound under the job timeout are the guesses, and the certain fix is
that the workflow uploads the install logs on `always()` now.

**Next:** dispatch `aria2` and read the install log it will now leave, then
`qbittorrent`'s `version` refusal, which will name itself. ⚠ Then what no capture
has established: a second route, a second connector, a record in the store, and
the Windows half of each Prove. Every adapter is `sh` with no PowerShell twin, so
a Windows client capture needs `CI-07`'s work first.

**Tree:** Re-measure it. This file is a claim about a tree that has moved. Check
the branch, the remote, the clone depth, `git status` and `HEAD..origin/main`
before editing anything.

⚠ **The container may start on a `claude/*` branch with `user.name` set to an
agent and a shallow clone.** All three were true at the start of the last three
sessions. Correct them before any edit: the branch to `main` per rule 7, the
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

⛔ **And a subset of the gate chosen by hand is not the gate.** `check-no-secrets`
is TWO rows: the second carries the public rules, and re-running "the checks this
edit touched" after writing a record missed it. CI run 67 went red on both lanes
over an info hash, which is a measurement and is also forty hex digits.

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
⚠ It happens between guards over one *input* too, not only over one code:
`check-capture`'s first run reported a segment-count guard proved by a stub that
had removed the evidence rows, and the evidence guard, which runs earlier, is
what refused it.

⛔ **A tool that streams is not the same as a tool that flushes.** A stub
observer piped through `awk '{ print; fflush() }'` produced nothing for a whole
run: mawk reads its INPUT in blocks, so there was no output to flush yet.
`fflush` is about the wrong end of the pipe, and `cat` in the same position had
three lines at the same instant. A `read` loop is what streams.

⛔ **A green local gate does not mean a green lane, because a DEFAULT can
change under you.** `$PSNativeCommandUseErrorActionPreference` is `$false` in
pwsh 7.4 and `$true` from 7.5, where a native command's non-zero exit becomes a
terminating error under `$ErrorActionPreference = 'Stop'` - so a guard that
REFUSES stops being a code the caller can read. It turned the Linux lane red and
left the Windows lane green, over a script neither lane had changed. ⭐ Every
`.ps1` states the behaviour it needs now, and `check-project` refuses one that
does not. ⚠ The general lesson is the shape: a comment saying "X is the default"
is a fact about one version being relied on as a promise.

⛔ **A step's exit status is whatever the block LEFT BEHIND, unless it is a
decision.** GitHub's `pwsh` wrapper reads the residual `$LASTEXITCODE`, so a
block whose last command is a guard fails the step with that guard's code - even
when the code was the outcome the step wanted. ⚠ The shape that bites is an
INVERTED guard: the capture workflow's Windows restore step puts the routes back
and then runs the egress guard, which must now REFUSE, because a host that can
reach the network again is one that guard refuses. It refused, exit 1, exactly as
designed, and that 1 failed the step and skipped the evidence upload. ⭐ The `sh`
twin never had it, because `if guard; then …; fi` consumes the status. Found by
the first dispatch and by nothing else; every reader in this repository had
passed the file.

⛔ **Two of a guard's three codes folded into one, on the half that was green.**
The same step's `sh` twin asked `if --egress; then not-restored; fi`, which
treats `2` (*could not run*) as `1` (*refuses*). A guard that never reached a
routing table would have read as one that found a public route on it. ⚠ Found by
making the two halves symmetrical rather than by either half failing, which is
the argument for making them symmetrical.

⛔ **A rule over `.ps1` files is not a rule over the same language in a
workflow.** All three of `check-project`'s PowerShell rules iterated
`git ls-files '*.ps1'`, and `capture.yml`'s `pwsh` blocks broke two of them the
whole time. ⚠ The rule that turned CI red twice had a second door open while it
was being written. ⛔ **And the replacement had the same shape**: it matched
`shell: pwsh` in `.github/workflows/*.yml` alone, while `shell: powershell` is
the same language, a composite action carries its own steps, and a workflow may
be `.yaml`. None of the three exists here, which is when a scope is easiest to
get wrong: every reading agrees on every file in the tree. Plant the fixture the
tree lacks.

⛔ **Not knowing is not agreement, and a comparison has to say so in its
type.** `LIB-02`'s adapter answers whether a tool's declared identity is what
this project measured, and every way of not knowing is one variant that
`agrees()` is false for. ⚠ Today **every** answer about a real client is *not
measured*: a design where that read as a pass would have reported an empty
catalogue as a clean bill of health for every client in it.

⛔ **A relative-path helper whose prefix depends on the path.** `Resolve-Path
-Relative` prepends `./` to most paths and NOT to one already starting with a
dot, so a fixed `Substring(2)` ate the `.g` of `.github` and a whole half
examined nothing. ⚠ **Both halves agreed perfectly on the clean tree while that
was true**, because a file set only appears in the output when something in it
fails. Seven per-plant cases named it at once.

⛔ **A message a machine matches on does not go through a display layer.**
PowerShell's `Write-Error` inside a script renders a source-context block and
WRAPS the message to the host's width: the same refusal is one line at width 200
and two at width 80, so a fixed-string match passes locally and fails on a CI
runner. `[Console]::Error.WriteLine` writes the bytes, and `check-project`
refuses `Write-Error` in a tracked `.ps1`. ⚠ Its needle fired on its own
enforcing file first, because the failure message contains the name: a rule has
to be describable in the file that enforces it, so the needle is an invocation
rather than the word.

⛔ **A red gate must say WHICH case failed.** The excerpt was the first twelve
lines of a harness that prints its failures last, so a red CI log contained
eleven passing rows and no failure at all. The excerpt is the tail now and
`store_report` reprints the failing rows above its summary. ⚠ Until that was
fixed, a CI failure could only be diagnosed by reproducing it locally.

⛔ **A PowerShell `param()` switch and a local of the same name are one
variable.** Names are case-insensitive, so adding `[switch]$Marker` beside an
existing `$marker` local made every invocation of that guard fail to bind. Found
by running it once; invisible to reading.

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

⛔ **A killed run and a new one writing to one file with `>` produce a report
that describes neither.** The second truncates at open and the first keeps
writing at its own offset, so a summary line no run ever printed appeared in the
middle of the output - `2 skipped` where both runs said `1` - padded with a
sparse hole of spaces. ⚠ Read a gate summary out of a file that one process
owns, and kill the previous run by PID: `pkill -f` matches the wrapper shell
that carries the pattern on its own command line, so it kills the caller and
leaves the target running.

⛔ **`check-twins` cannot see a rule whose difference the tree does not
exercise**, and that is written in `check-twins.sh` itself. A new rule over
`.ps1` files differed between the halves on carriage returns, and no `.ps1` here
is ASCII-only, so the comparison agreed on every file while the halves disagreed
about the case none of them is. Prove a scope rule by planting the fixture the
tree lacks.

⭐ **The strongest control available is a reader this project did not write.**
`sha256sum -c` verifies a release, `cbor2` reads a canonical encoding,
`libtorrent` and `torf` read a generated torrent, `curl` is a complete HTTP
client, Python's `pow` is an arbitrary-precision modexp and its `hashlib`
re-derives an identifier from a restated encoding.
⭐ **Whether such a reader can be IN the gate turns on where it comes from.**
`cbor2` needs the package index, so it stayed in an entry's evidence; Python's
`sqlite3` is in the standard library, so `PUB-05`'s reader runs on every gate.
Ask which one a control is before deciding it cannot be a check.

⭐ **A push path drives for real with no network and no credential.** A bare
repository in a scratch directory is a remote as far as git is concerned.

⭐ **A documented command is a copy that drifts unless the document is what
runs.** Both documentation pages have their blocks extracted and executed on every
gate, and each refused a command on its first run.

---

## Facts a session must not restate wrongly

⛔ **Nothing has been published, and no measured record exists.** Everything in
the store is synthetic and says so, and the publisher must not run against this
repository's own remote until that changes. ⭐ **A build HAS now been measured**:
on 2026-09-08 a hosted runner installed Transmission 4.0.5 and captured it, with
`kind=client`, `measured_build=4.0.5`, `stock_client=true`. ⚠ What that produced
is an evidence bundle and an attestation, not a `Profile`: nothing wrote it into
the store, one route ran rather than two, one connector observed it rather than
two, and Windows is untouched, so it does not close `CLIENT-06`.

⛔ **A hosted Windows runner's fingerprint is not a freshness signal.** Two fresh
hosts report the same value; the two Linux runs differed. The claim marker is
what detects a survived host. `docs/capture-host.md` carries why that cannot be
patched by adding a varying input.

⭐ **One observer has now been driven by a stock client, once.** Transmission
4.0.5 announced twice to the HTTP tracker observer on a hosted runner with no
default route. ⛔ Every other observer's driver is still an independent
implementation written from a specification, which shares this project's reading
of the protocol, and that one capture is a single route, a single platform and a
single connector.

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

⭐ This session's record is
[`SESSION-2026-09-08-DISPATCH.md`](SESSION-2026-09-08-DISPATCH.md): the first
dispatch and the exit code it left behind, the fingerprint that answers nothing,
the queryable rendering, and the adapter that fails closed.

⭐ The earlier session record is
[`SESSION-2026-09-06-ADJACENT.md`](SESSION-2026-09-06-ADJACENT.md). ⚠ It is linked
from here and nowhere else, so a rewrite of this file that drops the link orphans
it and `check-docs` refuses that.
