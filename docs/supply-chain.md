# Supply chain

What this repository executes that it did not write, how each of those is
pinned, and what refuses an unpinned one.

The threat is narrow and specific. This project publishes measurements taken by
code running unattended in CI, and a reader trusts a published profile because
they trust the observer that produced it. A dependency that can change without
a reviewed commit changes the observer without changing the record of what the
observer was.

## The three layers

| layer | pinned by | enforced by |
| --- | --- | --- |
| Rust toolchain | [`../rust-toolchain.toml`](../rust-toolchain.toml), an exact channel | `cargo` invoked through `rustup` selects that toolchain and installs it if absent |
| crates | [`../Cargo.lock`](../Cargo.lock), committed | `--locked` in CI, plus the source and checksum test in `check-project` |
| actions | a 40-character commit in [`../.github/workflows/`](../.github/workflows/) and any composite action under `.github/actions/` | the pin test in `check-project`, and `check-remote-items` against GitHub |
| CI tools fetched at run time | a module version, verified against Go's checksum database | the Go toolchain, which refuses a module whose contents no longer match |

⚠ **There is no separate register of any of these.** The lockfile is the crate
inventory and the workflow is the action inventory. A second file listing the
same commits would be a value in two places with nothing checking that they
agree, which is the drift these rules exist to prevent.

### The fourth layer is the loose one

The Linux lane installs `shfmt` with `go install mvdan.cc/sh/v3/cmd/shfmt@v3.14.0`.
That is a version tag, not a commit, and `check-project` does not see it: the
pin rule reads `uses:` lines, not `run:` scripts.

⭐ **It is a tag that cannot move underneath us, for a reason specific to Go.**
`go install module@version` verifies the downloaded module against the public
checksum database, so a re-tagged release fails to install rather than
installing different code. That is a real immutability mechanism and not the
same as a git tag, which can be force-pushed and resolves to whatever it points
at today.

⚠ **The reasoning, not the form, is what makes it safe.** Do not copy
`@v1.2.3` to a fetch that has no equivalent verification: a `curl | sh` of a
release asset at a tag has none of this. If the checksum database is disabled
for a runner, through `GONOSUMDB`, `GOFLAGS` or `GOPRIVATE`, this layer loses
its guarantee and the tool needs pinning some other way.

## Crates

Every package in the lockfile either has no `source`, meaning it is a member of
this workspace, or comes from the crates.io registry with a checksum.
`check-project` reads the lockfile and refuses anything else, so a `git` or
`path` dependency is caught in the file cargo generates whatever the manifest
said. A second test names the manifest line as well, because a report that
points at the file somebody edited is easier to act on than one that points at
generated output.

⛔ **A dependency is a decision, not a detail.** `SCHEMA-01` added the first
three and recorded why in its entry: a hand-written JSON reader would have been
a new silent-corruption surface in the layer that must not corrupt. The next
one gets the same argument in its own entry, including what was considered and
rejected.

⭐ `FOUND-03` added a second workspace member, `bit-ids-wire`, and **no** new
third-party crate: its lockfile diff is the member and nothing else. A
`BitTorrent` codec crate off the registry would have been the obvious shortcut
and the wrong one here, because the decode is the measurement. A dependency that
normalises what it reads, by sorting dictionary keys, folding header case or
percent-decoding on arrival, destroys the evidence this project publishes, and
that behaviour is the sensible default for every library written for a client
rather than for an observer.

`OBS-01` and the observers that followed it added two more workspace members,
`bit-ids-lab` and `bit-ids-probe`, and no third-party crate: the lab binds
sockets with `std::net` and one thread per endpoint rather than an async runtime,
which would have been a dependency tree an order of magnitude larger than
everything here, in the component that must stay reviewable.

`OBS-08` added `sha1` 0.11.0, the fourth third-party crate and the first since
`SCHEMA-01`. The info hash is SHA-1 by BEP 3 and this project does not get to
choose otherwise. ⭐ **Measured rather than argued: the lockfile diff is one
package.** `sha2` 0.11.0 is already here from the same RustCrypto release train,
so `digest`, `block-buffer`, `crypto-common`, `cpufeatures`, `typenum` and
`hybrid-array` were all present and no new maintainer joined the trust set. The
rejected alternative was implementing SHA-1 in this repository: new unaudited
cryptographic code in the component that decides whether two captures are of the
same torrent, to save one small crate. ⚠ The crate is checked against RFC 3174's
own vectors rather than trusted, in
[`../crates/bit-ids-lab/src/torrent.rs`](../crates/bit-ids-lab/src/torrent.rs).

