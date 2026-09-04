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

The packet oracle required by `OBS-07` is not assumed present. That entry and
`CI-03` must select and prove the Linux and Windows routes before any capture
depends on them.

## Remote read routes

Prefer `gh` for GitHub. If a GitHub REST path is fetched without authenticated
GitHub CLI, prefix it with `https://api.gh.pkgforge.dev/`. GraphQL and routes
that require authentication stay with `gh`.

Fetch another web URL directly first. On 401/403 or route failure, prefix the
original URL with `https://api.rv.pkgforge.dev/`. Record the original URL and
the route that answered in acquisition/reference provenance.
