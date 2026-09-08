#!/bin/sh
# transmission.sh - the CLIENT-06 capture adapter.
#
# ⚠ THE ONE TARGET WHOSE ENTRY IS ABOUT NOT USING SOURCE. CLIENT-06 exists
# because published formulas for Transmission's peer ID are good hypotheses and
# violate the live-only evidence rule, so everything this adapter contributes is
# a running process and nothing it contributes is a formula.
#
# ⭐ ITS CONTROL SURFACE IS A SECOND PROCESS RATHER THAN A FILE. transmission-cli
# was removed upstream, so the supported unattended path is the daemon plus
# transmission-remote, and that is what this drives: the daemon is started with
# its whole settings file written, and the torrent is added by the tool the
# product ships for the purpose.
#
# -- ⛔ WHAT THIS ASSUMES ABOUT THE PRODUCT, AND HAS NOT MEASURED --------------
#
# Nothing below has been run against an installed Transmission. Each line is a
# claim a dispatch settles:
#
#   * `transmission-daemon --version` names the version in its second field,
#     with a build identifier beside it that is a separate fact;
#   * `transmission-remote` arrives with `transmission-cli` on this
#     distribution, and the daemon in `transmission-daemon`;
#   * the `dht-enabled`, `pex-enabled` and `lpd-enabled` settings are the three
#     adjacent surfaces, and `settings.json` under `--config-dir` is read at
#     start;
#   * `--foreground` keeps the daemon as the process this backgrounds;
#   * the distribution starts the daemon as a service on install, which is why
#     the package route stops and disables it.
#
# ⚠ The version fields are the claim most likely to be wrong, and it fails
# loudly: `capture-client` refuses a version that is not printable text and one
# that is empty, so a mis-parse is a refused capture rather than a record
# naming a build nobody ran.
#
# See adapters/README.md for the contract.
#
# Exit codes: 0 done, 1 refused, 2 could not run.

set -u

ME=transmission
RPC_PORT=${BIT_IDS_TRANSMISSION_RPC:-9091}

refuse() {
  printf '%s: %s\n' "$ME" "$1" >&2
  exit 1
}

cannot() {
  printf '%s: %s\n' "$ME" "$1" >&2
  exit 2
}

daemon() {
  if [ -n "${BIT_IDS_TRANSMISSION_DAEMON:-}" ]; then
    printf '%s' "$BIT_IDS_TRANSMISSION_DAEMON"
    return 0
  fi
  command -v transmission-daemon 2>/dev/null
}

remote() {
  if [ -n "${BIT_IDS_TRANSMISSION_REMOTE:-}" ]; then
    printf '%s' "$BIT_IDS_TRANSMISSION_REMOTE"
    return 0
  fi
  command -v transmission-remote 2>/dev/null
}

