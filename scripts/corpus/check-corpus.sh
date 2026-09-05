#!/bin/sh
# check-corpus.sh - plant, in a real store on a disk, every defect the semantic
# corpus validator exists to refuse, and read the exit code from the process
# that produced it.
#
# CORPUS-01 asks whether a successor may replace a predecessor. This asks the
# other question: is a tree a coherent corpus at all. ⛔ The one that matters
# most is a citation nothing resolves. bind() compares the profile against the
# manifest, so a run that agreed with itself about an artifact nobody wrote
# satisfied every check this project had before CORPUS-02, and a profile whose
# evidence is unreachable is a parsed value with no recoverable bytes behind it.
#
# -- ⛔ THE STORE IS BUILT, NOT COMMITTED -----------------------------------
#
# The schema fixtures declare digests for artifacts that were never written, so
# they are not a corpus and cannot be one. `build-store` writes the artifacts and
# then rewrites each document to describe the bytes it actually put down, through
# to_json, which validates. That is the clean case here, and it has to be clean
# before any refusal below is attributable.
#
# -- ⛔ IT VERIFIES ITS OWN EDITS APPLIED ------------------------------------
#
# Every plant is bracketed by a digest of the whole tree, a literal replacement
# must match exactly once, and a plant that did not move the tree is reported
# NOT-PLANTED and counted as a failure rather than as a refusal that did not
# come. store-lib.sh carries those, and its own guards are exercised at the end.
#
# Usage:
#   sh scripts/corpus/check-corpus.sh
#   sh scripts/corpus/check-corpus.sh --json
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
      printf 'check-corpus: unknown argument: %s\n' "$1" >&2
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
ME=check-corpus
# shellcheck disable=SC1091
# shellcheck source=scripts/corpus/store-lib.sh
. "$HERE/store-lib.sh"

store_require cargo sha256sum
BUILDER=$(store_build "$ROOT" build-store) || exit 2
VALIDATOR=$(store_build "$ROOT" validate-corpus) || exit 2
LOCATOR=$(store_build "$ROOT" check-store) || exit 2

WORK=$(store_workdir checkcorpus) || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

STORE="$WORK/store"

# Paths inside the store, asked for rather than spelled here.
PROFILE_REL=$("$LOCATOR" --where "$FIXTURES/valid-profile.json") || exit 2
MANIFEST_REL=$("$LOCATOR" --where "$FIXTURES/valid-manifest.json") || exit 2
BUNDLE_REL=$(dirname "$MANIFEST_REL")

build_store() {
  rm -rf "$STORE"
  mkdir -p "$STORE" || return 1
  "$BUILDER" "$STORE" >/dev/null 2>&1
}

# ⛔ Unpiped. The output goes to a file and $? is read on the next line.
run_check() {
  "$VALIDATOR" "$STORE" >"$WORK/out" 2>&1
  RC=$?
}

if ! build_store; then
  printf 'check-corpus: cannot build the fixture store\n' >&2
  exit 2
fi

# ⛔ The clean case first. A harness whose clean case is not green is measuring
# its own setup, and every refusal after it would be unattributable.
run_check
if [ "$RC" = "0" ]; then
  pass "clean    a record, its run, and every artifact they cite"
else
  fail "clean    the unmutated store was refused (exit $RC): $(head -3 "$WORK/out" | tr '\n' ' ')"
fi

# ⚠ The clean store's shape is pinned, because every case below takes exactly
# one thing away from it. A build_store that quietly stopped writing artifacts
# would make all of them pass over an almost empty tree.
OBJECTS=$(tree_files "$STORE")
if [ "$OBJECTS" = "11" ]; then
  pass "clean    the store holds 11 objects: one record, one run, nine artifacts"
else
  fail "clean    the store holds $OBJECTS objects, expected 11"
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

  run_check
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
  run_check
  if [ "$RC" != "0" ]; then
    fail "$CASE_NAME  the restored store is not clean (exit $RC)"
    return 0
  fi
  pass "$CASE_CODE  $CASE_NAME"
}

begin_case "a record with no run" "E-CRP-01"
rm -f "$STORE/$MANIFEST_REL"
end_case $?

begin_case "a run no record cites" "E-CRP-02"
rm -f "$STORE/$PROFILE_REL"
end_case $?

# ⛔ The invariant this entry exists for. Both documents still agree about the
# artifact; the bytes are simply not there.
begin_case "a citation nothing resolves" "E-CRP-03"
rm -f "$STORE/$BUNDLE_REL/observer/events.jsonl"
end_case $?

begin_case "an artifact longer than declared" "E-CRP-04"
printf 'appended\n' >>"$STORE/$BUNDLE_REL/observer/events.jsonl"
end_case $?

# ⚠ Same length, different bytes, so this reaches the digest comparison rather
# than the length one. Appending would fire both and prove neither on its own.
begin_case "an artifact edited without changing its length" "E-CRP-05"
replace_once "$STORE/$BUNDLE_REL/observer/events.jsonl" "artifact" "artifacX"
end_case $?

begin_case "a file the run does not declare" "E-CRP-06"
printf 'not declared anywhere\n' >"$STORE/$BUNDLE_REL/stray.log"
end_case $?

# A correction whose target is not in the store: a chain with a hole in it.
begin_case "a correction naming an absent record" "E-CRP-07"
place "$LOCATOR" "$STORE" "$FIXTURES/valid-correction.json" >/dev/null &&
  rm -f "$STORE/$PROFILE_REL"
end_case $?

# The store's own placement rule, reached through the corpus reader rather than
# through check-store, because a rule enforced on one of two paths into the same
# action is the hole this project keeps finding.
begin_case "a record filed at another platform" "E-STO-30"
WRONG="profiles/v1/fixture-client/0.0.0-fixture/windows/x86-64/tar-gz/fixture-capture-0001.json"
mkdir -p "$STORE/$(dirname "$WRONG")" && mv "$STORE/$PROFILE_REL" "$STORE/$WRONG"
end_case $?

build_store >/dev/null 2>&1
store_probe_guards "$STORE/$PROFILE_REL" "fixture-client" "sha256:"

store_report check-corpus/1 cases "$JSON"
