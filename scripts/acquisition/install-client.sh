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
# ⛔ IT ALSO ASKS THE HOST WHAT IT ALREADY HAD, BEFORE THE ROUTE RUNS, so the
# record says whether this route acquired anything or reported a build that was
# there before it started. See the block above that call for what a route that
# installs nothing does to the two-route rule.
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

# ⛔ EVERY ADAPTER CALL IS BOUNDED, AND THE BOUND IS HERE RATHER THAN ONLY IN THE
# ADAPTER. An adapter shells out to a product this project did not write, and a
# product that waits on a prompt waits forever: measured on 2026-09-08, the first
# client dispatch had two of three jobs sit in this step for over half an hour
# and report nothing at all, because a hung install is indistinguishable from a
# slow one until the job's own timeout kills the runner and takes the log with
# it. ⚠ A convention in each adapter would be a bound the next adapter forgets;
# this is the one call site every adapter passes through.
#
# ⭐ AND STDIN IS /dev/null, so a prompt gets end-of-file rather than a wait.
# ⚠ THE BOUND SITS WELL UNDER THE WORKFLOW STEP'S OWN TIMEOUT, so this reports
# rather than being killed with its log. Measured on 2026-09-08: a 900s bound
# under a 30-minute job did not end the job, and the runner was killed at the
# job timeout with the diagnosis still on it.
# ⛔ AND -k IS NOT DECORATION. `timeout` sends TERM and then waits; a child that
# blocks or ignores it is never killed at all, so the bound reports nothing.
INSTALL_SECONDS=${BIT_IDS_INSTALL_TIMEOUT:-420}
VERSION_SECONDS=${BIT_IDS_VERSION_TIMEOUT:-60}
KILL_AFTER=${BIT_IDS_KILL_AFTER:-20}
command -v timeout >/dev/null 2>&1 || cannot "timeout is not on this host"

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

# ⛔ WHAT THE HOST ALREADY HAD, ASKED BEFORE THE ROUTE RUNS. `install` is
# contracted to install the target and nothing established that it did: a route
# that installed nothing exits 0, and this file then reported a version the route
# had not put there as the version that route acquired.
#
# ⚠ MEASURED, NOT HYPOTHETICAL. `aria2` ships on the `ubuntu-24.04` image, so
# `apt-get install aria2` there prints `already the newest version`, installs
# nothing and exits 0; client capture runs 3 and 4 recorded `route=package` over
# exactly that. ⛔ AND THE `release` ROUTES HAVE THE SAME SHAPE FROM THE OTHER
# END: each fetches an artifact into the workdir and never makes it the
# executable `version` asks, so a release route run on a host that already has
# the package build reports the PACKAGE build's version as its own.
#
# ⛔ THAT IS THE TWO-ROUTE RULE FAILING THROUGH THE ONE DOOR NOTHING WATCHES.
# `E-ACQ-07` and `E-ACQ-08` compare what a route DECLARES - its resolver and its
# delivery - so two routes that each installed nothing declare two independent
# routes, agree on the version for the most trivial reason available, and hand
# `ACQ-03` one binary to compare against itself. That comparison is by executable
# digest, so the answer is `byte_identical`: the STRONGEST verdict the
# classification has, reached by acquiring nothing at all.
#
# ⚠ A REFUSAL HERE WOULD THROW AWAY A REAL MEASUREMENT. The build on such a host
# is genuine and its identity is worth capturing; what is not genuine is the
# claim that this route acquired it. So the fact is recorded and said out loud,
# and the gate that compares two routes is where it bites - the same argument
# this file's header makes for not owning the two-route rule itself.
#
# ⚠ ITS STDERR IS KEPT AND NEVER PRINTED AS A DIAGNOSIS. A target that is not yet
# installed is the ORDINARY case here, so the adapter's refusal goes to the
# workdir, where the install artifact carries it, rather than to a log where it
# would read as a failure of the step.
PREEXISTING=$(timeout -k "$KILL_AFTER" "$VERSION_SECONDS" sh "$ADAPTER" version \
  2>"$WORKDIR/version-before.err" </dev/null)
PREEXISTING_RC=$?
[ "$PREEXISTING_RC" = 0 ] || PREEXISTING=""

timeout -k "$KILL_AFTER" "$INSTALL_SECONDS" sh "$ADAPTER" install "$ROUTE" "$WORKDIR" </dev/null
INSTALL_RC=$?

# ⚠ THE LOGS THE ROUTE WROTE ARE PRINTED ON EVERY FAILURE, INDENTED. Without
# this the job log carries the refusal and not its cause, and the workdir is
# uploaded only when the capture that follows succeeds - so a route that failed
# took its own diagnosis with it.
show_route_logs() {
  for _log in "$WORKDIR/update.log" "$WORKDIR/install.log"; do
    [ -f "$_log" ] || continue
    printf '          -- %s (last 20 lines)\n' "$_log" >&2
    tail -20 "$_log" | sed 's/^/          /' >&2
  done
}

case "$INSTALL_RC" in
  0) : ;;
  # ⛔ 124 IS coreutils' VERDICT FOR "IT NEVER ANSWERED", and it is a different
  # fact from a route that refused. A run that folded them together would report
  # a package index as unavailable over a product that asked a question.
  124)
    show_route_logs
    refuse "the $ROUTE route did not answer within ${INSTALL_SECONDS}s; it is hung, not slow"
    ;;
  1)
    show_route_logs
    refuse "the $ROUTE route did not install $TARGET"
    ;;
  *)
    show_route_logs
    cannot "the $ROUTE route could not run (exit $INSTALL_RC)"
    ;;
