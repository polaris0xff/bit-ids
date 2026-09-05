#!/bin/sh
# check-indexes.sh - prove the two things a derived file has to be: byte-identical
# between clean builds, and resolvable back to the records it came from.
#
# CORPUS-03's Prove is exactly those two, and neither is a tautology. ⛔ The
# builder walks maps, and a map's iteration order is not a contract a consumer
# can check, so "two builds agree" is a real question. And an index is read
# INSTEAD of the records, so a row naming a record nobody can open is an answer
# with nothing behind it.
#
# -- ⛔ THE STORE CARRIES THREE VERSIONS ON PURPOSE -------------------------
#
# 1.2.3, 1.2.9 and 1.2.10. As text, 1.2.9 sorts last; numerically it does not.
# A latest view that answered 1.2.9 would point a consumer at a superseded build
# with complete confidence, and that is what this harness plants against.
#
# ⚠ The fixture's own version, 0.0.0-fixture, is not orderable under any numeric
# scheme, so build-store is asked for versions that are. That the shipped
# fixture blocks the view is itself a case below.
#
# -- ⛔ IT VERIFIES ITS OWN EDITS APPLIED -----------------------------------
#
# store-lib.sh carries the plant verification and its own four self-guards.
#
# Usage:
#   sh scripts/corpus/check-indexes.sh
#   sh scripts/corpus/check-indexes.sh --json
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
      printf 'check-indexes: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

# ⛔ Resolved from this script's own location, never from the working directory.
HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH='' cd -- "$HERE/../.." && pwd)

# ⚠ ME is set here and read by store-lib.sh, which this sources on the next
# line. shellcheck cannot see across a source it is not told to follow, so it
# reads as unused unless every file is handed to one invocation.
# ⛔ The disable is per file rather than left to that: a lint whose verdict
# depends on how the arguments were grouped is a lint that answers differently
# for CI and for the contributor checking one file.
# shellcheck disable=SC2034
ME=check-indexes
# shellcheck disable=SC1091
# shellcheck source=scripts/corpus/store-lib.sh
. "$HERE/store-lib.sh"

store_require cargo sha256sum
BUILDER=$(store_build "$ROOT" build-store) || exit 2
INDEXER=$(store_build "$ROOT" build-indexes) || exit 2

WORK=$(store_workdir checkindexes) || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

STORE="$WORK/store"
SCHEME="fixture-client:-:3:3"

build_store() {
  rm -rf "$STORE"
  mkdir -p "$STORE" || return 1
  "$BUILDER" --version 1.2.3 --version 1.2.10 --version 1.2.9 "$STORE" >/dev/null 2>&1
}

# ⛔ Unpiped. The output goes to a file and $? is read on the next line.
run_indexes() {
  "$INDEXER" --scheme "$SCHEME" "$STORE" >"$WORK/out" 2>&1
  RC=$?
}

if ! build_store; then
  printf 'check-indexes: cannot build the fixture store\n' >&2
  exit 2
fi

run_indexes
if [ "$RC" = "0" ]; then
  pass "clean    a three-version store indexes and every row resolves"
else
  fail "clean    the unmutated store was refused (exit $RC): $(head -3 "$WORK/out" | tr '\n' ' ')"
fi

# ⛔ THE PROVE'S FIRST HALF. Two clean builds, compared as bytes rather than as
# a summary line, because a digest printed by the thing under test is a claim
# and the document is the artefact.
"$INDEXER" --scheme "$SCHEME" "$STORE" "$WORK/first.json" >/dev/null 2>&1
FIRST_RC=$?
"$INDEXER" --scheme "$SCHEME" "$STORE" "$WORK/second.json" >/dev/null 2>&1
SECOND_RC=$?
if [ "$FIRST_RC" != "0" ] || [ "$SECOND_RC" != "0" ]; then
  fail "determinism  a build failed ($FIRST_RC, $SECOND_RC)"
elif cmp -s "$WORK/first.json" "$WORK/second.json"; then
  pass "determinism  two clean builds are byte-identical"
else
  fail "determinism  two clean builds differ"
fi

# ⚠ And the document is not empty. Two builds of nothing are also identical,
# which is the shape a determinism check passes vacuously in.
DOCUMENT_ROWS=$(grep -c '"record"' "$WORK/first.json")
if [ "$DOCUMENT_ROWS" -ge 18 ]; then
  pass "determinism  the document carries $DOCUMENT_ROWS rows, so the comparison had something to compare"
else
  fail "determinism  the document carries only $DOCUMENT_ROWS rows"
fi

# ⛔ THE ORDERING, WHICH IS THE ONE A TEXT SORT GETS WRONG.
if grep -q '"version": "1.2.10"' "$WORK/first.json"; then
  pass "ordering  the latest view answers 1.2.10 over 1.2.9"
else
  fail "ordering  the latest view did not answer 1.2.10: $(grep -o '"version": "[^"]*"' "$WORK/first.json" | tail -1)"
fi

SETUP_OK=0

begin_case() { # name expected-code
  CASE_NAME="$1"
  CASE_CODE="$2"
  if build_store; then
    CASE_BEFORE=$(tree_digest "$STORE")
    SETUP_OK=1
  else
    fail "$CASE_NAME  could not build the store"
    SETUP_OK=0
  fi
}

end_case() { # the mutation's own exit status
  _mutation_rc="$1"
  [ "$SETUP_OK" = "1" ] || return 0

  if [ "$_mutation_rc" != "0" ]; then
    fail "$CASE_NAME  NOT-PLANTED (the mutation reported it did not apply)"
    return 0
  fi
  if [ "$CASE_BEFORE" = "$(tree_digest "$STORE")" ]; then
    fail "$CASE_NAME  NOT-PLANTED (the tree digest did not move)"
    return 0
  fi

  run_indexes
  if [ "$RC" != "1" ]; then
    fail "$CASE_NAME  expected exit 1, got $RC"
    return 0
  fi
  if ! grep -q -F -e "$CASE_CODE" "$WORK/out"; then
    fail "$CASE_NAME  refused, but not as $CASE_CODE: $(head -3 "$WORK/out" | tr '\n' ' ')"
    return 0
  fi

  if ! build_store; then
    fail "$CASE_NAME  could not restore the store"
    return 0
  fi
  run_indexes
  if [ "$RC" != "0" ]; then
    fail "$CASE_NAME  the restored store is not clean (exit $RC)"
    return 0
  fi
  pass "$CASE_CODE  $CASE_NAME"
}

# A record the scheme cannot order blocks the view rather than being skipped.
# ⚠ Planted by adding the shipped fixture, whose version is 0.0.0-fixture.
begin_case "a version the scheme cannot order" "E-VIW-02"
"$BUILDER" "$STORE" >/dev/null 2>&1
end_case $?

# ⛔ Asked without a scheme at all, which is a different refusal from a scheme
# that cannot order: one is a target nobody declared, the other is a version the
# declaration does not cover.
build_store >/dev/null 2>&1
"$INDEXER" "$STORE" >"$WORK/out" 2>&1
RC=$?
if [ "$RC" = "1" ] && grep -q -F -e "E-VIW-01" "$WORK/out"; then
  pass "E-VIW-01  a target with no declared version scheme"
else
  fail "E-VIW-01  expected exit 1 with E-VIW-01, got $RC: $(head -2 "$WORK/out" | tr '\n' ' ')"
fi

build_store >/dev/null 2>&1
PROBE=$(find "$STORE/profiles" -name '*.json' | LC_ALL=C sort | head -1)
store_probe_guards "$PROBE" "fixture-client" "sha256:"

store_report check-indexes/1 cases "$JSON"
