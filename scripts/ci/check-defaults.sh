#!/bin/sh
# check-defaults.sh - does any check here answer differently because of
# something it INHERITED from the host rather than STATED for itself?
#
# ⛔ THE DEFECT CLASS IS `CI-08`'s AND IT HAS BITTEN TWICE. A PowerShell default
# changed between 7.4 and 7.5 and turned a green lane red over a script nobody
# had edited; and exporting `CARGO_TARGET_DIR`, which a great many Rust
# developers do, put every built example somewhere five provers did not look, so
# the whole tier exited 2 and the gate read a silent stop as a skip. Neither was
# a defect in the check that failed. Both were a value the check took from its
# environment without saying so.
#
# ⭐ SO THIS IS AN INSTRUMENT RATHER THAN A READING. That entry's Approach asks
# for an enumeration of what the scripts inherit, and an enumeration written by
# reading is a list of the defaults somebody thought of. This runs the checks
# under a perturbed environment and compares their MACHINE-READABLE answer
# against the baseline: a difference is a default that check inherits, named by
# the variable that produced it, whether or not anybody had thought of it.
#
# -- ⛔ THE CONTROL IS THE HALF THAT MATTERS ---------------------------------
#
# Every "the answer did not change" row passes equally over an instrument whose
# environment never reached the child. So two probes run first whose answer MUST
# change, and this exits 2 rather than 0 if either of them agrees: an instrument
# that cannot detect a difference has not established there is none.
#
# ⚠ AND ONE PERTURBATION IS NOT AVAILABLE AND SAYS SO. `IFS` does not survive
# into a new shell - measured here: `IFS=: sh -c 'printf %s "$IFS"'` prints the
# default - so a case setting it would report a guard proved by a value the child
# never saw. It is listed as unavailable rather than quietly dropped.
#
# ⛔ NOT A GATE ROW, AND `CI-08` CARRIES THE MEASUREMENT. Every subject runs once
# per environment, so this is 28 seconds - free beside a concurrent gate and NOT
# free inside `check-workflow`, which runs the whole gate about ten times. As a
# row it pushed that CI job past its 30-minute bound. It is its own CI step, the
# way `check-workflow` itself is.
#
# Usage:
#   sh scripts/ci/check-defaults.sh
#   sh scripts/ci/check-defaults.sh --json
#
# Exit codes: 0 every check answered the same under every environment, 1 one did
# not, 2 could not run.
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
      printf 'check-defaults: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH='' cd -- "$HERE/.." && pwd)
ROOT=$(CDPATH='' cd -- "$ROOT/.." && pwd)

# shellcheck disable=SC2034
ME=check-defaults
# shellcheck disable=SC1091
# shellcheck source=scripts/corpus/store-lib.sh
. "$ROOT/scripts/corpus/store-lib.sh"

store_require env go

WORK=$(store_workdir checkdefaults) || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

mkdir -p "$WORK/tmp" "$WORK/target" || exit 2

# ⛔ EACH ENVIRONMENT IS A REAL THING A HOST HAS, not an invented one. The list
# is the point of this file, so each row says what it would change if a check
# read it.
#
#   LC_ALL=C            byte collation and byte character classes
#   LC_ALL=C.utf8       multibyte collation and classes
#   TMPDIR              where a harness puts the tree it plants in
#   CARGO_TARGET_DIR    where a built example lands; this one has already bitten
#   POSIXLY_CORRECT     changes GNU tool behaviour in several places
#   SOURCE_DATE_EPOCH   read by build tooling that wants a fixed clock
#
# ⚠ `umask` is NOT in this list and is not forgotten: it is a shell attribute
# rather than an environment variable, so it cannot be passed through `env`, and
# a case that set it in this process would change the modes of everything this
# harness itself writes. It belongs to whatever check reads a file mode, and
# nothing here does.
ENVIRONMENTS="LC_ALL=C
LC_ALL=C.utf8
TMPDIR=$WORK/tmp
CARGO_TARGET_DIR=$WORK/target
POSIXLY_CORRECT=1
SOURCE_DATE_EPOCH=1700000000"

# Runs one command under one assignment and prints the machine-readable line.
#
# ⛔ UNPIPED, AND THE STATUS KEPT BESIDE THE ANSWER. Two checks that print the
# same JSON and exit differently are not the same answer, and a comparison of the
# text alone would call them equal.
answer_under() { # assignment command...
  _assignment="$1"
  shift
  if [ -z "$_assignment" ]; then
    (cd "$ROOT" && "$@" >"$WORK/out" 2>/dev/null)
  else
    (cd "$ROOT" && env "$_assignment" "$@" >"$WORK/out" 2>/dev/null)
  fi
  _rc=$?
  printf 'exit %s %s' "$_rc" "$(grep -m1 '^{' "$WORK/out" 2>/dev/null || printf '(no json)')"
}

