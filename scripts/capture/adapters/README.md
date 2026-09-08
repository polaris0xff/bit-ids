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
| `describe` | any time | print `target=<slug>` and `kind=stock\|stub`, one per line |
| `install <route> <workdir>` | ⛔ **before the route is cut** | install the target through that route alone |
| `version` | under containment | ⛔ ask the **installed executable** and print exactly what it answered |
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
