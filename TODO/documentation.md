# Documentation entries

## DOC-01: Public data and library documentation

Source: operator publication requirement
Priority: P1 | Effort: M | Status: DONE

Problem: Formats and Rust APIs are unusable without field semantics, examples,
compatibility rules, and integrity instructions.

Approach: Generate field references from the schema where possible and add
small verified examples for raw URLs, release assets, SQLite, and Rust lookup.

Prove: `sh scripts/common/check-examples.sh` passes, extracting every shell
example out of [`../docs/consuming.md`](../docs/consuming.md) and running it
against a fixture release the harness assembles.

### Decision: the examples are extracted from the page and executed

⛔ **A documented command that is a copy of a tested snippet is a copy that
drifts, and the copy a reader runs is the one nobody checked.** So the harness
reads the fenced blocks out of the page and runs those, rather than holding its
own copies beside it.

⚠ **The fence language is the whole rule.** A `sh` block is a command a reader
can run and the harness runs it; a `text` block is a shape, such as the plan's
output columns, and the harness never runs one. That rule is checked against
itself: a run where the extractor took *every* fenced block is a run whose
language rule has stopped applying, and it fails.

⚠ The Rust block is compiled by the crate's own suite rather than by the
harness, and the harness checks that the case exists. Extracting and compiling
the block itself would need a build script, which this entry does not need to
pay for.

### ⛔ What the harness found on its first run: a documented command that failed

The example showing that an unmeasured build line answers nothing was written as
`cmd && exit 1`. Under `set -e` the list's status is the failing command's, so
the block exited 1 and the example was refused. It is an `if` now, which reads
better as documentation and is the form that actually runs.

⭐ **That is the entry paying for itself in its first minute.** A page carrying
that line would have been correct in intent and wrong in fact, and nothing but
running it would have said so.

### Decision: what is deliberately not on the page

⚠ **No SQLite example**, because `PUB-05` is blocked on a dependency decision and
nothing writes `formats/bit-ids-v1.sqlite3`. ⚠ **No fetched URL**, because
nothing has been published; the forms live in `docs/publishing.md` and say they
are unexercised. ⛔ **No measured profile**, because there are none. Each is
named on the page rather than left as an absence a reader has to notice.

⚠ **The field reference is a pointer rather than a generated copy.**
`docs/architecture.md` section 4 is the authority on the record shape and the six
field states, and generating a second copy of it would be the value-in-two-places
hazard this repository refuses everywhere else. `check-one-home` enforces that;
the page carries what a *consumer* needs and points for the rest.

### Acceptance, all run on 2026-09-08

- `sh scripts/common/check-examples.sh`
- `cargo test -p bit-ids --locked --test catalogue`
- `sh scripts/common/check-gate.sh`

### Closure evidence, 2026-09-08

| what | measured |
| --- | --- |
| `sh scripts/common/check-examples.sh` | 9 cases, 9 passed, 0 failed; 6 shell examples extracted and run |
| `cargo test -p bit-ids --locked --test catalogue` | 19 passed, 0 failed, including the compiled Rust example |
| `sh scripts/common/check-gate.sh` | 24 checks, 23 passed, 0 failed, 1 skipped, 0 unavailable |
| `pwsh -File scripts/common/check-gate.ps1` | 24 checks, 11 passed, 0 failed, 1 skipped, 12 unavailable |
| `cargo test --workspace --locked --all-targets` | 50 binaries, 542 passed, 0 failed |
| driven pass | the page's own commands, run in document order against a two-version publication the harness assembles, indexes, renders and describes |
| guard mutation over the page and the harness | 5 plants, 5 refused: a command that does not work, a command naming a target nothing measured, the Rust example removed, every example removed, and the extractor rewritten to take every fenced block |
| finding | one documented command did not work and was corrected before the page was committed |

### What the door sweep found

⚠ **`check-examples` is in the gate and not in `check-twins`' pair list**, which
is the same contract `check-runner` and the corpus provers hold: it is an `sh`
harness with no PowerShell half, so a pair comparison would have nothing to
compare. The PowerShell lane declares it with that reason rather than omitting
the row.

⚠ **The page is linked from three places and each is a different reader.** The
`README` is where somebody arriving at the repository looks, `publishing.md` is
where a publisher looks, and `AGENTS.md`'s routing table is where the next
session looks. ⛔ `check-docs` refused the page while it was linked from none:
an unlinked page is not read, so it is not corrected.

### Residuals

- ⚠ The Rust example is a copy in `tests/catalogue.rs` rather than an extraction.
  Both are small enough to compare by eye and the harness refuses a page whose
  Rust block count is zero, but a page whose Rust block drifts from the test
  would pass. Extracting it needs a build script.
- ⚠ Three of the Approach's four example subjects exist. Raw URLs and release
  assets wait on a first publication, and SQLite waits on `PUB-05`.
