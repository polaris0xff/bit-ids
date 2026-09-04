# Bootstrap review

Review instant: 2026-09-04
Scope: initial repository foundation before its single bootstrap commit

## Pass 1: structural and mechanical integration

The first pass ran the Rust compiler and tests, Clippy with warnings denied,
ShellCheck, shfmt, both project-check implementations, and both fast aggregate
gates.

Findings corrected in place:

- one Rust documentation lint;
- inherited shell formatting drift;
- seven broken or unlinked documentation paths;
- the template's unspecialized security-advisory URL;
- seven uses of a product-name character outside the repository's documented
  text policy; and
- three full revision hashes that the public secret guard correctly required
  to be represented as linked short revisions.

Residual scope: semantic consistency and release-readiness review remain to be
performed and recorded below before the bootstrap commit.

## Pass 2: semantic consistency and dependency routing

The second pass read the architecture, capture method, publishing contract,
client matrix, agent router, tooling guide, and work order as one system. It
compared named components, target IDs, evidence thresholds, acquisition rules,
and entry routes.

Findings corrected in place:

- two router links used shortened category filenames that do not exist;
- two documents referred to an uncreated `CONN-01` rather than the observer
  control and trusted-runner entries that own the decisions;
- the human matrix used `bittorrent-mainline` while the canonical catalogue
  uses `bittorrent`; and
- the tooling guide described the project validator as shell-only after its
  PowerShell twin was added.
- the project validator trusted fixed status counts instead of deriving them
  and did not compare index rows with entry bodies. Both twins now derive the
  totals, priority table, summary, and body status for every entry.

The review confirmed that source and crawler information is non-authoritative,
each field requires independent live corroboration, latest stable versions
append rather than replace, and proprietary targets remain unpublished when
the acquisition or safety gates cannot be satisfied.

## Pass 3: release readiness, mutation, and claim audit

The third pass exposed all 93 files as one diff, ran Git's whitespace check,
parsed the workflow and catalogue, loaded locked Cargo metadata, searched for
stale routes, and read the critical workflow, Rust API, validator, and agent
router changes.

Findings corrected in place:

- the action-pin guard could miss a floating tag followed by a YAML comment;
- the official Ubuntu 24.04 runner inventory includes ShellCheck but not
  `shfmt`, so CI now installs a pinned `shfmt` version before using it; and
- two copied configuration comments still directed readers to template
  `dotfiles/` paths that are not part of this repository.

The action guard was mutation-proved by replacing one checkout SHA with `@v7`.
Both the shell and PowerShell checks rejected it with exit 1. After restoring
the SHA, both returned exit 0. The complete twin run then reported agreement
for both doctors, every check pair, the reference-miner self-test, and remote
item inspection.

No identity profile, data branch, release, client installation, or public-swarm
capture is claimed by this bootstrap.
