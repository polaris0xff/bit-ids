#!/bin/sh
# aria2.sh - the CLIENT-05 capture adapter.
#
# ⭐ THE SIMPLEST TARGET IN THE MATRIX, AND THAT IS WHY IT IS HERE FIRST AMONG
# EQUALS. aria2c takes every setting on its command line, needs no daemon, no
# profile directory and no interactive acceptance, and prints its version to
# stdout. Nothing about a run of it depends on state a previous run left behind,
# which is the property a capture host is built to guarantee and an adapter
# should not also have to.
#
# ⚠ CLIENT-05 ALSO WANTS aria2 AS A CONNECTOR, AND THIS IS NOT THAT. Driving
# aria2 as a target and using it to corroborate another target are the two roles
# the entry says must not create circular corroboration. This file is the target
# half alone; nothing here observes anything.
#
# -- ⛔ WHAT THIS ASSUMES ABOUT THE PRODUCT, AND HAS NOT MEASURED --------------
#
# Nothing below has been run against an installed aria2. A session host is not
# disposable, so this file was written from the product's documented interface
# and driven only through a stub. Each line is a claim a dispatch settles:
#
#   * `aria2c --version` opens with `aria2 version <v>`, so the third field is
#     the version;
#   * `--enable-dht`, `--enable-dht6`, `--bt-enable-lpd` and
#     `--enable-peer-exchange` are the switches for the three adjacent surfaces;
#   * `--listen-port` refuses zero, which is why an unset peer port drops it;
#   * a torrent as a positional argument makes it announce to the tracker named
#     inside, needing no other control.
#
# ⚠ A claim here that turns out to be wrong shows up as a capture that refuses
# rather than as one that lies: `capture-client` will not attest to a build it
# did not observe announce.
#
# See adapters/README.md for the contract.
#
# Exit codes: 0 done, 1 refused, 2 could not run.

set -u

ME=aria2

refuse() {
  printf '%s: %s\n' "$ME" "$1" >&2
  exit 1
}

cannot() {
  printf '%s: %s\n' "$ME" "$1" >&2
  exit 2
}

# ⛔ THE INSTALLED EXECUTABLE, LOOKED UP ONCE. A path composed per subcommand is
# a second spelling, and the day a route installs somewhere else one subcommand
# finds the build and another reports it absent.
binary() {
  if [ -n "${BIT_IDS_ARIA2:-}" ]; then
    printf '%s' "$BIT_IDS_ARIA2"
    return 0
  fi
  command -v aria2c 2>/dev/null
}

