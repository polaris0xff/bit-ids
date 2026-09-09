#!/bin/sh
# install-step.sh - what *Install the client* does, in a file, so the step's own
# process can be bounded from outside it.
#
# ⛔ EVERY BOUND TRIED SO FAR HAS FAILED, AND EACH FOR ITS OWN REASON. `timeout`
# inside `install-client` ends its own child, so it cannot end a shell blocked in
# a command substitution around that child. `timeout-minutes` is the runner's and
# was measured on run 6 not to end the step at all. A watchdog loop in the step's
# own shell was measured on run 9 not to fire either - the deadline passed by
# three minutes - which says the step's shell is not the thing running.
#
# ⭐ SO THE BOUND MOVES OUTSIDE THE SHELL ENTIRELY. The workflow runs this file
# under `timeout`, so the step's own process is the one being ended, and nothing
# inside it has to be reachable for that to work.
#
# ⚠ THE POINT IS NOT THE INSTALL. It is that the step ENDS: a job whose runner is
# killed leaves no log and no artifact - measured on runs 7, 8 and 9 - so an
# aria2 lane that hangs teaches nothing at all unless something makes the job
# reach its uploads.
#
# ⛔ AND NOTHING IT SPAWNS INHERITS THE STEP'S OUTPUT. A runner ends a step when
# the command has exited AND the step's output pipe has reached end of file, so
# the route's output goes to a file and is printed afterwards.
#
# Usage:
#   ADAPTER=<path> ROUTE=<name> RUNNER_TEMP=<dir> sh scripts/acquisition/install-step.sh
#
# Exit codes: 0 installed, 1 the route or a guard refused, 2 could not run,
# 124 from the caller's `timeout` when this file itself never answered.
#
# ⛔ Read the exit code from this process, unpiped.

set -u

: "${ADAPTER:?install-step: ADAPTER is required}"
: "${ROUTE:?install-step: ROUTE is required}"
: "${RUNNER_TEMP:?install-step: RUNNER_TEMP is required}"

HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH='' cd -- "$HERE/.." && pwd)
ROOT=$(CDPATH='' cd -- "$ROOT/.." && pwd)

WORKDIR="$RUNNER_TEMP/install-$ROUTE"
RECORD="$RUNNER_TEMP/install-$ROUTE.txt"
mkdir -p "$WORKDIR" || {
  printf 'install-step: cannot create %s\n' "$WORKDIR" >&2
  exit 2
}

# ⚠ THE URL TRAVELS THROUGH A FILE RATHER THAN THROUGH GITHUB_ENV, so the value
# the install used is on disk beside the record that names it.
if [ "$ROUTE" = release ]; then
  [ -f "$RUNNER_TEMP/release/url.txt" ] || {
    printf 'install-step: the release lane has no resolved artifact\n' >&2
    exit 2
  }
  BIT_IDS_RELEASE_URL=$(cat "$RUNNER_TEMP/release/url.txt")
  export BIT_IDS_RELEASE_URL
  printf 'the release route will fetch %s\n' "$BIT_IDS_RELEASE_URL"
fi

# ⛔ THE SOURCE ROUTE TAKES ITS TAG FROM ITS OWN RESOLUTION, AND THIS BLOCK USED
# TO TAKE IT FROM THE RELEASE LANE'S. The paragraph that stood here argued that
# one resolver answering both lanes is how absolute 4's same-version rule is
# "made true rather than checked afterwards". ⛔ That is precisely what
# `E-ACQ-07` refuses: two routes sharing a resolver are one route, and
# `assemble-capture` measured it on capture-client run 14 - both lanes'
# resolution records differ only in their timestamps.
#
# ⭐ ABSOLUTE 4 ASKS FOR THE OPPOSITE OF WHAT THAT PARAGRAPH DID. Version
# equality is checked AFTER installation, on what each build reported when
# asked. Two independent resolutions landing on two versions is a vendor that
# moved between two reads, and catching it is the correct outcome; making them
# equal beforehand manufactures the agreement this project exists to measure.
if [ "$ROUTE" = source ]; then
  [ -f "$RUNNER_TEMP/source/resolution.txt" ] || {
    printf 'install-step: the source lane has no resolution of its own to take a tag from\n' >&2
    exit 2
  }
  BIT_IDS_SOURCE_TAG=$(sed -n 's/^selected_tag=//p' "$RUNNER_TEMP/source/resolution.txt")
  [ -n "$BIT_IDS_SOURCE_TAG" ] || {
    printf 'install-step: the source resolution record names no selected_tag\n' >&2
    exit 2
  }
  export BIT_IDS_SOURCE_TAG
  printf 'the source route will build %s, resolved from this repository refs\n' \
    "$BIT_IDS_SOURCE_TAG"
fi

# ⛔ THE INSTALL RUNS IN THE BACKGROUND SO THIS SHELL CAN RECORD WHAT THE HOST IS
# DOING WHILE IT RUNS. ⚠ That loop is NOT the bound any more - run 9 measured it
# not firing - it is the evidence: `stat` beside `etimes` is what separates a
# command that is slow from one that is stopped, and eight runs of step timings
# could not tell those apart.
# ⛔ THE REDIRECTION IS THIS SHELL'S AND NOT sudo's, WHICH IS THE POINT RATHER
# THAN AN OVERSIGHT. shellcheck warns that sudo does not affect a redirect; here
# the caller opening the file is exactly what is wanted, because the descriptor
# the install inherits must be that FILE and not this step's output pipe. ⚠ The
# workdir is under RUNNER_TEMP and was created by this shell, so there is no
# privilege question to answer either.
# shellcheck disable=SC2024
sudo -E sh "$ROOT/scripts/acquisition/install-client.sh" \
  --adapter "$ADAPTER" \
  --route "$ROUTE" \
  --workdir "$WORKDIR" \
  --record "$RECORD" \
  >"$WORKDIR/step.log" 2>&1 &
INSTALL_PID=$!

INTERVAL=${BIT_IDS_INSTALL_INTERVAL:-5}
WAITED=0
while kill -0 "$INSTALL_PID" 2>/dev/null; do
  sleep "$INTERVAL"
  WAITED=$((WAITED + INTERVAL))
  {
    printf '## +%ss %s\n' "$WAITED" "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    ps -eo pid,ppid,pgid,stat,etimes,comm,args 2>&1 || :
  } >>"$WORKDIR/watchdog.log"
done

wait "$INSTALL_PID" && RC=0 || RC=$?
cat "$WORKDIR/step.log"

# ⭐ AND WHATEVER STILL HOLDS THAT FILE IS NAMED. ⚠ Bounded, because it walks
# every process's descriptors and is the one thing added to this step that could
# itself be slow; a diagnostic that held the step open would be the failure it
# exists to explain, wearing its own name. ⚠ Under sudo, because the install ran
# as root and an unprivileged reader of /proc reports nobody holding a file that
# root processes are holding.
HELD=0
timeout 60 sudo sh "$ROOT/scripts/ci/report-holders.sh" "$WORKDIR/step.log" \
  >>"$WORKDIR/holders.log" 2>&1 || HELD=$?
printf 'the holder report exited %s\n' "$HELD"

[ "$RC" = 0 ] || exit "$RC"
cat "$RECORD"
exit 0