# -- ⛔ THE CONTROL, BEFORE ANY SUBJECT --------------------------------------
#
# ⛔ AN INSTRUMENT THAT CANNOT DETECT A DIFFERENCE HAS NOT ESTABLISHED THERE IS
# NONE. These two probes read the variable and print it, so their answers MUST
# differ; if either agrees, the environment is not reaching the child and every
# row below is worthless. Exit 2, because that is `could not run` rather than a
# subject that failed.
PROBE="$WORK/probe.sh"
cat >"$PROBE" <<'PROBE'
#!/bin/sh
printf '{"tmpdir":"%s","locale":"%s"}\n' "${TMPDIR:-unset}" "${LC_ALL:-unset}"
PROBE

CONTROL_BASE=$(answer_under "" sh "$PROBE")
CONTROL_TMP=$(answer_under "TMPDIR=$WORK/tmp" sh "$PROBE")
CONTROL_LOC=$(answer_under "LC_ALL=C" sh "$PROBE")
if [ "$CONTROL_BASE" = "$CONTROL_TMP" ] || [ "$CONTROL_BASE" = "$CONTROL_LOC" ]; then
  printf 'check-defaults: the perturbation does not reach the child; this instrument\n' >&2
  printf 'check-defaults: cannot tell "no difference" from "no experiment"\n' >&2
  printf 'check-defaults:   baseline %s\n' "$CONTROL_BASE" >&2
  printf 'check-defaults:   TMPDIR   %s\n' "$CONTROL_TMP" >&2
  printf 'check-defaults:   LC_ALL   %s\n' "$CONTROL_LOC" >&2
  exit 2
fi
pass "control   a probe that READS the variable answers differently under it"

# -- ⛔ AND THE REACH OF THE `CARGO_TARGET_DIR` ROW, WHICH IS NOT WHAT IT LOOKS
#
# ⛔ MEASURED BY REPLANTING THE HISTORICAL DEFECT AND WATCHING IT SURVIVE. With
# `store_build` reverted to composing `$ROOT/target` and ignoring the variable,
# this file still reported "the same answer under all 6 environments" - because
# `$ROOT/target` already held the example from an earlier build, so the defective
# path resolved to a STALE BINARY and the check answered normally.
#
# ⚠ SO THAT ROW FIRES ON A CLEAN CHECKOUT AND IS BLIND ON A HOST THAT HAS BUILT
# BEFORE, which is every host a contributor runs this on twice. The condition is
# reported rather than hidden: a reader who is told "the same answer under all 6"
# without being told one of the six could not have answered otherwise has been
# told something weaker than it sounds.
# ⛔ AND THE PATH IS ASKED FOR RATHER THAN COMPOSED, which is the rule `CI-01`
# wrote after that very defect - and this file's first draft broke it, on the one
# line that exists to talk about it. ⚠ The masking condition is strictly about
# where a DEFECTIVE `store_build` would look, so honouring the variable can
# report MASKED on a host that exports it while the row could in fact have fired.
# That error is in the safe direction: it understates this instrument's reach and
# never overstates it.
TARGET_EXAMPLES="${CARGO_TARGET_DIR:-$ROOT/target}/debug/examples"
if [ -d "$TARGET_EXAMPLES" ]; then
  pass "control   CARGO_TARGET_DIR is MASKED here: $TARGET_EXAMPLES already holds built examples"
else
  pass "control   CARGO_TARGET_DIR can fire here: $TARGET_EXAMPLES holds no built example"
fi

# ⚠ AND THE ONE THAT IS NOT AVAILABLE, STATED RATHER THAN DROPPED. A shell
# resets IFS at startup, so an exported one never reaches a script; a case
# setting it would report a guard proved by a value the child never saw.
# ⚠ SINGLE-QUOTED ON PURPOSE, and shellcheck is right to ask. `$IFS` has to be
# expanded by the CHILD shell, because the whole question is what the child sees;
# expanding it here would print this shell's own value and the case would pass
# over any child at all. `check-project.sh` disables the same rule for the same
# shape.
# shellcheck disable=SC2016
IFS_SEEN=$(env "IFS=:" sh -c 'printf "%s" "$IFS"' | od -An -c | tr -d ' \n')
if [ "$IFS_SEEN" = "\\t\\n" ]; then
  pass "control   IFS does not survive into a child, so it is not perturbable here"
else
  fail "control   IFS reached a child as [$IFS_SEEN]; this file says it cannot"
fi

