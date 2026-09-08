#!/bin/sh
# check-access.sh - publish twice to a disposable bare repository, fetch each
# publication back, and check every documented access path against what came
# back.
#
# ⛔ THIS IS PUB-04'S PROVE, WITH ONE SUBSTITUTION THAT IS NAMED RATHER THAN
# HIDDEN. The Prove asks for a link checker that fetches every documented path
# through the approved GitHub read route and verifies its digest. Nothing has
# ever been published, so those URLs resolve to nothing and no fetch of them can
# say anything. A bare repository in a scratch directory is a real remote as far
# as git is concerned: the push runs, the fetch runs, and what is checked is the
# bytes that came back over a transport rather than the bytes left in a
# directory. What is NOT proved here is the GitHub URL form itself, and
# TODO/publishing.md carries that as a residual with the event that closes it.
#
# -- ⛔ THE IMMUTABILITY RULE IS THE ONE THAT NEEDS TWO PUBLICATIONS ---------
#
# "This path never changes" is not a property of one tree. It is a comparison
# between two, and a checker given one publication can only read a label off a
# document that asserted it. So this publishes a one-record catalogue, fetches
# it, publishes a two-record one, fetches that, and compares every path the
# first contract called immutable byte for byte across the two.
#
# ⚠ AND THE OTHER HALF, WHICH A NAIVE VERSION PASSES VACUOUSLY. A checker that
# only asserts "the immutable paths did not move" passes over a publication
# where nothing moved at all. At least one current path must have moved, and
# that is a case rather than an assumption.
#
# -- ⭐ THE DIGESTS ARE VERIFIED BY A READER THIS PROJECT DID NOT WRITE ------
#
# sha256sum recomputes every digest the contract states, over the fetched bytes.
# A contract compared against the manifest it was derived from agrees with
# itself.
#
# Usage:
#   sh scripts/publishing/check-access.sh
#   sh scripts/publishing/check-access.sh --json
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
      printf 'check-access: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

# ⛔ Resolved from this script's own location, never from the working directory.
HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH='' cd -- "$HERE/../.." && pwd)

# ⚠ ME is read by store-lib.sh, sourced on the next line. shellcheck cannot
# follow a source it was not handed, so the directive is per file.
# shellcheck disable=SC2034
ME=check-access
# shellcheck disable=SC1091
# shellcheck source=scripts/corpus/store-lib.sh
. "$ROOT/scripts/corpus/store-lib.sh"

store_require cargo sha256sum git tar python3
BUILDER=$(store_build "$ROOT" build-store) || exit 2
INDEXER=$(store_build "$ROOT" build-indexes) || exit 2
FORMATTER=$(store_build "$ROOT" build-formats) || exit 2
ASSEMBLER=$(store_build "$ROOT" assemble-release) || exit 2
CONTRACT=$(store_build "$ROOT" access-contract) || exit 2
PUBLISHER="$ROOT/scripts/publishing/publish-data.sh"
[ -f "$PUBLISHER" ] || {
  printf 'check-access: %s not found\n' "$PUBLISHER" >&2
  exit 2
}

WORK=$(store_workdir checkaccess) || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

BUNDLE="$WORK/bundle"
REMOTE="$WORK/remote"

# Rebuilds the derived half of a bundle over whatever records it holds.
# ⚠ The formats go beside the indexes because docs/publishing.md files them at
# formats/, and build-formats appends that segment itself.
reassemble() {
  rm -f "$BUNDLE/MANIFEST.json" "$BUNDLE/SHA256SUMS"
  mkdir -p "$BUNDLE/indexes/v1" || return 1
  "$INDEXER" --scheme fixture-client:-:3:3 "$BUNDLE" "$BUNDLE/indexes/v1/profiles.json" \
    >/dev/null 2>&1 || return 1
  "$FORMATTER" --scheme fixture-client:-:3:3 "$BUNDLE" "$BUNDLE" >/dev/null 2>&1 || return 1
  cp "$ROOT/LICENSE" "$BUNDLE/LICENSE" || return 1
  "$ASSEMBLER" "$BUNDLE" >/dev/null 2>&1
}

# Publishes the bundle and fetches the branch back into a directory, which is
# the transport this whole harness exists to put in the middle.
publish_and_fetch() { # destination
  sh "$PUBLISHER" --bundle "$BUNDLE" --remote "$REMOTE" >"$WORK/publish.log" 2>&1 || return 1
  rm -rf "$1"
  mkdir -p "$1" || return 1
  git -C "$REMOTE" archive data | tar -x -C "$1"
}

# ⛔ Unpiped. Output to a file, $? on the next line.
run_contract() { # tree out
  "$CONTRACT" "$1" "$2" >"$WORK/contract.out" 2>&1
  RC=$?
}

field() { # file python-expression
  python3 -c "
import json,sys
d=json.load(open(sys.argv[1]))
print($2)
" "$1" 2>/dev/null
}

