#!/bin/sh
# check-runner.sh - prove the disposable-host guards refuse what they exist to refuse.
#
# ⛔ A GUARD THAT HAS NEVER BEEN SEEN TO REFUSE IS A GUARD NOBODY KNOWS WORKS.
# `assert-disposable.sh` stands between this project and installing an untrusted
# client on a machine somebody keeps. It is the last thing that runs before a
# capture and the first thing whose silence would be catastrophic, so it is
# mutation-proven here rather than trusted.
#
# Each case plants the exact condition the guard exists to catch and reads the
# exit code from the guard's own process, unpiped.
#
# ⚠ This does NOT prove the guards are sufficient. It proves each one fires. A
# host can be non-disposable in ways neither guard models, which is why
# `docs/security/remote-ops.md` and the runner contract in `TODO/acquisition.md`
# carry the rest.
#
# Usage:
#   sh scripts/acquisition/check-runner.sh
#   sh scripts/acquisition/check-runner.sh --json
#
# Exit codes: 0 every guard refused what it should, 1 one did not, 2 could not run.
#
# ⛔ Read the exit code from this process, unpiped.

set -u

JSON=0
while [ $# -gt 0 ]; do
  case "$1" in
    --json) JSON=1 ;;
    -h | --help)
      awk 'NR>1 { if (/^#/) { sub(/^# ?/, ""); print } else exit }' "$0"
      exit 0
      ;;
    *)
      printf 'check-runner: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
GUARD="$HERE/assert-disposable.sh"
[ -f "$GUARD" ] || {
  printf 'check-runner: %s is not present\n' "$GUARD" >&2
  exit 2
}

WORK="${TMPDIR:-/tmp}/.checkrunner.$$"
mkdir -p "$WORK" || {
  printf 'check-runner: cannot write to %s\n' "$WORK" >&2
  exit 2
}
trap 'rm -rf "$WORK"' EXIT INT TERM

PASS=0
FAIL=0
ROWS=""

# ⛔ The guard runs unpiped and $? is read on the next line. Piping it into
# anything reports the pipe's status, so a guard that failed to refuse would
# read as having refused. That is this repository's oldest stated rule.
expect() { # want-code  name  args...
  _want="$1"
  _name="$2"
  shift 2
  sh "$GUARD" "$@" >"$WORK/out" 2>"$WORK/err"
  _got=$?
  if [ "$_got" = "$_want" ]; then
    ROWS="$ROWS  ✅ ok    $_name
"
    PASS=$((PASS + 1))
  else
    ROWS="$ROWS  ❌ FAIL  $_name (wanted exit $_want, got $_got)
"
    FAIL=$((FAIL + 1))
    [ "$JSON" = "1" ] || sed 's/^/          /' "$WORK/err" | head -4
  fi
}

# --- forbidden egress -------------------------------------------------------
# A table with a default route is a host that can reach the public network, and
# a capture there puts an untrusted client on it.
printf 'Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT\n' >"$WORK/route-open"
printf 'eth0\t00000000\t010200C0\t0003\t0\t0\t0\t00000000\t0\t0\t0\n' >>"$WORK/route-open"
expect 1 "a default route is refused" --egress "$WORK/route-open"

# ⚠ The same table with the route DOWN. The destination alone is not the test:
# a rule that matched only `00000000` would refuse a host whose default route is
# configured and inactive, and more importantly would have no reason to look at
# the flags at all, which is where a real difference lives.
printf 'Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT\n' >"$WORK/route-down"
printf 'eth0\t00000000\t010200C0\t0002\t0\t0\t0\t00000000\t0\t0\t0\n' >>"$WORK/route-down"
expect 0 "a default route that is not up is not egress" --egress "$WORK/route-down"

# Loopback only, which is what a capture host looks like.
printf 'Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT\n' >"$WORK/route-loopback"
printf 'lo\t0000007F\t00000000\t0001\t0\t0\t0\t000000FF\t0\t0\t0\n' >>"$WORK/route-loopback"
expect 0 "loopback only passes" --egress "$WORK/route-loopback"

