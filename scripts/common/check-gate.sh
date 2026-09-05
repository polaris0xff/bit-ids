#!/bin/sh
# check-gate.sh - run the whole local gate, in one command, and read every
# exit code from the process that produced it.
#
# The defect this exists to catch is a gate that is a LIST. Part (a) of
# docs/methodology/gate.md names several checks, and a list run by hand is run
# in the order somebody recalls it. ⛔ The session that first wrote this ran its
# gate five times and typed a different subset each time. Nothing failed; the
# gate simply was not the same gate twice.
#
# ⭐ IT DELEGATES. It holds no rules of its own and it is not a second opinion
# about anything. Every verdict here is some other script's, read unpiped.
#
# -- ⛔ A SKIPPED CHECK IS A SKIP, NEVER A PASS -----------------------------
#
# `pwsh`, `jq`, `gh` and `shellcheck` are not on every machine. A runner that
# quietly dropped one and printed green would be the row in
# docs/conventions/forbidden-patterns.md that reads *a step that exits 0 having
# done nothing it was asked to do*.
#
# So a skip is counted, named, and printed on its own line. ⚠ The exit code is
# still 0, because a machine that cannot run a check has not failed it; ⭐ pass
# --strict to make a skip a failure, which is what a CI job should do, since
# there the tools are installed on purpose and a skip means the install broke.
#
# -- ⛔ AND A CHECK THIS HOST CANNOT RUN IS NOT A CHECK THAT BROKE ------------
#
# Those are two different facts and --strict could not tell them apart, which
# made it unusable on the lane that needed it most. Measured on 2026-09-06:
# with check-project rewritten to exit 2, the Windows lane's own invocation
# still exited 0, because it ran without --strict; and it ran without --strict
# because six of its rows are checks Windows genuinely cannot run, so the flag
# would have refused every green tree.
#
# ⭐ So a row is one of two kinds. A DECLARED unavailability is written in the
# runner, with a reason and the entry that owns it, and prints as `n/a`. An
# OBSERVED skip is a check that ran and answered 2, or one whose file is gone,
# and prints as `SKIP`. --strict refuses the second and permits the first, so
# both lanes run strict and a check that quietly stops running turns a lane red
# wherever it happens.
#
# ⚠ --fast counts as declared, because the caller asked for it rather than the
# host failing to supply it. The summary names the two totals separately so a
# run cannot claim a strictness it did not have.
#
# -- ⛔ IT DOES NOT RUN ITSELF, AND THAT IS NOT THEORETICAL ------------------
#
# This runs check-twins.sh, which runs both halves of every pair. ⚠ A version
# of this idea in another repository hit an unbounded recursion with
# check-twins that left twenty stray shells holding their own files open. That
# is the reported symptom; the mechanism here is plain enough that it does not
# need re-deriving. A runner that appears in the pair list runs the comparison
# that runs the runner.
#
# So check-gate is NOT in check-twins.sh's pair list, and check-twins is
# invoked here directly rather than through anything that could re-enter.
# ⚠ The two exclusions are a shared contract: removing one reintroduces the
# hang.
#
# Usage:
#   sh scripts/common/check-gate.sh
#   sh scripts/common/check-gate.sh --fast     # skips check-twins
#   sh scripts/common/check-gate.sh --strict   # a skip is a failure
#   sh scripts/common/check-gate.sh --json
#
# Exit codes: 0 nothing failed, 1 something failed, 2 could not run.
#
# ⛔ Read the exit code from this process, unpiped.

set -u

JSON=0
FAST=0
STRICT=0

while [ $# -gt 0 ]; do
  case "$1" in
    --json) JSON=1 ;;
    --fast) FAST=1 ;;
    --strict) STRICT=1 ;;
    -h | --help)
      awk 'NR>1 { if (/^#/) { sub(/^# ?/, ""); print } else exit }' "$0"
      exit 0
      ;;
    *)
      printf 'check-gate: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

# ⛔ RESOLVED FROM THIS SCRIPT'S OWN LOCATION, not from the working directory.
# A runner found by a relative path runs a different set depending on who
# called it, which is the same class of defect as a guard whose scope depends
# on the process working directory.
HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)

PASS=0
FAIL=0
SKIP=0
NA=0
ROWS=""