esac

# ⛔ AND THE BUILD IS ASKED, HERE, WHILE THERE IS STILL SOMETHING TO DO ABOUT A
# FAILURE. An install whose executable cannot answer is a route that did not
# work, and finding that out under containment would be finding it out too late.
# ⛔ THE ADAPTER'S OWN MESSAGE IS KEPT, NOT DISCARDED. This read `2>/dev/null`,
# so a refusal arrived here as a bare exit code and the step printed "would not
# report a version" over an adapter that had said exactly which of its three
# refusals fired. ⚠ Measured on 2026-09-08: a client capture reported that and
# the cause could not be read from the run at all. Silencing the diagnosis and
# then reporting its absence is one defect, not two.
VERSION=$(timeout -k "$KILL_AFTER" "$VERSION_SECONDS" sh "$ADAPTER" version 2>"$WORKDIR/version.err" </dev/null)
VERSION_RC=$?
show_adapter_error() {
  [ -s "$WORKDIR/version.err" ] || return 0
  printf '          -- the adapter said\n' >&2
  sed 's/^/          /' "$WORKDIR/version.err" >&2
}
case "$VERSION_RC" in
  0) : ;;
  124)
    show_adapter_error
    refuse "$TARGET installed through the $ROUTE route and did not answer --version within ${VERSION_SECONDS}s"
    ;;
  *)
    show_adapter_error
    refuse "$TARGET installed through the $ROUTE route but would not report a version (adapter exit $VERSION_RC)"
    ;;
esac
[ -n "$VERSION" ] ||
  refuse "$TARGET installed through the $ROUTE route and reported an empty version"

# ⛔ AND HERE IS WHETHER THE ROUTE ACQUIRED ANYTHING AT ALL. Absent before and
# answering now is an install. A DIFFERENT version is an upgrade, which is also
# an install, and reading it as anything else would refuse the ordinary case of a
# package index carrying a newer build than the image. The same version over a
# target that was already there is neither, whatever the route reported.
if [ -n "$PREEXISTING" ] && [ "$PREEXISTING" = "$VERSION" ]; then
  ACQUIRED=no
else
  ACQUIRED=yes
fi

# ⚠ SAID OUT LOUD, ON stderr, WHERE THE STEP LOG KEEPS IT. A field in a file
# nobody opens is how this went unnoticed for four dispatches.
[ "$ACQUIRED" = yes ] ||
  printf 'install-client: %s %s was ALREADY on this host and the %s route installed nothing; this run acquired no build\n' \
    "$TARGET" "$VERSION" "$ROUTE" >&2

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
    # ⚠ EMPTY MEANS THE TARGET WAS NOT INSTALLED WHEN THIS ROUTE STARTED, which
    # is a different fact from a target whose version could not be read. The
    # adapter's own words for the second are in `version-before.err` beside this.
    printf 'preexisting_version=%s\n' "$PREEXISTING"
    printf 'acquired=%s\n' "$ACQUIRED"
    printf 'adapter=%s\n' "$ADAPTER"
    printf 'adapter_sha256=%s\n' "$(sha256sum "$ADAPTER" | cut -d' ' -f1)"
    printf 'workdir=%s\n' "$WORKDIR"
    printf 'started_at=%s\n' "$STARTED"
    printf 'finished_at=%s\n' "$FINISHED"
    printf 'platform=%s\n' "$(uname -srm)"
  } >"$RECORD" || cannot "the install record could not be written to $RECORD"

  # ⛔ EVERY FIELD IS PRESENT, ASKED AS A FIELD RATHER THAN AS A VALUE. A record
  # truncated part-way through the block would carry a correct
  # `reported_version` and no verdict at all, and a reader taking an absent
  # `acquired` for `yes` is the failure this whole change exists to remove.
  for _field in target route reported_version preexisting_version acquired; do
    grep -q "^$_field=" "$RECORD" ||
      cannot "the install record was written without $_field"
  done

  # ⛔ AND THE RECORD IS CHECKED AGAINST ITSELF RATHER THAN RESTATED. `acquired`
  # is derived from two fields that are also in this file, which is one value in
  # two places; this reads all three back off disk and re-derives the third, so a
  # record whose verdict does not follow from its own evidence is refused here
  # instead of being believed downstream.
  # ⚠ READ WITH `sed`, COMPARED WITH `=`. The old check was
  # `grep -q "^reported_version=$VERSION\$"`, where a version is a regular
  # expression: every `.` in `1.37.0` matches any byte, so the check passed over
  # values it was written to distinguish.
  _pre=$(sed -n 's/^preexisting_version=//p' "$RECORD")
  _rep=$(sed -n 's/^reported_version=//p' "$RECORD")
  _acq=$(sed -n 's/^acquired=//p' "$RECORD")
  [ "$_rep" = "$VERSION" ] ||
    refuse "the install record says reported_version=[$_rep] over a build that said [$VERSION]"
  if [ -n "$_pre" ] && [ "$_pre" = "$_rep" ]; then _want=no; else _want=yes; fi
  [ "$_acq" = "$_want" ] ||
    refuse "the install record says acquired=[$_acq] over preexisting [$_pre] and reported [$_rep]"
fi

printf '%s\n' "$VERSION"
printf 'install-client: %s %s through the %s route (acquired=%s)\n' \
  "$TARGET" "$VERSION" "$ROUTE" "$ACQUIRED"
exit 0