[ $# -ge 1 ] || cannot "a subcommand is required"
COMMAND="$1"
shift

case "$COMMAND" in
  describe)
    printf 'target=aria2\n'
    printf 'kind=stock\n'
    ;;

  install)
    [ $# -ge 2 ] || cannot "install takes <route> <workdir>"
    ROUTE="$1"
    WORKDIR="$2"
    mkdir -p "$WORKDIR" || cannot "cannot create $WORKDIR"
    case "$ROUTE" in
      package)
        # ⚠ The distribution's own index. Its version trails upstream, and that
        # is a packaging observation rather than a defect: ACQ-03 compares what
        # the two installed builds REPORT, and a divergence is recorded.
        command -v apt-get >/dev/null 2>&1 || cannot "apt-get is not on this host"
        # ⚠ THE INDEX IS REFRESHED FIRST, and this is the one network call the
        # package route makes that is not the install. A runner image ships a
        # package list that was current when the image was built, and an install
        # against a stale one fails with a 404 on a version that has moved rather
        # than with anything naming the cause.
        DEBIAN_FRONTEND=noninteractive apt-get update \
          >"$WORKDIR/update.log" 2>&1 || refuse "the package index could not be refreshed"
        DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends aria2 \
          >"$WORKDIR/install.log" 2>&1 || refuse "the package route did not install aria2"
        ;;
      release)
        # ⛔ THE VENDOR'S OWN PUBLISHED ARTIFACT, AND NOT A SECOND PACKAGE INDEX.
        # E-ACQ-07 and E-ACQ-08 refuse two routes sharing a resolver or a
        # delivery mechanism, so a "second route" that asked another mirror of
        # the same index would be one route under two names.
        [ -n "${BIT_IDS_RELEASE_URL:-}" ] ||
          cannot "the release route needs BIT_IDS_RELEASE_URL, resolved before the route was cut"
        command -v curl >/dev/null 2>&1 || cannot "curl is not on this host"
        curl -fsSL --retry 2 -o "$WORKDIR/aria2.tar.bz2" "$BIT_IDS_RELEASE_URL" \
          >"$WORKDIR/install.log" 2>&1 || refuse "the release route could not be fetched"
        ;;
      *) cannot "unknown route: $ROUTE" ;;
    esac
    ;;

  version)
    BINARY=$(binary) || cannot "aria2c is not installed"
    [ -n "$BINARY" ] || cannot "aria2c is not installed"
    # ⛔ THE BUILD SPEAKING. `aria2 version 1.37.0` on its first line; the field
    # taken is the version alone, and an empty answer is could-not-run rather
    # than an empty string somebody downstream renders as unknown.
    LINE=$("$BINARY" --version 2>/dev/null | head -1) ||
      cannot "aria2c would not report its version"
    VERSION=$(printf '%s' "$LINE" | awk '{ print $3 }')
    [ -n "$VERSION" ] || cannot "aria2c reported no parseable version: $LINE"
    printf '%s\n' "$VERSION"
    ;;

  start)
    [ $# -ge 3 ] || cannot "start takes <torrent> <workdir> <peer-port>"
    TORRENT="$1"
    WORKDIR="$2"
    PORT="$3"
    BINARY=$(binary) || cannot "aria2c is not installed"
    [ -n "$BINARY" ] || cannot "aria2c is not installed"
    [ -s "$TORRENT" ] || cannot "there is no torrent at $TORRENT"
    mkdir -p "$WORKDIR/downloads" || cannot "cannot create $WORKDIR/downloads"

    # ⛔ ALL THREE ADJACENT SURFACES OFF, EXPLICITLY. A host with no default
    # route cannot reach a DHT bootstrap node, and it can still multicast a
    # local discovery announce and gossip over peer exchange. Turning off one
    # and inheriting the others is the one-gated-door shape.
    #
    # ⚠ --seed-time=0 and a stop timeout keep the process from outliving the
    # observer's own deadline: this returns while the build runs, and `stop`
    # below is what ends it, but a build that ignored both would hold the port
    # after the evidence was written.
    set -- \
      --enable-dht=false \
      --enable-dht6=false \
      --bt-enable-lpd=false \
      --enable-peer-exchange=false \
      --bt-require-crypto=false \
      --dir="$WORKDIR/downloads" \
      --seed-time=0 \
      --allow-overwrite=true \
      --console-log-level=info \
      --summary-interval=0

    # ⚠ ZERO IS NOT A LISTEN PORT aria2 ACCEPTS, so an unset peer port drops the
    # flag rather than passing a value the build would refuse. It is appended
    # here rather than written into the list above, because a list built twice
    # to vary one element is two lists that drift.
    [ "$PORT" = "0" ] || set -- "$@" --listen-port="$PORT"

    "$BINARY" "$@" "$TORRENT" >"$WORKDIR/client.out" 2>&1 &
    printf '%s\n' "$!" >"$WORKDIR/pid"
    printf 'started aria2c pid %s on port %s\n' "$(cat "$WORKDIR/pid")" "$PORT"
    ;;

  stop)
    [ $# -ge 1 ] || cannot "stop takes <workdir>"
    WORKDIR="$1"
    # ⚠ A stop over a build that already exited is done, not refused. aria2c
    # ends itself when its seed time expires, and a capture whose client
    # finished early is a capture, not a failure.
    [ -f "$WORKDIR/pid" ] || exit 0
    PID=$(cat "$WORKDIR/pid")
    case "$PID" in
      '' | *[!0-9]*) exit 0 ;;
    esac
    kill "$PID" 2>/dev/null || exit 0
    # ⚠ Given a moment, then killed. A client asked to stop writes its state and
    # closes its sockets; one that ignores the ask holds the port open into the
    # upload step.
    WAITED=0
    while [ "$WAITED" -lt 5 ]; do
      kill -0 "$PID" 2>/dev/null || exit 0
      sleep 1
      WAITED=$((WAITED + 1))
    done
    kill -9 "$PID" 2>/dev/null
    exit 0
    ;;

  *) cannot "unknown subcommand: $COMMAND" ;;
esac
