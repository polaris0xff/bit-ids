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

for tool in cargo sha256sum mkfifo; do
  command -v "$tool" >/dev/null 2>&1 || {
    printf 'check-store: %s not found\n' "$tool" >&2
    exit 2
  }
done

BIN="$ROOT/target/debug/examples/check-store"
if ! cargo build --manifest-path "$ROOT/Cargo.toml" -p bit-ids --locked \
  --example check-store >/dev/null 2>&1; then
  printf 'check-store: cannot build the example\n' >&2
  exit 2
fi
[ -x "$BIN" ] || {
  printf 'check-store: %s is not executable after a successful build\n' "$BIN" >&2
  exit 2
}

WORK="${TMPDIR:-/tmp}/.checkstore.$$"
mkdir -p "$WORK" || {
  printf 'check-store: cannot write to %s\n' "$WORK" >&2
  exit 2
}
trap 'rm -rf "$WORK"' EXIT INT TERM

PASS=0
FAIL=0
ROWS=""

row() {
  ROWS="$ROWS  $1
"
}

fail() {
  row "❌ $1"
  FAIL=$((FAIL + 1))
}

pass() {
  row "✅ $1"
  PASS=$((PASS + 1))
}

# ⛔ THE LAYOUT IS ASKED FOR, NEVER SPELLED HERE. A second copy of the path
# derivation in this file is the drift check-twins.sh exists to catch, and it
# would drift in the direction that makes every case below pass over a tree the
# store would never have written.
place() { # tree record
  _rel=$("$BIN" --where "$2")
  [ -n "$_rel" ] || return 1
  mkdir -p "$1/$(dirname "$_rel")" || return 1
  cp "$2" "$1/$_rel" || return 1
  printf '%s\n' "$_rel"
}

# A digest over every path in a tree and what sits at it, so a plant that did
# not land is visible as a digest that did not move.
tree_digest() {
  (
    cd "$1" 2>/dev/null || exit 1
    find . \( -type f -o -type l -o -type p \) | LC_ALL=C sort | while read -r p; do
      if [ -L "$p" ]; then
        printf '%s symlink\n' "$p"
      elif [ -p "$p" ]; then
        printf '%s fifo\n' "$p"
      else
        printf '%s %s\n' "$p" "$(sha256sum "$p" | cut -d' ' -f1)"
      fi
    done
  ) | sha256sum | cut -d' ' -f1
}

tree_files() {
  find "$1" \( -type f -o -type l -o -type p \) | wc -l | tr -d ' '
}

# ⛔ EXACTLY ONCE, OR NOT AT ALL. A literal that matches twice edits something
# other than what the case names, and one that matches nothing edits nothing
# while the case still reports a guard that failed to fire.
#
# ⚠ SINGLE-LINE LITERALS ONLY, AND THAT IS CHECKED RATHER THAN ASSUMED. grep -F
# splits a pattern containing a newline into separate alternatives, so a unique
# multi-line literal counts as the sum of its lines and this function would
# report it ambiguous. Measured while writing this entry's review pass, where
# exactly that miscounted three plants as NOT-PLANTED. Refusing one outright is
# honest; counting one wrongly is the defect this function exists to prevent.
replace_once() { # file literal replacement
  case "$2" in
    *"
"*)
      return 1
      ;;
  esac
  _hits=$(grep -o -F -e "$2" "$1" 2>/dev/null | wc -l | tr -d ' ')
  [ "$_hits" = "1" ] || return 1
  _before=$(sha256sum "$1" | cut -d' ' -f1)
  sed -i "s/$2/$3/" "$1" || return 1
  _after=$(sha256sum "$1" | cut -d' ' -f1)
  [ "$_before" != "$_after" ] || return 1
  return 0
}

# published/ is one record and its manifest. proposed/ adds the correction that
# supersedes the record, which is the shape the store is for: a correction
# appends and the record it corrects is left exactly as it was.
build_trees() {
  rm -rf "$WORK/published" "$WORK/proposed"
  mkdir -p "$WORK/published" "$WORK/proposed"
  PROFILE_REL=$(place "$WORK/published" "$FIXTURES/valid-profile.json") || return 1
  place "$WORK/published" "$FIXTURES/valid-manifest.json" >/dev/null || return 1
  cp -R "$WORK/published/." "$WORK/proposed/" || return 1
  CORRECTION_REL=$(place "$WORK/proposed" "$FIXTURES/valid-correction.json") || return 1
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

# ⭐ The harness's own three guards, exercised. Each must report the plant did
# not apply; a harness that reports one of these as applied is the harness that
# reports every guard above as failing to fire.
build_trees >/dev/null 2>&1
PROBE="$WORK/proposed/$PROFILE_REL"

if replace_once "$PROBE" "a literal this record does not carry" "x"; then
  fail "probe    an absent literal was reported as planted"
else
  pass "probe    an absent literal is refused"
fi

if replace_once "$PROBE" "sha256:" "x"; then
  fail "probe    an ambiguous literal was reported as planted"
else
  pass "probe    an ambiguous literal is refused"
fi

if replace_once "$PROBE" "Schema Fixture Client" "Schema Fixture Client"; then
  fail "probe    a no-op edit was reported as planted"
else
  pass "probe    a no-op edit is refused"
fi

if replace_once "$PROBE" "Schema Fixture Client
" "x"; then
  fail "probe    a multi-line literal was reported as planted"
else
  pass "probe    a multi-line literal is refused"
fi

TOTAL=$((PASS + FAIL))

if [ "$PASS" -eq 0 ]; then
  RC=1
elif [ "$FAIL" -gt 0 ]; then
  RC=1
else
  RC=0
fi

if [ "$JSON" = "1" ]; then
  printf '{"schema":"check-store/1","total":%s,"passed":%s,"failed":%s}\n' \
    "$TOTAL" "$PASS" "$FAIL"
  exit "$RC"
fi

printf '\n%s\n' "$ROWS"
printf '%s cases: %s passed, %s failed\n' "$TOTAL" "$PASS" "$FAIL"
if [ "$PASS" -eq 0 ]; then
  printf -- '❌ NOTHING RAN. Zero cases passed, so this is red whatever else it says.\n'
elif [ "$FAIL" -gt 0 ]; then
  printf -- '❌ a store guard did not refuse its defect.\n'
else
  printf -- '✅ every planted defect was refused, and the clean tree was not.\n'
fi
exit "$RC"
