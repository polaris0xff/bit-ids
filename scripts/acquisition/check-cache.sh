#!/bin/sh
# check-cache.sh - does the artifact cache survive a source that moved, and does
# it keep bytes only where the licence register permits?
#
# `ACQ-05`'s Prove is both halves at once: the cache tests enforce the licence
# register, and the artifact's identity survives a source URL change.
#
# -- ⭐ THE PERMITTED SET COMES FROM THE REGISTER, NOT FROM HERE ------------
#
# `catalogue/licences.toml` has one parser per twin and it is `check-licences`.
# This harness asks that check what the register permits and hands the answer to
# the scenario, so the tie between `FOUND-04`'s register and this cache is a
# call rather than a second reading. ⚠ Today it answers nothing, which is the
# state the first case asserts rather than assumes.
#
# ⛔ AND THAT MAKES THE REFUSAL CASE AMBIGUOUS ON ITS OWN. A cache that refused
# to keep any bytes for any reason would pass a case built only on the empty
# answer. The third case hands the scenario a target explicitly and watches the
# same cache accept it, so the refusal is shown to be the register's answer
# rather than an inability to store at all.
#
# ⚠ NOTHING HERE PLANTS IN A TRACKED FILE. The register is checked out, not
# scratch, and a gate check that edits one leaves a dirty tree behind when it is
# interrupted. That the flag can report a row was measured against a planted
# register while closing the entry, and `TODO/acquisition.md` records it.
#
# Usage:
#   sh scripts/acquisition/check-cache.sh
#   sh scripts/acquisition/check-cache.sh --json
#
# Exit codes: 0 every case held, 1 one did not, 2 could not run.
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
      printf 'check-cache: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

# ⛔ Resolved from this script's own location, never from the working directory.
HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH='' cd -- "$HERE/.." && pwd)
ROOT=$(CDPATH='' cd -- "$ROOT/.." && pwd)

# ⚠ ME is set here and read by store-lib.sh, which this sources on the next
# line. shellcheck cannot see across a source it is not told to follow.
# shellcheck disable=SC2034
ME=check-cache
# shellcheck disable=SC1091
# shellcheck source=scripts/corpus/store-lib.sh
. "$ROOT/scripts/corpus/store-lib.sh"

store_require cargo
SCENARIO=$(store_build "$ROOT" cache-scenario) || exit 2

WORK=$(store_workdir checkcache) || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

LICENCES="$ROOT/scripts/common/check-licences.sh"
[ -f "$LICENCES" ] || {
  printf 'check-cache: %s is missing\n' "$LICENCES" >&2
  exit 2
}

# ⛔ Unpiped, and the answer kept in a file rather than a variable, so an empty
# answer and a failed call are distinguishable.
sh "$LICENCES" --permitted >"$WORK/permitted" 2>"$WORK/permitted.err"
PERMITTED_RC=$?
if [ "$PERMITTED_RC" != "0" ]; then
  printf 'check-cache: check-licences --permitted exited %s\n' "$PERMITTED_RC" >&2
  exit 2
fi

PERMITTED_COUNT=$(wc -l <"$WORK/permitted" | tr -d ' ')
if [ "$PERMITTED_COUNT" = "0" ]; then
  pass "register  nothing in the register permits keeping an artifact's bytes"
else
  fail "register  the register permits $PERMITTED_COUNT target(s): $(tr '\n' ' ' <"$WORK/permitted")"
fi

# The scenario, run with exactly what the register said.
ARGS=""
while read -r id; do
  [ -n "$id" ] || continue
  ARGS="$ARGS --permitted $id"
done <"$WORK/permitted"

# shellcheck disable=SC2086
# ARGS is built from the register's own output, one flag and one id at a time,
# and is deliberately word-split.
"$SCENARIO" $ARGS >"$WORK/refused.out" 2>"$WORK/refused.err"
REFUSED_RC=$?

if [ "$REFUSED_RC" = "0" ]; then
  pass "scenario  the cache behaves as the model says under the register's answer"
else
  fail "scenario  exit $REFUSED_RC: $(head -3 "$WORK/refused.err" | tr '\n' ' ')"
fi

# ⛔ THE IDENTITY HALF OF THE PROVE. One artifact and two retrievals means the
# digest named the same thing from both locations.
if grep -q -F -e 'artifacts: 1' "$WORK/refused.out" &&
  grep -q -F -e 'retrievals: 2' "$WORK/refused.out"; then
  pass "identity  a moved source adds a retrieval and not an artifact"
else
  fail "identity  a moved source produced: $(grep -E '^(artifacts|retrievals):' "$WORK/refused.out" | tr '\n' ' ')"
fi

if grep -q -F -e 'keeping nothing: accepted' "$WORK/refused.out"; then
  pass "policy    a cache that keeps no bytes is accepted"
else
  fail "policy    a cache that keeps no bytes was refused"
fi

if grep -q -F -e 'keeping the bytes: refused as E-CAC-01' "$WORK/refused.out"; then
  pass "E-CAC-01  keeping the bytes is refused while the register refuses them"
else
  fail "E-CAC-01  the refusal did not appear: $(tail -2 "$WORK/refused.out" | tr '\n' ' ')"
fi

# ⛔ THE CONTROL THE REFUSAL NEEDS. Without this the case above passes over a
# cache that can never store anything, which is a different program from one
# that asks the register.
"$SCENARIO" --permitted aria2 >"$WORK/permitted.out" 2>"$WORK/permitted.err2"
PERMITTED_RUN_RC=$?
if [ "$PERMITTED_RUN_RC" != "0" ]; then
  fail "control   the permitted run exited $PERMITTED_RUN_RC: $(head -2 "$WORK/permitted.err2" | tr '\n' ' ')"
elif grep -q -F -e 'keeping the bytes: permitted by the register' "$WORK/permitted.out"; then
  pass "control   the same cache keeps the bytes when a target is permitted"
else
  fail "control   the permitted run did not keep the bytes"
fi

# ⚠ And the two runs differ only in that line, so the refusal is about the
# permission and not about anything else the scenario did.
if diff "$WORK/refused.out" "$WORK/permitted.out" >"$WORK/diff" 2>&1; then
  fail "control   the refused and permitted runs are identical, so nothing changed"
elif [ "$(grep -c '^[<>]' "$WORK/diff")" = "2" ]; then
  pass "control   the two runs differ on exactly the permission line"
else
  fail "control   the two runs differ in $(grep -c '^[<>]' "$WORK/diff") line(s), expected 2"
fi

# ⚠ The probe guards need a file to plant in, and it is a copy rather than the
# register itself.
cp "$ROOT/catalogue/licences.toml" "$WORK/register.toml" || exit 2
store_probe_guards "$WORK/register.toml" 'artifact_policy = "measurements-only"' 'redistribute'

store_report check-cache/1 cases "$JSON"