row() { ROWS="$ROWS  $1
"; }

# ⛔ DECLARED HERE, WITH A REASON, OR IT IS NOT DECLARED. The reason is the
# whole difference between this row and an observed skip: one is a fact about
# the platform that somebody wrote down and can be argued with, and the other
# is a check that stopped working. A row added here without a reason turns
# --strict back into the flag that could not be used.
unavailable() { # name reason
  row "n/a   $1  ($2)"
  NA=$((NA + 1))
}

# ⛔ THE EXIT CODE IS TAKEN FROM THE PROCESS, UNPIPED. Output goes to a file
# and $? is read on the next line. `run ... | tee` would report tee's status,
# which is 0 whatever the check did, and that is the single defect this whole
# repository is most emphatic about.
OUT="${TMPDIR:-/tmp}/.checkgate.$$"
mkdir -p "$OUT" || {
  printf 'check-gate: cannot write to %s\n' "$OUT" >&2
  exit 2
}
trap 'rm -rf "$OUT"' EXIT INT TERM

# ⚠ NO PRESENCE TEST LIVES HERE. An earlier version tested `$1` after the
# shift, which is the interpreter rather than the script, so every row reported
# "not present" and the runner printed a green verdict having executed nothing.
# ⭐ That is the exact defect this script's header is about, produced by the
# script itself on its first run. Presence is decided by the caller, which is
# the only place that knows the path.
run() { # name  command...
  _name="$1"
  shift
  "$@" >"$OUT/log" 2>&1
  _rc=$?
  case "$_rc" in
    0)
      row "✅ ok    $_name"
      PASS=$((PASS + 1))
      ;;
    2)
      row "SKIP  $_name  ($(head -1 "$OUT/log" 2>/dev/null | cut -c1-60))"
      SKIP=$((SKIP + 1))
      ;;
    *)
      row "❌ FAIL  $_name  (exit $_rc)"
      FAIL=$((FAIL + 1))
      [ "$JSON" = "1" ] || sed 's/^/          /' "$OUT/log" | head -12
      ;;
  esac
}

have_pwsh=0
command -v pwsh >/dev/null 2>&1 && have_pwsh=1

# The sh halves. Each is the authority on its own subject.
for c in check-docs check-markers check-one-home check-placeholders \
  check-control-bytes check-changelog check-no-secrets check-project \
  check-licences; do
  if [ -f "$HERE/$c.sh" ]; then
    run "$c" sh "$HERE/$c.sh"
  else
    row "SKIP  $c  (not present)"
    SKIP=$((SKIP + 1))
  fi
done

# ⚠ --public is a DIFFERENT question from the default run, not a stricter one.
# Emails, absolute home paths and long hex are legitimate content in a private
# project, so this row is a second call rather than a flag on the first.
[ -f "$HERE/check-no-secrets.sh" ] && run "check-no-secrets --public" sh "$HERE/check-no-secrets.sh" --public

# ⚠ NEEDS gh AND THE NETWORK, so it exits 2 on a machine without them and that
# reads as a skip rather than a pass. That is correct: nothing was verified.
[ -f "$HERE/check-remote-items.sh" ] && run "check-remote-items" sh "$HERE/check-remote-items.sh"

# ⛔ NOT IN common/, AND IN THE GATE ANYWAY. `check-runner` mutation-proves the
# guards that stand between this project and installing an untrusted client on a
# machine somebody keeps. It is hermetic, it takes no network, and it is the one
# check whose silence would be worst, so it runs on every gate rather than only
# where captures happen. It is not in check-twins' pair list because it has no
# PowerShell half; scripts/README.md carries why.
RUNNER="$HERE/../acquisition/check-runner.sh"
if [ -f "$RUNNER" ]; then
  run "check-runner" sh "$RUNNER"
else
  row "SKIP  check-runner  (not present)"
  SKIP=$((SKIP + 1))
fi

