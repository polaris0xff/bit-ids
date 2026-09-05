#!/bin/sh
# check-store.sh - plant, in a disposable tree, every defect the append-only
# store exists to refuse, and read the exit code from the process that produced
# it.
#
# CORPUS-01's guards stand between this project and silently destroying
# published evidence. A store that regenerated a latest-only view would erase
# the older stable releases the catalogue exists to have measured, and the loss
# is not recoverable afterwards because the deleted record was never anywhere
# else. ⛔ A guard of that class that has never been seen to refuse is theatre,
# so this plants each defect and reads the refusal.
#
# -- ⛔ IT VERIFIES ITS OWN EDITS APPLIED ------------------------------------
#
# A mutation harness that does not check its plant landed reports the guards
# failing to fire over unmutated source, and it reads exactly like a green run.
# This project has been burned by that three times. So every plant is bracketed
# by a digest of the whole tree, a literal replacement must match EXACTLY ONCE
# before it is made, and a plant that did not move the tree is reported as
# NOT-PLANTED and counted as a failure rather than as a refusal that did not
# come.
#
# ⭐ Those three guards are themselves exercised, at the end, with an absent
# literal, an ambiguous one and a no-op edit. A probe's guard is a guard like
# any other.
#
# -- ⚠ WHAT THIS CANNOT PLANT -----------------------------------------------
#
# Three refusals are unreachable through a filesystem and are proved by the
# unit tests beside the module instead, which is where a tree can be assembled
# that a disk cannot hold:
#
#   E-STO-12  a path that is a file here and a directory there. A filesystem
#             refuses the state outright; a tar or a zip carries it happily,
#             and PUB-01 assembles both.
#   E-STO-22  one digest over two lengths. Walking takes the size and the digest
#             off the same bytes, so it cannot produce the pair; a tree built
#             out of a manifest can, because there the length is a second copy.
#   E-STO-01  a version that cannot be a path segment, and E-STO-04 a composed
#   E-STO-04  path over the canonical ceiling. Both are refusals of the
#             derivation, so no tree carrying them can be built to walk.
#
# Usage:
#   sh scripts/corpus/check-store.sh
#   sh scripts/corpus/check-store.sh --json
#
# Exit codes: 0 every guard refused its defect, 1 one did not, 2 could not run.
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
      printf 'check-store: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

# ⛔ Resolved from this script's own location, never from the working directory.
HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH='' cd -- "$HERE/../.." && pwd)
FIXTURES="$ROOT/crates/bit-ids/tests/fixtures"

# ⚠ ME is set here and read by store-lib.sh, which this sources on the next
# line. shellcheck cannot see across a source it is not told to follow, so it
# reads as unused unless every file is handed to one invocation.
# ⛔ The disables are per file rather than left to that: a lint whose verdict
# depends on how the arguments were grouped is a lint that answers differently
# for CI and for the contributor checking one file.
# shellcheck disable=SC2034
ME=check-store
# shellcheck disable=SC1091
# shellcheck source=scripts/corpus/store-lib.sh
. "$HERE/store-lib.sh"

store_require cargo sha256sum mkfifo
BIN=$(store_build "$ROOT" check-store) || exit 2

WORK=$(store_workdir checkstore) || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

# published/ is one record and its manifest. proposed/ adds the correction that
# supersedes the record, which is the shape the store is for: a correction
# appends and the record it corrects is left exactly as it was.
build_trees() {
  rm -rf "$WORK/published" "$WORK/proposed"
  mkdir -p "$WORK/published" "$WORK/proposed"
  PROFILE_REL=$(place "$BIN" "$WORK/published" "$FIXTURES/valid-profile.json") || return 1
  place "$BIN" "$WORK/published" "$FIXTURES/valid-manifest.json" >/dev/null || return 1
  cp -R "$WORK/published/." "$WORK/proposed/" || return 1
  CORRECTION_REL=$(place "$BIN" "$WORK/proposed" "$FIXTURES/valid-correction.json") || return 1
  return 0
}

# ⛔ Unpiped. The output goes to a file and $? is read on the next line.
run_check() {
  "$BIN" "$WORK/published" "$WORK/proposed" >"$WORK/out" 2>&1
  RC=$?
}

if ! build_trees; then
  printf 'check-store: cannot assemble the fixture trees\n' >&2
  exit 2
fi

# The clean case first. ⛔ A harness whose clean case is not green is measuring
# its own setup, and every refusal after it would be unattributable.
run_check
if [ "$RC" = "0" ]; then
  pass "clean    a correction appends and the record it corrects is untouched"
