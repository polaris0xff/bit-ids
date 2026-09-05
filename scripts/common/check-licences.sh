#!/bin/sh
# check-licences.sh - does every target and every dependency have a disposition,
# and does this repository carry an artifact it may not redistribute?
#
# `FOUND-04` owns the register in catalogue/licences.toml. The defect this
# exists to catch is a target or a package acquiring a licence position by
# nobody's decision: the row is absent, the field is empty, or the row has
# drifted from the catalogue and the lockfile it is supposed to describe.
#
# -- ⛔ unverified IS A DISPOSITION AND NOT A GAP ---------------------------
#
# Six of the nine GitHub-hosted targets answer NOASSERTION when their licence
# endpoint is asked, which means a detector could not name one. Writing an SPDX
# identifier there anyway would be inventing the one kind of fact this project
# is most careful about. `unverified` records that nobody has established it,
# and the redistribution rule is refused regardless, so the register never
# reads as permission.
#
# -- ⛔ BOTH DIRECTIONS, ALWAYS ---------------------------------------------
#
# A row nothing names and a name nothing has a row for are different defects
# and each is silent on its own. A register checked in one direction grows
# stale rows that look like coverage.
#
# ⚠ THE DEPENDENCY ROWS CARRY A VERSION AND IT IS COMPARED. A licence claim
# about a package is a claim about a version of it, so a row that stayed behind
# when the lockfile moved is a claim about something nobody depends on.
#
# Usage:
#   sh scripts/common/check-licences.sh
#   sh scripts/common/check-licences.sh --json
#   sh scripts/common/check-licences.sh --permitted   # the ids that may be kept
#
# Exit codes: 0 clean, 1 invalid, 2 could not run.
#
# ⛔ Read the exit code from this process, unpiped.

set -u

JSON=0
PERMITTED=0
case "${1:-}" in
  "") ;;
  --json) JSON=1 ;;
  # ⭐ THE REGISTER HAS ONE PARSER AND THIS IS IT. ACQ-05's cache has to know
  # which targets may have their bytes kept, and a second reader of this file
  # would be a second answer to what it permits. --permitted prints the ids and
  # nothing else, so a caller can ask rather than parse.
  --permitted) PERMITTED=1 ;;
  -h | --help)
    awk 'NR>1 { if (/^#/) { sub(/^# ?/, ""); print } else exit }' "$0"
    exit 0
    ;;
  *)
    printf 'check-licences: unknown argument: %s\n' "$1" >&2
    exit 2
    ;;
esac

command -v git >/dev/null 2>&1 || {
  printf 'check-licences: git not found\n' >&2
  exit 2
}
ROOT=$(git -C "$(dirname "$0")" rev-parse --show-toplevel 2>/dev/null) || {
  printf 'check-licences: not in a git repository\n' >&2
  exit 2
}
cd "$ROOT" || exit 2

REGISTER=catalogue/licences.toml
CATALOGUE=catalogue/clients.toml
LOCK=Cargo.lock
for path in "$REGISTER" "$CATALOGUE" "$LOCK"; do
  [ -f "$path" ] || {
    printf 'check-licences: %s is missing\n' "$path" >&2
    exit 2
  }
done

FAIL=0
say_fail() {
  FAIL=$((FAIL + 1))
  [ "$JSON" = 1 ] || printf 'FAIL: %s\n' "$1"
}

TMP="${TMPDIR:-/tmp}/.check-licences.$$"
mkdir -p "$TMP" || exit 2
trap 'rm -rf "$TMP"' EXIT INT TERM

# ⚠ The register's shape is line-based on purpose: a key per line inside a
# block, which is what both twins can parse identically. A TOML library in one
# half and a hand parser in the other is two answers to one question.
# ⚠ A block is closed by the NEXT header or by the end of file, never by its own
# last key, so a register whose final block is the only one of its kind still
# produces a row. Getting that wrong loses exactly one row, at the end, which is
# the position a reader checks last.
awk -v T="$TMP/reg-targets" -v D="$TMP/reg-deps" '
  function flush() {
    if (section == "target" && id != "")
      printf "%s\t%s\t%s\t%s\t%s\n", id, lic, src, red, (note == "" ? "-" : note) > T
    else if (section == "dep" && id != "")
      printf "%s\t%s\t%s\t%s\t%s\t%s\n", id, ver, lic, src, red, (note == "" ? "-" : note) > D
    id = ""; ver = ""; lic = ""; src = ""; red = ""; note = ""
  }
  function value(line,   v) { v = line; sub(/^[a-z_]+ = "/, "", v); sub(/"$/, "", v); return v }
  /^\[\[targets\]\]/      { flush(); section = "target"; next }
  /^\[\[dependencies\]\]/ { flush(); section = "dep"; next }
  /^id = "/               { id  = value($0); next }
  /^name = "/             { id  = value($0); next }
  /^version = "/          { ver = value($0); next }
  /^licence = "/          { lic = value($0); next }
  /^licence_source = "/   { src = value($0); next }
  /^redistribute = "/     { red = value($0); next }
  /^notice = "/           { note = value($0); next }
  END { flush() }