# ⛔ NOT IN common/ EITHER, AND IN THE GATE FOR THE SAME REASON. These seven
# mutation-prove the guards standing between this project and silently deleting
# or rewriting published evidence, publishing a record whose evidence nothing
# can resolve, pointing a consumer at a superseded build, publishing a retracted
# measurement in a rendering the lookups had stopped naming, shipping two
# different byte sets under one release label, and force-pushing over the data
# branch, and keeping somebody else's installer in this repository. The first is
# unrecoverable afterwards and the rest are worse than errors, because each
# answers confidently. ⭐ All seven are hermetic:
# check-publish creates its own bare repository in a scratch directory and
# touches no real remote. ⚠ They need cargo, so they exit 2 on a host without
# one, which is a skip and not a pass. None has a PowerShell half;
# scripts/README.md carries why.
for spec in acquisition/check-cache corpus/check-store corpus/check-corpus \
  corpus/check-indexes publishing/check-release publishing/check-formats \
  publishing/check-publish; do
  PROVER="$HERE/../$spec.sh"
  NAME=${spec#*/}
  if [ -f "$PROVER" ]; then
    run "$NAME" sh "$PROVER"
  else
    row "SKIP  $NAME  (not present)"
    SKIP=$((SKIP + 1))
  fi
done

# ⭐ THE SLOW ONE. Measured on one Windows 11 Pro 26200 machine, 2026-08-28:
# check-twins alone is most of a full run's wall time, because it starts both
# halves of every pair. --fast drops it and nothing else.
if [ "$FAST" = "1" ]; then
  unavailable "check-twins" "--fast"
elif [ -f "$HERE/check-twins.sh" ]; then
  run "check-twins" sh "$HERE/check-twins.sh"
else
  row "SKIP  check-twins  (not present)"
  SKIP=$((SKIP + 1))
fi

# ⚠ THE POWERSHELL HALVES ARE NOT RE-RUN HERE. check-twins already runs both
# halves of every pair and compares them, so running them again would double
# the slowest part of the gate to learn nothing. On a machine with no pwsh at
# all, check-twins reports that itself.
[ "$have_pwsh" = "1" ] || row "note  pwsh absent; the PowerShell halves were not exercised"

TOTAL=$((PASS + FAIL + SKIP + NA))

# ⛔ A RUN THAT PASSED NOTHING IS NOT A GREEN RUN. Zero failures out of zero
# checks executed is the shape this script exists to refuse, and it produced
# exactly that on its own first run through a broken presence test. Nothing
# passing is a failure of the gate regardless of --strict.
#
# ⚠ --strict reads SKIP and never NA. A declared row is a documented gap and
# refusing it would refuse every correct tree on the lane that declared it.
if [ "$PASS" -eq 0 ]; then
  RC=1
elif [ "$STRICT" = "1" ] && [ "$SKIP" -gt 0 ]; then
  RC=1
elif [ "$FAIL" -gt 0 ]; then
  RC=1
else
  RC=0
fi

if [ "$JSON" = "1" ]; then
  printf '{"schema":"check-gate/2","total":%s,"passed":%s,"failed":%s,"skipped":%s,"unavailable":%s,"strict":%s}\n' \
    "$TOTAL" "$PASS" "$FAIL" "$SKIP" "$NA" "$([ "$STRICT" = "1" ] && printf true || printf false)"
  exit "$RC"
fi

printf '\n%s\n' "$ROWS"
printf '%s checks: %s passed, %s failed, %s skipped, %s unavailable\n' \
  "$TOTAL" "$PASS" "$FAIL" "$SKIP" "$NA"

if [ "$SKIP" -gt 0 ]; then
  printf -- '⚠ A SKIP IS NOT A PASS. Those checks did not run and nothing about\n'
  printf 'their subject was verified. Pass --strict to make a skip a failure.\n'
fi
if [ "$NA" -gt 0 ]; then
  printf -- '⚠ An n/a row is a gap this runner declares, with the reason beside it.\n'
  printf -- '--strict permits those and refuses a SKIP, so read both numbers.\n'
fi
if [ "$PASS" -eq 0 ]; then
  printf -- '❌ NOTHING RAN. Zero checks passed, so this is red whatever the skips say.\n'
elif [ "$FAIL" -gt 0 ]; then
  printf -- '❌ the gate is red.\n'
else
  printf -- '✅ nothing failed.\n'
  printf -- '⚠ That is part (a) of the gate only. Driving the real thing and the\n'
  printf 'deep reviews are the other two, and each is blind to what this catches.\n'
  printf 'docs/methodology/gate.md.\n'
fi
exit "$RC"
