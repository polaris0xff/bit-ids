# Session record, 2026-09-09: the second route, and a lane that was one step

A saved summary of one session's measurements. ⚠ Not a diary and not current
truth: [`../../TODO/PROGRESS.md`](../../TODO/PROGRESS.md) is the state and each
entry carries its own evidence.

## What it set out to do, and what it found instead

The work order's next step was wiring the second acquisition route into
`capture-client.yml`. That was done. Three things found on the way were worth
more than the wiring:

1. ⛔ **A single-route capture cannot become a record at all**, and the work order
   said it could.
2. ⛔ **The aria2 hang is neither the package nor apt nor the upload step**, and
   two named causes were refuted.
3. ⛔ **CI was thirty minutes and one step was twenty-six of them.**

## 1. The artifact nothing chose

⛔ **Every `release` route in the tree refused without a URL "resolved before the
route was cut", and nothing in the repository resolved one.** `resolve-stable`
orders versions and selects no asset, so four dispatches passed `--route package`
and only that. The second route existed as three adapters that could not be
called.

⭐ `AssetPattern` and `select_asset` choose one artifact of the selected release,
out of **the same response** the version came from, so one recorded digest covers
both decisions. The location is read from the listing and never composed.

⭐ **`{version}` in a pattern is what makes it unambiguous on a real vendor's
release**, measured against the live listings:

| target | assets in the newest stable release | a `*` pattern matches | the pinned pattern |
| --- | ---: | ---: | ---: |
| `aria2` 1.37.0 | 6 | 3 | 1 |
| `qbittorrent` 5.2.3 | 14 | 2 | 1 |
| `transmission` 4.1.3 | 11 | 1 | 1 |

⛔ **Two matches refuses rather than taking the first.** Choosing by the order a
source listed its assets is choosing by a property of the source.

⛔ **`CLIENT-01`'s own record was one build short.** It said qBittorrent's release
"offers a Linux AppImage, a Windows `x64_setup.exe` and a source `tar.xz`". It
carries fourteen assets and **two** Linux AppImages, differing only by an
`_lt20` - in exactly the place a route has to choose.

⭐ **Driven end to end, with a control this project did not write.** The URL the
aria2 selection produced was fetched and `sha256sum -c` verified it against the
digest `ACQ-03` recorded on 2026-09-08 from a different fetch in a different
session.

## 2. The record that cannot be written

⛔ **`docs/history/RESUME.md` carried "a single-route, single-connector capture
VALIDATES and refuses to publish", and `CI-09`'s dependency was written from it.**
Half of that is right:

| the capture that exists | what the library does | where it bites |
| --- | --- | --- |
| one connector | **validates**, then `E-PUB-02` refuses to publish | publishability |
| one route | `E-ACQ-01`: *two independent routes are required* | ⛔ **validity** |

⚠ Measured by stripping the golden fixture twice and running `validate-profile`
over each copy. ⭐ Nothing in the code was wrong and both facts were already in
the suite; a sentence in a handoff disagreed with them, and that sentence is what
an entry's dependency rested on.

## 3. Three dispatches, and what each refuted

⭐ **Run 5: a `release` route acquired a build on a capture host** - the first
time any route of that kind in this tree has installed anything on one. Resolve
step one second, source build 144 seconds. And two builds, one version, measured
on the host: the image's aria2 enables Async DNS, Firefox3 Cookie, Metalink,
XML-RPC and SFTP against GnuTLS, and the build the route compiled enables none of
them and speaks HTTPS through OpenSSL.

⛔ **Run 5 also refuted the recorded boundary.** The release route runs no package
operation of any kind and *Upload the install logs* hung identically, so the hang
is not the aria2 package and not apt. Its artifact was written **five seconds**
into a step that had not returned thirteen minutes later.

⛔ **Run 6 refuted the mitigation.** `timeout-minutes: 5` did not fire: the
package lane's step began at 03:50:11Z, wrote its 3247-byte artifact at
03:50:12Z, and was still running at 04:07:39Z. ⭐ That also closed the size
question - three kilobytes hangs exactly as a thirteen-times-larger artifact
does.

⭐ **Run 7 moved the boundary twice.** Two transmission lanes ran that step in
**one second** each, one after a green capture and one after an install that
refused by design, in the same run as two aria2 lanes that hung. And with the
step moved to the end of the job, the hang appeared in the step it used to
follow: run 6's aria2 package install took six seconds, run 7's ran ten minutes.

⛔ **So the subject is the boundary after the aria2 install, not any particular
action.** Two local reproductions came back negative and neither settles it: the
package route run directly on this host left 82 processes before and after, and
`aria2c --version` under the same bound left 78 before and after. ⚠ This host is
not the runner image and proved it in the same run - `aria2` was absent here.