' "$REGISTER"

sort -o "$TMP/reg-targets" "$TMP/reg-targets" 2>/dev/null || : >"$TMP/reg-targets"
sort -o "$TMP/reg-deps" "$TMP/reg-deps" 2>/dev/null || : >"$TMP/reg-deps"

# ⚠ Reported before any rule runs, because a caller asking what is permitted is
# asking about the file as written rather than about whether it is coherent.
if [ "$PERMITTED" = 1 ]; then
  awk -F '\t' '$4 == "permitted" { print $1 }' "$TMP/reg-targets"
  exit 0
fi

grep -q '^schema = "bit-ids/licences/1"$' "$REGISTER" ||
  say_fail "$REGISTER does not declare the licences schema"

# -- 1. every catalogue target has exactly one row, and the reverse -----------
awk '/^id = "/ { v = $0; sub(/^id = "/, "", v); sub(/"$/, "", v); print v }' \
  "$CATALOGUE" | sort >"$TMP/catalogue-ids"
cut -f 1 "$TMP/reg-targets" | sort >"$TMP/register-ids"

ONLY_CATALOGUE=$(comm -23 "$TMP/catalogue-ids" "$TMP/register-ids" | tr '\n' ' ')
ONLY_REGISTER=$(comm -13 "$TMP/catalogue-ids" "$TMP/register-ids" | tr '\n' ' ')
[ -z "$ONLY_CATALOGUE" ] || say_fail "catalogue targets with no register row: $ONLY_CATALOGUE"
[ -z "$ONLY_REGISTER" ] || say_fail "register rows naming no catalogue target: $ONLY_REGISTER"
DUPLICATE=$(cut -f 1 "$TMP/reg-targets" | sort | uniq -d | tr '\n' ' ')
[ -z "$DUPLICATE" ] || say_fail "register carries a target more than once: $DUPLICATE"

