# Agent tooling

Read this before installing software or writing a new helper. Presence is not
capability; run the doctor and the tool's own version command.

## In this repository

| tool | purpose |
| --- | --- |
| [`../scripts/doctor/`](../scripts/doctor/) | read-only host, repository and tool probe |
| [`../scripts/common/check-gate.sh`](../scripts/common/check-gate.sh) | one local gate entry point |
| [`../tools/check/`](../tools/check/) | one Go binary carrying the ported rules, run by both lanes |
| [`../scripts/common/mine-repo.sh`](../scripts/common/mine-repo.sh) | reproducible read-only reference mining |
| [`../scripts/common/check-project.sh`](../scripts/common/check-project.sh) | bit-ids skeleton, catalogue and TODO invariants |

⛔ **The PowerShell twin layer is being DELETED rather than extended**, which is
`CI-10`. A rule used to be a `.sh` and a hand-written `.ps1` with `check-twins`
comparing them, because two hand-written halves drift; one binary that runs on
both platforms removes that class instead of checking for it. ⚠ So a new
checking rule goes into [`../tools/check/`](../tools/check/), and a new `.ps1`
twin of an existing check is work added to a layer that is going away.

⭐ **`check-no-secrets` and `check-docs` are in that binary**, which is why this
table no longer names a script for either: `bit-check check-no-secrets` is the
default run and `bit-check check-no-secrets --public` adds the rules that only
matter for a public repository. ⚠ `bit-check --rows` is the list; five file pairs
still have twins and `check-twins` compares them as five rows.

`CI-01` eventually adds an independent Rust validator for the growing corpus.

## Existing external tools

| job | preferred tool |
| --- | --- |
| authenticated GitHub reads and this repository's authorized writes | `gh` |
| ordinary web fetch | `curl.exe` on Windows, `curl` on POSIX; use the configured read proxy after a direct 401/403 or route failure |
| JSON/YAML inspection | `jq` / `yq` |
| Rust build, formatting and lint | pinned `cargo`, `rustfmt`, `clippy` |
| shell lint and formatting | `shellcheck`, `shfmt` |
| cross-platform stock BitTorrent peer | `aria2c` |
| disposable Linux environment on Windows | WSL tooling or the existing container engine, per [`containers.md`](containers.md) |

The packet oracle required by `OBS-07` is not assumed present, and that entry
must select and prove the Linux and Windows routes before any capture depends on
one. ⚠ `CI-03` used to share that sentence and no longer does: the runner
question it owned is settled, and a hosted runner with its default routes
deleted is the host a capture runs on.

## Remote read routes

[`AGENTS.md`](AGENTS.md) rules 8 and 9 are the rule; this page carries what
measuring it produced.

⚠ **Not "direct first, then the proxy".** An unauthenticated read of
`api.github.com` answers 403 or 404 for a repository the caller cannot see, which
is indistinguishable from a repository that does not exist. Measured on
2026-09-08: `api.github.com/repos/actions/upload-artifact/releases/latest`
answered 403 from this harness, and the same path through the proxy answered 200.

⚠ **Record the original URL and the route that answered** in acquisition or
reference provenance. A value fetched through a proxy and a value fetched
directly are the same value only if somebody can see which one it was.

⭐ **Two routes to one fact is a control, not a fallback.** The
`actions/upload-artifact` pin in the capture workflow was resolved through the
GitHub proxy and then re-read through `raw.githubusercontent.com` behind the
other one, which is what establishes that the commit really carries that action
and accepts the inputs the workflow passes it.
