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
#   * ⛔ **REFUTED on 2026-09-08.** `--confirm-legal-notice` was assumed to be
#     what stops a fresh profile blocking on a prompt. This build answers
#     `Bad command line: --confirm-legal-notice is an unknown command line
#     parameter`, so it was never a control; the acceptance is written into the
#     profile as `[LegalNotice] Accepted=true` instead, which is the next
#     assumption and is not measured either;
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

# ⛔ AN AppImage MOUNTS ITSELF WITH FUSE, AND A CAPTURE HOST NEED NOT HAVE IT.
# This makes the AppImage extract itself and run instead, which costs an
# extraction per call and works on a host with FUSE as well as one without. ⚠ It
# is exported here rather than at each call site so `version`, `start` and any
# later subcommand inherit one answer; a build that is not an AppImage ignores
# it. ⛔ Neither branch has been driven: nothing has installed through the
# release route.
export APPIMAGE_EXTRACT_AND_RUN=1

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
  # ⭐ THE RELEASE ROUTE'S OWN PREFIX, BEFORE THE PATH. Its default is
  # `/usr/local`, which already precedes `/usr` on this platform's PATH, so this
  # changes nothing for a normal install and is what stops the two routes racing
  # on PATH order - a property of the host rather than of the acquisition.
  if [ -n "${BIT_IDS_PREFIX:-}" ] && [ -x "$BIT_IDS_PREFIX/bin/qbittorrent-nox" ]; then
    printf '%s' "$BIT_IDS_PREFIX/bin/qbittorrent-nox"
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
    # ⭐ AND WHERE THE BUILD IS, WHEN THERE IS ONE. `install-client` asks twice -
    # once before the route runs and once after - so a route that replaced the
    # executable is visible as a changed path or a changed digest even when both
    # builds report the same version. ⛔ That case is real rather than
    # theoretical: `aria2` ships on `ubuntu-24.04` at the same version the vendor
    # publishes, so a release route there installs a genuinely different build
    # and a version comparison alone reports it as no acquisition at all.
    # ⚠ The key is OMITTED when nothing is installed, which is an answer rather
    # than an empty value: a caller that saw `binary=` could not tell a missing
    # build from an adapter that declines to say.
    _where=$(binary) || _where=""
    [ -z "$_where" ] || printf 'binary=%s\n' "$_where"
    # ⛔ WHAT THE RELEASE ROUTE NEEDS TO KNOW. `CLIENT-01` recorded on 2026-09-05
    # that this release "offers a Linux AppImage, a Windows x64_setup.exe and a
    # source tar.xz"; re-read on 2026-09-09, release-5.2.3 carries FOURTEEN
    # assets and TWO Linux AppImages - `qbittorrent-5.2.3_x86_64.AppImage` and
    # `qbittorrent-5.2.3_lt20_x86_64.AppImage` - so the summary was one build
    # short in exactly the place a route has to choose.
    #
    # ⭐ `{version}` IS WHAT SEPARATES THEM. `qbittorrent-*_x86_64.AppImage`
    # matches both, because a run swallows the `_lt20`; the pinned form matches
    # the unsuffixed one alone and `select-asset` refuses the ambiguity rather
    # than taking whichever the source listed first.
    #
    # ⚠ WHAT THIS DOES NOT ESTABLISH IS WHICH LIBTORRENT EACH APPIMAGE CARRIES.
    # The suffix plainly names a variant and nothing here has measured either, so
    # the pattern selects the vendor's unsuffixed build and the record names the
    # asset it took: a capture through this route measures whichever it
    # installed, and says which.
    #
    # ⚠ THREE OR FOUR COMPONENTS, from `ACQ-02`'s live dry run. The newest
    # release object on 2026-09-09 is `release-5.3.0beta1`, which the source
    # flags a prerelease and whose version text says so too, so the resolver
    # selects 5.2.3 - which is the pessimistic stability rule doing its job on a
    # live listing rather than on a fixture.
    printf 'release_repo=qbittorrent/qBittorrent\n'
    printf 'release_tag_prefix=release-\n'
    printf 'release_min_components=3\n'
    printf 'release_max_components=4\n'
    printf 'release_asset=qbittorrent-{version}_x86_64.AppImage\n'
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
        DEBIAN_FRONTEND=noninteractive NEEDRESTART_MODE=l apt-get install -y --no-install-recommends qbittorrent-nox \
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
        #
        # ⛔ AND IT INSTALLS WHAT IT FETCHED. This route used to leave the
        # AppImage in the workdir and return 0, which is a route reporting an
        # install it did not perform: `version` then answered from whatever was
        # already on PATH, so a second route would have measured the FIRST
        # route's build and the two would have agreed for the most trivial reason
        # available. Measured on 2026-09-08 and recorded in `ACQ-03`.
        #
        # ⭐ AN AppImage IS THE INSTALLED PROGRAM, so installing it is placing it
        # where `binary()` looks. There is nothing to unpack and no prefix to
        # configure, which is the one thing this route has that aria2's does not.
        #
        # ⚠ WHAT THIS ASSUMES AND HAS NOT MEASURED: that the AppImage can run on
        # the capture host. An AppImage mounts itself with FUSE, and a runner
        # without it needs `APPIMAGE_EXTRACT_AND_RUN=1`; that variable is set on
        # every call this adapter makes into the build, which costs an extraction
        # per call and works either way. ⛔ Neither branch has been driven: this
        # adapter has never installed through this route.
        PREFIX=${BIT_IDS_PREFIX:-/usr/local}
        curl -fsSL --retry 2 -o "$WORKDIR/qbittorrent-nox.AppImage" "$BIT_IDS_RELEASE_URL" \
          </dev/null >"$WORKDIR/install.log" 2>&1 || refuse "the release route could not be fetched"
        mkdir -p "$PREFIX/bin" || cannot "cannot create $PREFIX/bin"
        install -m755 "$WORKDIR/qbittorrent-nox.AppImage" "$PREFIX/bin/qbittorrent-nox" ||
          refuse "the fetched AppImage could not be installed into $PREFIX/bin"
        [ -x "$PREFIX/bin/qbittorrent-nox" ] ||
          refuse "the release route reported an install and left no qbittorrent-nox in $PREFIX/bin"
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
    # ⚠ THE FLAG THAT USED TO BE HERE WAS AN ASSUMPTION AND IT WAS WRONG ON BOTH
    # PATHS AT ONCE. Adding it to this call to match `start` looked like closing
    # a one-gated door; what it actually did was spread a refused argument to a
    # second place. ⭐ Reading the product's own message is what settled it.
    # ⛔ THE EXIT CODE IS READ FROM THE PROCESS THAT PRODUCED IT, UNPIPED. This
    # was `$(... | head -1) || cannot`, whose `||` reads HEAD's status: head
    # exits 0 over anything, so the guard was dead code and a build that refused
    # to answer reached the parse instead of the refusal. ⚠ Measured on
    # 2026-09-08, when a client capture reported "would not report a version" and
    # nothing said which of three refusals had fired. This repository's oldest
    # stated rule, broken in every adapter at once.
    # ⛔ NO --confirm-legal-notice HERE, AND THAT IS MEASURED RATHER THAN
    # ASSUMED. This build answers `Bad command line: --confirm-legal-notice is
    # an unknown command line parameter`, so the flag was not a control at all;
    # it was an argument the product refuses. Client capture run 3 said so in
    # those words. ⚠ Stdin is still /dev/null and the caller still bounds this,
    # because a prompt is a separate hazard from a rejected flag.
    #
    # ⚠ stderr is KEPT here rather than discarded. A product that refuses to
    # answer says why on it, and the caller prints what this prints.
    OUTPUT=$("$BINARY" --version </dev/null 2>&1)
    VERSION_RC=$?
    [ "$VERSION_RC" = 0 ] ||
      cannot "qbittorrent-nox --version exited $VERSION_RC: $(printf '%s' "$OUTPUT" | head -3 | tr '\n' ' ')"
    LINE=$(printf '%s\n' "$OUTPUT" | grep -i qbittorrent | head -1)
    [ -n "$LINE" ] || LINE=$(printf '%s\n' "$OUTPUT" | head -1)
    VERSION=$(printf '%s' "$LINE" | awk '{ print $NF }' | sed 's/^v//')
    [ -n "$VERSION" ] || cannot "qbittorrent-nox reported no parseable version: [$LINE]"
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
      printf '\n'
      printf '[LegalNotice]\n'
      printf 'Accepted=true\n'
    } >"$CONFIG/qBittorrent.conf" || cannot "the profile could not be written"

    # ⛔ THE LEGAL NOTICE IS ACCEPTED IN THE PROFILE, NOT ON THE COMMAND LINE.
    # `--confirm-legal-notice` is an argument this build REFUSES - measured on
    # 2026-09-08, in those words - so passing it here would have failed the
    # start exactly as it failed the version call. ⚠ The same wrong assumption
    # was on both paths into this product, which is why removing it from one
    # would have left the other.
    #
    # ⛔ AND THE TORRENT IS A POSITIONAL ARGUMENT, which is the one control this
    # product already has that needs no web interface, no credential and no
    # second process. An adapter driving the WebUI would be measuring a build
    # through an authenticated API it had to configure first.
    HOME="$WORKDIR" "$BINARY" \
      --profile="$PROFILE" \
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
