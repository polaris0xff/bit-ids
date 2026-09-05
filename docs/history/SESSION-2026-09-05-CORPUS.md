# Session record: 2026-09-05, the corpus and publishing path

The session summary printed to the operator, saved so it survives the chat. It
carries no work order; [`../../TODO/PROGRESS.md`](../../TODO/PROGRESS.md) has
that and is where it is correct.

Baseline `5a21e32`, work range `5a21e32..ebaaf55`. ⚠ The closing review commit
that carries this file is deliberately outside that range: a total including it
changes when this file is written.

| row | measured |
| --- | --- |
| Commits | 6 over `5a21e32..ebaaf55`, all pushed and read back from `origin/main` |
| Work | 5 entries completed, 0 checkpointed, 0 failed |
| Changes | 32 files over `5a21e32..ebaaf55`, +6960 / -155 |
| Checks | gate 17 checks, 16 passed, 0 failed, 1 skipped on `sh`; 10 passed, 0 failed, 7 skipped on `pwsh` |
| Suite | 36 binaries, 337 passed, 0 failed, plus 2 doctests |
| Mutation | 41 defects planted across 5 source passes, all refused after correction, with 2 guards named below as unrefuted. The 5 filesystem harnesses hold 66 cases between them, all passing |
| CI | runs 30 to 33 read back per commit; ⚠ run 30 at `b71f767` was red on both lanes at *Repository gate*, cause named below and fixed in the next commit |
| Cost | no money. Network: git pushes to this repository's own `main` and read-only GitHub API calls for CI status. No artifact was downloaded and nothing was installed on a host |
| Health | tree clean, level with `origin/main`, nothing deployed, no capture taken, nothing published |

## What closed

| entry | what it establishes |
| --- | --- |
| `CORPUS-01` | where a record is filed, and that a published path never changes or disappears |
| `CORPUS-02` | what only a whole store can answer, of which evidence reachability is the one nothing else could |
| `CORPUS-03` | the consumer-facing lookups and the latest view, derived and resolvable back to the records |
| `PUB-01` | a bundle assembled once, described by itself, byte-identical between runs |
| `PUB-02` | a publisher that appends, never forces, and reads the branch back |

## Defects found, by the pass that found them

⭐ **The driven pass out-found the suite four times, and each time the defect was
a gate on one of two doors into the same action.**

- ⛔ `check-store` selected a record to place by its **name** and opened it, so a
  named pipe blocked the process forever while `validate_tree` already carried
  the refusal for it. The reader takes the entry kind from the walk now.
- ⛔ `validate_corpus` reported a store of nine orphan artifacts as **valid**.
  The bundle sweep ran inside the per-manifest check and was scoped to that
  manifest's bundle, so a store with no manifest had nothing to sweep with. It
  sweeps the whole tree now.
- ⛔ **The append rule and the derived files collided.** A correct second
  publication changes the manifest, the checksums and the indexes by design, and
  treating every published path as immutable made one impossible.
  `CANONICAL_ROOTS` names the roots the rule is about.
- ⛔ **The read-back could not catch a remote that discarded the push.** It
  compared the fetched tree only against the prior one, and a rewound ref appends
  to the prior tree because it *is* the prior tree.

**Door sweep.** Two spellings of one layout. The example decided whether a path
was a record, and therefore whether the placement check ran at all, from its own
copy of the prefixes: a layout change would have left it recognising nothing with
the suite still green. And `check-store.sh` spelled three plant targets by hand,
so the same change would have landed them elsewhere while the cases still passed
under the name of a rule they had stopped testing. Both are derived now.

**Guard mutation.** ⛔ **A surviving plant is a question, not a verdict.** Of
eight over the index builder, two survived because the mutation was equivalent on
the fixture data and four because the tests were genuinely thin: the row sort was
invisible because the store was read in one order both times, nothing excluded a
provisional record, the varying-first peer-ID rule was never reached, and one
code was asserted where it could not fire.

**Claim audit.** The entry `Prove` for `CORPUS-02` selected one integration
binary with `--test`, which skips the library's own tests; it is rewritten and
the original recorded rather than silently replaced. `docs/publishing.md`'s
layout was not injective over the identity tuple and is amended.

**Writing the probes.** ⚠ Three separate harness defects, each of which would
have reported a guard proved or broken when it was neither:

- `grep -F` splits a pattern containing a newline into alternatives, so a unique
  multi-line literal counted as the sum of its lines and three plants read as
  ambiguous. It failed safe and still left those guards unproven.
- A harness exit of 2 is *could not run*, never *refused*, and one review pass
  counted it as a refusal on its filesystem path.
- A harness assigned its own `ROWS` and overwrote the shared library's
  accumulator: ten passes printed over eight lines. `store_report` now compares
  the rows it holds against the count it prints.
- A source check grepping the publisher for a force flag matched the header
  sentence explaining that it uses none. Comments are stripped first, and the
  stripper is exercised against a line that really is code.

⛔ **And this table had two fabricated numbers in its first draft.** The change
counts were written from memory as 45 files and +6742 / -279; `git diff
--shortstat` over the same range says 32 files and +6960 / -155. A number nobody
measured is worse than a blank, because a blank gets checked.

**What was measured but never verified.** That a `tar` archive can carry `a` and
`a/b` together while a filesystem refuses the pair was measured here rather than
asserted, because it is the reachability argument for a refusal the shell harness
cannot plant.

## The guards that are not refuted

| guard | why nothing refutes it | what would have to be true |
| --- | --- | --- |
| `index::build`'s `latest.sort()` | `best` is a `BTreeMap` keyed by the build line and `LatestRow` orders on those same fields first | `best` stops being an ordered map, or the row's ordering stops agreeing with the key's |
| `release::assemble`'s `entries.sort()` | the entries come out of a `StoreTree`, which is an ordered map, and `ReleaseEntry` orders on its path first | `assemble` takes anything but an ordered map |

⭐ Both are kept and both say so where they are written, so a later reader does
not take an unreached guard for a proven one.

## What this session could not do

- ⛔ **No capture was taken and none could be.**
  `sh scripts/acquisition/assert-disposable.sh --egress` exits 1 on a session
  host. Everything in the tree is synthetic and says so.
- ⛔ **Nothing was published.** The publisher has never run against this
  repository's own remote and must not until a measured record exists. Its
  acceptance runs against a bare repository the harness creates and deletes.
- ⚠ **One CI run was red and it was the tree's fault.** Run 30 at `b71f767`
  failed on both lanes at *Repository gate*, below *Rust check*, so not the
  runner's network: `check-one-home` refused a sentence duplicated between the
  record and `docs/architecture.md`. ⛔ It reached CI because the last local run
  before the push was a hand-typed subset of the gate rather than
  `check-gate.sh`, which is the defect that runner exists to prevent, produced
  against the runner itself. Fixed in `bdfeb8f`; runs 31 to 33 are green on both
  lanes.
- `check-remote-items` still cannot run on this host, for the reason two sessions
  ago measured. ⚠ It passes on the Linux CI lane under `--strict`, read back
  from the run log.
- ⚠ `validate-profile` and `validate-run`, the two oldest examples, have no
  mutation harness driving them. They belong to `SCHEMA-01` and `SCHEMA-02` and
  are recorded here rather than silently left.
