# Session 2026-09-08: the button, the database and the question a client has

What `CI-06`, `PUB-05` and `LIB-02` took, and the six findings that were not
about the code being written.

⚠ **This is narrative history and goes stale on purpose.** The current truth is
[`../architecture.md`](../architecture.md) and the entries in
[`../../TODO/`](../../TODO/). Nothing here is amended to match a later tree.

## What closed

| unit | what it settled |
| --- | --- |
| `CI-06` | the capture workflow, dispatched twice; three unmeasured facts answered and one defect bought |
| `PUB-05` | the SQLite rendering, so every path `publishing.md` promises is written |
| `LIB-02` | the bit-cli adapter: a comparison that fails closed |

## ⛔ The dispatch bought a defect no reader here could have found

Capture run 1's Windows job went red on *Restore the route*, **and the restore
had worked.** That step ends by running the egress guard inverted: a host that
can reach the network again is one the guard refuses, so a refusal is the proof
the route came back. It refused, exit 1, exactly as designed - and GitHub's
`pwsh` wrapper reads a block's residual `$LASTEXITCODE` as the step's verdict.
A guard succeeding at its job failed the step, and the evidence upload was
skipped as a consequence.

⭐ The `sh` twin never had it, because `if guard; then …; fi` consumes the
status. One of two paths into one mistake, and only the path GitHub reads was
wrong.

⚠ **The reproduction had to be faithful or it said the opposite.** A first
driver ran the block with `pwsh -command ". '<file>'"` and reported the broken
version **passing**. GitHub also *appends*
`if ((Test-Path -LiteralPath variable:/LASTEXITCODE)) { exit $LASTEXITCODE }`,
and that append is the entire mechanism.

⭐ And a fact about that wrapper worth keeping: a dot-sourced script's exit code
**collapses**. Measured across 0, 1, 2, 3 and 42 on pwsh 7.4.6, every non-zero
value arrives as 1 through `-command`, while `-File` preserves it. So a `pwsh`
step's exit code says *failed* and never says which refusal fired, which is a
second and independent argument for writing the message with
`[Console]::Error.WriteLine`.

## ⛔ The Prove refuted itself on one platform

`CI-06` asked that two runs of one platform report different fingerprints. Linux
did. **Windows reported the same value twice, on two hosts that were both
fresh** - each run's claim succeeded, so the marker the previous run wrote was
gone.

⭐ That is structural rather than a defect to patch. A fingerprint must differ
between two hosts *and* survive a reboot within one; every input with the second
property is a property of the **image**, and hosted runners are clones of one
image. Adding a per-boot value would buy the first requirement by destroying the
second, which is the failure the guard exists to catch. The claim marker is what
actually answers the question, and always was.

⚠ The workflow's Windows summary said "A repeat means the host was not
destroyed". It says what the value identifies now.

## ⛔ Three rules, and the same defect inside them

`check-project`'s PowerShell rules iterated `git ls-files '*.ps1'` and never
reached a `pwsh` block inside a workflow - the same language, on the same
runners, with the same hazards - and `capture.yml` was carrying a live instance
of each.

⚠ **The replacement had the same shape.** It matched `shell: pwsh` in
`.github/workflows/*.yml` alone, while `shell: powershell` is Windows PowerShell
rather than a different language, a composite action carries its own steps, and
a workflow may be `.yaml`. None of the three exists in this tree, which is
precisely when a scope is easiest to get wrong: every reading agrees with the
correct rule on every file here. The fix is proved by planting each fixture the
tree lacks.

⛔ **And widening it broke one half while both halves went on agreeing.**
`Resolve-Path -Relative` prepends `./` to most paths and **not** to one already
starting with a dot, so a fixed `Substring(2)` ate the `.g` of `.github` and
that half examined nothing at all. A file set only appears in the output when
something in it fails, so the clean tree could not show it; seven of fourteen
per-plant cases named it at once. That is the whole argument for comparing the
twins per planted mutation.

## ⭐ A reader in the gate, rather than in an entry's evidence

`PUB-05`'s database is opened by Python's `sqlite3`. Unlike `cbor2`, which needs
the package index and therefore stayed in `PUB-03`'s evidence, Python's is in
the standard library - so the independent reader runs on **every** gate. It is
older than the writer, 3.45.1 against the bundled 3.50.2, which is the direction
that matters for a published file, and it has been seen to refuse a truncated
one.

⚠ The version of `rusqlite` was measured rather than taken. The newest resolves
thirteen further packages - a WebAssembly stack this project never builds for -
for the same API and the same bundled library.

## ⭐ Not knowing is not agreement

`LIB-02`'s adapter answers whether a tool's declared identity is what this
project measured. `Answer::agrees` is true for one variant; every way of not
knowing is `Unmeasured`. **Today every answer about a real client is
`Unmeasured`, because nothing has been measured** - and a design where that read
as a pass would have reported this project's own emptiness as a clean bill of
health for every client in it.

⛔ No document here claims to have measured `bit-cli`. Its prefix `-CL0200-` was
derived from its own source and used as the *claim*; the catalogue on the other
side is this project's own, and the agreeing cases use a synthetic record that
says it is one.

## ⚠ What reading the run back found that a green gate could not

The five new plants each run the whole gate, and the Linux lane went from 13.4
minutes to **24.4 of the 30 it then had**. It passed, with under six minutes to
spare. A lane that runs out of time reports as infrastructure rather than as a
defect and reads as a flake nobody root-causes. The budget is 45 now, with the
measurement written beside it.
