#!/bin/sh
# check-source-route.sh - does the `source` route resolve its own tag, from the
# repository's own refs, and refuse every way a refs listing can be wrong?
#
# ⛔ WHAT THIS EXISTS FOR IS AN INDEPENDENCE THAT WAS NEVER THERE. Until
# 2026-09-09 the source lane was handed the tag the RELEASE lane's listing had
# chosen, so both lanes resolved through one index and `E-ACQ-07` called them one
# route - measured by assembling `capture-client` run 14, whose two resolution
# records differ only in their timestamps. The workflow argued for it: one
# resolver is how two lanes cannot land on different versions.
#
# ⭐ ABSOLUTE 4 ANSWERS THAT BETTER. Version equality is checked AFTER
# installation, on what each build reported when asked, so two resolutions that
# disagree are a vendor that moved between two reads and catching it is the
# correct outcome. This harness is what proves the second resolver exists.
#
# -- ⭐ IT RUNS OVER GENERATED LISTINGS AND SAYS SO ---------------------------
#
# `resolve-source --refs` makes the source a file, the way
# `resolve-release --listing` does. ⚠ The listings here are WRITTEN by this
# harness rather than recorded from a vendor, which is a departure from
# `check-release-route` and has a reason: an object name is forty hex digits,
# which is what `check-no-secrets --public` refuses because it is also the shape
# of a credential. `docs/security/secrets.md` says to narrow a pattern rather
# than switch a rule off, and computing the values needs no narrowing at all.
# ⛔ SO THIS PROVES THE READER AND THE ORDERING, and nothing about what any
# vendor tags today. The live measurement is a driven pass in `TODO/ci.md`.
#
# ⛔ NOTHING HERE PLANTS IN A TRACKED FILE. Every listing a case reads is written
# into scratch state, because a gate check that edits a tracked file leaves a
# dirty tree behind when it is interrupted.
#
# Usage:
#   sh scripts/acquisition/check-source-route.sh
#   sh scripts/acquisition/check-source-route.sh --json
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
      printf 'check-source-route: unknown argument: %s\n' "$1" >&2
      exit 2
      ;;
  esac
  shift
done

HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH='' cd -- "$HERE/.." && pwd)
ROOT=$(CDPATH='' cd -- "$ROOT/.." && pwd)

# shellcheck disable=SC2034
ME=check-source-route
# shellcheck disable=SC1091
# shellcheck source=scripts/corpus/store-lib.sh
. "$ROOT/scripts/corpus/store-lib.sh"

store_require cargo sha256sum
store_build "$ROOT" resolve-stable >/dev/null || exit 2

WORK=$(store_workdir checksource) || exit 2
trap 'rm -rf "$WORK"' EXIT INT TERM

RESOLVE="$ROOT/scripts/acquisition/resolve-source.sh"
[ -f "$RESOLVE" ] || {
  printf 'check-source-route: %s is missing\n' "$RESOLVE" >&2
  exit 2
}

# ⛔ COMPUTED, NEVER TYPED. See the header: forty hex digits is the shape the
# public secret rule refuses, and it is right to.
oid() { # seed
  printf '%s' "$1" | sha256sum | cut -c1-40
}

# One `git ls-remote --tags --refs` line.
ref_line() { # tag
  printf '%s\trefs/tags/%s\n' "$(oid "$1")" "$1"
}

# The adapter this harness resolves for: a stub declaring the same four fields
# every shipped adapter declares, so nothing here depends on a real target's
# tagging convention holding still.
STUB="$WORK/stub.sh"
cat >"$STUB" <<'STUB'
#!/bin/sh
set -u
case "${1:-}" in
  describe)
    printf 'target=fixture-client\n'
    printf 'kind=stock\n'
    printf 'release_repo=fixture-owner/fixture-client\n'
    printf "release_tag_prefix=${BIT_IDS_STUB_PREFIX-v}\n"
    printf "release_min_components=${BIT_IDS_STUB_MINC-3}\n"
    printf "release_max_components=${BIT_IDS_STUB_MAXC-3}\n"
    ;;
  *) exit 2 ;;
esac
STUB
chmod +x "$STUB"

# Runs the resolver over one refs listing, keeping the three streams apart.
# ⛔ The exit code is read from the process that produced it.
resolve() { # tag refs-file
  rm -rf "$WORK/$1.work"
  sh "$RESOLVE" --adapter "$STUB" --workdir "$WORK/$1.work" \
    --record "$WORK/$1.rec" --refs "$2" \
    >"$WORK/$1.out" 2>"$WORK/$1.err"
  return $?
}

said() { # tag literal
  grep -q -F -e "$2" "$WORK/$1.err" "$WORK/$1.out" 2>/dev/null
}

case_is() { # expected-rc literal name tag refs-file
  resolve "$4" "$5"
  _rc=$?
  if [ "$_rc" != "$1" ]; then
    fail "$3 (exit $_rc, expected $1)"
    [ "$JSON" = "1" ] || sed 's/^/          /' "$WORK/$4.err" | head -4
    return
  fi
  if [ -n "$2" ] && ! said "$4" "$2"; then
    fail "$3 (exit $1, and it did not say '$2')"
    [ "$JSON" = "1" ] || sed 's/^/          /' "$WORK/$4.err" | head -4
    return
  fi
  pass "$3"
}

