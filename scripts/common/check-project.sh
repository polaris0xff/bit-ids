#!/bin/sh
# Validate bit-ids-specific repository invariants.
# Usage: sh scripts/common/check-project.sh [--json]
# Exit codes: 0 clean, 1 invalid, 2 could not run.

set -u
JSON=0
case "${1:-}" in
  "") ;;
  --json) JSON=1 ;;
  *)
    printf 'check-project: unknown argument: %s\n' "$1" >&2
    exit 2
    ;;
esac

command -v git >/dev/null 2>&1 || {
  printf 'check-project: git not found\n' >&2
  exit 2
}
ROOT=$(git -C "$(dirname "$0")" rev-parse --show-toplevel 2>/dev/null) || {
  printf 'check-project: not in a git repository\n' >&2
  exit 2
}
cd "$ROOT" || exit 2

FAIL=0
say_fail() {
  FAIL=$((FAIL + 1))
  [ "$JSON" = 1 ] || printf 'FAIL: %s\n' "$1"
}

for path in README.md LICENSE Cargo.toml Cargo.lock catalogue/clients.toml \
  TODO/INDEX.md TODO/PROGRESS.md TODO/SUMMARY.md docs/AGENTS.md \
  docs/reference-sweeps/bit-cli.md .github/workflows/ci.yml; do
  [ -f "$path" ] || say_fail "missing $path"
done

for id in qbittorrent qbittorrent-enhanced utorrent bitcomet aria2 transmission \
  deluge bittorrent biglybt tixati ktorrent fdm zona libtorrent \
  anacrolix-torrent rqbit; do
  grep -Fq "id = \"$id\"" catalogue/clients.toml 2>/dev/null || say_fail "missing target $id"
  # shellcheck disable=SC2016
  MATRIX_NEEDLE=$(printf '| `%s` |' "$id")
  grep -Fq "$MATRIX_NEEDLE" docs/client-matrix.md 2>/dev/null ||
    say_fail "client matrix lacks target $id"
done

TMP="${TMPDIR:-/tmp}/.check-project.$$"
mkdir -p "$TMP" || exit 2
trap 'rm -rf "$TMP"' EXIT INT TERM

awk -F '|' '
  /^\| (FOUND|SCHEMA|OBS|ACQ|CLIENT|ENGINE|CORPUS|LIB|PUB|CI|DOC)-[0-9][0-9] / {
    id=$2; status=$5
    gsub(/^ +| +$/, "", id); gsub(/^ +| +$/, "", status)
    print id "|" status
  }
' TODO/INDEX.md | sort >"$TMP/index"

awk '
  /^## (FOUND|SCHEMA|OBS|ACQ|CLIENT|ENGINE|CORPUS|LIB|PUB|CI|DOC)-[0-9][0-9]:/ {
    id=$2; sub(/:$/, "", id); next
  }
  /^Priority: / && id != "" {
    status=$0; sub(/^.*Status: /, "", status)
    print id "|" status; id=""
  }
' TODO/*.md | sort >"$TMP/bodies"

ROWS=$(wc -l <"$TMP/index" | tr -d ' ')
OPEN_ROWS=$(awk -F '|' '$2 == "OPEN" { n++ } END { print n+0 }' "$TMP/index")
IN_PROGRESS_ROWS=$(awk -F '|' '$2 == "IN_PROGRESS" { n++ } END { print n+0 }' "$TMP/index")
BLOCKED_ROWS=$(awk -F '|' '$2 == "BLOCKED" { n++ } END { print n+0 }' "$TMP/index")
DONE_ROWS=$(awk -F '|' '$2 == "DONE" { n++ } END { print n+0 }' "$TMP/index")

INVALID_STATUS=$(awk -F '|' '$2 !~ /^(OPEN|IN_PROGRESS|BLOCKED|DONE)$/ { print $1 }' "$TMP/index")
[ -z "$INVALID_STATUS" ] || say_fail "TODO index has invalid statuses: $INVALID_STATUS"

[ "$(wc -l <"$TMP/bodies" | tr -d ' ')" -eq "$ROWS" ] ||
  say_fail "TODO body count does not match index count"
