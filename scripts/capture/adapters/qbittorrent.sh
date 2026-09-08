#!/bin/sh
# qbittorrent.sh - the CLIENT-01 capture adapter.
#
# ⚠ THE HIGHEST-PRIORITY TARGET AND NOT THE SIMPLEST ONE. qbittorrent-nox keeps
# a profile directory, asks once for a legal notice to be accepted, and reads
# its switches out of an INI file rather than off its command line. Each of
# those is a place a run can differ from the previous one, which is what a
# disposable host exists to remove, so this adapter writes the whole profile
# itself instead of inheriting whatever is in HOME.
#
# -- ⛔ WHAT THIS ASSUMES ABOUT THE PRODUCT, AND HAS NOT MEASURED --------------
#
# Nothing below has been run against an installed qBittorrent. Each line is a
# claim a dispatch settles, and this adapter carries more of them than the
# others because this product has the most configuration:
#
#   * `qbittorrent-nox --version` ends its first line with the version, with a
#     leading `v` this strips;
#   * `--profile` roots the whole configuration, so a written profile is the one
#     that is read;
#   * `--confirm-legal-notice` is what stops a fresh profile blocking on a
#     prompt;
#   * the `Session\DHTEnabled`, `Session\PeXEnabled` and `Session\LSDEnabled`
#     keys under `[BitTorrent]` are the three adjacent surfaces;
#   * a torrent as a positional argument is added and started.
#
# ⚠ The prompt claim is the one worth watching. If it is wrong the capture does
# not lie, it stalls: the observer serves its whole deadline, nothing announces,
# and `capture-client` refuses over a build that put no bytes on the wire.
#
# See adapters/README.md for the contract.
#
# Exit codes: 0 done, 1 refused, 2 could not run.

set -u

ME=qbittorrent

refuse() {
  printf '%s: %s\n' "$ME" "$1" >&2
  exit 1
}

cannot() {
  printf '%s: %s\n' "$ME" "$1" >&2
  exit 2
}

binary() {
  if [ -n "${BIT_IDS_QBITTORRENT:-}" ]; then
    printf '%s' "$BIT_IDS_QBITTORRENT"
    return 0
  fi
  command -v qbittorrent-nox 2>/dev/null
}

