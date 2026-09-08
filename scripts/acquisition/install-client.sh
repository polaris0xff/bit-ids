#!/bin/sh
# install-client.sh - install one target through one route, on a host that has
# already been claimed and still has a route off itself.
#
# ⛔ THIS IS THE ONE STEP THAT RUNS WITH THE NETWORK STILL UP, AND THE ORDER IS
# THE CONTAINMENT. `capture-client` refuses to install anything, so a capture
# that discovered a missing package would have to restore egress on the host
# that exists to have none. Everything a run needs is fetched here and nothing
# is fetched after.
#
# ⚠ IT DELIBERATELY DOES NOT CHECK THE EGRESS GUARD. Being able to reach the
# network is the precondition of this step, not a violation, and a guard here
# would refuse every host it is meant to run on. The claim IS checked: a host
# that already ran a capture must not have a second product installed on it, and
# that is the same marker `capture-client` reads later.
#
# -- ⛔ TWO ROUTES ARE TWO INVOCATIONS ----------------------------------------
#
# One route per call, so a caller that ran one and skipped the other has run one
# and the record says so. A script that installed both would make "two
# independent routes" a property of this file rather than of the workflow that
# is supposed to prove it, and `E-ACQ-07` and `E-ACQ-08` refuse a pair sharing a
# resolver or a delivery mechanism.
#
# ⭐ AND IT PRINTS WHAT THE BUILD SAYS AFTERWARDS, which is the only version
# this project believes. `ACQ-03`'s gate compares those, not the filenames.
#
# Usage:
#   sh scripts/acquisition/install-client.sh --adapter <path> --route <name>
#                                            --workdir <dir> [--record <file>]
#
# Exit codes: 0 installed and the build reported a version, 1 a guard or the
# route refuses, 2 could not run.
#
# ⛔ Read the exit code from this process, unpiped.

set -u

ADAPTER=""
ROUTE=""
WORKDIR=""
RECORD=""

usage() {
  awk 'NR>1 { if (/^#/) { sub(/^# ?/, ""); print } else exit }' "$0"
}

refuse() {
  printf 'install-client: %s\n' "$1" >&2
  exit 1
}

cannot() {
  printf 'install-client: %s\n' "$1" >&2
  exit 2
}

while [ $# -gt 0 ]; do
  case "$1" in
    --adapter)
      [ $# -ge 2 ] || cannot "--adapter takes a value"
      ADAPTER="$2"
      shift
      ;;
    --route)
      [ $# -ge 2 ] || cannot "--route takes a value"
      ROUTE="$2"
      shift
      ;;
    --workdir)
      [ $# -ge 2 ] || cannot "--workdir takes a value"
      WORKDIR="$2"
      shift
      ;;
    --record)
      [ $# -ge 2 ] || cannot "--record takes a value"
      RECORD="$2"
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      printf 'install-client: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

[ -n "$ADAPTER" ] || cannot "--adapter is required"
[ -n "$ROUTE" ] || cannot "--route is required"
[ -n "$WORKDIR" ] || cannot "--workdir is required"
[ -f "$ADAPTER" ] || cannot "the adapter $ADAPTER is not present"

# ⛔ THE ROUTE VOCABULARY IS CLOSED. `docs/client-matrix.md` names two candidate
# routes per target and `RouteKind` is the type they become, so a third spelling
# arriving here would install through something no record can describe.
case "$ROUTE" in
  package | release) : ;;
  *) cannot "unknown route: $ROUTE (package or release)" ;;
esac

HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH='' cd -- "$HERE/.." && pwd)
ROOT=$(CDPATH='' cd -- "$ROOT/.." && pwd)
GUARD="$ROOT/scripts/acquisition/assert-disposable.sh"
[ -f "$GUARD" ] || cannot "$GUARD is not present"

# ⛔ THE CLAIM, AND ONLY THE CLAIM. A host that already ran a capture must not
# have a second product installed on it: the marker is the evidence the host
# survived, and installing over it would contaminate the next run with this
# one's state. ⚠ Egress is deliberately not asserted here; see the header.
MARKER=$(sh "$GUARD" --marker) || cannot "the guard could not report its marker path"
[ -n "$MARKER" ] || cannot "the guard reported an empty marker path"
[ -f "$MARKER" ] ||
  refuse "the host was never claimed; $MARKER does not exist"

mkdir -p "$WORKDIR" || cannot "cannot create $WORKDIR"
WORKDIR=$(CDPATH='' cd -- "$WORKDIR" && pwd)

TARGET=$(sh "$ADAPTER" describe 2>/dev/null | awk -F= '$1 == "target" { print $2; exit }')
[ -n "$TARGET" ] || cannot "the adapter named no target"

STARTED=$(date -u +%Y-%m-%dT%H:%M:%SZ)

sh "$ADAPTER" install "$ROUTE" "$WORKDIR"
INSTALL_RC=$?
case "$INSTALL_RC" in
  0) : ;;
  1) refuse "the $ROUTE route did not install $TARGET" ;;
  *) cannot "the $ROUTE route could not run (exit $INSTALL_RC)" ;;
esac

# ⛔ AND THE BUILD IS ASKED, HERE, WHILE THERE IS STILL SOMETHING TO DO ABOUT A
# FAILURE. An install whose executable cannot answer is a route that did not
# work, and finding that out under containment would be finding it out too late.
VERSION=$(sh "$ADAPTER" version 2>/dev/null)
VERSION_RC=$?
case "$VERSION_RC" in
  0) : ;;
  *) refuse "$TARGET installed through the $ROUTE route but would not report a version" ;;
esac
[ -n "$VERSION" ] ||
  refuse "$TARGET installed through the $ROUTE route and reported an empty version"

FINISHED=$(date -u +%Y-%m-%dT%H:%M:%SZ)

# ⚠ Key=value, the shape every other document in this directory uses. ⛔ It
# records the route and the version SEPARATELY per call, because `ACQ-03`
# compares two of these and a single file holding one merged answer would have
# nothing to compare.
if [ -n "$RECORD" ]; then
  {
    printf 'bit-ids/install-record/1\n'
    printf 'target=%s\n' "$TARGET"
    printf 'route=%s\n' "$ROUTE"
    printf 'reported_version=%s\n' "$VERSION"
    printf 'adapter=%s\n' "$ADAPTER"
    printf 'adapter_sha256=%s\n' "$(sha256sum "$ADAPTER" | cut -d' ' -f1)"
    printf 'workdir=%s\n' "$WORKDIR"
    printf 'started_at=%s\n' "$STARTED"
    printf 'finished_at=%s\n' "$FINISHED"
    printf 'platform=%s\n' "$(uname -srm)"
  } >"$RECORD" || cannot "the install record could not be written to $RECORD"
  grep -q "^reported_version=$VERSION\$" "$RECORD" ||
    refuse "the install record was not written"
fi

printf '%s\n' "$VERSION"
printf 'install-client: %s %s through the %s route\n' "$TARGET" "$VERSION" "$ROUTE"
exit 0