# ⛔ An unreadable table is exit 2, not exit 0. Not knowing is not a pass.
expect 2 "an unreadable table is not a pass" --egress "$WORK/no-such-table"

# --- persistent state -------------------------------------------------------
#
# ⛔ EVERY CLAIM HERE GOES TO A SCRATCH DIRECTORY. The first version of this
# ran one case without the override, so the test claimed the real machine: it
# wrote /var/lib/bit-ids/host-claimed, passed once, and failed on every run
# after. On a persistent CI runner that is a check which goes green on the day
# it is written and red forever, for a reason that looks like the guard being
# broken rather than the test being. Found by running it twice.
FRESH="$WORK/host-a"
BIT_IDS_STATE_DIR="$FRESH" sh "$GUARD" --claim capture-0001 >"$WORK/out" 2>&1
_first=$?
BIT_IDS_STATE_DIR="$FRESH" sh "$GUARD" --claim capture-0002 >"$WORK/out2" 2>&1
_second=$?
if [ "$_first" = "0" ] && [ "$_second" = "1" ]; then
  ROWS="$ROWS  ✅ ok    a second capture on one host is refused
"
  PASS=$((PASS + 1))
else
  ROWS="$ROWS  ❌ FAIL  a second capture on one host is refused (first $_first, second $_second)
"
  FAIL=$((FAIL + 1))
fi

# --- a fresh host for the next job -----------------------------------------
# ⚠ Destroying the marker is what a real teardown does. This shows the NEXT job
# is unblocked once it happens, which is the other half of the same guard: it
# must refuse a survived host without refusing every host forever.
rm -rf "$FRESH"
BIT_IDS_STATE_DIR="$FRESH" sh "$GUARD" --claim capture-0002 >"$WORK/out3" 2>&1
_third=$?
if [ "$_third" = "0" ]; then
  ROWS="$ROWS  ✅ ok    a torn-down host claims again for the next job
"
  PASS=$((PASS + 1))
else
  ROWS="$ROWS  ❌ FAIL  a torn-down host claims again for the next job (exit $_third)
"
  FAIL=$((FAIL + 1))
fi

# The fingerprint the next job compares against.
PRINT=$(sh "$GUARD" --fingerprint)
case "$PRINT" in
  [0-9a-f]*)
    if [ "${#PRINT}" = "64" ]; then
      ROWS="$ROWS  ✅ ok    the host fingerprint is a sha256
"
      PASS=$((PASS + 1))
    else
      ROWS="$ROWS  ❌ FAIL  the host fingerprint is ${#PRINT} characters
"
      FAIL=$((FAIL + 1))
    fi
    ;;
  *)
    ROWS="$ROWS  ❌ FAIL  the host fingerprint is not hexadecimal
"
    FAIL=$((FAIL + 1))
    ;;
esac

TOTAL=$((PASS + FAIL))
if [ "$JSON" = "1" ]; then
  printf '{"schema":"check-runner/1","total":%s,"passed":%s,"failed":%s,"fingerprint":"%s"}\n' \
    "$TOTAL" "$PASS" "$FAIL" "$PRINT"
  [ "$FAIL" -eq 0 ] || exit 1
  exit 0
fi

printf '\n%s\n' "$ROWS"
printf '%s guard case(s): %s passed, %s failed\n' "$TOTAL" "$PASS" "$FAIL"
printf 'host fingerprint for the next job: %s\n' "$PRINT"
# ⛔ Zero cases passing is not a green run, whatever the failure count says.
if [ "$PASS" -eq 0 ]; then
  printf -- '❌ NOTHING RAN.\n'
  exit 1
fi
if [ "$FAIL" -gt 0 ]; then
  printf -- '❌ a guard did not refuse what it exists to refuse.\n'
  exit 1
fi
printf -- '✅ every guard refused its own defect.\n'