else
  fail "clean    the unmutated tree was refused (exit $RC): $(head -3 "$WORK/out" | tr '\n' ' ')"
fi

# Each case is three explicit steps: begin_case rebuilds the trees and records
# the tree digest, the mutation runs inline, and end_case verifies the plant
# landed, runs the check, requires the exit code AND the named refusal, then
# rebuilds and requires the clean case again.
#
# ⚠ The mutations are written inline rather than as functions called through a
# variable. Indirection here buys nothing and costs the reader the ability to
# see, at the call site, exactly which bytes each case moves.
SETUP_OK=0

begin_case() { # name expected-code
  CASE_NAME="$1"
  CASE_CODE="$2"
  if build_trees; then
    CASE_BEFORE=$(tree_digest "$WORK/proposed")
    SETUP_OK=1
  else
    fail "$CASE_NAME  could not assemble the trees"
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
  if [ "$CASE_BEFORE" = "$(tree_digest "$WORK/proposed")" ]; then
    fail "$CASE_NAME  NOT-PLANTED (the tree digest did not move)"
    return 0
  fi

  run_check
  if [ "$RC" != "1" ]; then
    fail "$CASE_NAME  expected exit 1, got $RC"
    return 0
  fi
  if ! grep -q -F -e "$CASE_CODE" "$WORK/out"; then
    fail "$CASE_NAME  refused, but not as $CASE_CODE: $(head -3 "$WORK/out" | tr '\n' ' ')"
    return 0
  fi

  if ! build_trees; then
    fail "$CASE_NAME  could not restore the trees"
    return 0
  fi
  run_check
  if [ "$RC" != "0" ]; then
    fail "$CASE_NAME  the restored tree is not clean (exit $RC)"
    return 0
  fi
  pass "$CASE_CODE  $CASE_NAME"
}

begin_case "a published record is deleted" "E-STO-20"
rm -f "$WORK/proposed/$PROFILE_REL"
end_case $?

begin_case "a published record is edited" "E-STO-21"
replace_once "$WORK/proposed/$PROFILE_REL" \
  "Schema Fixture Client" "Schema Fixture Cliant"
end_case $?

begin_case "a published record becomes a link" "E-STO-21"
rm -f "$WORK/proposed/$PROFILE_REL" &&
  ln -s "$WORK/published/$PROFILE_REL" "$WORK/proposed/$PROFILE_REL"
end_case $?

begin_case "a record is filed at another platform" "E-STO-30"
WRONG="profiles/v1/fixture-client/0.0.0-fixture/windows/x86-64/tar-gz/fixture-capture-0002.json"
mkdir -p "$WORK/proposed/$(dirname "$WRONG")" &&
  mv "$WORK/proposed/$CORRECTION_REL" "$WORK/proposed/$WRONG"
end_case $?

# ⚠ This plant is only real on a case-sensitive filesystem. The file count is
# what says so: on a case-insensitive host the copy lands on the original and
# the count does not move, which reports NOT-PLANTED rather than reporting a
# guard that did not fire.
begin_case "two records differ only in case" "E-STO-10"
UPPER="profiles/v1/fixture-client/0.0.0-fixture/linux/x86-64/tar-gz/Fixture-capture-0001.json"
FILES_BEFORE=$(tree_files "$WORK/proposed")
cp "$WORK/proposed/$PROFILE_REL" "$WORK/proposed/$UPPER" &&
  [ "$(tree_files "$WORK/proposed")" != "$FILES_BEFORE" ]
end_case $?

begin_case "a segment is a Windows device" "E-STO-11"
RESERVED="$WORK/proposed/raw/v1/fixture-client/0.0.0-fixture/linux/x86-64/tar-gz/nul"
mkdir -p "$RESERVED" && printf 'x\n' >"$RESERVED/a.json"
end_case $?

LEAF="$WORK/proposed/profiles/v1/fixture-client/0.0.0-fixture/linux/x86-64/tar-gz"

begin_case "an artifact has no bytes" "E-STO-13"
: >"$LEAF/empty.json"
end_case $?

begin_case "an artifact is a symbolic link" "E-STO-14"
ln -s "$WORK/published/$PROFILE_REL" "$LEAF/link.json"
end_case $?

begin_case "an artifact is a named pipe" "E-STO-15"
mkfifo "$LEAF/pipe.json"
end_case $?

# ⭐ The harness's own guards, exercised. A probe's guard is a guard like any
# other, and a harness that reports a plant it did not make reports every guard
# above as failing to fire.
build_trees >/dev/null 2>&1
store_probe_guards "$WORK/proposed/$PROFILE_REL" "Schema Fixture Client" "sha256:"

store_report check-store/1 cases "$JSON"