# -- 2. every third-party package has exactly one row, at its version ---------
#
# ⚠ A package with no source is a member of this workspace and is not third
# party, which is the same rule check-project applies to the same file.
awk '
  /^\[\[package\]\]/ { name = ""; version = ""; source = 0 }
  /^name = "/    { name = $0; sub(/^name = "/, "", name); sub(/"$/, "", name) }
  /^version = "/ { version = $0; sub(/^version = "/, "", version); sub(/"$/, "", version) }
  /^source = "/  { source = 1 }
  /^checksum = "/ { if (source) printf "%s\t%s\n", name, version }
' "$LOCK" | sort >"$TMP/lock-deps"
cut -f 1,2 "$TMP/reg-deps" | sort >"$TMP/register-deps"

ONLY_LOCK=$(comm -23 "$TMP/lock-deps" "$TMP/register-deps" | tr '\t' ' ' | tr '\n' ';')
ONLY_REG=$(comm -13 "$TMP/lock-deps" "$TMP/register-deps" | tr '\t' ' ' | tr '\n' ';')
[ -z "$ONLY_LOCK" ] || say_fail "locked packages with no register row: $ONLY_LOCK"
[ -z "$ONLY_REG" ] || say_fail "register rows naming no locked package: $ONLY_REG"

# -- 3. every row carries a disposition --------------------------------------
BAD=$(awk -F '\t' '
  $2 == "" || $2 == "-" { print $1 " has no licence"; next }
  $4 != "refused" && $4 != "permitted" { print $1 " has an unknown redistribute value: " $4 }
' "$TMP/reg-targets")
BAD="$BAD$(awk -F '\t' '
  $3 == "" || $3 == "-" { print $1 " has no licence"; next }
  $5 != "refused" && $5 != "permitted" { print $1 " has an unknown redistribute value: " $5 }
' "$TMP/reg-deps")"
[ -z "$BAD" ] || say_fail "register rows with no disposition: $(printf '%s' "$BAD" | tr '\n' ';')"

# -- 4. permitted is the expensive value and it has to be earned -------------
#
# ⛔ Redistribution needs a licence somebody established and a notice to carry
# with it. `unverified` plus `permitted` is the combination that would publish
# somebody's bytes on nobody's authority.
EARNED=$(awk -F '\t' '$4 == "permitted" && ($2 == "unverified" || $5 == "-") { print $1 }' "$TMP/reg-targets")
EARNED="$EARNED$(awk -F '\t' '$5 == "permitted" && ($3 == "unverified" || $6 == "-") { print $1 }' "$TMP/reg-deps")"
[ -z "$EARNED" ] ||
  say_fail "permitted without a verified licence and a notice: $(printf '%s' "$EARNED" | tr '\n' ' ')"

# -- 5. a closed-source target is never recorded under an open licence -------
#
# ⚠ The catalogue already says which targets are closed source, so this compares
# two files rather than restating one of them.
awk '
  /^\[\[targets\]\]/ { id = ""; open = "" }
  /^id = "/ { id = $0; sub(/^id = "/, "", id); sub(/"$/, "", id) }
  /^open_source = / { open = $0; sub(/^open_source = /, "", open); if (open == "false" && id != "") print id }
' "$CATALOGUE" | sort >"$TMP/closed"
MISLABELLED=$(while read -r id; do
  [ -n "$id" ] || continue
  lic=$(awk -F '\t' -v want="$id" '$1 == want { print $2 }' "$TMP/reg-targets")
  case "$lic" in
    proprietary | unverified | "") ;;
    *) printf '%s is closed source and recorded as %s\n' "$id" "$lic" ;;
  esac
done <"$TMP/closed")
[ -z "$MISLABELLED" ] ||
  say_fail "closed-source targets under an open licence: $(printf '%s' "$MISLABELLED" | tr '\n' ';')"

# -- 6. no artifact this repository may not redistribute is tracked ----------
#
# ⛔ THE HALF A REGISTER CANNOT ANSWER BY ITSELF. Every row above says the bytes
# are never shipped; this is what checks that none are here. An installer that
# arrived in a commit is exactly the thing a licence register exists to prevent
# and exactly the thing no amount of recording prevents.
BUNDLED=$({
  git ls-files
  git ls-files --others --exclude-standard
} | sort -u | grep -Ei '\.(exe|msi|dmg|pkg|deb|rpm|appimage|apk|jar|7z|zip|xz|bz2|gz|tgz|torrent|dll|so|dylib)$' || true)
[ -z "$BUNDLED" ] ||
  say_fail "tracked artifact this repository may not redistribute: $(printf '%s' "$BUNDLED" | tr '\n' ' ')"

# ⛔ wc, NOT grep -c. `grep -c .` prints 0 and EXITS 1 on an empty file, so a
# `|| printf 0` fallback fires on the very case that matters and the variable
# becomes two zeroes on two lines. The comparison below then said "Illegal
# number" and the guard did not run at all: the check that exists to refuse a
# register of nothing was itself disabled by an empty register. Found by
# comparing the two halves per planted mutation rather than on a clean tree,
# which is the only place it was visible.
TARGET_ROWS=$(wc -l <"$TMP/reg-targets" | tr -d ' ')
DEP_ROWS=$(wc -l <"$TMP/reg-deps" | tr -d ' ')

# ⛔ A REGISTER OF NOTHING IS NOT A CLEAN REGISTER. Every rule above is
# satisfied by two empty lists, which is the shape this whole check exists to
# refuse, and it is how a broken parser reports success.
if [ "$TARGET_ROWS" -eq 0 ] || [ "$DEP_ROWS" -eq 0 ]; then
  say_fail "the register parsed to $TARGET_ROWS target row(s) and $DEP_ROWS dependency row(s)"
fi

if [ "$JSON" = 1 ]; then
  printf '{"schema":"check-licences/1","failures":%s,"targets":%s,"dependencies":%s}\n' \
    "$FAIL" "$TARGET_ROWS" "$DEP_ROWS"
elif [ "$FAIL" -eq 0 ]; then
  printf 'licence register: %s target(s) and %s dependency row(s), every one with a disposition\n' \
    "$TARGET_ROWS" "$DEP_ROWS"
fi
[ "$FAIL" -eq 0 ]