rm -rf "$REMOTE" "$BUNDLE"
mkdir -p "$BUNDLE"
git init -q --bare "$REMOTE" || exit 2
if ! "$BUILDER" --version 1.2.3 "$BUNDLE" >/dev/null 2>&1 || ! reassemble; then
  printf 'check-access: cannot build the first bundle\n' >&2
  exit 2
fi

FIRST="$WORK/first"
if ! publish_and_fetch "$FIRST"; then
  printf 'check-access: the first publication did not go through\n' >&2
  sed 's/^/  /' "$WORK/publish.log" >&2
  exit 2
fi

run_contract "$FIRST" "$WORK/first.json"
if [ "$RC" = "0" ]; then
  pass "clean    a fetched publication derives a contract"
else
  fail "clean    the fetched tree was refused (exit $RC): $(head -3 "$WORK/contract.out" | tr '\n' ' ')"
fi

TOTAL=$(field "$WORK/first.json" "len(d['paths'])")
IMMUTABLE=$(field "$WORK/first.json" "sum(1 for p in d['paths'] if p['stability']=='immutable')")
CURRENT=$(field "$WORK/first.json" "sum(1 for p in d['paths'] if p['stability']=='current')")
# ⚠ Both classes must be non-empty. A contract that is all one class classified
# nothing, and every assertion below it would hold vacuously.
if [ "${IMMUTABLE:-0}" -gt 0 ] && [ "${CURRENT:-0}" -gt 0 ]; then
  pass "classes  $TOTAL path(s): $IMMUTABLE immutable and $CURRENT current, so neither class is empty"
else
  fail "classes  ${IMMUTABLE:-none} immutable and ${CURRENT:-none} current"
fi

# ⛔ EVERY DOCUMENTED PATH RESOLVES IN WHAT CAME BACK. This is the link check:
# a path in the contract that is not in the fetched tree is a documented URL
# that would 404.
MISSING=0
for p in $(field "$WORK/first.json" "'\n'.join(x['path'] for x in d['paths'])"); do
  [ -f "$FIRST/$p" ] || MISSING=$((MISSING + 1))
done
if [ "$MISSING" = "0" ]; then
  pass "resolve  every documented path exists in the fetched branch"
else
  fail "resolve  $MISSING documented path(s) are not in the fetched branch"
fi

# ⭐ AND EVERY DIGEST IS RECOMPUTED BY sha256sum OVER THE FETCHED BYTES.
field "$WORK/first.json" \
  "'\n'.join(x['sha256'].split(':')[1] + '  ' + x['path'] for x in d['paths'])" \
  >"$WORK/first.sums"
if (cd "$FIRST" && sha256sum -c "$WORK/first.sums" --quiet) >"$WORK/sums.log" 2>&1; then
  pass "digests  sha256sum agrees with every digest the contract states"
else
  fail "digests  a stated digest does not match the fetched bytes: $(head -2 "$WORK/sums.log" | tr '\n' ' ')"
fi

# ⛔ THE SECOND PUBLICATION, AND THE COMPARISON THAT NEEDS IT.
SECOND="$WORK/second"
if ! "$BUILDER" --version 1.2.10 "$BUNDLE" >/dev/null 2>&1 || ! reassemble; then
  printf 'check-access: cannot build the second bundle\n' >&2
  exit 2
fi
if ! publish_and_fetch "$SECOND"; then
  fail "append   the second publication did not go through"
else
  run_contract "$SECOND" "$WORK/second.json"
  if [ "$RC" = "0" ]; then
    pass "append   a second publication derives a contract too"
  else
    fail "append   the second fetched tree was refused (exit $RC)"
  fi

  MOVED=0
  GONE=0
  for p in $(field "$WORK/first.json" \
    "'\n'.join(x['path'] for x in d['paths'] if x['stability']=='immutable')"); do
    if [ ! -f "$SECOND/$p" ]; then
      GONE=$((GONE + 1))
    elif ! cmp -s "$FIRST/$p" "$SECOND/$p"; then
      MOVED=$((MOVED + 1))
    fi
  done
  if [ "$MOVED" = "0" ] && [ "$GONE" = "0" ]; then
    pass "immutable  every immutable path survived the second publication unchanged"
  else
    fail "immutable  $MOVED path(s) changed and $GONE disappeared"
  fi

  # ⚠ THE NON-VACUOUS HALF. If nothing at all changed, the case above proves
  # only that the second publication was the first one.
  CHANGED=0
  for p in $(field "$WORK/first.json" \
    "'\n'.join(x['path'] for x in d['paths'] if x['stability']=='current')"); do
    if [ -f "$SECOND/$p" ] && ! cmp -s "$FIRST/$p" "$SECOND/$p"; then
      CHANGED=$((CHANGED + 1))
    fi
  done
  if [ "$CHANGED" -gt 0 ]; then
    pass "current  $CHANGED current path(s) moved, so the comparison had something to see"
  else
    fail "current  no current path moved between two different publications"
  fi

  # ⚠ And the second publication carries strictly more immutable paths, because
  # a record was appended. A contract that did not grow would mean the append
  # published nothing.
  FIRST_IMM=$(field "$WORK/first.json" "sum(1 for p in d['paths'] if p['stability']=='immutable')")
  SECOND_IMM=$(field "$WORK/second.json" "sum(1 for p in d['paths'] if p['stability']=='immutable')")
  if [ "${SECOND_IMM:-0}" -gt "${FIRST_IMM:-0}" ]; then
    pass "growth   the appended record added immutable paths ($FIRST_IMM to $SECOND_IMM)"
  else
    fail "growth   immutable paths went $FIRST_IMM to ${SECOND_IMM:-none}"
  fi