# -- the subjects -------------------------------------------------------------
#
# ⚠ FAST, HERMETIC AND JSON-EMITTING, in that order of importance. This runs
# every subject once per environment, so a slow one is multiplied by the length
# of the list above; and a subject that touched the network would be measuring a
# vendor's uptime under six environments rather than this project's defaults.
#
# ⭐ `check-cache` IS IN THE LIST FOR ONE REASON: it is the only subject here
# that BUILDS an example and makes a scratch tree, which is what `TMPDIR` and
# `CARGO_TARGET_DIR` actually reach. A list of pure readers would have run the
# `CARGO_TARGET_DIR` case six times over checks that never build anything.
run_subject() { # label command...
  _label="$1"
  shift
  _base=$(answer_under "" "$@")
  case "$_base" in
    *'(no json)'*)
      fail "$_label  the baseline printed no machine-readable line, so nothing can be compared"
      return
      ;;
  esac
  : >"$WORK/verdicts"
  printf '%s\n' "$ENVIRONMENTS" | while IFS= read -r _assignment; do
    [ -n "$_assignment" ] || continue
    _under=$(answer_under "$_assignment" "$@")
    if [ "$_under" = "$_base" ]; then
      printf 'same\n' >>"$WORK/verdicts"
    else
      printf 'DIFF %s %s | baseline %s | under %s\n' \
        "$_label" "${_assignment%%=*}" "$_base" "$_under" >>"$WORK/verdicts"
    fi
  done
  # ⛔ THE LOOP ABOVE RUNS IN A SUBSHELL because it is the right-hand side of a
  # pipe, so a counter incremented inside it is lost when the pipe closes. That
  # is the oldest shell trap there is and the reason the verdicts go to a FILE.
  #
  # ⛔ AND `grep -c` IS NOT WRAPPED IN `|| printf 0`, WHICH IS HOW THIS FILE'S
  # FIRST RUN BROKE. `grep -c` over a file with no matches PRINTS `0` and EXITS
  # 1, so the fallback ran as well and the value became two lines - which went
  # into a row, so `store_report` counted fourteen rows against eight cases and
  # refused to describe itself. ⭐ That guard is why the defect was a red line
  # rather than a miscounted report.
  _diffs=$(grep -c '^DIFF' "$WORK/verdicts" 2>/dev/null)
  [ -n "$_diffs" ] || _diffs=0
  _total=$(grep -c . "$WORK/verdicts" 2>/dev/null)
  [ -n "$_total" ] || _total=0
  if [ "$_diffs" = "0" ]; then
    pass "$_label  the same answer under all $_total environment(s)"
  else
    fail "$_label  $_diffs of $_total environment(s) changed the answer"
    [ "$JSON" = "1" ] || grep '^DIFF' "$WORK/verdicts" | sed 's/^/          /'
  fi
  rm -f "$WORK/verdicts"
}

# ⛔ `check-docs` IS NOT IN THIS LIST AND THAT IS A COST DECISION, STATED.
# Every subject runs once per environment, so a one-second check costs seven
# seconds here and seventy inside `check-workflow`, which runs the whole gate
# about ten times and is already the CI wall clock. `check-docs` resolves links
# and parses fenced blocks - the least plausible subject for a locale or a
# scratch directory to reach - so it is the one dropped. ⚠ `check-markers`
# decodes UTF-8 BY HAND and stays, because that is the most plausible one.
#
# ⭐ IT IS THE GO BINARY NOW RATHER THAN AN `sh` HALF, and the question this
# harness asks is unchanged by that: a check that reads a value from its host
# without saying so answers differently under a different environment whatever
# language it is written in. ⚠ A Go program inherits a DIFFERENT set of host
# values than a shell script - it does not read `IFS`, and it resolves `TMPDIR`
# through the runtime rather than through a shell expansion - so this row is now
# asking the same question of a subject with its own answers, which is the reason
# to keep asking it rather than a reason to drop it.
# ⛔ Its absence is `could not run` and never a pass: without the binary the
# subject cannot be started, and `store_require` refuses the whole harness.
if [ -x "$WORK/bit-check" ] || (cd "$ROOT/tools/check" && go build -o "$WORK/bit-check" .) >/dev/null 2>&1; then
  run_subject "markers   " "$WORK/bit-check" check-markers --json
else
  fail "markers     tools/check did not build, so the ported subject could not be run"
fi
run_subject "licences  " "$WORK/bit-check" check-licences --json
run_subject "secrets   " sh "$ROOT/scripts/common/check-no-secrets.sh" --public --json
run_subject "project   " sh "$ROOT/scripts/common/check-project.sh" --json
run_subject "cache     " sh "$ROOT/scripts/acquisition/check-cache.sh" --json

store_report check-defaults/1 cases "$JSON"