# -- ⭐ THE CONTROL, WHICH EVERY REFUSAL BELOW NEEDS -------------------------
{
  ref_line v1.2.3
  ref_line v1.2.10
  ref_line v1.2.9
  ref_line v1.10.0
} >"$WORK/refs-good.txt"
case_is 0 "" "a refs listing resolves a tag" good "$WORK/refs-good.txt"
if [ "$(cat "$WORK/good.out" 2>/dev/null)" = "v1.10.0" ]; then
  pass "the newest tag is selected, and 1.10.0 beats 1.2.10 rather than sorting under it"
else
  fail "the newest tag is selected (got [$(cat "$WORK/good.out" 2>/dev/null)])"
fi
# ⛔ 1.2.10 OVER 1.2.9 IS THE ORDERING THIS PROJECT REFUSES TO GET WRONG, and a
# lexical sort answers the other way. The listing above puts them out of order on
# purpose so a reader that preserved input order would fail here.
if grep -q -F -e 'selected_version=1.10.0' "$WORK/good.rec" 2>/dev/null; then
  pass "the record names the version the tag carried, with the prefix stripped"
else
  fail "the record names the version the tag carried, with the prefix stripped"
fi

# ⛔ THE FIELD `assemble-capture` SLUGIFIES INTO A RESOLVER IDENTITY. If this
# named the releases endpoint, the two routes would slugify to one name and
# `E-ACQ-07` would call them one route - which is the defect this whole file
# exists to close, reappearing in the record it writes.
if grep -q -F -e 'source_url=https://recorded.invalid/' "$WORK/good.rec" 2>/dev/null; then
  pass "the record names the refs source it read rather than a releases endpoint"
else
  fail "the record names the refs source it read rather than a releases endpoint"
  [ "$JSON" = "1" ] || sed 's/^/          /' "$WORK/good.rec" | head -6
fi

# -- The refusals -----------------------------------------------------------
#
# ⛔ A LISTING THAT IS NOT A LISTING IS COULD-NOT-RUN, NEVER A REFUSAL, and these
# cases expected 1 until they were run. `resolve-stable` exits 2 when a reader
# cannot make candidates out of the bytes it was handed, which is the same
# verdict `check-release-route` asserts for a releases listing that is not JSON:
# nothing was judged, so nothing was refused. ⚠ Each case therefore asserts the
# REASON as well, because three of these share exit 2 and a case reading the code
# alone would pass when a different one fired - the defect `CI-03`'s Windows
# harness found in itself.

# ⛔ A PEELED REF IS REFUSED RATHER THAN SKIPPED. `ls-remote` without `--refs`
# emits a second `^{}` line per annotated tag; accepting it offers every such tag
# twice and skipping it hides a caller that forgot the flag.
{
  cat "$WORK/refs-good.txt"
  printf '%s\trefs/tags/v1.10.0^{}\n' "$(oid peeled)"
} >"$WORK/refs-peeled.txt"
case_is 2 "could not run" "a peeled ref is refused rather than counted twice" \
  peeled "$WORK/refs-peeled.txt"
said peeled "peeled ref" ||
  fail "the peeled-ref refusal names the peeled ref"
said peeled "peeled ref" &&
  pass "the peeled-ref refusal names the peeled ref"

# ⛔ A BRANCH IS NOT A TAG. `ls-remote` without `--tags` lists heads too, and a
# reader that took them would offer `main` as a version candidate.
{
  cat "$WORK/refs-good.txt"
  printf '%s\trefs/heads/main\n' "$(oid head)"
} >"$WORK/refs-head.txt"
case_is 2 "could not run" "a branch ref is refused" head "$WORK/refs-head.txt"
said head "is not a tag ref" ||
  fail "the branch-ref refusal names the ref"
said head "is not a tag ref" &&
  pass "the branch-ref refusal names the ref"

printf '%s refs/tags/v1.2.3\n' "$(oid space)" >"$WORK/refs-space.txt"
case_is 2 "could not run" "a line with no tab is refused" space "$WORK/refs-space.txt"

printf 'abc123\trefs/tags/v1.2.3\n' >"$WORK/refs-short.txt"
case_is 2 "could not run" "an abbreviated object name is refused" \
  short "$WORK/refs-short.txt"
said short "not a full object name" ||
  fail "the abbreviated-object refusal names the object"
said short "not a full object name" &&
  pass "the abbreviated-object refusal names the object"

: >"$WORK/refs-empty.txt"
case_is 1 "offered no tags at all" "an empty refs listing is refused" \
  empty "$WORK/refs-empty.txt"

# ⛔ THE ONE PRERELEASE SIGNAL A REFS SOURCE STILL HAS IS THE TAG TEXT, and a
# refs listing carries no `prerelease` flag at all. This is the case that says
# what survives: a tag whose version text no scheme can read blocks the
# resolution rather than being selected.
{
  cat "$WORK/refs-good.txt"
  ref_line v2.0.0beta1
} >"$WORK/refs-pre.txt"
case_is 1 "" "a tag no scheme can order blocks rather than being selected" \
  pre "$WORK/refs-pre.txt"

# ⛔ AND A MISSING DECLARATION IS COULD-NOT-RUN, never a resolution over
# defaults this file invented.
BIT_IDS_STUB_PREFIX='' case_is 2 "no release_tag_prefix" \
  "an adapter declaring no tag prefix cannot run" noprefix "$WORK/refs-good.txt"
BIT_IDS_STUB_MINC=x case_is 2 "non-numeric component count" \
  "a non-numeric component count cannot run" nonnum "$WORK/refs-good.txt"

store_report check-source-route/1 cases "$JSON"