[ $# -ge 1 ] || cannot "a subcommand is required"
COMMAND="$1"
shift

case "$COMMAND" in
  describe)
    printf 'target=transmission\n'
    printf 'kind=stock\n'
    ;;

  install)
    [ $# -ge 2 ] || cannot "install takes <route> <workdir>"
    ROUTE="$1"
    WORKDIR="$2"
    mkdir -p "$WORKDIR" || cannot "cannot create $WORKDIR"
    case "$ROUTE" in
      package)
        command -v apt-get >/dev/null 2>&1 || cannot "apt-get is not on this host"
        # ⚠ THE INDEX IS REFRESHED FIRST, and this is the one network call the
        # package route makes that is not the install. A runner image ships a
        # package list that was current when the image was built, and an install
        # against a stale one fails with a 404 on a version that has moved rather
        # than with anything naming the cause.
        # ⛔ NEEDRESTART_MODE=l IS `list`, AND THE LETTER IS THE WHOLE POINT.
        # Ubuntu 24.04 ships needrestart, which after a package install opens an
        # interactive dialog listing the services to restart; DEBIAN_FRONTEND
        # does not suppress it, and a dialog on a runner is a step that never
        # returns. ⚠ `a` stops the dialog by RESTARTING those services instead,
        # which on a runner means restarting daemons the job is standing on:
        # measured on 2026-09-08, an install under `a` finished and the step
        # after it then hung. `l` reports and touches nothing.
        # ⚠ Every apt call reads /dev/null, so anything that still asks gets
        # end-of-file rather than a wait.
        DEBIAN_FRONTEND=noninteractive NEEDRESTART_MODE=l apt-get update \
          </dev/null >"$WORKDIR/update.log" 2>&1 || refuse "the package index could not be refreshed"
        # ⚠ BOTH HALVES, because the control surface is a separate binary. A
        # route that installed the daemon alone would leave `start` unable to
        # add a torrent and the failure would read as the build declining one.
        DEBIAN_FRONTEND=noninteractive NEEDRESTART_MODE=l apt-get install -y --no-install-recommends \
          transmission-daemon transmission-cli \
          </dev/null >"$WORKDIR/install.log" 2>&1 || refuse "the package route did not install transmission"
        # ⚠ The distribution starts the daemon as a service on install. A
        # capture must own the only running copy, or it would measure whichever
        # one the torrent reached.
        if command -v systemctl >/dev/null 2>&1; then
          systemctl stop transmission-daemon >>"$WORKDIR/install.log" 2>&1 || :
          systemctl disable transmission-daemon >>"$WORKDIR/install.log" 2>&1 || :
        fi
        ;;
      release)
        [ -n "${BIT_IDS_RELEASE_URL:-}" ] ||
          cannot "the release route needs BIT_IDS_RELEASE_URL, resolved before the route was cut"
        command -v curl >/dev/null 2>&1 || cannot "curl is not on this host"
        curl -fsSL --retry 2 -o "$WORKDIR/transmission-release" "$BIT_IDS_RELEASE_URL" \
          </dev/null >"$WORKDIR/install.log" 2>&1 || refuse "the release route could not be fetched"
        ;;
      *) cannot "unknown route: $ROUTE" ;;
    esac
    ;;

  version)
    BINARY=$(daemon) || cannot "transmission-daemon is not installed"
    [ -n "$BINARY" ] || cannot "transmission-daemon is not installed"
    # ⛔ THE BUILD SPEAKING. `transmission-daemon 4.0.5 (fac9a5f1c8)` is the
    # shape; the field taken is the version alone and the build hash beside it
    # is left where it is, because a version and a build are two facts.
    # ⛔ THE EXIT CODE IS READ FROM THE PROCESS THAT PRODUCED IT, UNPIPED. This
    # was `$(... | head -1) || cannot`, whose `||` reads HEAD's status: head
    # exits 0 over anything, so the guard was dead code and a build that refused
    # to answer reached the parse instead of the refusal. ⚠ Measured on
    # 2026-09-08, when a client capture reported "would not report a version" and
    # nothing said which of three refusals had fired. This repository's oldest
    # stated rule, broken in every adapter at once.
    # ⚠ Merged on purpose: this product prints its version to stderr, which
    # shell.md section 3 names as the case where merging is the decision.
    OUTPUT=$("$BINARY" --version 2>&1 </dev/null)
    VERSION_RC=$?
    [ "$VERSION_RC" = 0 ] ||
      cannot "transmission-daemon --version exited $VERSION_RC"
    LINE=$(printf '%s\n' "$OUTPUT" | head -1)
    VERSION=$(printf '%s' "$LINE" | awk '{ print $2 }')
    [ -n "$VERSION" ] || cannot "transmission-daemon reported no parseable version: [$LINE]"
    # ⛔ WHAT THE BUILD SAID ABOUT ITSELF IS KEPT, NOT ONLY THE FIELD PARSED OUT
    # OF IT. Two routes can install builds that report ONE version and are not
    # one build: measured on 2026-09-08, Ubuntu's aria2 1.37.0 and a 1.37.0 built
    # from the vendor's own release tarball answer the same version and enable
    # different features - the package build lists Async DNS, Metalink, XML-RPC,
    # SFTP and Firefox3 Cookie, and a source build configured without those
    # dependencies does not. ⚠ Features are what a build DOES on the wire, so a
    # record holding only `1.37.0` has dropped the evidence that would have
    # distinguished them.
    # ⭐ stderr, because both callers already keep it: install-client writes it to
    # `version.err` beside the install record and capture-client appends it to
    # `adapter.err` inside the evidence bundle. Nothing about the parsed value on
    # stdout changes.
    printf '%s reported:\n' "$ME" >&2
    printf '%s\n' "$OUTPUT" >&2
    printf '%s\n' "$VERSION"
    ;;

  start)
    [ $# -ge 3 ] || cannot "start takes <torrent> <workdir> <peer-port>"
    TORRENT="$1"
    WORKDIR="$2"
    PORT="$3"
    BINARY=$(daemon) || cannot "transmission-daemon is not installed"
    [ -n "$BINARY" ] || cannot "transmission-daemon is not installed"
    REMOTE=$(remote) || cannot "transmission-remote is not installed"
    [ -n "$REMOTE" ] || cannot "transmission-remote is not installed"
    [ -s "$TORRENT" ] || cannot "there is no torrent at $TORRENT"

    CONFIG="$WORKDIR/config"
    mkdir -p "$CONFIG" "$WORKDIR/downloads" || cannot "cannot create the config under $WORKDIR"

    # ⛔ THE WHOLE SETTINGS FILE IS WRITTEN. The three adjacent surfaces are
    # named individually, and port forwarding is off as well: a host with no
    # default route cannot reach a NAT gateway, and a build asking one is a
    # destination this run did not intend.
    {
      printf '{\n'
      printf '  "dht-enabled": false,\n'
      printf '  "pex-enabled": false,\n'
      printf '  "lpd-enabled": false,\n'
      printf '  "utp-enabled": false,\n'
      printf '  "port-forwarding-enabled": false,\n'
      printf '  "encryption": 0,\n'
      printf '  "download-dir": "%s/downloads",\n' "$WORKDIR"
      printf '  "incomplete-dir-enabled": false,\n'
      printf '  "rpc-enabled": true,\n'
      printf '  "rpc-bind-address": "127.0.0.1",\n'
      printf '  "rpc-port": %s,\n' "$RPC_PORT"
      printf '  "rpc-authentication-required": false,\n'
      printf '  "rpc-whitelist": "127.0.0.1",\n'
      printf '  "rpc-host-whitelist-enabled": false,\n'
      printf '  "ratio-limit-enabled": false,\n'
      [ "$PORT" = "0" ] || printf '  "peer-port": %s,\n' "$PORT"
      printf '  "peer-port-random-on-start": false\n'
      printf '}\n'
    } >"$CONFIG/settings.json" || cannot "the settings could not be written"

    # ⚠ --foreground, so the process this backgrounds is the daemon itself. A
    # daemon that forked would leave `stop` holding the pid of a shell that had
    # already returned, and the build would outlive the capture.
    "$BINARY" --foreground --config-dir "$CONFIG" --log-error \
      >"$WORKDIR/client.out" 2>&1 &
    printf '%s\n' "$!" >"$WORKDIR/pid"

    # ⚠ The daemon is asked for its session before a torrent is handed over,
    # because transmission-remote against a socket nothing is listening on
    # reports a connection error, which would read as the build refusing the
    # torrent rather than as a daemon that had not finished starting.
    WAITED=0
    while [ "$WAITED" -lt 20 ]; do
      "$REMOTE" "127.0.0.1:$RPC_PORT" --session-info >/dev/null 2>&1 && break
      kill -0 "$(cat "$WORKDIR/pid")" 2>/dev/null || refuse "the daemon exited before it answered"
      sleep 1
      WAITED=$((WAITED + 1))
    done
    [ "$WAITED" -lt 20 ] || refuse "the daemon never answered on 127.0.0.1:$RPC_PORT"

    "$REMOTE" "127.0.0.1:$RPC_PORT" --add "$TORRENT" >>"$WORKDIR/client.out" 2>&1 ||
      refuse "transmission-remote would not add the torrent"
    "$REMOTE" "127.0.0.1:$RPC_PORT" --torrent all --start >>"$WORKDIR/client.out" 2>&1 ||
      refuse "transmission-remote would not start the torrent"
    printf 'started transmission-daemon pid %s on port %s\n' "$(cat "$WORKDIR/pid")" "$PORT"
    ;;

  stop)
    [ $# -ge 1 ] || cannot "stop takes <workdir>"
    WORKDIR="$1"
    [ -f "$WORKDIR/pid" ] || exit 0
    PID=$(cat "$WORKDIR/pid")
    case "$PID" in
      '' | *[!0-9]*) exit 0 ;;
    esac
    kill "$PID" 2>/dev/null || exit 0
    WAITED=0
    while [ "$WAITED" -lt 8 ]; do
      kill -0 "$PID" 2>/dev/null || exit 0
      sleep 1
      WAITED=$((WAITED + 1))
    done
    kill -9 "$PID" 2>/dev/null
    exit 0
    ;;

  *) cannot "unknown subcommand: $COMMAND" ;;
esac