⭐ **Run 7 also produced a complete verified capture**: transmission 4.0.5,
`acquired=yes`, two announces, a **third** distinct peer ID, and `sha256sum -c`
reporting `OK` for all three evidence files.

## 4. The lane that was one step

⚠ **Measured before anything was changed**, because "CI is slow" is not a place
to start optimising. Run 83: *Workflow acceptance* 25.9 minutes of a 30.5-minute
Linux lane, *Repository gate* 3.5, everything else under one, Windows 2.4. That
step runs the gate nine times, so the gate was measured next: 198 seconds for 29
checks, of which 183 were three.

| | run 83 | run 85 |
| --- | ---: | ---: |
| wall clock | 30.5 min | **21.3 min** |
| Linux gate | 30.5 min | **3.7 min** |
| Windows gate | 2.4 min | 2.6 min |

⛔ **The all-concurrent gate reached 73 seconds and was wrong.** `check-capture`
drives real sockets against a three-second deadline and, under twenty-seven other
checks, reported a refusal that arrived for a reason the case had not planted -
passing alone on the same tree minutes later.

⚠ **Raising the deadline was tried, measured and rejected**: 45 seconds at `3`
against 79 at `6`, and 48 at `5` against 168 at `20`. The shipped gate is
**118-125 seconds** with the two socket harnesses running after the batch.

## What each review pass found

**Door sweep.** Four readers of `describe`'s `key=value` format in three
spellings, one of which - `awk -F= '{ print $2 }'` - truncates at a second `=`.
Measured over `weird=a=b`, which it reads as `a`. No key an adapter prints today
carries one, which is exactly when a reader is easiest to get wrong. All four now
read the whole value. ⭐ It also established that no **existing** caller of
`replace_once` had been mis-planting: their literals carry no regex
metacharacter, so the defect below was latent for every shipped case and live
only for the one added here.

**Guard mutation.** Every new rule was planted against and seen to refuse:
`tree-unchanged` in both halves, the duplicate row name, both evidence-upload
rules plus a reader probe, the resolve-order rule, and - added by this pass,
because it was the one rule without its own refutation - the upload-after-restore
rule and its silence on a workflow that uploads no install logs.

**Claim audit.** Two numbers written into the record were checked against the
tree. "Nine gate runs inside `check-workflow`" is right. "Nine checks call
`cargo build --example`" was **wrong** - it is fourteen - and is now "most of
them", because a count in prose is a value in two places with nothing comparing
them.

## Two defects in the tools, found by using them

⛔ **`replace_once` counted with `grep -F` and edited with `sed`**, so its literal
and its pattern were different languages. Over `axb then a.b` the literal occurs
once, the count accepted, and sed replaced `axb`: a plant that applied where no
case named it. A literal carrying a `/` could not plant at all, which turned
`store_probe_guards`' no-op row into a pass over a sed that never parsed its own
expression. Both are probes now and both refuse the old implementation.

⛔ **A gate run left four untracked files in `scripts/capture/`**, because two
probe cases wrote beside the file they were handed and two callers hand that
function a tracked path. `tree-unchanged` is a row in both gate halves now, each
proved by planting its own defect. ⚠ It sees the run that makes the mess and not
the one after, which a fresh CI checkout makes moot and a developer's host does
not.

⛔ **Two checks joined by `&&` are one check.** `shellcheck file && shfmt -d file
>/dev/null && echo "LINT OK"` ran shfmt only when shellcheck said nothing, so an
info-level finding meant shfmt never ran and the absent "LINT OK" read as an
empty diff. `check-workflow` caught the formatting defect eleven minutes later.

⛔ **An in-place `sed` edits every line that matches.**
`sed -i 's/^SECS=[0-9]*/SECS=6/'` changed two lines rather than one, because
`[0-9]*` matches zero digits, so `SECS="$2"` inside an embedded stub became
`SECS=6"$2"`.

## What this session did not establish

⚠ **No record was written and nothing was published**, and the reason is now
sharper than "it has not been done": every capture this project has run is a
single route, and a single-route record is refused at the validity gate.

⚠ **The aria2 hang has a boundary and no cause.** Three readings have now been
named and all three were wrong.

⚠ **No Windows client capture exists.** The adapters are `sh` and
`capture-client.yml` is a Linux-only workflow.

⚠ **`check-workflow` is now the whole CI wall clock**, and sharding it across
runners is deliberately left as its own unit: a shard selector that silently
claims no case leaves a rule nobody runs while every lane stays green.