[ $# -ge 1 ] || cannot "a subcommand is required"
COMMAND="$1"
shift

case "$COMMAND" in
  describe)
    printf 'target=qbittorrent\n'
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
        # ⛔ NEEDRESTART_MODE=a IS NOT TIDINESS. Ubuntu 24.04 ships needrestart,
        # which opens an interactive dialog after a package install listing the
        # services to restart. DEBIAN_FRONTEND does not suppress it, and a
        # dialog on a runner is a step that never returns.
        # ⚠ Every apt call reads /dev/null, so anything that still asks gets
        # end-of-file rather than a wait.
        DEBIAN_FRONTEND=noninteractive NEEDRESTART_MODE=a apt-get update \
          </dev/null >"$WORKDIR/update.log" 2>&1 || refuse "the package index could not be refreshed"
        DEBIAN_FRONTEND=noninteractive NEEDRESTART_MODE=a apt-get install -y --no-install-recommends qbittorrent-nox \
          </dev/null >"$WORKDIR/install.log" 2>&1 || refuse "the package route did not install qbittorrent-nox"
        ;;
      release)
        # ⛔ THE VENDOR'S OWN ARTIFACT. CLIENT-01 measured on 2026-09-05 that the
        # release carries a Linux AppImage, a Windows setup executable and a
        # source archive, each with a detached signature beside it. The URL is
        # resolved before the route is cut and handed in; resolving it here
        # would be a network call under containment.
        [ -n "${BIT_IDS_RELEASE_URL:-}" ] ||
          cannot "the release route needs BIT_IDS_RELEASE_URL, resolved before the route was cut"
        command -v curl >/dev/null 2>&1 || cannot "curl is not on this host"
        curl -fsSL --retry 2 -o "$WORKDIR/qbittorrent-nox.AppImage" "$BIT_IDS_RELEASE_URL" \
          </dev/null >"$WORKDIR/install.log" 2>&1 || refuse "the release route could not be fetched"
        chmod +x "$WORKDIR/qbittorrent-nox.AppImage" ||
          refuse "the fetched AppImage could not be made executable"
        ;;
      *) cannot "unknown route: $ROUTE" ;;
    esac
    ;;

  version)
    BINARY=$(binary) || cannot "qbittorrent-nox is not installed"
    [ -n "$BINARY" ] || cannot "qbittorrent-nox is not installed"
    # ⛔ THE BUILD SPEAKING. `qBittorrent v5.0.2` on its first line; the leading
    # `v` is stripped because a version is compared by VersionScheme::components
    # and a label carrying a sigil is a second spelling of one value.
    # ⛔ --confirm-legal-notice IS ON THE VERSION CALL TOO, AND THAT IS WHAT THE
    # FIRST DISPATCH BOUGHT. A fresh machine has no accepted notice, so asking
    # this product its version can be the thing that opens the prompt; the
    # `start` call carried the flag and this one did not, which is a control
    # applied to one of two paths into the same product.
    # ⚠ Stdin is /dev/null as well, because a flag that stops one prompt is not
    # a flag that stops every prompt.
    # ⛔ THE EXIT CODE IS READ FROM THE PROCESS THAT PRODUCED IT, UNPIPED. This
    # was `$(... | head -1) || cannot`, whose `||` reads HEAD's status: head
    # exits 0 over anything, so the guard was dead code and a build that refused
    # to answer reached the parse instead of the refusal. ⚠ Measured on
    # 2026-09-08, when a client capture reported "would not report a version" and
    # nothing said which of three refusals had fired. This repository's oldest
    # stated rule, broken in every adapter at once.
    # ⚠ stderr is KEPT here rather than discarded. A product that refuses to
    # answer says why on it, and the caller prints what this prints.
    OUTPUT=$("$BINARY" --confirm-legal-notice --version </dev/null 2>&1)
    VERSION_RC=$?
    [ "$VERSION_RC" = 0 ] ||
      cannot "qbittorrent-nox --version exited $VERSION_RC: $(printf '%s' "$OUTPUT" | head -3 | tr '\n' ' ')"
    LINE=$(printf '%s\n' "$OUTPUT" | grep -i qbittorrent | head -1)
    [ -n "$LINE" ] || LINE=$(printf '%s\n' "$OUTPUT" | head -1)
    VERSION=$(printf '%s' "$LINE" | awk '{ print $NF }' | sed 's/^v//')
    [ -n "$VERSION" ] || cannot "qbittorrent-nox reported no parseable version: [$LINE]"
    printf '%s\n' "$VERSION"
    ;;

  start)
    [ $# -ge 3 ] || cannot "start takes <torrent> <workdir> <peer-port>"
    TORRENT="$1"
    WORKDIR="$2"
    PORT="$3"
    BINARY=$(binary) || cannot "qbittorrent-nox is not installed"
    [ -n "$BINARY" ] || cannot "qbittorrent-nox is not installed"
    [ -s "$TORRENT" ] || cannot "there is no torrent at $TORRENT"

    PROFILE="$WORKDIR/profile"
    CONFIG="$PROFILE/qBittorrent/config"
    mkdir -p "$CONFIG" "$WORKDIR/downloads" || cannot "cannot create the profile under $WORKDIR"

    # ⛔ THE WHOLE PROFILE IS WRITTEN, NOT PATCHED. A capture that edited an
    # existing configuration would measure a build under whatever else was in
    # it, and the settings a run used are part of what that run recorded. ⚠ The
    # three adjacent surfaces are named individually: DHT, peer exchange and
    # local peer discovery are three destinations, and a host with no default
    # route still multicasts.
    {
      printf '[BitTorrent]\n'
      printf 'Session\\DHTEnabled=false\n'
      printf 'Session\\PeXEnabled=false\n'
      printf 'Session\\LSDEnabled=false\n'
      printf 'Session\\AnonymousModeEnabled=false\n'
      printf 'Session\\Encryption=0\n'
      printf 'Session\\DefaultSavePath=%s/downloads\n' "$WORKDIR"
      printf 'Session\\TempPathEnabled=false\n'
      printf 'Session\\AddTorrentStopped=false\n'
      printf 'Session\\GlobalMaxSeedingMinutes=1\n'
      [ "$PORT" = "0" ] || printf 'Session\\Port=%s\n' "$PORT"
      printf '\n'
      printf '[Preferences]\n'
      printf 'Connection\\UPnP=false\n'
      printf 'Downloads\\SavePath=%s/downloads\n' "$WORKDIR"
      printf 'General\\Locale=en\n'
      printf 'WebUI\\Enabled=false\n'
    } >"$CONFIG/qBittorrent.conf" || cannot "the profile could not be written"

    # ⚠ --confirm-legal-notice IS NOT OPTIONAL. Without it the first run of a
    # fresh profile blocks on a prompt, and a capture would sit there until the
    # observer's deadline with nothing on the wire and no error to read.
    #
    # ⛔ AND THE TORRENT IS A POSITIONAL ARGUMENT, which is the one control this
    # product already has that needs no web interface, no credential and no
    # second process. An adapter driving the WebUI would be measuring a build
    # through an authenticated API it had to configure first.
    HOME="$WORKDIR" "$BINARY" \
      --profile="$PROFILE" \
      --confirm-legal-notice \
      --relative-fastresume \
      "$TORRENT" >"$WORKDIR/client.out" 2>&1 &
    printf '%s\n' "$!" >"$WORKDIR/pid"
    printf 'started qbittorrent-nox pid %s on port %s\n' "$(cat "$WORKDIR/pid")" "$PORT"
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
