# Agent tooling

Read this before installing software or writing a new helper. Presence is not
capability; run the doctor and the tool's own version command.

## In this repository

| tool | purpose |
| --- | --- |
| [`../scripts/doctor/`](../scripts/doctor/) | read-only host, repository and tool probe |
| [`../scripts/common/check-gate.sh`](../scripts/common/check-gate.sh) | one local gate entry point |
| [`../scripts/common/check-docs.sh`](../scripts/common/check-docs.sh) | documentation/link checks |
| [`../scripts/common/check-no-secrets.sh`](../scripts/common/check-no-secrets.sh) | known secret and public-fingerprint patterns |
| [`../scripts/common/mine-repo.sh`](../scripts/common/mine-repo.sh) | reproducible read-only reference mining |
| [`../scripts/common/check-project.sh`](../scripts/common/check-project.sh) | bit-ids skeleton, catalogue and TODO invariants |

Most checks have PowerShell twins for native Windows, including the
project-specific check. `CI-01` eventually adds an independent Rust validator
for the growing corpus.

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