`OBS-09` added `serde_json` 1.0.151 to `bit-ids-lab` as a **dev**-dependency, and
no new package: `bit-ids` already depends on it, so the lockfile diff is one line
naming the existing crate against another member. ⭐ **It is a dev-dependency on
purpose.** The acceptance suite needs a JSON reader to splice the bundle's rows
into the golden manifest and profile; the crate itself does not, because a
transcript is serialised by hand. The field order, the hex case and the exact
spacing are the bytes a digest names, and a derive that reordered a field on a
dependency bump would change every artifact's digest with nothing saying why.
The rejected alternative was serialising through the derive and accepting that
the on-disk form of published evidence is a dependency's choice.

⚠ **`check-no-secrets --public` refuses a bare run of long hex, and a published
test vector is one.** Forty lowercase hex digits is exactly what a token looks
like, so the rule is right to fire. It was narrowed rather than switched off, per
[`security/secrets.md`](security/secrets.md): the exclusion is anchored to a
constant named for its RFC and to exactly forty digits, and it was proven with a
credential beside an allowed vector on one line, the same value under another
name, and a 64-digit value under the allowed name, all still reported.

## Actions

A pin is a 40-character commit followed by a comment naming the tag it was:

```text
uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
```

⛔ **Both halves are required.** `check-project` refuses a tag, a branch, an
abbreviated commit and a bare commit with no comment. The comment is not
decoration: [`../scripts/common/check-remote-items.sh`](../scripts/common/check-remote-items.sh)
resolves it against the tag it names and refuses a pin whose comment has
drifted from the commit it labels, so a pin without a comment is a pin that
check can never examine.

⚠ **A rule written as a denylist of the floating forms somebody thought of is
not a rule.** This one named `main`, `master` and `vN.N.N`, and a branch called
anything else, or a seven-character commit, passed it. `FOUND-02` inverted it
to an allowlist of the forms that are immutable by construction, so a shape
nobody anticipated fails closed.

A local action, written `./path`, is this repository and is reviewed with
everything else. A container is pinned by digest rather than by tag.

## Updating

Dependabot proposes crate and action updates weekly, grouped, per
[`../.github/dependabot.yml`](../.github/dependabot.yml). A proposal is a
claim, and the procedure is what turns it into a reviewed change.

1. **Read what the item asserts, then check it.** Run
   `sh scripts/common/check-remote-items.sh`. For an action it verifies that
   the commit exists and belongs to the repository the ref names, that the tag
   in the comment really resolves to that commit, that the tag is a published
   release, that the runtime it declares is not deprecated, and whether a newer
   major already exists. ⚠ A bump that resolves cleanly can still be two majors
   behind; that has happened here.
2. **Regenerate the lockfile with the pinned toolchain**, never by hand.
   `cargo update -p NAME --precise VERSION` for one crate, `cargo update` for
   the group.
3. **Run the gate.** `sh scripts/common/check-gate.sh`. A crate that changes
   observed behaviour is a finding, not a formality.
4. **Write the changelog entry**, naming the record and saying whether it
   deployed, per [`conventions/docs.md`](conventions/docs.md).
5. **Merge is the operator's.** Nothing here merges, closes, comments on or
   approves an item. [`security/remote-ops.md`](security/remote-ops.md).

⛔ **Never take the item's word for the fact it asserts.** A bot's title says
what it believes it is doing, and by the hundredth bump nobody is looking. That
is the whole reason `check-remote-items` exists.

## What is not covered yet

- Artifact attestations, SBOMs and signed release metadata are `CI-04`. Nothing
  here binds a released asset to the workflow and commit that built it.
- Licence and redistribution disposition per dependency is `FOUND-04`.
- `check-remote-items` verifies pins a pull request proposes. It does not
  re-verify the pins already in the tree, so a tag deleted or moved upstream
  after a merge is not currently detected. `CI-04` owns closing that.
- ⚠ What `check-remote-items` verifies is described here from its own
  documentation. No run of it backs this page. It needs `gh` **authenticated**:
  installing the binary is not enough, and on 2026-09-04 a session host with
  `gh` 2.63.2 present had its environment token rejected by `gh auth status`,
  while the other GitHub route that host had was scoped to this repository
  alone and so could not resolve a pin in `actions/checkout`. The CI Linux lane
  runs it on every push.
- The `run:` scripts in a workflow are read by nobody. The pin test covers
  `uses:` lines; a tool fetched by a shell line is only as pinned as the fetch
  it uses, which is why the fourth layer above needed an argument rather than
  a rule.
