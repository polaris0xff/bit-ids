# Corpus entries

## CORPUS-01: Append-only canonical store

Source: b-ids data branch architecture and operator publication requirement
Priority: P0 | Effort: L | Status: DONE

Problem: Regenerating a latest-only dataset would erase older stable-release
records and destroy auditability.

Approach: Store run manifests and profiles under immutable product, version,
platform, route-set, and capture identifiers. New releases append; corrections
add records rather than rewriting evidence.

Prove: the validator rejects deletion or byte changes against the prior data
branch and accepts a new version directory.

Acceptance, both run on 2026-09-05:

- `cargo test -p bit-ids --locked --all-targets`
- `sh scripts/corpus/check-store.sh`

### Decision: the path is the identity tuple, and the route set is not in it

The Approach names a route-set component and it is not in the path. A store path
is derived from the tuple [`RecordId`](../crates/bit-ids/src/identity.rs)
digests, in full and nothing else, because that is the only choice under which
the path and the identifier cannot disagree. A component the identifier does not
carry files one record under two names; a component the identifier carries and
the path drops files two records at one name. The route set is recorded where it
is measured, in the record's own `acquisition`, and `E-ACQ-07` and `E-ACQ-08`
already refuse a record whose routes are not independent.

⛔ **The rejected alternative was the published layout itself, and it was wrong
in exactly that second way.** `docs/publishing.md` filed a profile at
`profiles/v1/<target>/<version>/<platform>/<arch>/<capture-id>.json`, with no
`package` segment, while the identity tuple carries one. A `deb` and an
`AppImage` of one version on one platform are two records and were one file.
The layout is amended and `two_packages_of_one_build_are_two_paths` pins it.

### Decision: a version that cannot be a path segment blocks publication

⛔ **`Version` accepts `../../etc`, measured rather than assumed**, because a
version string is what the installed build printed and imposing a grammar on it
would refuse builds that number themselves some other way. That value reaches a
path, so the store refuses it as `E-STO-01` rather than escaping it.

Percent-encoding lost: it needs `%`, which `RelPath` refuses, and widening that
alphabet weakens the rule every evidence path already depends on. A lossy escape
lost harder, because it maps two versions onto one directory, which is the
collision the append-only store cannot survive. `version_is_not_a_path_segment`
carries the measurement.

### Closure evidence, 2026-09-05

| what | measured |
| --- | --- |
| `sh scripts/corpus/check-store.sh` | 14 cases, 14 passed, 0 failed |
| `cargo test -p bit-ids --locked --all-targets` | 13 store unit tests, 6 store integration tests, 0 failed |
| `cargo test --workspace --locked --all-targets` | 32 binaries, 308 passed, 0 failed |
| `cargo test --workspace --locked --doc` | 2 passed, 0 failed |
| `sh scripts/common/check-gate.sh` | 13 checks, 12 passed, 0 failed, 1 skipped |
| `pwsh -File scripts/common/check-gate.ps1` | 13 checks, 10 passed, 0 failed, 3 skipped |
| `cargo fmt`, `cargo clippy --workspace --locked --all-targets -- -D warnings`, `shellcheck`, `shfmt -d -i 2 -ci` | exit 0 |
| guard mutation over `store.rs` | 10 plants, 10 refused |
| guard mutation against a real filesystem | 9 defects planted, 9 refused, plus 4 harness self-guards exercised |

⭐ **Nine of the ten source plants were refused by the filesystem harness and
the suite between them; the tenth needed a unique literal before it could be
planted at all.** `E-STO-12`, `E-STO-22`, `E-STO-01` and `E-STO-04` cannot be
reached through a filesystem, so they are planted in the unit tests instead:
⭐ **that a tar carries `a` and `a/b` together while `mkdir` refuses the pair
over a file was measured here, not assumed.**

⭐ **The driven pass found a defect the suite could not.** `check-store`
selected a record to place by its *name* and opened it, so the named-pipe plant
blocked the process forever while `validate_tree` already carried `E-STO-15` for
it. One action, two doors, and only one had a gate. The reader now takes the
entry kind from the walk.

⛔ **The door sweep found a second spelling of the layout.** The example decided
whether a path was a record, and therefore whether the placement check ran at
all, from its own copy of the prefixes. A layout change would have left it
recognising nothing, every record skipping its placement check, and the suite
green: a gate that stops applying rather than one that fails.
`is_profile_path` and `is_manifest_path` now live beside the composer and
`every_derived_path_is_recognised` closes the loop.

⛔ **The review's own harness miscounted three plants and said so.**
`grep -F` splits a pattern containing a newline into separate alternatives, so a
unique multi-line literal counts as the sum of its lines and reads as ambiguous.
It failed safe, refusing to plant rather than planting wrongly, and three guards
went briefly unproven until the counter moved off `grep`. `replace_once` in the
committed harness now refuses a multi-line literal outright, and that refusal is
one of its four self-guards.

### Residuals

- ⚠ `check-store.sh` has no PowerShell half, so the Windows lane reports it as a
  named skip. The rules are not platform-specific and the Rust suite exercises
  every one of them on both lanes; what the Windows lane does not do is plant a
  symbolic link and a named pipe against a real filesystem, neither of which an
  unprivileged Windows session can create. `CI-03` owns the Windows runner.
- ⚠ Nothing yet assembles a store to check. `PUB-01` builds the tree and
  `PUB-02` runs this comparison over the fetched `data` branch; until then the
  driving surface is pointed at two directories by hand.

## CORPUS-02: Semantic corpus validator

Source: operator accuracy requirement
Priority: P0 | Effort: L | Status: OPEN

Problem: Schema validity alone cannot prove route count, connector independence,
field provenance, agreement, stable status, or evidence reachability.

Approach: Implement all publication invariants in Rust with stable diagnostic
codes and adversarial fixtures.

Prove: `cargo test --workspace --locked --test corpus_validator` rejects one fixture for each
invariant and validates the complete golden corpus.

## CORPUS-03: Deterministic indexes and latest views

Source: b-ids consumer-oriented indexes
Priority: P0 | Effort: L | Status: OPEN

Problem: Consumers need convenient latest and lookup views without making
those derived files authoritative.

Approach: Generate sorted indexes by client, peer prefix, BEP 10 client value,
platform, version, and capture instant from canonical records only.

Prove: two clean builds have identical digests and every index row resolves to
one canonical profile.

## CORPUS-04: Supersession and correction records

Source: append-only publication constraint
Priority: P1 | Effort: M | Status: OPEN

Problem: A proven bad record must stop appearing in current views without
deleting the historical evidence.

Approach: Define signed correction records naming the original digest, reason,
replacement, and review evidence; derive current views accordingly.

Prove: fixtures retain the original bytes, exclude a superseded record from
latest views, and expose the full correction chain.