- ⚠ `DOC-02`, the contributor capture-run handbook, is separate and is `P2`.

## DOC-02: Contributor capture-run handbook

Source: future external contribution path
Priority: P2 | Effort: M | Status: DONE

Problem: A contributor can submit plausible output that lacks isolation,
two-route identity, independent observation, or redistribution review.

Approach: Document host preparation, safe acquisition, lab execution, evidence
review, correction flow, and exactly what cannot be accepted.

Prove: `sh scripts/common/check-handbook.sh` passes, extracting the runnable
steps out of [`../docs/contributing-captures.md`](../docs/contributing-captures.md),
running them in order in one directory with nothing the page does not carry, and
handing the result to the validator.

### Decision: the walkthrough is the page, and the harness contributes four names

⛔ **"Without undocumented steps" is the clause that decides the design.** A
walkthrough a harness helps along proves that the harness knows how to build a
submission, not that the page does. So the harness binds `REPO`, `BIN`,
`SUBMISSION` and `EMPTY`, which is what a contributor supplies by knowing where
they cloned to, and then runs the page's own blocks and nothing else.

⚠ **The steps share one directory and one shell**, unlike `DOC-01`'s examples,
which run independently because a reader can run any one of those alone. Here
the later steps depend on the earlier ones having happened, which is what makes
it a walkthrough.

### ⛔ What running the page found: a documented build that uses the wrong compiler

The build step was written as `cargo build --manifest-path "$REPO/Cargo.toml"`
with no `cd`. It fails: `rust-toolchain.toml` pins the compiler and rustup finds
it by walking up from the **working directory**, so the same command run from
anywhere else silently uses whatever that machine defaults to. ⭐ A contributor
following the page from their own scratch directory would have built with an
unpinned toolchain and never been told. The `cd` is documented now, with the
reason.

### ⛔ What cannot be walked, and why that is stated rather than worked around

Sections 2 through 5 are documented and not executed. A session host has a
public route, so `assert-disposable.sh --egress` refuses it, and running a
client there would be the capture the boundary exists to prevent. ⚠ The host
guards are in a `text` block for that reason, and the harness has a case
asserting the extractor did **not** take them: running `--claim` here would
write a capture marker onto this machine.

### ⛔ What the mutation pass found: two checks that could not fail

Nine plants over the page and the harness. Three survived the first pass and
two of the three were real.

⛔ **The page's own validation steps could be deleted with everything staying
green**, because the harness validates the result independently. A page that
stopped telling a contributor to check before submitting would still produce an
acceptable submission. The `teaches` case is what closes it.

⚠ **And that case's first version passed for the wrong reason.** It grepped the
walkthrough for `validate-corpus`, which matches the build step's own
`--example validate-corpus` argument, so deleting the invocation left it green.
It matches `$BIN/validate-corpus` now: a check that passes because a different
line happens to satisfy it is the shape
[`../docs/methodology/reviews.md`](../docs/methodology/reviews.md) names.

⛔ **The harness's own result validation could be replaced by `true`** and
nothing noticed, because an exit code of 0 is what `true` produces. The verdict
is read from the validator's own output line now, so the case cannot pass unless
the validator ran.

⚠ **The third survivor is not detectable from inside and is recorded rather than
fixed.** Rewriting the negative control's comparison to `if true` makes it pass
unconditionally, and no harness detects a weakened assertion in itself. That is
a reviewer's catch, and it is why a mutation pass is read rather than scored.

### Acceptance, all run on 2026-09-08

- `sh scripts/common/check-handbook.sh`
- `sh scripts/common/check-gate.sh`

### Closure evidence, 2026-09-08

| what | measured |
| --- | --- |
| `sh scripts/common/check-handbook.sh` | 7 cases, 7 passed, 0 failed; 5 steps extracted and run in order |
| `sh scripts/common/check-gate.sh` | 25 checks, 24 passed, 0 failed, 1 skipped, 0 unavailable |
| driven pass | the page's own steps, run in one directory, producing a submission the validator accepts with `valid store: 1 record(s), 1 run(s), 11 object(s)` |
| guard mutation | 9 plants; 8 refused after two real findings were fixed, 1 undetectable from inside and recorded above |
| finding | one documented command built with an unpinned toolchain and was corrected before the page was committed |

### Residuals

- ⛔ **The walkthrough starts where a contributor's capture ends.** Host
  preparation, acquisition, the lab run and the evidence review are documented
  and cannot be executed here, and the page says so in its own last section
  rather than leaving a reader to notice.
- ⚠ The submission the walkthrough builds is synthetic, which is what everything
  in this repository is today. It proves the acceptance path, not a measurement.
- ⚠ No contributor has followed the page. What is measured is that its steps run
  and that the result is accepted; whether a person finds it sufficient is a
  reading nobody here can do for them.
