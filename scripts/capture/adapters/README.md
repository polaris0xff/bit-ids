# Capture adapters

One file per target, and the only thing in this repository that knows how a
particular product is installed, asked its version, started and stopped.

⛔ **An adapter is the seam and never the boundary.** The guards that decide
whether a capture may happen at all live in
[`../../acquisition/assert-disposable.sh`](../../acquisition/assert-disposable.sh)
and are run by
[`../capture-client.sh`](../capture-client.sh) before an adapter is asked to
start anything. An adapter that tried to check the host would be a second
answer to a question one place already answers.

## The contract

Five subcommands. Each reads its arguments positionally, prints to stdout, and
returns 0 for done, 1 for refused and 2 for could-not-run.

| subcommand | when | must |
| --- | --- | --- |
| `describe` | any time | print `target=<slug>` and `kind=stock\|stub`, one per line, plus `binary=<path>` when one is installed |
| `install <route> <workdir>` | ⛔ **before the route is cut** | install the target through that route alone |
| `version` | ⛔ **before the route runs, again after it, and again under containment** | ⛔ ask the **installed executable** and print exactly what it answered |
| `start <torrent> <workdir> <peer-port>` | under containment | launch the build on that torrent and return; the build keeps running |
| `stop <workdir>` | under containment | stop whatever `start` launched |

⛔ **`kind` is a declaration the attestation copies rather than assumes.** A
`stub` adapter drives every guard `capture-client` has and is not a product, so
a run it drove writes `stock_client=false`. A field that said otherwise would be
the one sentence in the record nothing backs.

⛔ **`version` asks the build.** A version read from a filename, a package index
or the route that installed it is not the build speaking, which is the rule
`E-ACQ-10` states for a record and `capture-client` restates for a run. An
adapter that cannot ask exits 2, and the capture is *could not run* rather than
a fixture.

⛔ **`version` prints the parsed field on stdout and the build's WHOLE answer on
stderr.** Two routes can install builds that report one version and are not one
build: measured on 2026-09-08, Ubuntu's aria2 1.37.0 and a 1.37.0 compiled from
the vendor's own release tarball answer the same version and enable different
features. ⚠ Features are what a build does on the wire, so a record holding only
the version has dropped the evidence that would have separated them. ⭐ stderr
costs nothing to keep: `install-client` writes it to `version.err` beside the
install record and `capture-client` appends it to `adapter.err` inside the
evidence bundle.

⛔ **So `version` is a pure read and is called when the target may not be there
at all.** `install-client` asks it before the route runs, which is how a route
that installed nothing is detected, and a target that is absent then is the
ordinary case rather than an error: exit 2, and the caller records that the host
had nothing. ⚠ An adapter whose `version` started a daemon, wrote a profile or
assumed containment would be changing the host at the one moment the caller is
trying to observe it unchanged. All three shipped adapters run
`<binary> --version` and nothing else.

⛔ **`install` is never called under containment.** By the time a capture runs
there is no route off the host, so an install that needed one would have to
restore egress on the machine that exists to have none.
[`../../acquisition/install-client.sh`](../../acquisition/install-client.sh) is
the caller, and it runs in an earlier workflow step.

## Every call is bounded, and the bound is the caller's

⛔ **An adapter shells out to a product this project did not write, and a product
that waits on a question waits forever.** Measured on 2026-09-08, client capture
run 1: two of its three jobs sat in the install step for over half an hour and
reported nothing at all, because a hung install is indistinguishable from a slow
one until the job's own timeout kills the runner and takes the log with it.

⭐ **So the callers bound every subcommand rather than each adapter bounding
itself.** A convention in each adapter is a bound the next adapter forgets; the
callers are the one place every adapter passes through. `124` is coreutils'
verdict for *it never answered*, and it is reported as its own refusal because
the fix differs from a route that said no.

⛔ **And stdin is `/dev/null` on every call.** A product that asks a question
gets end-of-file rather than a wait. ⚠ That is a second control and not the same
one: it catches a prompt, the time limit catches everything else, and
[`../check-capture-client.sh`](../check-capture-client.sh) proves each with its
own case because a stub blocking on `read` would demonstrate only the redirect.

⚠ **An adapter still owns the switches its own product needs.** Ubuntu's
`needrestart` opens a dialog that `DEBIAN_FRONTEND` does not suppress, and
`qbittorrent-nox` asks for its legal notice to be confirmed - on `--version` as
well as on a run, which is a control on one of two paths into the same product.

## What an adapter must switch off

⛔ **Public peer discovery, on every route into it.** A capture host has no
default route, so a DHT bootstrap or a tracker on the internet fails rather than
leaks - but a *link-local* announce does not, and neither does anything the host
can still reach. Every adapter disables DHT, peer exchange and local peer
discovery explicitly, and the settings it wrote are part of what the run
records.

⚠ **Three switches, because they are three surfaces.** `OBS-06` calls them
adjacent for exactly this reason: each carries identity and each names a
destination of its own. An adapter that turned off DHT alone would leave the
other two on, which is the one-gated-door shape
[`../../../docs/methodology/reviews.md`](../../../docs/methodology/reviews.md)
calls the most recurring hole there is.

## The routes

⛔ **Two independent routes per target, and `E-ACQ-07`/`E-ACQ-08` are what
refuse a pair that shares a resolver or a delivery mechanism.** `package` is the
host's own package manager and `release` is the vendor's published artifact.
Two package aliases pointing at one index are one route.

⚠ **A route that installs is not yet a route that agreed.** `ACQ-03`'s
same-version gate compares what each installed build *reports*, which is why
`version` asks the executable rather than the installer.

⛔ **And a route that RAN is not yet a route that INSTALLED.** `install-client`
asks `version` and `describe` once before the route runs and once after, and
records `preexisting_version`, both executables, both digests and `acquired`
beside the version.

⛔ **A version is not an identity, which is why `describe` names the
executable.** `aria2` ships on `ubuntu-24.04` at the same version the vendor
publishes, so a release route there installs a genuinely different build - a
2.8-megabyte self-contained program in place of a 14-kilobyte shim over
`libaria2.so.0`, with a different feature list - and both answer `1.37.0`. ⚠ A
verdict on version strings alone calls that no acquisition at all, which is
measured rather than argued: it is what the first two-route capture would have
recorded. The digest is what makes that branch reachable, and it has its own
harness case.

⚠ **An adapter that names no executable is not refused.** The key is omitted when
nothing is installed, the two digest fields come back empty, and the verdict
falls back to the version - a lower bound rather than a wrong answer.

⚠ **Both halves of that are measured rather than imagined.** `aria2` ships on
the `ubuntu-24.04` image, so `apt-get install aria2` there prints `already the
newest version`, installs nothing and exits 0; client capture runs 3 and 4 wrote
`route=package` over exactly that. And every `release` route here fetches its
artifact into the workdir without making it the executable `binary()` finds, so a
release route run after a package route reports the **package** build's version
as its own. ⛔ Two routes in that state declare two independent resolvers, agree
on the version because it is one binary, and reach `ACQ-03` as `byte_identical` -
the strongest verdict the classification has, over an acquisition that never
happened.

⚠ **`acquired=no` is recorded and not refused.** The build on such a host is
real and its identity is worth capturing; what is not real is the claim that this
route acquired it. The refusal belongs to whatever compares two routes, which is
the same reason `install-client` runs one route per call.
