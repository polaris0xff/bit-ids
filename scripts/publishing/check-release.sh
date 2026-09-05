#!/bin/sh
# check-release.sh - assemble a release twice, compare the bytes, and plant every
# defect the assembler exists to refuse.
#
# PUB-01's Prove is that two independent assembly runs produce byte-identical
# output. ⛔ That is not a tautology: the assembler walks a directory and a map,
# and either could hand it a different order on a second pass. Nor is it enough
# on its own, which is why the checksums are also handed to sha256sum -c: a
# writer that reports the digest of what it meant to write cannot detect a short
# write, and comparing two of its own summaries would not either.
#
# ⭐ THE INDEPENDENT VERIFIER IS THE POINT OF THAT LAST CASE. sha256sum is not
# this project's code and does not share its reading of anything.
#
# -- ⛔ IT VERIFIES ITS OWN EDITS APPLIED -----------------------------------
#
# ../corpus/store-lib.sh carries the plant verification and its own four
# self-guards. ⚠ It lives under corpus/ because that is where the first harness
# to need it was; scripts/README.md says so rather than leaving a reader to
# wonder why a publishing check sources a corpus file.
#
# Usage:
#   sh scripts/publishing/check-release.sh
#   sh scripts/publishing/check-release.sh --json
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
      printf 'check-release: unknown argument: %s\n' "$1" >&2
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
# line. shellcheck cannot see across a source it is not told to follow, so it
# reads as unused unless every file is handed to one invocation.
# shellcheck disable=SC2034
ME=check-release
# shellcheck disable=SC1091
# shellcheck source=scripts/corpus/store-lib.sh
. "$ROOT/scripts/corpus/store-lib.sh"

store_require cargo sha256sum
BUILDER=$(store_build "$ROOT" build-store) || exit 2
INDEXER=$(store_build "$ROOT" build-indexes) || exit 2
ASSEMBLER=$(store_build "$ROOT" assemble-release) || exit 2

WORK=$(store_workdir checkrelease) || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

TREE="$WORK/tree"

# A tree holding everything a release carries except its own two descriptions.
build_tree() {
  rm -rf "$TREE"
  mkdir -p "$TREE/indexes/v1" || return 1
  "$BUILDER" --version 1.2.3 --version 1.2.10 "$TREE" >/dev/null 2>&1 || return 1
  "$INDEXER" --scheme fixture-client:-:3:3 "$TREE" "$TREE/indexes/v1/profiles.json" \
    >/dev/null 2>&1 || return 1
  cp "$ROOT/LICENSE" "$TREE/LICENSE"
}

# ⛔ Unpiped. The output goes to a file and $? is read on the next line.
run_assemble() {
  "$ASSEMBLER" "$TREE" >"$WORK/out" 2>&1
  RC=$?
}

if ! build_tree; then
  printf 'check-release: cannot build the fixture tree\n' >&2
  exit 2
fi

run_assemble
if [ "$RC" = "0" ]; then
  pass "clean    a store, its indexes and a licence assemble and read back"
else
  fail "clean    the unmutated tree was refused (exit $RC): $(head -3 "$WORK/out" | tr '\n' ' ')"
fi

# ⛔ THE PROVE. Two independent assemblies of one input, compared as bytes.
# ⚠ Two trees rather than two runs over one, because assembling writes into the
# tree and the second run would then see its own output.
build_tree >/dev/null 2>&1
"$ASSEMBLER" "$TREE" >/dev/null 2>&1
cp "$TREE/MANIFEST.json" "$WORK/first.json"
cp "$TREE/SHA256SUMS" "$WORK/first.sums"
build_tree >/dev/null 2>&1
"$ASSEMBLER" "$TREE" >/dev/null 2>&1
if cmp -s "$WORK/first.json" "$TREE/MANIFEST.json" &&
  cmp -s "$WORK/first.sums" "$TREE/SHA256SUMS"; then
  pass "determinism  two assemblies of one input are byte-identical"
else
  fail "determinism  two assemblies differ"
fi

DESCRIBED=$(grep -c '"path"' "$WORK/first.json")
if [ "$DESCRIBED" -ge 20 ]; then
  pass "determinism  the manifest describes $DESCRIBED files, so the comparison had something to compare"
else
  fail "determinism  the manifest describes only $DESCRIBED files"
fi

# ⭐ THE INDEPENDENT VERIFIER. sha256sum is not this project's code.
if (cd "$TREE" && sha256sum -c SHA256SUMS --quiet) >/dev/null 2>&1; then
  pass "checksums  sha256sum -c agrees with every row, MANIFEST.json included"
else
  fail "checksums  sha256sum -c disagreed"
fi

SETUP_OK=0

begin_case() { # name expected-code
  CASE_NAME="$1"
  CASE_CODE="$2"
  if build_tree; then
    CASE_BEFORE=$(tree_digest "$TREE")
    SETUP_OK=1
  else
    fail "$CASE_NAME  could not build the tree"
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
  if [ "$CASE_BEFORE" = "$(tree_digest "$TREE")" ]; then
    fail "$CASE_NAME  NOT-PLANTED (the tree digest did not move)"
    return 0
  fi

  run_assemble
  if [ "$RC" != "1" ]; then
    fail "$CASE_NAME  expected exit 1, got $RC"
    return 0
  fi
  if ! grep -q -F -e "$CASE_CODE" "$WORK/out"; then
    fail "$CASE_NAME  refused, but not as $CASE_CODE: $(head -3 "$WORK/out" | tr '\n' ' ')"
    return 0
  fi

  if ! build_tree; then
    fail "$CASE_NAME  could not restore the tree"
    return 0
  fi
  run_assemble
  if [ "$RC" != "0" ]; then
    fail "$CASE_NAME  the restored tree is not clean (exit $RC)"
    return 0
  fi
  pass "$CASE_CODE  $CASE_NAME"
}

begin_case "a file with no known media type" "E-REL-01"
printf 'notes\n' >"$TREE/notes.unknown"
end_case $?

begin_case "a symbolic link in a release" "E-REL-02"
ln -s "$TREE/LICENSE" "$TREE/indexes/v1/link.json"
end_case $?

begin_case "a published file with no bytes" "E-REL-03"
: >"$TREE/indexes/v1/empty.json"
end_case $?

# ⛔ A tree already carrying either document describes another run, and
# publishing over it would put one run's manifest beside another's files.
begin_case "a tree already carrying a manifest" "E-REL-04"
printf '{"stale": true}\n' >"$TREE/MANIFEST.json"
end_case $?

# An empty release publishes nothing and reports success, which is the shape a
# pipeline produces when its input directory was a typo.
mkdir -p "$WORK/empty"
"$ASSEMBLER" "$WORK/empty" >"$WORK/out" 2>&1
RC=$?
if [ "$RC" = "1" ] && grep -q -F -e "E-REL-05" "$WORK/out"; then
  pass "E-REL-05  an empty release"
else
  fail "E-REL-05  expected exit 1 with E-REL-05, got $RC: $(head -2 "$WORK/out" | tr '\n' ' ')"
fi

build_tree >/dev/null 2>&1
store_probe_guards "$TREE/LICENSE" "Permission" "e"

store_report check-release/1 cases "$JSON"
