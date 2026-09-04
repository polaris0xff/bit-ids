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
    for (i=3; i<=7; i++) { gsub(/^ +| +$/, "", $i) }
    print $3 "|" $4 "|" $5 "|" $6 "|" $7
  }
' TODO/SUMMARY.md)
EXPECTED_TOTAL="$OPEN_ROWS|$IN_PROGRESS_ROWS|$BLOCKED_ROWS|$DONE_ROWS|$ROWS"
[ "$SUMMARY_TOTAL" = "$EXPECTED_TOTAL" ] ||
  say_fail "TODO/SUMMARY.md total is $SUMMARY_TOTAL, computed $EXPECTED_TOTAL"

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

FLOATING=$(grep -R -nE 'uses: +[^ ]+@(main|master|v[0-9]+([.]([0-9]+))*)([[:space:]]+#.*)?[[:space:]]*$' .github/workflows 2>/dev/null || true)
[ -z "$FLOATING" ] || say_fail "workflow action is not pinned to an immutable commit: $FLOATING"

if [ "$JSON" = 1 ]; then
  printf '{"schema":"check-project/2","failures":%s,"todo_entries":%s,"open":%s,"in_progress":%s,"blocked":%s,"done":%s}\n' \
    "$FAIL" "$ROWS" "$OPEN_ROWS" "$IN_PROGRESS_ROWS" "$BLOCKED_ROWS" "$DONE_ROWS"
elif [ "$FAIL" -eq 0 ]; then
  printf 'bit-ids project invariants pass (%s entries; %s open; %s in progress; %s blocked; %s done)\n' \
    "$ROWS" "$OPEN_ROWS" "$IN_PROGRESS_ROWS" "$BLOCKED_ROWS" "$DONE_ROWS"
fi
[ "$FAIL" -eq 0 ]