fi

# -- The refusals, planted one at a time into a copy of the fetched tree ------
#
# ⛔ Each plant is against a tree that came back over the transport, so what is
# refused is a publication a consumer could really be handed.

PLANT="$WORK/plant"
replant() {
  rm -rf "$PLANT"
  cp -r "$FIRST" "$PLANT"
}

# A file whose bytes moved after publication. The manifest still describes the
# old ones, and re-deriving is what notices.
replant
RECORD=$(find "$PLANT/profiles" -name '*.json' | LC_ALL=C sort | head -1)
if [ -n "$RECORD" ] && printf '\n' >>"$RECORD"; then
  run_contract "$PLANT" "$WORK/plant.json"
  if [ "$RC" = "1" ]; then
    pass "E-tampered  a record whose bytes moved after publication is refused"
  else
    fail "E-tampered  expected exit 1, got $RC"
  fi
else
  fail "E-tampered  could not plant"
fi

# A published file the manifest never described.
replant
if printf 'x\n' >"$PLANT/formats/bit-ids-v1.extra.json"; then
  run_contract "$PLANT" "$WORK/plant.json"
  if [ "$RC" = "1" ]; then
    pass "E-undescribed  a file nobody described is refused"
  else
    fail "E-undescribed  expected exit 1, got $RC"
  fi
else
  fail "E-undescribed  could not plant"
fi

# ⛔ E-ACC-01, AND IT IS THE LIVE ONE. docs/publishing.md documented routes/ and
# nothing writes it. A path this build cannot classify has no stability a
# consumer can act on, so it blocks rather than being published with a guess.
replant
mkdir -p "$PLANT/routes/v1/fixture-client/latest/linux"
if printf '{}\n' >"$PLANT/routes/v1/fixture-client/latest/linux/x86-64.json"; then
  run_contract "$PLANT" "$WORK/plant.json"
  if [ "$RC" = "1" ]; then
    pass "E-ACC-01  a path this build cannot classify blocks the contract"
  else
    fail "E-ACC-01  expected exit 1, got $RC"
  fi
else
  fail "E-ACC-01  could not plant"
fi

# The manifest itself removed: a consumer has nothing to check anything against.
replant
if rm -f "$PLANT/MANIFEST.json"; then
  run_contract "$PLANT" "$WORK/plant.json"
  if [ "$RC" = "1" ]; then
    pass "E-ACC-02  a publication with no manifest is refused"
  else
    fail "E-ACC-02  expected exit 1, got $RC"
  fi
else
  fail "E-ACC-02  could not plant"
fi

# And the checksum file, which is the one document nothing else covers.
replant
if rm -f "$PLANT/SHA256SUMS"; then
  run_contract "$PLANT" "$WORK/plant.json"
  if [ "$RC" = "1" ]; then
    pass "E-ACC-02  a publication with no checksum file is refused"
  else
    fail "E-ACC-02  expected exit 1, got $RC"
  fi
else
  fail "E-ACC-02  could not plant"
fi

# ⚠ The clean control on either side, because every plant above rests on the
# unplanted tree being accepted.
replant
run_contract "$PLANT" "$WORK/plant.json"
if [ "$RC" = "0" ]; then
  pass "control  the restored copy of the fetched tree is accepted"
else
  fail "control  the restored copy was refused (exit $RC)"
fi

# ⚠ THE HARNESS'S OWN READER, CHECKED. Every comparison above rests on `field`
# answering from the document, and a reader that answered nothing would turn
# each one into a mismatch that reads as a failing guard.
if [ "$(field "$WORK/first.json" "d['schema']")" = "bit-ids/access/1" ]; then
  pass "probe    the harness reader answers from the contract"
else
  fail "probe    the harness reader cannot read the contract it just wrote"
fi
if [ -z "$(field "$WORK/first.json" "d['no-such-key']")" ]; then
  pass "probe    and answers nothing for a key the contract does not carry"
else
  fail "probe    the harness reader invented a value"
fi

store_report check-access/1 cases "$JSON"
