# Session record: 2026-09-04, foundations through acquisition

The session summary printed to the operator, saved so it survives the chat. It
carries no work order; [`../../TODO/PROGRESS.md`](../../TODO/PROGRESS.md) has
that and is where it is correct.

Start instant `2026-09-04T15:05:01Z`, baseline `ba07d65`, head `f7c20da`.

| row | measured |
| --- | --- |
| Elapsed | 15:05:01Z to 16:39Z, about 1h35m |
| Commits | 6 on `main`, all pushed and read back from `origin/main` |
| Work | 5 entries completed, 0 deferred, 0 failed |
| Changes | 61 files, +8462 / -111 |
| Size | 31,820 tracked lines over 146 files, from 23,469 over 116. Delta +8,351 |
| Checks | gate 12 checks, 11 passed, 0 failed, 1 skipped. At the start it was 11 checks, 10 passed, 0 failed, 1 skipped |
| Cost | no money. Network: 5 release listings retrieved through `api.gh.pkgforge.dev`, plus three tool downloads |
| Health | tree clean, level with `origin/main`; 4 debts cleared, 8 residuals recorded, nothing deployed |

## What closed

| entry | what it establishes |
| --- | --- |
| `FOUND-03` | byte-exact codecs for the three observed surfaces, and the fixture corpus every observer will parse against |
| `ACQ-01` | the acquisition route record: typed kinds, independence, and an installed version that cites evidence |
| `ACQ-02` | the newest-stable resolver, which fails closed rather than guessing |
| `ACQ-03` | what two routes agreeing is actually worth |
| `ACQ-04` | the boundary that runs before a client is installed |

## Defects found and fixed, by the pass that found them

⭐ **Seventeen, and only three came from the plan.** The rest came from running
things.

**Guard mutation.** Nine lossy codec defects planted; two were not caught on the
first pass. The fixture corpus never reached `bencode::encode` at all, and no
fixture carried a bare newline, so a terminator-repairing decoder failed only a
unit test. Both closed.

**Door sweep.** `FixtureIndex` derived `Deserialize`, skipping its own digest
check. `load_directory` filtered for `*.json` over a non-recursive listing, so a
fixture in a subdirectory would never have run. `HttpRequest::MAX_HEAD` was
written and did not bind. A `Verdict` method had no callers. `Verdict::as_str`
and serde's `rename_all` were two spellings with nothing comparing them.
`classify_across` guarded against two captures of one route but not one record
passed twice.

**Driving the real thing.** The resolver failed closed over 51 of
`transmission`'s own decade-old tags, which produced the publication-order rule
rather than a looser one. `E-BND-13` printed a spelling that appears nowhere in
the document it told an operator to read. `retrieved_at` was the response file's
mtime, which survives a copy. The egress guard used gawk-only functions and
could not run on a minimal image. The runner test claimed the real machine and
could only pass once.

**Writing the tests.** `4.1` and `4.1.0` were ordered rather than compared
equal. `E-RES-01` checked something `from_json` can never reach. A new code
silently reused `E-BND-12`. The divergence test set a field to the value the
fixture already had.

⚠ One finding was in the measuring rather than the code: an exit code was read
after a pipe into `tail`, so a script that exited 1 read as 0. That is this
repository's oldest stated rule, broken while checking a guard.

## What this session could not do

- `check-remote-items` cannot run here. `gh` installs and its token is rejected,
  and the other GitHub route this harness has is scoped to this repository, so
  an action pin elsewhere cannot be resolved by either. The CI Linux lane runs
  it, and it passed on all six pushes.
- The host fingerprint's discriminating power is argued from its inputs rather
  than measured across two machines, because this session had one host.
- No capture was taken and none could be: there is no observer yet.