cmp -s "$TMP/index" "$TMP/bodies" ||
  say_fail "TODO IDs or statuses disagree between index and category bodies"
[ -z "$(cut -d '|' -f 1 "$TMP/index" | uniq -d)" ] ||
  say_fail "TODO index contains duplicate IDs"

get_declared() {
  awk -F ': ' -v key="$2" '$1 == key { print $2; exit }' "$1"
}
check_declared() {
  _file="$1"
  _key="$2"
  _actual="$3"
  _declared=$(get_declared "$_file" "$_key")
  [ "$_declared" = "$_actual" ] ||
    say_fail "$_file declares $_key=$_declared, computed $_actual"
}

for file in TODO/INDEX.md TODO/PROGRESS.md; do
  check_declared "$file" Total "$ROWS"
  check_declared "$file" Open "$OPEN_ROWS"
  check_declared "$file" "In progress" "$IN_PROGRESS_ROWS"
  check_declared "$file" Blocked "$BLOCKED_ROWS"
  check_declared "$file" Done "$DONE_ROWS"
done

SUMMARY_TOTAL=$(awk -F '|' '
  /^\| Total \|/ {
    for (i=4; i<=8; i++) { gsub(/^ +| +$/, "", $i) }
    print $4 "|" $5 "|" $6 "|" $7 "|" $8
  }
' TODO/SUMMARY.md)
EXPECTED_TOTAL="$OPEN_ROWS|$IN_PROGRESS_ROWS|$BLOCKED_ROWS|$DONE_ROWS|$ROWS"
[ "$SUMMARY_TOTAL" = "$EXPECTED_TOTAL" ] ||
  say_fail "TODO/SUMMARY.md total is $SUMMARY_TOTAL, computed $EXPECTED_TOTAL"

# ⛔ THE TOTAL ROW WAS THE ONLY ROW CHECKED, AND IT IS ONE OF TWELVE. The
# eleven category rows are derived from the same index and nothing compared
# them: setting Observer to 9 over ten open observer entries passed this check
# with exit 0. That is the "a value in two places with no check between them"
# row of docs/conventions/forbidden-patterns.md, inside the file whose job is
# holding the counts.
#
# ⭐ The mapping from a category to the identifiers it counts is declared in
# TODO/SUMMARY.md itself rather than here. A table of names in this script
# would be a second copy in each twin, and check-twins compares answers on this
# tree, so two mappings that differ only for a category the tree does not have
# would agree until the day one arrived.
#
# Both directions are checked. A prefix in the index with no row, and a row
# naming a prefix the index does not have, are both failures; otherwise a new
# category silently goes uncounted, which is the state this check exists to
# refuse. That pair is also what catches a row this parser skipped: a malformed
# count means the prefix never registers, so the missing-row arm fires.
#
# ⚠ A DATA ROW IS RECOGNISED BY ITS SHAPE, NOT BY ITS CASE. The first version
# matched `^\| [A-Z]` and the PowerShell twin let the `category` header through,
# because PowerShell's -match is case-insensitive and awk's bracket expression
# is not. One rule, two answers, from a regex that looked identical.
SUMMARY_BAD=$(awk -F '|' '
  function trim(s) { gsub(/^ +| +$/, "", s); return s }
  FILENAME ~ /INDEX\.md$/ &&
  /^\| (FOUND|SCHEMA|OBS|ACQ|CLIENT|ENGINE|CORPUS|LIB|PUB|CI|DOC)-[0-9][0-9] / {
    id = trim($2); status = trim($5)
    prefix = id; sub(/-[0-9][0-9]$/, "", prefix)
    count[prefix, status]++
    total[prefix]++
    known[prefix] = 1
    next
  }
  FILENAME ~ /SUMMARY\.md$/ && /^\|/ && NF >= 9 {
    prefix = trim($3); gsub(/`/, "", prefix)
    if (prefix == "") next
    for (i = 4; i <= 8; i++) if (trim($i) !~ /^[0-9]+$/) next
    if (seen[prefix]++) { print "duplicate row for " prefix; next }
    declared[prefix] = 1
    if (trim($4) + 0 != count[prefix, "OPEN"] + 0 ||
        trim($5) + 0 != count[prefix, "IN_PROGRESS"] + 0 ||
        trim($6) + 0 != count[prefix, "BLOCKED"] + 0 ||
        trim($7) + 0 != count[prefix, "DONE"] + 0 ||
        trim($8) + 0 != total[prefix] + 0) print prefix
  }
  END {
    for (p in known) if (!declared[p]) print p " has no row"
    for (p in declared) if (!known[p]) print p " names nothing in the index"
  }
' TODO/INDEX.md TODO/SUMMARY.md | sort)
[ -z "$SUMMARY_BAD" ] ||
  say_fail "TODO/SUMMARY.md category rows disagree: $(printf '%s' "$SUMMARY_BAD" | tr '\n' ' ')"

PRIORITY_BAD=$(awk -F '|' '
  function trim(s) { gsub(/^ +| +$/, "", s); return s }
  /^\| (FOUND|SCHEMA|OBS|ACQ|CLIENT|ENGINE|CORPUS|LIB|PUB|CI|DOC)-[0-9][0-9] / {
    p=trim($3); s=trim($5); count[p,s]++; total[p]++
  }
  /^\| P[0-2] \|/ {
    p=trim($2); declared[p,"OPEN"]=trim($3)+0
    declared[p,"IN_PROGRESS"]=trim($4)+0
    declared[p,"BLOCKED"]=trim($5)+0
    declared[p,"DONE"]=trim($6)+0
    declared[p,"TOTAL"]=trim($7)+0; seen[p]=1
  }
  END {
    split("P0 P1 P2", priorities, " ")
    for (i in priorities) {
      p=priorities[i]
      if (!seen[p] ||
          declared[p,"OPEN"] != count[p,"OPEN"] ||
          declared[p,"IN_PROGRESS"] != count[p,"IN_PROGRESS"] ||
          declared[p,"BLOCKED"] != count[p,"BLOCKED"] ||
          declared[p,"DONE"] != count[p,"DONE"] ||
          declared[p,"TOTAL"] != total[p]) print p
    }
  }
' TODO/INDEX.md)
[ -z "$PRIORITY_BAD" ] ||
  say_fail "TODO priority table disagrees for: $PRIORITY_BAD"

PY=$({
  git ls-files --others --exclude-standard '*.py'
  git ls-files '*.py'
} | sort -u)
[ -z "$PY" ] || say_fail "Python exists without an approved exception: $PY"

# ⛔ AN ALLOWLIST OF IMMUTABLE FORMS, NOT A DENYLIST OF FLOATING ONES.
# This used to name the floating refs it knew: main, master and vN.N.N. A
# branch called anything else, or an abbreviated commit, is just as mutable and
# passed. ⭐ Inverting it means a form nobody thought of fails closed.
#
# ⚠ THE VERSION COMMENT IS PART OF THE PIN, not decoration. A 40-hex ref alone
# is unreviewable: nobody can say what it is without a network call.
# check-remote-items.sh resolves that comment against the tag it names and
# refuses a pin whose comment has drifted, so a pin without one is a pin that
# check never examines.
#
# ⛔ THE SCOPE INCLUDES COMPOSITE ACTIONS, NOT WORKFLOWS ALONE. A composite
# action under .github/actions/ carries its own `uses:` lines and runs with the
# same permissions, so a rule that read only .github/workflows/ would be a gate
# on one of two doors into the same operation. There is no composite action
# here yet, and that is exactly when a scope is easiest to get wrong and
# hardest to notice.
PIN_OUT=""
for wf in .github/workflows/*.yml .github/workflows/*.yaml \
  .github/actions/*/action.yml .github/actions/*/action.yaml; do
  [ -f "$wf" ] || continue
  # ⚠ NO INTERVAL EXPRESSIONS. `{40}` is not portable across every awk this
  # repository runs on, so lengths are counted rather than matched.
  PIN_OUT="$PIN_OUT$(awk -v FILE="$wf" '
    function hex_only(s) { return s !~ /[^0-9a-f]/ }
    /^[[:space:]]*(-[[:space:]]+)?uses:[[:space:]]/ {
      ref = $0
      sub(/^[^:]*uses:[[:space:]]*/, "", ref)
      comment = ""
      if (match(ref, /[[:space:]]+#/)) {
        comment = substr(ref, RSTART)
        ref = substr(ref, 1, RSTART - 1)
      }
      gsub(/[[:space:]]+$/, "", ref)
      gsub(/^["'"'"']|["'"'"']$/, "", ref)

      # A local action is this repository, reviewed with everything else.
      if (ref ~ /^\.\//) next

      at = 0
      for (i = length(ref); i > 0; i--) {
        if (substr(ref, i, 1) == "@") { at = i; break }
      }
      if (at == 0) {
        printf "%s:%d carries no ref at all: %s\n", FILE, NR, ref
        next
      }
      pinned = substr(ref, at + 1)

      if (ref ~ /^docker:\/\//) {
        if (pinned !~ /^sha256:/) {
          printf "%s:%d container is not pinned to a digest: %s\n", FILE, NR, ref
          next
        }
        digest = substr(pinned, 8)
        if (length(digest) != 64 || !hex_only(digest)) {
          printf "%s:%d container digest is not a sha256: %s\n", FILE, NR, ref
        }
        next
      }

      if (length(pinned) != 40 || !hex_only(pinned)) {
        printf "%s:%d not pinned to a 40-character commit: %s\n", FILE, NR, ref
        next
      }
      if (comment !~ /#[[:space:]]*[^[:space:]]/) {
        printf "%s:%d pin carries no version comment, so nothing can check it: %s\n", FILE, NR, ref
      }
    }
  ' "$wf")"
done
[ -z "$PIN_OUT" ] || say_fail "workflow action pin: $PIN_OUT"

# ⛔ A DEPENDENCY THIS PROJECT DID NOT REVIEW CANNOT REACH THE OBSERVER OR THE
# PUBLISHER. Cargo.lock is the inventory: a package with no `source` is a
# member of this workspace, and every other one must come from the crates.io
# registry with a checksum. A git or path dependency appears here as some other
# source, so this one test covers the shape whatever the manifest said.
REGISTRY='registry+https://github.com/rust-lang/crates.io-index'
LOCK_OUT=$(awk -v REG="$REGISTRY" '
  function flush(   ) {
    # A package with no source is a member of this workspace, which is
    # reviewed like everything else in the tree.
    if (name != "" && source != "") {
      if (source != REG)
        printf "  %s is not from the crates.io registry: %s\n", name, source
      else if (!checksum)
        printf "  %s has no checksum\n", name
    }
    name = ""; source = ""; checksum = 0
  }
  /^\[\[package\]\]/ { flush(); next }
  /^name = "/     { name = $0; sub(/^name = "/, "", name); sub(/"$/, "", name); next }
  /^source = "/   { source = $0; sub(/^source = "/, "", source); sub(/"$/, "", source); next }
  /^checksum = "/ { checksum = 1; next }
  END { flush() }
' Cargo.lock)
[ -z "$LOCK_OUT" ] || say_fail "unreviewed dependency source:
$LOCK_OUT"

# ⛔ AN ACCEPTANCE COMMAND MUST NOT BE ABLE TO PASS OVER NOTHING. `cargo test`
# with a bare word selects by test NAME, and a filter matching none prints
# `running 0 tests` for every binary and exits 0. `OBS-01`'s Prove did exactly
# that and was read as an acceptance that passed; measured on 2026-09-05, ten
# more invocations in TODO/ were of the same shape and had only ever worked
# because every test function happened to begin with its file's name, which is a
# convention nothing held. CI-05 is the entry.
#
# ⛔ TWO SOURCES, ONE TOKENISER. An entry's `Prove` is the acceptance a person
# runs and the workflow's `run:` is the one every push runs, and a bare filter in
# either exits 0 over nothing. A rule on one of two doors into the same mistake
# is the shape docs/methodology/reviews.md names, so the extractors are separate
# and the judgement is not: they emit `file<TAB>line<TAB>command` and one reader
# decides.
#
# ⚠ SCOPED TO `Prove:` PARAGRAPHS IN TODO, and that is the whole rule rather
# than an exclusion list. A `Prove` is the live acceptance and must be runnable;
# a `Closure evidence` paragraph records what was actually run on a past tree and
# rewriting it would falsify the record, and the two entries that document this
# defect have to be able to quote the command that caused it. A rule that fired
# on those would be a rule somebody switches off.
#
# ⚠ A code span wraps across lines, so the paragraph is joined before the spans
# are found.
# ⚠ The awk programs below are single-quoted on purpose: `$0` and `$1` are awk's
# own field references and expanding them in the shell would hand awk an empty
# program. SC2016 exists to catch the opposite mistake and is right to fire on
# the shape; `check-placeholders.sh` and `mine-repo.sh` disable it for the same
# reason.
# shellcheck disable=SC2016
PROVE_OUT=$({
  git ls-files 'TODO/*.md' | xargs awk '
    function emit(   n, parts, k) {
      if (inprove && buf != "") {
        n = split(buf, parts, "`")
        for (k = 2; k <= n; k += 2)
          if (parts[k] ~ /^cargo[ \t]+test([ \t]|$)/)
            printf "%s\t%d\t%s\n", startfile, startline, parts[k]
      }
      inprove = 0; buf = ""
    }
    FNR == 1 { emit() }
    /^[ \t]*$/ { emit(); next }
    /^Prove:/ { emit(); inprove = 1; startline = FNR; startfile = FILENAME; buf = $0; next }
    { if (inprove) buf = buf " " $0 }
    END { emit() }
  '
  git ls-files '.github/workflows/*.yml' | xargs awk '
    # A commented-out command is not one every push runs.
    /^[ \t]*#/ { next }
    /cargo[ \t]+test/ {
      line = $0
      sub(/^.*cargo[ \t]+test/, "cargo test", line)
      sub(/[ \t]*\\[ \t]*$/, "", line)
      printf "%s\t%d\t%s\n", FILENAME, FNR, line
    }
  '
} | awk -F '\t' '
  {
    file = $1; line = $2; command = $3
    n = split(command, tok, /[ \t]+/)
    expect = 0
    for (i = 3; i <= n; i++) {
      t = tok[i]
      if (t == "") continue
      if (substr(t, 1, 1) == "-") {
        # A flag that takes a value consumes the next bare word, which is then a
        # target or a package rather than a name filter.
        if (index(t, "=") == 0 && t ~ /^(-p|-j|-F|--package|--exclude|--test|--bin|--example|--bench|--features|--target|--target-dir|--manifest-path|--profile|--jobs|--message-format|--color|--config|--test-threads|--skip)$/)
          expect = 1
        else
          expect = 0
        continue
      }
      if (expect) { expect = 0; continue }
      printf "  %s:%d selects tests by name, so it exits 0 over nothing: %s\n", file, line, command
      break
    }
  }
')
[ -z "$PROVE_OUT" ] || say_fail "an acceptance that can pass over nothing:
$PROVE_OUT"

# ⚠ The lockfile test above is the authority; this one fires earlier and names
# the manifest line, so the report points at the file somebody edited rather
# than at the file cargo generated.
GIT_DEP=$({
  git ls-files '*Cargo.toml'
  git ls-files --others --exclude-standard '*Cargo.toml'
} | sort -u | xargs grep -nE '(^|[{,][[:space:]]*)git[[:space:]]*=' 2>/dev/null || true)
[ -z "$GIT_DEP" ] || say_fail "git dependency in a manifest: $GIT_DEP"

if [ "$JSON" = 1 ]; then
  printf '{"schema":"check-project/2","failures":%s,"todo_entries":%s,"open":%s,"in_progress":%s,"blocked":%s,"done":%s}\n' \
    "$FAIL" "$ROWS" "$OPEN_ROWS" "$IN_PROGRESS_ROWS" "$BLOCKED_ROWS" "$DONE_ROWS"
elif [ "$FAIL" -eq 0 ]; then
  printf 'bit-ids project invariants pass (%s entries; %s open; %s in progress; %s blocked; %s done)\n' \
    "$ROWS" "$OPEN_ROWS" "$IN_PROGRESS_ROWS" "$BLOCKED_ROWS" "$DONE_ROWS"
fi
[ "$FAIL" -eq 0 ]
